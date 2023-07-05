use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use derivative::Derivative;
use fixed::types::I80F48;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::program::invoke_signed;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::accounts_ix::Serum3Side;
use crate::accounts_ix::TriggerCheck;
use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use crate::PerpMarketIndex;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Triggers {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub account: Pubkey,

    pub next_trigger_id: u64,

    // TODO: do we care about fast lookup? how much CU does traversing the linked list cost?
    // pub orders: [(u64, u64); 255],
    pub reserved: [u8; 960],
}
const_assert_eq!(size_of::<Triggers>(), 32 * 2 + 8 + 960);
const_assert_eq!(size_of::<Triggers>(), 1032);
const_assert_eq!(size_of::<Triggers>() % 8, 0);

impl Triggers {
    // TODO: distinguish error from not-found?
    // NOTE: Currently iterating over 100 non-matching triggers costs around 3000 CU,
    // so around 30 CU per trigger.
    pub fn find_trigger_offset_by_id(bytes: &[u8], trigger_id: u64) -> Result<usize> {
        let mut offset = 8 + std::mem::size_of::<Triggers>();
        loop {
            // TODO: if empty, return not found?!
            let trigger = Trigger::from_bytes(&bytes[offset..])?;
            if trigger.id == trigger_id {
                return Ok(offset);
            }
            offset += trigger.total_bytes as usize;
        }
    }
}

#[account(zero_copy)]
#[derive(Debug)]
pub struct Trigger {
    pub total_bytes: u32,
    pub version: u32, // for future use, currently always 0

    // so ix have a stable reference to a trigger even if its bytes are moved
    pub id: u64,

    // Note that condition_bytes + action_bytes <= total_bytes, because there
    // may be trailing bytes after the action for alignment.
    // TODO: probably should also allow alignment bytes on the condition? maybe just align_of(condition_bytes)?
    pub condition_bytes: u32,
    pub action_bytes: u32,

    pub expiry_slot: u64,

    // Paid both on check and on execute. Stored in triggers account lamport balance.
    pub incentive_lamports: u64,

    // Did TriggerCheck ever succeed? (means the check incentive was already paid out)
    pub condition_was_met: u8,

    pub reserved: [u8; 127],
    // Condition and Action bytes trail this struct in the account!
}
const_assert_eq!(size_of::<Trigger>(), 4 * 2 + 8 + 4 * 2 + 8 * 2 + 1 + 127);
const_assert_eq!(size_of::<Trigger>(), 168);
const_assert_eq!(size_of::<Trigger>() % 8, 0);

impl Trigger {
    /// Reads a single trigger, condition and action
    // TODO: rename? move?
    // ignores trailing bytes
    pub fn all_from_bytes<'a>(
        bytes: &'a [u8],
        trigger_offset: usize,
    ) -> Result<(&Triggers, &Trigger, ConditionRef<'a>, ActionRef<'a>)> {
        let triggers_size = std::mem::size_of::<Triggers>();
        let trigger_start = 8 + triggers_size;
        let trigger_size = std::mem::size_of::<Trigger>();
        require_gte!(bytes.len(), trigger_offset + trigger_size);

        let (triggers_bytes, tail_bytes) = (&bytes[8..]).split_at(triggers_size);
        let triggers: &Triggers = bytemuck::from_bytes(&triggers_bytes);

        let (trigger_bytes, tail_bytes) =
            (&tail_bytes[trigger_offset - trigger_start..]).split_at(trigger_size);
        let trigger: &Trigger = bytemuck::from_bytes(&trigger_bytes);

        require_gte!(
            tail_bytes.len(),
            trigger.total_bytes as usize - trigger_size
        );

        let (condition_bytes, tail_bytes) =
            tail_bytes.split_at(trigger.condition_bytes.try_into().unwrap());

        let condition = ConditionRef::from_bytes(condition_bytes)?;
        let action = ActionRef::from_bytes(&tail_bytes[..trigger.action_bytes as usize])?;

        Ok((triggers, trigger, condition, action))
    }

    // ignores trailing bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<&Trigger> {
        let trigger_size = std::mem::size_of::<Trigger>();
        require_gte!(bytes.len(), trigger_size);
        Ok(bytemuck::from_bytes(&bytes[..trigger_size]))
    }

    // ignores trailing bytes
    pub fn from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Trigger> {
        let trigger_size = std::mem::size_of::<Trigger>();
        require_gte!(bytes.len(), trigger_size);
        Ok(bytemuck::from_bytes_mut(&mut bytes[..trigger_size]))
    }
}

#[repr(u32)]
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
pub enum ConditionType {
    OraclePrice,
}

#[derive(Debug)]
pub enum ConditionRef<'a> {
    OraclePrice(&'a OraclePriceCondition),
}

impl<'a> ConditionRef<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<ConditionRef<'a>> {
        let raw_type: u32 = *bytemuck::from_bytes(&bytes[0..4]);
        let condition_type: ConditionType = raw_type.try_into().unwrap();
        match condition_type {
            ConditionType::OraclePrice => {
                require_eq!(bytes.len(), size_of::<OraclePriceCondition>());
                Ok(ConditionRef::OraclePrice(bytemuck::from_bytes(bytes)))
            }
        }
    }

    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        match self {
            ConditionRef::OraclePrice(c) => c.check(accounts),
        }
    }

    // TODO: This function must move to a place closer to the client; a similar function for
    // actions needs too much context knowledge...
    pub fn accounts(&self) -> Vec<AccountMeta> {
        match self {
            ConditionRef::OraclePrice(op) => vec![
                AccountMeta {
                    pubkey: op.base_oracle,
                    is_writable: false,
                    is_signer: false,
                },
                AccountMeta {
                    pubkey: op.quote_oracle,
                    is_writable: false,
                    is_signer: false,
                },
            ],
        }
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct OraclePriceCondition {
    pub condition_type: u32, // always ConditionType::OraclePrice
    #[derivative(Debug = "ignore")]
    pub padding0: u32,
    pub base_oracle: Pubkey,
    pub quote_oracle: Pubkey,
    pub base_max_staleness_slots: i64, // negative means: don't check
    pub quote_max_staleness_slots: i64, // negative means: don't check
    pub base_conf_filter: f32,
    pub quote_conf_filter: f32,
    pub threshold_ui: f64,
    pub trigger_when_above: u8, // TODO: don't use a bool
    #[derivative(Debug = "ignore")]
    pub padding: [u8; 7],
}
const_assert_eq!(
    size_of::<OraclePriceCondition>(),
    4 + 4 + 32 * 2 + 8 * 2 + 4 * 2 + 8 + 1 + 7
);
const_assert_eq!(size_of::<OraclePriceCondition>(), 112);
const_assert_eq!(size_of::<OraclePriceCondition>() % 8, 0);

impl OraclePriceCondition {
    // TODO: should this distinguish a (possibly permanent) error from a temporary failure (due to staleness, price being wrong, etc)?
    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        require_eq!(accounts.len(), 2);
        let base_oracle_ai = &accounts[0];
        require_keys_eq!(*base_oracle_ai.key, self.base_oracle);
        let quote_oracle_ai = &accounts[1];
        require_keys_eq!(*quote_oracle_ai.key, self.quote_oracle);

        let base_config = OracleConfig {
            conf_filter: I80F48::from_num(self.base_conf_filter),
            max_staleness_slots: self.base_max_staleness_slots,
            ..Default::default()
        };
        let quote_config = OracleConfig {
            conf_filter: I80F48::from_num(self.quote_conf_filter),
            max_staleness_slots: self.quote_max_staleness_slots,
            ..Default::default()
        };
        let slot = Clock::get()?.slot;
        // Using this as decimals produces the ui price
        let decimals = QUOTE_DECIMALS.try_into().unwrap();

        let (base_price_ui, _) = oracle_price_and_state(
            &AccountInfoRef::borrow(base_oracle_ai)?,
            &base_config,
            decimals,
            Some(slot),
        )?;
        let (quote_price_ui, _) = oracle_price_and_state(
            &AccountInfoRef::borrow(quote_oracle_ai)?,
            &quote_config,
            decimals,
            Some(slot),
        )?;
        let price_ui = base_price_ui.to_num::<f64>() / quote_price_ui.to_num::<f64>();
        let price_good = if self.trigger_when_above != 0 {
            price_ui >= self.threshold_ui
        } else {
            price_ui <= self.threshold_ui
        };

        require_msg!(price_good, "price not yet over/under threshold"); // TODO: bad error

        Ok(())
    }
}

#[repr(u32)]
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
pub enum ActionType {
    PerpCpi,
    Serum3Cpi,
}

#[derive(Debug)]
pub enum ActionRef<'a> {
    PerpCpi((&'a PerpCpiAction, &'a [u8])),
    Serum3Cpi((&'a Serum3CpiAction, &'a [u8])),
}

impl<'a> ActionRef<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<ActionRef<'a>> {
        let raw_type: u32 = *bytemuck::from_bytes(&bytes[0..4]);
        let action_type: ActionType = raw_type.try_into().unwrap();
        match action_type {
            ActionType::PerpCpi => {
                require_gte!(bytes.len(), size_of::<PerpCpiAction>());
                let (action_data, ix_data) = bytes.split_at(size_of::<PerpCpiAction>());
                let action: &PerpCpiAction = bytemuck::from_bytes(action_data);
                Ok(ActionRef::PerpCpi((action, ix_data)))
            }
            ActionType::Serum3Cpi => {
                require_gte!(bytes.len(), size_of::<Serum3CpiAction>());
                let (action_data, ix_data) = bytes.split_at(size_of::<Serum3CpiAction>());
                let action: &Serum3CpiAction = bytemuck::from_bytes(action_data);
                Ok(ActionRef::Serum3Cpi((action, ix_data)))
            }
        }
    }

    pub fn check(&self) -> Result<()> {
        match self {
            ActionRef::PerpCpi((action, ix_data)) => action.check(ix_data),
            ActionRef::Serum3Cpi((action, ix_data)) => action.check(ix_data),
        }
    }

    pub fn execute<'info>(
        &self,
        triggers: &Triggers,
        execute_ix: &TriggerCheck<'info>,
        accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        match self {
            ActionRef::PerpCpi((action, ix_data)) => {
                action.execute(triggers, execute_ix, accounts, ix_data)
            }
            ActionRef::Serum3Cpi((action, ix_data)) => {
                action.execute(triggers, execute_ix, accounts, ix_data)
            }
        }
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerpCpiAction {
    pub action_type: u32, // always ActionType::PerpCpiAction
    pub perp_market_index: PerpMarketIndex,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 58],
}
const_assert_eq!(size_of::<PerpCpiAction>(), 4 + 2 + 58);
const_assert_eq!(size_of::<PerpCpiAction>(), 64);
const_assert_eq!(size_of::<PerpCpiAction>() % 8, 0);

impl PerpCpiAction {
    pub fn accounts(
        &self,
        group: Pubkey,
        account: Pubkey,
        ix_data: &[u8],
        perp_market: Pubkey,
        bids: Pubkey,
        asks: Pubkey,
        event_queue: Pubkey,
        oracle: Pubkey,
    ) -> Result<Vec<AccountMeta>> {
        require_gte!(ix_data.len(), 8);
        let discr: [u8; 8] = ix_data[..8].try_into().unwrap();

        use crate::instruction as ixs;
        let mut ams = vec![
            // To be able to call into ourselves, the program needs to be passed explicitly
            AccountMeta {
                pubkey: crate::id(),
                is_writable: false,
                is_signer: false,
            },
        ];

        if discr == ixs::PerpPlaceOrder::DISCRIMINATOR
            || discr == ixs::PerpPlaceOrderV2::DISCRIMINATOR
            || discr == ixs::PerpPlaceOrderPegged::DISCRIMINATOR
            || discr == ixs::PerpPlaceOrderPeggedV2::DISCRIMINATOR
        {
            ams.extend(
                crate::accounts::PerpPlaceOrder {
                    group,
                    account,
                    owner: account,
                    perp_market,
                    bids,
                    asks,
                    event_queue,
                    oracle,
                }
                .to_account_metas(None)
                .into_iter(),
            );
        } else if discr == ixs::PerpCancelAllOrders::DISCRIMINATOR {
            ams.extend(
                crate::accounts::PerpCancelAllOrders {
                    group,
                    account,
                    owner: account,
                    perp_market,
                    bids,
                    asks,
                }
                .to_account_metas(None)
                .into_iter(),
            );
        } else if discr == ixs::PerpCancelAllOrdersBySide::DISCRIMINATOR {
            ams.extend(
                crate::accounts::PerpCancelAllOrdersBySide {
                    group,
                    account,
                    owner: account,
                    perp_market,
                    bids,
                    asks,
                }
                .to_account_metas(None)
                .into_iter(),
            );
        } else {
            return Err(error_msg!("bad perp cpi action discriminator"));
        }

        // The signer flag for the owner must not be set: it'll be toggled by the trigger execution ix
        ams[3].is_signer = false;

        // TODO: Add health accounts too!

        Ok(ams)
    }

    pub fn check(&self, ix_data: &[u8]) -> Result<()> {
        require_gte!(ix_data.len(), 8);

        // Whitelist of acceptable instructions
        use crate::instruction as ixs;
        let allowed_inner_ix = [
            ixs::PerpCancelAllOrders::discriminator(),
            ixs::PerpCancelAllOrdersBySide::discriminator(),
            ixs::PerpPlaceOrder::discriminator(),
            ixs::PerpPlaceOrderV2::discriminator(),
            ixs::PerpPlaceOrderPegged::discriminator(),
            ixs::PerpPlaceOrderPeggedV2::discriminator(),
        ];
        let ix_discriminator: [u8; 8] = ix_data[..8].try_into().unwrap();
        require!(
            allowed_inner_ix.contains(&ix_discriminator),
            MangoError::SomeError
        );
        Ok(())
    }

    pub fn execute<'info>(
        &self,
        triggers: &Triggers,
        outer_accounts: &TriggerCheck<'info>,
        account_infos: &[AccountInfo<'info>],
        ix_data: &[u8],
    ) -> Result<()> {
        self.check(ix_data)?;

        let mango_program = crate::id();

        // Skip the accountinfo for our program, which is passed first
        let passed_accounts = &account_infos[1..];

        // Verify that the accounts passed to the inner instruction are safe:
        // - same group as the trigger
        // - same account as the trigger
        // - the right owner account
        // - the desired perp market
        struct AccountIndexes {
            group: usize,
            account: usize,
            owner: usize,
            perp_market: usize,
        }
        // Currently all instructions have the accounts in the same order, so this
        // can be a constant instead of depending on the instruction discriminator.
        let account_indexes = AccountIndexes {
            group: 0,
            account: 1,
            owner: 2,
            perp_market: 3,
        };

        require_keys_eq!(passed_accounts[account_indexes.group].key(), triggers.group);
        require_keys_eq!(
            passed_accounts[account_indexes.account].key(),
            triggers.account
        );
        // To let the inner instruction know it's authorized, we pass the account itself
        // as the owner and sign for it.
        require_keys_eq!(
            passed_accounts[account_indexes.owner].key(),
            triggers.account
        );

        {
            let perp_market_loader = AccountLoader::<PerpMarket>::try_from(
                &passed_accounts[account_indexes.perp_market],
            )?;
            let perp_market = perp_market_loader.load()?;
            require_eq!(perp_market.perp_market_index, self.perp_market_index);
        }

        // Prepare and execute the cpi call, forwarding the accounts

        let account_metas = passed_accounts
            .iter()
            .enumerate()
            .map(|(i, ai)| {
                AccountMeta {
                    pubkey: ai.key(),
                    is_writable: ai.is_writable,
                    // We call the inner instruction without being able to provide the true `owner`.
                    // Instead the account is passed as owner and this ix signs for the PDA, which is something
                    // no user is able to do.
                    is_signer: ai.is_signer || i == account_indexes.owner,
                }
            })
            .collect();

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::id(),
            accounts: account_metas,
            data: ix_data.to_vec(),
        };

        // Prepare the MangoAccount seeds, so we can sign for it
        let (owner, account_num, bump) = {
            let loader = AccountLoader::<MangoAccountFixed>::try_from(
                &passed_accounts[account_indexes.account],
            )?;
            let account = loader.load()?;
            (account.owner, account.account_num, account.bump)
        };
        let account_seeds = &[
            b"MangoAccount".as_ref(),
            triggers.group.as_ref(),
            owner.as_ref(),
            &account_num.to_le_bytes(),
            &[bump],
        ];

        invoke_signed(&instruction, account_infos, &[account_seeds])?;

        Ok(())
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Serum3CpiAction {
    pub action_type: u32, // always ActionType::Serum3CpiAction
    pub serum3_market: Pubkey,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 60],
}
const_assert_eq!(size_of::<Serum3CpiAction>(), 4 + 32 + 60);
const_assert_eq!(size_of::<Serum3CpiAction>(), 96);
const_assert_eq!(size_of::<Serum3CpiAction>() % 8, 0);

impl Serum3CpiAction {
    // TODO: the number of pubkeys passed here is incredibly ugly, at least make a struct
    pub fn accounts(
        &self,
        group: Pubkey,
        account: Pubkey,
        ix_data: &[u8],
        open_orders: Pubkey,
        serum_market: Pubkey,
        serum_program: Pubkey,
        serum_market_external: Pubkey,
        market_bids: Pubkey,
        market_asks: Pubkey,
        market_event_queue: Pubkey,
        market_request_queue: Pubkey,
        market_base_vault: Pubkey,
        market_quote_vault: Pubkey,
        market_vault_signer: Pubkey,
        quote_bank: Pubkey,
        quote_vault: Pubkey,
        quote_oracle: Pubkey,
        base_bank: Pubkey,
        base_vault: Pubkey,
        base_oracle: Pubkey,
    ) -> Result<Vec<AccountMeta>> {
        require_gte!(ix_data.len(), 8);
        let discr: [u8; 8] = ix_data[..8].try_into().unwrap();

        use crate::instruction as ixs;
        let mut ams = vec![
            // To be able to call into ourselves, the program needs to be passed explicitly
            AccountMeta {
                pubkey: crate::id(),
                is_writable: false,
                is_signer: false,
            },
        ];

        // TODO: support more instructions, like cancel all etc
        if discr == ixs::Serum3PlaceOrder::DISCRIMINATOR {
            let ix = ixs::Serum3PlaceOrder::deserialize(&mut &ix_data[8..])?;
            let (payer_bank, payer_vault, payer_oracle) = match ix.side {
                Serum3Side::Bid => (quote_bank, quote_vault, quote_oracle),
                Serum3Side::Ask => (base_bank, base_vault, base_oracle),
            };
            ams.extend(
                crate::accounts::Serum3PlaceOrder {
                    group,
                    account,
                    owner: account,
                    open_orders,
                    serum_market,
                    serum_program,
                    serum_market_external,
                    market_bids,
                    market_asks,
                    market_request_queue,
                    market_event_queue,
                    market_base_vault,
                    market_quote_vault,
                    market_vault_signer,
                    payer_bank,
                    payer_vault,
                    payer_oracle,
                    token_program: anchor_spl::token::ID,
                }
                .to_account_metas(None)
                .into_iter(),
            );
        } else {
            return Err(error_msg!("bad serum3 cpi action discriminator"));
        }

        // The signer flag for the owner must not be set: it'll be toggled by the trigger execution ix
        ams[3].is_signer = false;

        // TODO: Add health accounts too!

        Ok(ams)
    }

    pub fn check(&self, ix_data: &[u8]) -> Result<()> {
        require_gte!(ix_data.len(), 8);

        // Whitelist of acceptable instructions
        use crate::instruction as ixs;
        let allowed_inner_ix = [
            // TODO: support more
            ixs::Serum3PlaceOrder::discriminator(),
        ];
        let ix_discriminator: [u8; 8] = ix_data[..8].try_into().unwrap();
        require!(
            allowed_inner_ix.contains(&ix_discriminator),
            MangoError::SomeError
        );
        Ok(())
    }

    pub fn execute<'info>(
        &self,
        triggers: &Triggers,
        outer_accounts: &TriggerCheck<'info>,
        account_infos: &[AccountInfo<'info>],
        ix_data: &[u8],
    ) -> Result<()> {
        self.check(ix_data)?;

        let mango_program = crate::id();

        // Skip the accountinfo for our program, which is passed first
        let passed_accounts = &account_infos[1..];

        // Verify that the accounts passed to the inner instruction are safe:
        // - same group as the trigger
        // - same account as the trigger
        // - the right owner account
        // - the desired serum market
        // All other accounts are essentially derived from these.
        struct AccountIndexes {
            group: usize,
            account: usize,
            owner: usize,
            serum_market: usize,
        }
        // Currently all instructions have the accounts in the same order, so this
        // can be a constant instead of depending on the instruction discriminator.
        let account_indexes = AccountIndexes {
            group: 0,
            account: 1,
            owner: 2,
            serum_market: 4,
        };

        require_keys_eq!(passed_accounts[account_indexes.group].key(), triggers.group);
        require_keys_eq!(
            passed_accounts[account_indexes.account].key(),
            triggers.account
        );
        // To let the inner instruction know it's authorized, we pass the account itself
        // as the owner and sign for it.
        require_keys_eq!(
            passed_accounts[account_indexes.owner].key(),
            triggers.account
        );
        require_keys_eq!(
            passed_accounts[account_indexes.serum_market].key(),
            self.serum3_market
        );

        // Prepare and execute the cpi call, forwarding the accounts

        let account_metas = passed_accounts
            .iter()
            .enumerate()
            .map(|(i, ai)| {
                AccountMeta {
                    pubkey: ai.key(),
                    is_writable: ai.is_writable,
                    // We call the inner instruction without being able to provide the true `owner`.
                    // Instead the account is passed as owner and this ix signs for the PDA, which is something
                    // no user is able to do.
                    is_signer: ai.is_signer || i == account_indexes.owner,
                }
            })
            .collect();

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::id(),
            accounts: account_metas,
            data: ix_data.to_vec(),
        };

        // Prepare the MangoAccount seeds, so we can sign for it
        let (owner, account_num, bump) = {
            let loader = AccountLoader::<MangoAccountFixed>::try_from(
                &passed_accounts[account_indexes.account],
            )?;
            let account = loader.load()?;
            (account.owner, account.account_num, account.bump)
        };
        let account_seeds = &[
            b"MangoAccount".as_ref(),
            triggers.group.as_ref(),
            owner.as_ref(),
            &account_num.to_le_bytes(),
            &[bump],
        ];

        invoke_signed(&instruction, account_infos, &[account_seeds])?;

        Ok(())
    }
}

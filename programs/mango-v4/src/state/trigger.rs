use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use derivative::Derivative;
use fixed::types::I80F48;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::program::invoke_signed;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::accounts_ix::TriggerCheck;
use crate::error::*;
use crate::state::*;
use crate::PerpMarketIndex;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Trigger {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub account: Pubkey,

    pub removed: Pubkey,

    // For recreating the seed, just in case
    pub trigger_num: u64,
    // TODO: bump too then?
    pub expiry_slot: u64,

    // Paid on check and on execute
    pub incentive_lamports: u64,

    pub condition_type: u32,
    pub condition_bytes: u32,
    pub action_type: u32,
    pub action_bytes: u32,

    pub condition_was_met: u8,

    pub reserved: [u8; 999],
    // Condition and Action bytes trail this struct in the account!
}
const_assert_eq!(size_of::<Trigger>(), 32 * 3 + 8 * 3 + 4 * 4 + 1 + 999);
const_assert_eq!(size_of::<Trigger>(), 1136);
const_assert_eq!(size_of::<Trigger>() % 8, 0);

impl Trigger {
    // TODO: check owner, doesn't work with Ref<>
    pub fn from_account_bytes<'a>(
        bytes: &'a [u8],
    ) -> Result<(&Trigger, ConditionRef<'a>, ActionRef<'a>)> {
        if bytes.len() < 8 {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let disc_bytes = &bytes[0..8];
        if disc_bytes != &Trigger::DISCRIMINATOR {
            return Err(ErrorCode::AccountDiscriminatorMismatch.into());
        }

        let (fixed_bytes, tail_bytes) = bytes.split_at(8 + std::mem::size_of::<Trigger>());
        let fixed: &Trigger = bytemuck::from_bytes(&fixed_bytes[8..]);

        let (condition_bytes, action_bytes) =
            tail_bytes.split_at(fixed.condition_bytes.try_into().unwrap());
        require_eq!(action_bytes.len(), fixed.action_bytes as usize);

        let condition_type: u32 = *bytemuck::from_bytes(&condition_bytes[0..4]);
        require_eq!(condition_type, fixed.condition_type);
        let condition = ConditionRef::from_bytes(condition_bytes)?;

        let action_type: u32 = *bytemuck::from_bytes(&action_bytes[0..4]);
        require_eq!(action_type, fixed.action_type);
        let action = ActionRef::from_bytes(action_bytes)?;

        Ok((fixed, condition, action))
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
            ConditionRef::OraclePrice(op) => vec![AccountMeta {
                pubkey: op.oracle,
                is_writable: false,
                is_signer: false,
            }],
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
    pub oracle: Pubkey,
    pub threshold: I80F48,
    pub trigger_when_above: u8, // TODO: don't use a bool
    // TODO: also oracle staleness and confidence
    #[derivative(Debug = "ignore")]
    pub padding: [u8; 7],
}

impl OraclePriceCondition {
    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        require_eq!(accounts.len(), 1);
        let oracle_ai = &accounts[0];
        require_keys_eq!(*oracle_ai.key, self.oracle);

        // TODO: grab the price and compare it

        Ok(())
    }
}

#[repr(u32)]
#[derive(Clone, Copy, TryFromPrimitive, IntoPrimitive)]
pub enum ActionType {
    PerpCpi,
}

#[derive(Debug)]
pub enum ActionRef<'a> {
    PerpCpi((&'a PerpCpiAction, &'a [u8])),
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
        }
    }

    pub fn check(&self) -> Result<()> {
        match self {
            ActionRef::PerpCpi((action, ix_data)) => action.check(ix_data),
        }
    }

    pub fn execute<'info>(
        &self,
        trigger: &Trigger,
        execute_ix: &TriggerCheck<'info>,
        accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        match self {
            ActionRef::PerpCpi((action, ix_data)) => {
                action.execute(trigger, execute_ix, accounts, ix_data)
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
        trigger: &Trigger,
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
                    group: trigger.group,
                    account: trigger.account,
                    owner: trigger.account,
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
                    group: trigger.group,
                    account: trigger.account,
                    owner: trigger.account,
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
                    group: trigger.group,
                    account: trigger.account,
                    owner: trigger.account,
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
        trigger: &Trigger,
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

        require_keys_eq!(passed_accounts[account_indexes.group].key(), trigger.group);
        require_keys_eq!(
            passed_accounts[account_indexes.account].key(),
            trigger.account
        );
        // To let the inner instruction know it's authorized, we pass the account itself
        // as the owner and sign for it.
        require_keys_eq!(
            passed_accounts[account_indexes.owner].key(),
            trigger.account
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
            trigger.group.as_ref(),
            owner.as_ref(),
            &account_num.to_le_bytes(),
            &[bump],
        ];

        invoke_signed(&instruction, account_infos, &[account_seeds])?;

        Ok(())
    }
}

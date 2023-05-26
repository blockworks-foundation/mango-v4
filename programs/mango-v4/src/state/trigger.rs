use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use derivative::Derivative;
use fixed::types::I80F48;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::program::invoke_signed;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::accounts_ix::PerpPlaceOrderAccountInfos;
use crate::accounts_ix::PerpPlaceOrderAccounts;
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

    pub owner: Pubkey,

    // For recreating the seed
    // TODO: needed?
    pub trigger_num: u64,

    pub expiry_slot: u64,

    pub incentive_lamports: u64,

    // TODO: expiry?
    // TODO: incentive (just SOL on this account maybe?)
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
                Ok(ConditionRef::OraclePrice(bytemuck::from_bytes(bytes)))
            }
        }
    }

    pub fn check(&self, accounts: &[AccountInfo]) -> Result<()> {
        match self {
            ConditionRef::OraclePrice(c) => c.check(accounts),
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
    PerpPlaceOrder,
    MangoCpi,
}

#[derive(Debug)]
pub enum ActionRef<'a> {
    PerpPlaceOrder(&'a PerpPlaceOrderAction),
    MangoCpi(&'a [u8]),
}

impl<'a> ActionRef<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<ActionRef<'a>> {
        let raw_type: u32 = *bytemuck::from_bytes(&bytes[0..4]);
        let action_type: ActionType = raw_type.try_into().unwrap();
        match action_type {
            ActionType::PerpPlaceOrder => {
                Ok(ActionRef::PerpPlaceOrder(bytemuck::from_bytes(bytes)))
            }
            ActionType::MangoCpi => Ok(ActionRef::MangoCpi(bytes)),
        }
    }

    pub fn execute<'info>(
        &self,
        trigger: &Trigger,
        execute_ix: &TriggerCheck<'info>,
        accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        match self {
            ActionRef::PerpPlaceOrder(p) => p.execute(trigger, execute_ix, accounts),
            ActionRef::MangoCpi(p) => execute_mango_cpi(trigger, execute_ix, accounts, p),
        }
    }
}

#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerpPlaceOrderAction {
    pub action_type: u32, // always ActionType::PerpPlaceOrder
    // TODO: all these fields are just slapdash
    pub max_oracle_staleness_slots: i32,
    pub max_base_lots: i64,
    pub max_quote_lots: i64,
    pub client_order_id: u64,
    pub price_lots_or_offset: i64,
    pub peg_limit: i64,
    pub perp_market_index: PerpMarketIndex,
    pub side: u8,
    pub reduce_only: u8,
    pub self_trade_behavior: u8,
    pub place_order_type: u8,
    pub is_pegged: u8,
    pub limit: u8,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 64],
}
const_assert_eq!(
    size_of::<PerpPlaceOrderAction>(),
    4 * 2 + 8 * 5 + 2 + 1 * 5 + 1 + 64
);
const_assert_eq!(size_of::<PerpPlaceOrderAction>(), 120);
const_assert_eq!(size_of::<PerpPlaceOrderAction>() % 8, 0);

impl PerpPlaceOrderAction {
    fn to_order(&self) -> Result<Order> {
        let place_order_type = PlaceOrderType::try_from(self.place_order_type).unwrap();
        let params = match place_order_type {
            PlaceOrderType::Market => {
                require_eq!(self.is_pegged, 0);
                OrderParams::Market
            }
            PlaceOrderType::ImmediateOrCancel => {
                require_eq!(self.is_pegged, 0);
                OrderParams::ImmediateOrCancel {
                    price_lots: self.price_lots_or_offset,
                }
            }
            _ => {
                let order_type = place_order_type.to_post_order_type()?;
                if self.is_pegged == 0 {
                    OrderParams::Fixed {
                        price_lots: self.price_lots_or_offset,
                        order_type,
                    }
                } else {
                    OrderParams::OraclePegged {
                        price_offset_lots: self.price_lots_or_offset,
                        order_type,
                        peg_limit: self.peg_limit,
                        max_oracle_staleness_slots: self.max_oracle_staleness_slots,
                    }
                }
            }
        };
        Ok(Order {
            side: Side::try_from(self.side).unwrap(),
            max_base_lots: self.max_base_lots,
            max_quote_lots: self.max_quote_lots,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only != 0,
            time_in_force: 0,
            self_trade_behavior: SelfTradeBehavior::try_from(self.self_trade_behavior).unwrap(),
            params,
        })
    }

    pub fn execute<'info>(
        &self,
        trigger: &Trigger,
        execute_ix: &TriggerCheck<'info>,
        accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        // Use the accounts and validate them
        let perp_place_order_ais = PerpPlaceOrderAccountInfos {
            group: execute_ix.group.as_ref(),
            account: &accounts[0],
            perp_market: &accounts[1],
            bids: &accounts[2],
            asks: &accounts[3],
            event_queue: &accounts[4],
            oracle: &accounts[5],
            remaining: &accounts[6..],
        };
        let perp_place_order_accounts =
            PerpPlaceOrderAccounts::from_account_infos(perp_place_order_ais)?;
        perp_place_order_accounts.validate(&trigger.owner)?;

        require_eq!(
            perp_place_order_accounts
                .perp_market
                .load()?
                .perp_market_index,
            self.perp_market_index
        );

        let order = self.to_order()?;

        // TODO: probably move this whole set of functions into GPL?
        #[cfg(feature = "enable-gpl")]
        return crate::instructions::perp_place_order(perp_place_order_accounts, order, self.limit)
            .map(|_| ());

        #[cfg(not(feature = "enable-gpl"))]
        Err(error_msg!("not gpl"))
    }
}

fn execute_mango_cpi<'info>(
    trigger: &Trigger,
    outer_accounts: &TriggerCheck<'info>,
    account_infos: &[AccountInfo<'info>],
    ix_data: &[u8],
) -> Result<()> {
    use solana_program::instruction::Instruction;
    let mango_program = crate::id();

    // Whitelist of acceptable instructions
    let allowed_inner_ix = [
        crate::instruction::PerpCancelAllOrders::discriminator(),
        crate::instruction::PerpCancelAllOrdersBySide::discriminator(),
        crate::instruction::PerpPlaceOrder::discriminator(),
        crate::instruction::PerpPlaceOrderV2::discriminator(),
        crate::instruction::PerpPlaceOrderPegged::discriminator(),
        crate::instruction::PerpPlaceOrderPeggedV2::discriminator(),
        crate::instruction::Serum3CancelAllOrders::discriminator(),
        crate::instruction::Serum3PlaceOrder::discriminator(),
    ];
    let ix_discriminator: [u8; 8] = ix_data[..8].try_into().unwrap();
    require!(
        allowed_inner_ix.contains(&ix_discriminator),
        MangoError::SomeError
    );

    let mut account_metas = Vec::with_capacity(account_infos.len());
    for ai in account_infos {
        let pubkey = ai.key();
        let mut is_signer = ai.is_signer;

        if ai.owner == &mango_program {
            let discriminator: [u8; 8] = ai.try_borrow_data()?[..8].try_into().unwrap();
            if discriminator == Group::DISCRIMINATOR {
                // Allowing execution for foreign groups could be a security issue.
                require_keys_eq!(pubkey, outer_accounts.group.key());
                // We call instructions without being able to provide the true `owner`.
                // Instead we pass the group as owner and sign for it, which is something
                // no user is able to do.
                is_signer = true;
            }
            if discriminator == MangoAccount::DISCRIMINATOR {
                // Since we provide a generic `owner` for the cpi call, it's essential that
                // we only allow interaction with the account that the trigger order should be for.
                // TODO: do we need to check if the trigger.owner has the rights for trigger.account?
                // do delegates add trigger orders to the delegated account's Trigger account? probably.
                require_keys_eq!(pubkey, trigger.account);
            }
        }

        account_metas.push(AccountMeta {
            pubkey,
            is_writable: ai.is_writable,
            is_signer,
        });
    }

    let instruction = Instruction {
        program_id: crate::id(),
        accounts: account_metas,
        data: ix_data.to_vec(),
    };

    let group = outer_accounts.group.load()?;
    let group_seeds = group_seeds!(group);
    invoke_signed(&instruction, account_infos, &[group_seeds])?;

    Ok(())
}

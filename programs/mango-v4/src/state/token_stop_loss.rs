use anchor_lang::prelude::*;

use derivative::Derivative;
use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::cmp::Ordering;
use std::mem::size_of;

use crate::i80f48::ClampToInt;
use crate::state::*;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, bytemuck::Pod)]
#[derivative(Debug)]
pub struct TokenStopLoss {
    pub is_active: u8,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 127],
}

const_assert_eq!(size_of::<TokenStopLoss>(), 1 + 127);
const_assert_eq!(size_of::<TokenStopLoss>(), 128);
const_assert_eq!(size_of::<TokenStopLoss>() % 8, 0);

impl Default for TokenStopLoss {
    fn default() -> Self {
        Self {
            is_active: 0,
            reserved: [0; 127],
        }
    }
}

impl TokenStopLoss {
    pub fn is_active(&self) -> bool {
        self.is_active == 1
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = u8::from(active);
    }
}

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
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 128],
}

const_assert_eq!(size_of::<TokenStopLoss>(), 128);
const_assert_eq!(size_of::<TokenStopLoss>(), 128);
const_assert_eq!(size_of::<TokenStopLoss>() % 8, 0);

impl Default for TokenStopLoss {
    fn default() -> Self {
        Self { reserved: [0; 128] }
    }
}

impl TokenStopLoss {}

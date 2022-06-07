use crate::error::MangoError;
use anchor_lang::prelude::*;

#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}
#[allow(unused_imports)]
pub(crate) use zip;

#[macro_export]
macro_rules! checked_math {
    ($x: expr) => {
        checked_math::checked_math!($x).unwrap_or_else(|| panic!("math error"))
    };
}
pub(crate) use checked_math;

pub fn fill16_from_str(name: String) -> Result<[u8; 16]> {
    let name_bytes = name.as_bytes();
    require!(name_bytes.len() < 16, MangoError::SomeError);
    let mut name_ = [0u8; 16];
    name_[..name_bytes.len()].copy_from_slice(name_bytes);
    Ok(name_)
}

pub fn fill32_from_str(name: String) -> Result<[u8; 32]> {
    let name_bytes = name.as_bytes();
    require!(name_bytes.len() < 32, MangoError::SomeError);
    let mut name_ = [0u8; 32];
    name_[..name_bytes.len()].copy_from_slice(name_bytes);
    Ok(name_)
}

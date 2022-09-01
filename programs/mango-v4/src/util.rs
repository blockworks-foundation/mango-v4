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

pub fn fill_from_str<const N: usize>(name: &str) -> Result<[u8; N]> {
    let name_bytes = name.as_bytes();
    require!(name_bytes.len() < N, MangoError::SomeError);
    let mut name_ = [0u8; N];
    name_[..name_bytes.len()].copy_from_slice(name_bytes);
    Ok(name_)
}

pub fn format_zero_terminated_utf8_bytes(
    name: &[u8],
    fmt: &mut std::fmt::Formatter,
) -> std::result::Result<(), std::fmt::Error> {
    fmt.write_str(
        std::str::from_utf8(name)
            .unwrap()
            .trim_matches(char::from(0)),
    )
}

#[cfg(test)]
mod tests {
    use fixed::types::I80F48;

    #[test]
    pub fn test_i80f48_mul_rounding() {
        // It's not desired, but I80F48 seems to round to -inf
        let price = I80F48::from_num(0.04);
        let x = I80F48::from_bits(96590783907000000);
        assert_eq!((x * price).to_string(), "13.726375969298193");
        assert_eq!(((-x) * price).to_string(), "-13.726375969298196");
    }
}

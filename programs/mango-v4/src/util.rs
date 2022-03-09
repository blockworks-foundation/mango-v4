#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}

use fixed::types::I80F48;
use fixed_macro::types::I80F48;
pub(crate) use zip;

pub const ZERO_I80F48: I80F48 = I80F48!(0);
pub const ONE_I80F48: I80F48 = I80F48!(1);
pub const NEG_ONE_I80F48: I80F48 = I80F48!(-1);

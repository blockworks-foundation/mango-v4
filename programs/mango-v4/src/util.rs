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

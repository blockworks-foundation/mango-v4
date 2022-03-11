#[macro_export]
macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}
pub(crate) use zip;

#[macro_export]
macro_rules! checked_math {
    ($x: expr) => {
        checked_math::checked_math!($x).ok_or(error!(crate::error::MangoError::MathError))?
    };
}
pub(crate) use checked_math;

use anchor_lang::error::Error::AnchorError;
use mango_v4::state::TokenIndex;

pub trait UnwrappableOracleError<T> {
    /// Returns
    /// - Ok(Some(..)) on success
    /// - Ok(None) on oracle error (and call callback)
    /// - Err(..) on any other error
    fn unwrap_unless_oracle_error<F>(self, on_oracle_error: F) -> anyhow::Result<Option<T>>
    where
        F: FnOnce(TokenIndex, String, anyhow::Error);
}

impl<T: std::fmt::Debug> UnwrappableOracleError<T> for anyhow::Result<T> {
    fn unwrap_unless_oracle_error<F>(self, on_oracle_error: F) -> anyhow::Result<Option<T>>
    where
        F: FnOnce(TokenIndex, String, anyhow::Error),
    {
        if self.is_ok() {
            return Ok(Some(self.unwrap()));
        }

        let e = self.unwrap_err();
        let root_cause = e.root_cause().downcast_ref::<anchor_lang::error::Error>();

        if root_cause.is_none() {
            return Err(e);
        }

        if let AnchorError(ae) = root_cause.unwrap() {
            let is_oracle_error = ae.error_code_number == MangoError::OracleConfidence.error_code()
                || ae.error_code_number == MangoError::OracleStale.error_code();
            let error_str = ae.to_string();

            if !is_oracle_error || !error_str.contains("token index ") {
                return Err(e);
            }
            use mango_v4::error::MangoError;
            use std::str::FromStr;
            let ti_res = u16::from_str(
                error_str.split("token index ").collect::<Vec<&str>>()[1]
                    .split(',')
                    .collect::<Vec<&str>>()[0],
            );
            let ti_name = error_str.split(" name:").collect::<Vec<&str>>()[1]
                .split(',')
                .collect::<Vec<&str>>()[0]
                .to_string();

            if !ti_res.is_err() {
                on_oracle_error(TokenIndex::from(ti_res.unwrap()), ti_name, e);
                return Ok(None);
            }
        }

        return Err(e);
    }
}

use anchor_lang::error::Error::AnchorError;
use mango_v4::state::TokenIndex;

pub trait UnwrappableOracleError {
    fn try_unwrap_oracle_error(&self) -> Option<(TokenIndex, String)>;
}

impl UnwrappableOracleError for anyhow::Error {
    fn try_unwrap_oracle_error(&self) -> Option<(TokenIndex, String)> {
        let root_cause = self
            .root_cause()
            .downcast_ref::<anchor_lang::error::Error>();

        if root_cause.is_none() {
            return None;
        }

        if let AnchorError(ae) = root_cause.unwrap() {
            let is_oracle_error = ae.error_code_number == MangoError::OracleConfidence.error_code()
                || ae.error_code_number == MangoError::OracleStale.error_code();
            let error_str = ae.to_string();

            if !is_oracle_error || !error_str.contains("token index ") {
                return None;
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
                return Some((TokenIndex::from(ti_res.unwrap()), ti_name));
            }
        }

        return None;
    }
}

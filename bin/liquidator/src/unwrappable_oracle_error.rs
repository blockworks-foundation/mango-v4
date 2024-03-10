use anchor_lang::error::Error::AnchorError;
use mango_v4::error::MangoError;
use mango_v4::state::TokenIndex;
use regex::Regex;

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

            if !is_oracle_error {
                return None;
            }

            let error_str = ae.to_string();
            return parse_oracle_error_string(&error_str);
        }

        None
    }
}

fn parse_oracle_error_string(error_str: &str) -> Option<(TokenIndex, String)> {
    let token_name_regex = Regex::new(r#"name: (\w+)"#).unwrap();
    let token_index_regex = Regex::new(r#"token index (\d+)"#).unwrap();
    let token_name = token_name_regex
        .captures(error_str)
        .map(|c| c[1].to_string())
        .unwrap_or_default();
    let token_index = token_index_regex
        .captures(error_str)
        .map(|c| c[1].parse::<u16>().ok())
        .unwrap_or_default();

    if token_index.is_some() {
        return Some((TokenIndex::from(token_index.unwrap()), token_name));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::error;
    use anyhow::Context;
    use mango_v4::error::Contextable;
    use mango_v4::error::MangoError;
    use mango_v4::state::{oracle_log_context, OracleConfig, OracleState, OracleType};

    fn generate_errored_res() -> std::result::Result<u8, error::Error> {
        return Err(MangoError::OracleConfidence.into());
    }

    fn generate_errored_res_with_context() -> anyhow::Result<u8> {
        let value = Contextable::with_context(
            Contextable::with_context(generate_errored_res(), || {
                oracle_log_context(
                    "SOL",
                    &OracleState {
                        price: Default::default(),
                        deviation: Default::default(),
                        last_update_slot: 0,
                        oracle_type: OracleType::Pyth,
                    },
                    &OracleConfig {
                        conf_filter: Default::default(),
                        max_staleness_slots: 0,
                        reserved: [0; 72],
                    },
                    None,
                )
            }),
            || {
                format!(
                    "getting oracle for bank with health account index {} and token index {}, passed account {}",
                    10,
                    11,
                    12,
                )
            },
        )?;

        Ok(value)
    }

    #[test]
    fn should_extract_oracle_error_and_token_infos() {
        let error = generate_errored_res_with_context()
            .context("Something")
            .unwrap_err();
        println!("{}", error);
        println!("{}", error.root_cause());
        let oracle_error_opt = error.try_unwrap_oracle_error();

        assert!(oracle_error_opt.is_some());
        assert_eq!(
            oracle_error_opt.unwrap(),
            (TokenIndex::from(11u16), "SOL".to_string())
        );
    }

    #[test]
    fn should_parse_oracle_error_message() {
        assert!(parse_oracle_error_string("").is_none());
        assert!(parse_oracle_error_string("Something went wrong").is_none());
        assert_eq!(
            parse_oracle_error_string("Something went wrong token index 4, name: SOL, Stale")
                .unwrap(),
            (TokenIndex::from(4u16), "SOL".to_string())
        );
    }
}

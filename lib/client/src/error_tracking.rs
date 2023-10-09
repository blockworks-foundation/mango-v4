use anchor_lang::prelude::Pubkey;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::*;

#[derive(Clone)]
pub struct AccountErrorState {
    pub messages: Vec<String>,
    pub count: u64,
    pub last_at: Instant,
}

#[derive(Default)]
pub struct ErrorTracking {
    pub accounts: HashMap<Pubkey, AccountErrorState>,
    pub skip_threshold: u64,
    pub skip_duration: Duration,
}

impl ErrorTracking {
    pub fn had_too_many_errors(&self, pubkey: &Pubkey, now: Instant) -> Option<AccountErrorState> {
        if let Some(error_entry) = self.accounts.get(pubkey) {
            if error_entry.count >= self.skip_threshold
                && now.duration_since(error_entry.last_at) < self.skip_duration
            {
                Some(error_entry.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn record_error(&mut self, pubkey: &Pubkey, now: Instant, message: String) {
        let error_entry = self.accounts.entry(*pubkey).or_insert(AccountErrorState {
            messages: Vec::with_capacity(1),
            count: 0,
            last_at: now,
        });
        error_entry.count += 1;
        error_entry.last_at = now;
        if !error_entry.messages.contains(&message) {
            error_entry.messages.push(message);
        }
        if error_entry.messages.len() > 5 {
            error_entry.messages.remove(0);
        }
    }

    pub fn clear_errors(&mut self, pubkey: &Pubkey) {
        self.accounts.remove(pubkey);
    }

    #[instrument(skip_all, fields(%error_type))]
    #[allow(unused_variables)]
    pub fn log_persistent_errors(&self, error_type: &str, min_duration: Duration) {
        let now = Instant::now();
        for (pubkey, errors) in self.accounts.iter() {
            if now.duration_since(errors.last_at) < min_duration {
                continue;
            }
            info!(
                %pubkey,
                count = errors.count,
                messages = ?errors.messages,
                "has persistent errors",
            );
        }
    }
}

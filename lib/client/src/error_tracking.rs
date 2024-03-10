use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::*;

#[derive(Clone)]
pub struct ErrorState {
    pub errors: Vec<String>,
    pub count: u64,
    pub last_at: Instant,
}

#[derive(Clone)]
struct ErrorTypeState<Key> {
    state_by_key: HashMap<Key, ErrorState>,

    // override global
    skip_threshold: Option<u64>,
    skip_duration: Option<Duration>,
}

impl<Key> Default for ErrorTypeState<Key> {
    fn default() -> Self {
        Self {
            state_by_key: Default::default(),
            skip_threshold: None,
            skip_duration: None,
        }
    }
}

#[derive(Builder)]
pub struct ErrorTracking<Key, ErrorType> {
    #[builder(default, setter(custom))]
    errors_by_type: HashMap<ErrorType, ErrorTypeState<Key>>,

    /// number of errors of a type after which had_too_many_errors returns true
    #[builder(default = "2")]
    pub skip_threshold: u64,

    /// duration that had_too_many_errors returns true for after skip_threshold is reached
    #[builder(default = "Duration::from_secs(60)")]
    pub skip_duration: Duration,

    #[builder(default = "3")]
    pub unique_messages_to_keep: usize,

    /// after what time of no-errors may error info be wiped?
    #[builder(default = "Duration::from_secs(300)")]
    pub keep_duration: Duration,

    #[builder(setter(skip), default = "Instant::now()")]
    last_log: Instant,

    #[builder(default = "Duration::from_secs(300)")]
    pub log_interval: Duration,
}

impl<Key, ErrorType> ErrorTrackingBuilder<Key, ErrorType>
where
    ErrorType: Copy + std::hash::Hash + std::cmp::Eq + std::fmt::Display,
{
    pub fn skip_threshold_for_type(&mut self, error_type: ErrorType, threshold: u64) -> &mut Self {
        if self.errors_by_type.is_none() {
            self.errors_by_type = Some(Default::default());
        }
        let errors_by_type = self.errors_by_type.as_mut().unwrap();
        errors_by_type.entry(error_type).or_default().skip_threshold = Some(threshold);
        self
    }
}

impl<Key, ErrorType> ErrorTracking<Key, ErrorType>
where
    Key: Clone + std::hash::Hash + std::cmp::Eq + std::fmt::Display,
    ErrorType: Copy + std::hash::Hash + std::cmp::Eq + std::fmt::Display,
{
    pub fn builder() -> ErrorTrackingBuilder<Key, ErrorType> {
        ErrorTrackingBuilder::default()
    }

    fn should_skip(
        &self,
        state: &ErrorState,
        error_type_state: &ErrorTypeState<Key>,
        now: Instant,
    ) -> bool {
        let skip_threshold = error_type_state
            .skip_threshold
            .unwrap_or(self.skip_threshold);
        let skip_duration = error_type_state.skip_duration.unwrap_or(self.skip_duration);
        state.count >= skip_threshold && now.duration_since(state.last_at) < skip_duration
    }

    pub fn had_too_many_errors(
        &self,
        error_type: ErrorType,
        key: &Key,
        now: Instant,
    ) -> Option<ErrorState> {
        let error_type_state = self.errors_by_type.get(&error_type)?;
        let state = error_type_state.state_by_key.get(key)?;
        self.should_skip(state, error_type_state, now)
            .then(|| state.clone())
    }

    pub fn record(&mut self, error_type: ErrorType, key: &Key, message: String) {
        let now = Instant::now();
        let state = self
            .errors_by_type
            .entry(error_type)
            .or_default()
            .state_by_key
            .entry(key.clone())
            .or_insert(ErrorState {
                errors: Vec::with_capacity(1),
                count: 0,
                last_at: now,
            });
        state.count += 1;
        state.last_at = now;
        if let Some(pos) = state.errors.iter().position(|m| m == &message) {
            state.errors.remove(pos);
        }
        state.errors.push(message);
        if state.errors.len() > self.unique_messages_to_keep {
            state.errors.remove(0);
        }

        // log when skip threshold is reached the first time
        if state.count == self.skip_threshold {
            trace!(%error_type, %key, count = state.count, messages = ?state.errors, "had repeated errors, skipping...");
        }
    }

    pub fn clear(&mut self, error_type: ErrorType, key: &Key) {
        if let Some(error_type_state) = self.errors_by_type.get_mut(&error_type) {
            error_type_state.state_by_key.remove(key);
        }
    }

    pub fn wipe_old(&mut self) {
        let now = Instant::now();
        for error_type_state in self.errors_by_type.values_mut() {
            error_type_state
                .state_by_key
                .retain(|_, state| now.duration_since(state.last_at) < self.keep_duration);
        }
    }

    /// Wipes old errors and occasionally logs errors that caused skipping
    pub fn update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_log) < self.log_interval {
            return;
        }
        self.last_log = now;
        self.wipe_old();
        self.log_error_skips();
    }

    /// Log all errors that cause skipping
    pub fn log_error_skips(&self) {
        let now = Instant::now();
        for (error_type, error_type_state) in self.errors_by_type.iter() {
            let span = info_span!("log_error_skips", %error_type);
            let _enter = span.enter();

            for (key, state) in error_type_state.state_by_key.iter() {
                if self.should_skip(state, error_type_state, now) {
                    info!(
                        %key,
                        count = state.count,
                        messages = ?state.errors,
                    );
                }
            }
        }
    }
}

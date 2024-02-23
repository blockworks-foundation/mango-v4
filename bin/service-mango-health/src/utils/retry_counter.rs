use std::future::Future;

pub struct RetryCounter {
    error_count: u64,
    max_retry_count: u64,
}

impl RetryCounter {
    pub fn new(max_retry_count: u64) -> Self {
        RetryCounter {
            max_retry_count,
            error_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.error_count = 0;
    }

    /// true if should retry, false if should bail
    pub fn on_new_error(&mut self) -> bool {
        self.error_count += 1;
        self.error_count <= self.max_retry_count
    }

    pub fn fail_or_ignore<T>(&mut self, result: anyhow::Result<T>) -> anyhow::Result<Option<T>> {
        match result {
            Err(e) => match self.on_new_error() {
                true => Ok(None),
                false => Err(e),
            },
            Ok(v) => {
                self.reset();
                Ok(Some(v))
            }
        }
    }

    pub fn fail_or_retry<T>(&mut self, act: impl Fn() -> anyhow::Result<T>) -> anyhow::Result<T> {
        loop {
            let result = self.fail_or_ignore(act())?;

            if let Some(value) = result {
                return Ok(value);
            }
        }
    }
}

#[macro_export]
macro_rules! fail_or_retry_async {
    ($retry_counter:expr, $f:expr) => {{
        loop {
            let result = $retry_counter.fail_or_ignore($f);
            match result {
                Ok(opt) => {
                    if let Some(value) = opt {
                        break Ok(value);
                    }
                }
                Err(e) => {
                    break Err(e);
                }
            }
        }
    }};
}

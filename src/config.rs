//! Testing Library's `configure`.
use std::{cell::RefCell, time::Duration};

/// Library-wide options. Each test thread has its own copy.
#[derive(Clone, Debug)]
pub struct Config {
    async_util_timeout: Duration,
    async_util_interval: Duration,
    default_hidden: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            async_util_timeout: Duration::from_millis(1000),
            async_util_interval: Duration::from_millis(50),
            default_hidden: false,
        }
    }
}

impl Config {
    /// How long `find_*` and `wait_for` retry, in GPUI test time. Default 1 s.
    pub fn with_async_util_timeout(mut self, timeout: Duration) -> Self {
        self.async_util_timeout = timeout;
        self
    }
    pub fn async_util_timeout(&self) -> Duration {
        self.async_util_timeout
    }
    /// How often `find_*` and `wait_for` retry, in GPUI test time. Default 50 ms.
    pub fn with_async_util_interval(mut self, interval: Duration) -> Self {
        self.async_util_interval = interval;
        self
    }
    pub fn async_util_interval(&self) -> Duration {
        self.async_util_interval
    }
    /// Whether role queries include elements that are not visible. Default false.
    pub fn with_default_hidden(mut self, hidden: bool) -> Self {
        self.default_hidden = hidden;
        self
    }
    pub fn default_hidden(&self) -> bool {
        self.default_hidden
    }
}

thread_local! { static CONFIG: RefCell<Config> = RefCell::new(Config::default()); }

/// Changes the configuration for the current test thread.
///
/// ```
/// use gpui_testing_library::configure;
/// use std::time::Duration;
/// configure(|config| config.with_async_util_timeout(Duration::from_secs(2)));
/// ```
pub fn configure(update: impl FnOnce(Config) -> Config) {
    CONFIG.with(|config| {
        let current = config.borrow().clone();
        *config.borrow_mut() = update(current);
    });
}

/// The current configuration.
pub fn get_config() -> Config {
    get()
}

pub(crate) fn get() -> Config {
    CONFIG.with(|config| config.borrow().clone())
}

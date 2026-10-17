//! `waitFor` and `waitForElementToBeRemoved`, driven by GPUI's test clock.
use crate::{config, node::Node, screen::Screen};
use std::{fmt::Debug, time::Duration};

/// A `wait_for` callback result: `true`, `Some(value)` or `Ok(value)` stops
/// waiting. An `Err` is reported if the wait times out.
pub trait Ready {
    type Output;
    fn ready(self) -> Result<Self::Output, Option<String>>;
}

impl Ready for bool {
    type Output = ();
    fn ready(self) -> Result<(), Option<String>> {
        if self { Ok(()) } else { Err(None) }
    }
}

impl<T> Ready for Option<T> {
    type Output = T;
    fn ready(self) -> Result<T, Option<String>> {
        self.ok_or(None)
    }
}

impl<T, E: Debug> Ready for Result<T, E> {
    type Output = T;
    fn ready(self) -> Result<T, Option<String>> {
        self.map_err(|error| Some(format!("{error:?}")))
    }
}

/// Overrides for one wait; unset values come from [`crate::Config`].
#[derive(Clone, Copy, Debug, Default)]
pub struct WaitOptions {
    timeout: Option<Duration>,
    interval: Option<Duration>,
}

impl WaitOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }
}

/// Retries `check` on GPUI's test clock until it returns `Ok(Some)`, fails
/// with `Err`, or the timeout elapses (`Ok(None)`).
pub(crate) async fn poll<T>(
    screen: &Screen,
    options: Option<WaitOptions>,
    mut check: impl FnMut() -> Result<Option<T>, String>,
) -> Result<Option<T>, String> {
    let options = options.unwrap_or_default();
    let config = config::get();
    let timeout = options.timeout.unwrap_or(config.async_util_timeout());
    let interval = options.interval.unwrap_or(config.async_util_interval());
    let executor = screen.cx().executor();
    let mut elapsed = Duration::ZERO;
    loop {
        screen.cx().run_until_parked();
        if let Some(value) = check()? {
            return Ok(Some(value));
        }
        if elapsed >= timeout {
            return Ok(None);
        }
        let step = interval.min(timeout - elapsed);
        executor.timer(step).await;
        elapsed += step;
    }
}

/// Calls `callback` until it is [`Ready`], advancing GPUI's test clock
/// between attempts. Panics after the timeout with the last error and the
/// observed tree.
pub async fn wait_for<T: Ready>(
    screen: &Screen,
    options: WaitOptions,
    mut callback: impl FnMut() -> T,
) -> T::Output {
    let mut last_error = None;
    let result = poll(screen, Some(options), || match callback().ready() {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            last_error = error;
            Ok(None)
        }
    })
    .await
    .expect("wait_for never fails early");
    result.unwrap_or_else(|| {
        let reason = last_error.map_or_else(String::new, |error| format!(": {error}"));
        panic!("wait_for timed out{reason}\n\n{}", screen.pretty())
    })
}

/// Waits until every element in `nodes` is gone. Panics if they are already
/// gone when called, as Testing Library does, or on timeout.
pub async fn wait_for_element_to_be_removed(
    screen: &Screen,
    nodes: impl IntoIterator<Item = Node>,
) {
    let nodes: Vec<Node> = nodes.into_iter().collect();
    assert!(
        nodes.iter().any(Node::is_connected),
        "wait_for_element_to_be_removed requires elements that are present initially"
    );
    let gone = poll(screen, None, || {
        Ok::<_, String>(nodes.iter().all(|node| !node.is_connected()).then_some(()))
    })
    .await
    .expect("never fails early");
    if gone.is_none() {
        panic!(
            "Timed out waiting for elements to be removed: {:?}\n\n{}",
            nodes,
            screen.pretty()
        );
    }
}

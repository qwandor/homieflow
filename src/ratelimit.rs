// Copyright 2022 the homieflow authors.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    sync::Notify,
    task::{self, JoinHandle},
    time,
};

/// Utility to rate limit the number of times a function is called.
#[derive(Debug)]
pub struct RateLimiter {
    notify: Arc<Notify>,
    handle: JoinHandle<()>,
}

impl RateLimiter {
    /// Creates a new rate limiter that will call the given `callback` no more than once every
    /// `period`.
    pub fn new<T: FnMut() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static>(
        period: Duration,
        callback: T,
    ) -> Self {
        let notify = Arc::new(Notify::new());
        let handle = task::spawn(callback_run_loop(notify.clone(), period, callback));
        Self { notify, handle }
    }

    /// Calls the callback, either immediately or after waiting enough time.
    ///
    /// If the callback has not been called for at least the period of the rate limiter, calls it
    /// immediately. If it has, then waits until the period has elapsed since the last time it was
    /// called. If `execute` is called multiple times within the period the callback will still only
    /// be called once.
    pub fn execute(&self) {
        self.notify.notify_one();
    }
}

impl Drop for RateLimiter {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn callback_run_loop(
    notify: Arc<Notify>,
    period: Duration,
    mut callback: impl FnMut() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + 'static,
) {
    loop {
        notify.notified().await;
        callback().await;
        time::sleep(period).await;
    }
}

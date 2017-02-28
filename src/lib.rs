#![warn(missing_docs)]

extern crate abstract_ns;
extern crate futures;
extern crate num_traits;
extern crate tokio_core;
extern crate tk_bufstream;

#[macro_use] extern crate log;

mod public;
mod element;
mod proto;
mod pool;
mod config;
mod channel;

pub use public::Carbon;
pub use proto::Proto;

use std::sync::Arc;
use std::time::Duration;

/// A helper structure for initializing protocol instance
pub struct Init {
    chan: channel::Receiver,
    config: Arc<Config>,
}

/// Configuration of carbon protocol
///
/// This configuration is used both for single connection and connection pool.
#[derive(Clone, Debug)]
pub struct Config {
    write_timeout: Duration,
    watermarks: (usize, usize),
    max_metrics_buffered: usize,

    /// Reconnect delay in milliseconds, so it's easier to generate random
    reconnect_delay: (u64, u64),
}

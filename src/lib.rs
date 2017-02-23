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

pub use public::Carbon;
pub use proto::Proto;

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;
use futures::sync::mpsc::{UnboundedReceiver};

/// A helper structure for initializing protocol instance
pub struct Init {
    channel: UnboundedReceiver<element::Metric>,
    buffered: Arc<AtomicUsize>,
}

/// Configuration of carbon protocol
///
/// This configuration is used both for single connection and connection pool.
pub struct Config {
    write_timeout: Duration,
    watermarks: (usize, usize),
    max_metrics_buffered: usize,

    /// Reconnect delay in milliseconds, so it's easier to generate random
    reconnect_delay: (u64, u64),
}

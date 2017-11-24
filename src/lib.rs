//! Carbon client protocol implementation
//!
//! # High Level Interface
//!
//! ```rust,ignore
//! let (carbon, init) = Carbon::new(&Config::new().done());
//! init.connect_to(resolver.subscribe("localhost:2003"), &handle);
//! // now you can submit metrics
//! carbon.add_metric("my.metric", 10);
//! ```
//!
//! This allows protocol to:
//!
//! 1. Establish connection to all addressses address resolves too
//! 2. Reconnect in case of failure
//! 3. Reconnect to new host(s) when IPs the DNS name resolves to changes
//!
//! See [examples](https://github.com/tailhook/tk-carbon/tree/master/examples)
//! for more full examples.
//!
//! # Lower Level Interface
//!
//! In case you don't want to use connection pooling, you may connect carbon
//! instance to a specific connection:
//!
//! ```rust,ignore
//! use tk_carbon::{Carbon, Config};
//!
//! let (carbon, init) = Carbon::new(&Config::new().done());
//! handle.spawn(TcpStream::connect(&addr, &handle)
//!     .and_then(move |sock| init.from_connection(sock, &handle2))
//!     .map_err(|e| unimplemented!()));
//! // use carbon the same way as above
//! carbon.add_metric("my.metric", 10);
//! ```
//!
//! # General
//!
//! [`Carbon`](struct.Carbon.html) object is the same for connection pool and
//! raw interface and may be used from any thread as long as the tokio loop
//! running the networking code is still alive.
//!
//! You don't have to wait until connection is established to send metrics,
//! they will be buffered up till configuration limit
//! (see docs on [`Config`](struct.Config.html))
//!
#![warn(missing_docs)]

extern crate abstract_ns;
extern crate futures;
extern crate num_traits;
extern crate tokio_core;
extern crate tk_bufstream;
extern crate rand;

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

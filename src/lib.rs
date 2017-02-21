#![warn(missing_docs)]

extern crate futures;
extern crate num_traits;
extern crate tokio_core;
extern crate tk_bufstream;

#[macro_use] extern crate log;

mod public;
mod element;
mod proto;

pub use public::Carbon;
pub use proto::Proto;

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use futures::sync::mpsc::{UnboundedReceiver};

/// A helper structure for initializing protocol instance
pub struct Init {
    channel: UnboundedReceiver<element::Metric>,
    buffered: Arc<AtomicUsize>,
}

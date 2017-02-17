#![warn(missing_docs)]

extern crate futures;
extern crate num_traits;
extern crate tokio_core;

#[macro_use] extern crate log;

mod public;
mod element;

pub use public::Carbon;

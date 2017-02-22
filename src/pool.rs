use futures::{Future, Async, Stream};
use abstract_ns::Address;
use tokio_core::reactor::Handle;

use {Init};


struct Pool<S> {
    address_stream: S,
    handle: Handle,
}


impl Init {
    /// Establishes connections to all the hosts
    ///
    /// This method spawns a future (or futures) in the loop represented
    /// by handle. The future exits when all references to API (`Carbon`
    /// structure) are dropped and all buffers are flushed.
    pub fn connect_to<S>(self, address_stream: S, handle: &Handle)
        where S: Stream<Item=Address> + 'static,
    {
        handle.spawn(Pool {
            address_stream: address_stream,
            handle: handle.clone(),
        });
    }
}

impl<S: Stream<Item=Address>> Future for Pool<S> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        unimplemented!();
    }
}

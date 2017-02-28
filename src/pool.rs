use std::collections::VecDeque;
use std::net::SocketAddr;

use abstract_ns::Address;
use futures::{Future, Async, Stream};
use tokio_core::reactor::Handle;
use tokio_core::net::TcpStream;

use channel::Receiver;
use {Init, Proto};


struct Pool<A> {
    address_stream: A,
    channel: Receiver,
    handle: Handle,

    cur_address: Option<Address>,
    active: VecDeque<(SocketAddr, Proto<TcpStream>)>,
    pending: VecDeque<(SocketAddr,
                       Box<Future<Item=Proto<TcpStream>, Error=()>>)>,
    retired: VecDeque<Proto<TcpStream>>,
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
            channel: self.chan,

            cur_address: None,
            active: VecDeque::new(),
            pending: VecDeque::new(),
            retired: VecDeque::new(),
        });
    }
}

impl<S: Stream<Item=Address>> Future for Pool<S> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        self.update_connections();
        Ok(Async::NotReady)
    }
}

impl<S: Stream<Item=Address>> Pool<S> {
    fn update_connections(&mut self) -> Result<(), ()> {
        let new_addr = match self.address_stream.poll() {
            Ok(Async::Ready(Some(new_addr))) => new_addr,
            Ok(Async::NotReady) => return Ok(()),
            Ok(Async::Ready(None)) => {
                panic!("Address stream must be infinite");
            }
            Err(e) => {
                // TODO(tailhook) poll crashes on address error?
                error!("Address stream error");
                return Err(());
            }
        };
        if let Some(ref mut old_addr) = self.cur_address {
            if old_addr != &new_addr {
                let (old, new) = old_addr.at(0)
                               .compare_addresses(&new_addr.at(0));
                debug!("New addresss, to be retired {:?}, \
                        to be connected {:?}", old, new);
                for _ in 0..self.pending.len() {
                    let (addr, c) = self.pending.pop_front().unwrap();
                    // Drop pending connections to non-existing
                    // addresses
                    if !old.contains(&addr) {
                        self.pending.push_back((addr, c));
                    } else {
                        debug!("Dropped pending {}", addr);
                    }
                }
                for _ in 0..self.active.len() {
                    let (addr, c) = self.active.pop_front().unwrap();
                    // Active connections are waiting to become idle
                    if old.contains(&addr) {
                        debug!("Retiring {}", addr);
                        self.retired.push_back(c);
                    } else {
                        self.active.push_back((addr, c));
                    }
                }
            }
        } else {
            for addr in new_addr.at(0).addresses() {
                self.pending.push_back((addr, Box::new(
                    TcpStream::connect(&addr, &self.handle)
                    .map(|sock | { unimplemented!() })
                    .map_err(|e | error!("Conection error: {}", e))
                )));
            }
        }
        Ok(())
    }
}

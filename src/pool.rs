use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Instant, Duration};

use abstract_ns::Address;
use futures::{Future, Async, Stream};
use rand::{thread_rng, Rng};
use tk_bufstream::IoBuf;
use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use tokio_core::reactor::{Handle, Timeout};

use channel::Receiver;
use {Init, Config};


struct Pool<A> {
    address_stream: A,
    channel: Receiver,
    config: Arc<Config>,
    handle: Handle,
    deadline: Instant,
    timeo: Timeout,

    cur_address: Option<Address>,
    normal: VecDeque<(SocketAddr, Conn<TcpStream>)>,
    crowded: VecDeque<(SocketAddr, Conn<TcpStream>)>,
    pending: VecDeque<(SocketAddr,
                       Box<Future<Item=Conn<TcpStream>, Error=io::Error>>)>,
    retired: VecDeque<Conn<TcpStream>>,
    failed: VecDeque<(SocketAddr, Instant)>,
}

struct Conn<T: Io> {
    io: IoBuf<T>,
    deadline: Instant,
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
            channel: self.chan,
            handle: handle.clone(),
            deadline: Instant::now() + self.config.write_timeout,
            timeo: Timeout::new(self.config.write_timeout, &handle)
                .expect("can always set a timeout"),
            config: self.config,

            cur_address: None,
            normal: VecDeque::new(),
            crowded: VecDeque::new(),
            pending: VecDeque::new(),
            retired: VecDeque::new(),
            failed: VecDeque::new(),
        });
    }
}

impl<S: Stream<Item=Address>> Future for Pool<S> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        loop {
            self.update_addresses()?;
            self.check_pending();
            self.read_check();
            self.push_crowded();
            self.new_metrics();
            self.flush_metrics();
            self.reconnect_failed();
            let ndeadline = self.calc_deadline();
            if ndeadline != self.deadline {
                self.deadline = ndeadline;
                self.timeo = Timeout::new_at(self.deadline, &self.handle)
                    .expect("can always set a timeout");
                let res = self.timeo.poll()
                    .expect("timeout future never fails");
                match res {
                    Async::Ready(()) => continue,
                    Async::NotReady => break,
                }
            } else {
                break;
            }
        }
        Ok(Async::NotReady)
    }
}

impl<S: Stream<Item=Address>> Pool<S> {
    fn update_addresses(&mut self) -> Result<(), ()> {
        let new_addr = match self.address_stream.poll() {
            Ok(Async::Ready(Some(new_addr))) => new_addr,
            Ok(Async::NotReady) => return Ok(()),
            Ok(Async::Ready(None)) => {
                panic!("Address stream must be infinite");
            }
            Err(_) => {
                // TODO(tailhook) pool should crash?
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
                for _ in 0..self.normal.len() {
                    let (addr, c) = self.normal.pop_front().unwrap();
                    // Active connections are waiting to become idle
                    if old.contains(&addr) {
                        debug!("Retiring {}", addr);
                        self.retired.push_back(c);
                    } else {
                        self.normal.push_back((addr, c));
                    }
                }
                for _ in 0..self.crowded.len() {
                    let (addr, c) = self.crowded.pop_front().unwrap();
                    // Active connections are waiting to become idle
                    if old.contains(&addr) {
                        debug!("Retiring {}", addr);
                        self.retired.push_back(c);
                    } else {
                        self.crowded.push_back((addr, c));
                    }
                }
                for addr in new {
                    self.pending.push_back((addr, Box::new(
                        // TODO(tailhook) timeout on connect
                        TcpStream::connect(&addr, &self.handle)
                        .map(|sock | Conn {
                            io: IoBuf::new(sock),
                            deadline: Instant::now()
                                // no data yet
                                + Duration::new(86400, 0),
                        })
                    )));
                }
            }
        } else {
            for addr in new_addr.at(0).addresses() {
                self.pending.push_back((addr, Box::new(
                    // TODO(tailhook) timeout on connect
                    TcpStream::connect(&addr, &self.handle)
                    .map(|sock | Conn {
                        io: IoBuf::new(sock),
                        deadline: Instant::now()
                            // no data yet
                            + Duration::new(86400, 0),
                    })
                )));
            }
        }
        Ok(())
    }
    fn check_pending(&mut self) {
        for _ in 0..self.pending.len() {
            let (a, mut c) = self.pending.pop_front().unwrap();
            match c.poll() {
                Ok(Async::Ready(c)) => {
                    // Can use it immediately
                    debug!("Connected {}", a);
                    self.normal.push_front((a, c));
                }
                Ok(Async::NotReady) => {
                    self.pending.push_back((a, c));
                }
                Err(e) => {
                    warn!("Can't establish connection to {}: {}", a, e);
                    // TODO(tailhook) set timer to reconnect
                    // Add to the end of the list
                    self.reconnect(a);
                }
            }
        }
    }
    fn read_check(&mut self) {
        for _ in 0..self.normal.len() {
            let (a, mut c) = self.normal.pop_front().unwrap();
            if let Err(e) = c.io.read() {
                warn!("Read error from {}: {}", a, e);
                self.reconnect(a);
            } else if c.io.in_buf.len() > 0 {
                warn!("Input data in carbon socket from {} (protocol error)",
                    a);
                self.reconnect(a);
            } else if c.io.done() {
                warn!("Connection from {} closed by peer", a);
                self.reconnect(a);
            } else {
                self.normal.push_back((a, c));
            }
        }
        for _ in 0..self.crowded.len() {
            let (a, mut c) = self.crowded.pop_front().unwrap();
            if let Err(e) = c.io.read() {
                warn!("Read error from {}: {}", a, e);
                self.reconnect(a);
            } else if c.io.in_buf.len() > 0 {
                warn!("Input data in carbon socket from {} (protocol error)",
                    a);
                self.reconnect(a);
            } else if c.io.done() {
                warn!("Connection from {} closed by peer", a);
                self.reconnect(a);
            } else {
                self.crowded.push_back((a, c));
            }
        }
    }
    fn reconnect(&mut self, addr: SocketAddr) {
        let (min, max) = self.config.reconnect_delay;
        let ms = thread_rng().gen_range(min, max);
        self.failed.push_back((
            addr,
            Instant::now() + Duration::from_millis(ms),
        ));
    }
    fn push_crowded(&mut self) {
        for _ in 0..self.crowded.len() {
            let (a, mut c) = self.crowded.pop_front().unwrap();
            if let Err(e) = c.flush(&*self.config) {
                warn!("Write error for {}: {}", a, e);
                self.reconnect(a);
            } else if c.io.out_buf.len() < self.config.watermarks.0 {
                self.normal.push_back((a, c));
            } else {
                self.crowded.push_back((a, c));
            }
        }
    }
    fn new_metrics(&mut self) {
        if self.normal.len() == 0 {
            // do not accept new metrics
            return;
        }
        while let Ok(Async::Ready(Some(metric))) = self.channel.poll() {
            for &mut (_, ref mut c) in self.normal.iter_mut()
                .chain(&mut self.crowded)
            {
                c.io.out_buf.extend(&metric.0);
            }
        }
    }
    fn flush_metrics(&mut self) {
        // we're flushing only normal metrics, because crowded have already
        // been flushed at the start of poll
        for _ in 0..self.normal.len() {
            let (a, mut c) = self.normal.pop_front().unwrap();
            if let Err(e) = c.flush(&*self.config) {
                warn!("Write error for {}: {}", a, e);
                self.reconnect(a);
            } else if c.io.out_buf.len() > self.config.watermarks.1 {
                warn!("Buffer overflow for {}: {}/{}. \
                    Dropping buffer and reconnecting... ", a,
                    c.io.out_buf.len(), self.config.watermarks.1);
                self.reconnect(a);
            } else if c.io.out_buf.len() < self.config.watermarks.0 {
                self.normal.push_back((a, c));
            } else {
                self.crowded.push_back((a, c));
            }
        }
    }
    fn reconnect_failed(&mut self) {
        let now = Instant::now();
        for _ in 0..self.failed.len() {
            let (addr, time) = self.failed.pop_front().unwrap();
            if time <= now {
                self.pending.push_back((addr, Box::new(
                    // TODO(tailhook) timeout on connect
                    TcpStream::connect(&addr, &self.handle)
                    .map(move |sock| Conn {
                        io: IoBuf::new(sock),
                        deadline: now
                            // no data yet
                            + Duration::new(86400, 0),
                    })
                )));
            } else {
                self.failed.push_back((addr, time));
            }
        }
    }
    fn calc_deadline(&mut self) -> Instant {
        // We assume that there are only few connections at any point in
        // time, so iterating is faster than keeping a heap of timers
        self.failed.iter().map(|&(_, dline)| dline)
        .chain(self.normal.iter().map(|&(_, ref c)| c.deadline))
        .chain(self.crowded.iter().map(|&(_, ref c)| c.deadline))
        // TODO(tailhook) make timeouts for pending connections
        .min()
        // We can have all the queues empty, when we're waiting for address
        // to be resolved
        .unwrap_or_else(|| Instant::now() +  Duration::new(86400, 0))
    }
}

impl<S: Io> Conn<S> {
    fn flush(&mut self, cfg: &Config) -> Result<(), io::Error> {
        let old_out = self.io.out_buf.len();
        if old_out > 0 {
            self.io.flush()?;
            let new_out = self.io.out_buf.len();
            if new_out != old_out {
                self.deadline = Instant::now() + cfg.write_timeout;
            } else {
                if self.deadline < Instant::now() {
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }
        }
        Ok(())
    }
}

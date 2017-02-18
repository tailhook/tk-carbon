use std::io;
use std::time::Duration;

use futures::{Stream, Future, Async};
use tk_bufstream::IoBuf;
use tokio_core::io::Io;
use tokio_core::reactor::{Handle, Timeout};


/// Low-level interface to a single carbon connection
pub struct Proto<T: Io, S: Stream> {
    io: IoBuf<T>,
    channel: S,
    timeout: Duration,
    timeo: Timeout,
    handle: Handle,
    waterline: usize,
}

impl<T: Io, S: Stream> Proto<T, S> {
    /// Wrap existing connection into a future that implements carbon protocol
    pub fn new(conn: T, metric_stream: S,
        write_timeout: Duration,
        waterline: usize,
        handle: &Handle)
        -> Proto<T, S>
    {
        Proto {
            io: IoBuf::new(conn),
            channel: metric_stream,
            timeout: write_timeout,
            timeo: Timeout::new(write_timeout, &handle)
                .expect("can always set a timeout"),
            handle: handle.clone(),
            waterline: waterline,
        }
    }
}

impl<T: Io, S: Stream> Future for Proto<T, S> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        self.io.read().map_err(|_| ())?;
        if self.io.in_buf.len() > 0 {
            // invalid protocol is an error
            return Err(());
        }
        if self.io.done() {
            // connection closed by peer is just finish of a future
            return Ok(Async::Ready(()));
        }
        self.flush_output().map_err(|_| ())?;
        return Ok(Async::NotReady);
    }
}

impl<T: Io, S: Stream> Proto<T, S> {

    fn flush_output(&mut self) -> io::Result<()> {
        let old_out = self.io.out_buf.len();
        if old_out > 0 {
            self.io.flush()?;
            let new_out = self.io.out_buf.len();
            if new_out != old_out {
                if new_out != 0 {
                    self.timeo = Timeout::new(self.timeout, &self.handle)?;
                    self.timeo.poll()?;  // schedule a timeout
                }
            } else {
                let poll_result = self.timeo.poll()?;
                if poll_result.is_ready() {
                    // timeout, no byte is written within the period
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }
        }
        Ok(())
    }
}

use std::io;
use std::sync::Arc;

use futures::{Stream, Future, Async};
use tk_bufstream::IoBuf;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_core::reactor::{Handle, Timeout};

use channel::Receiver;
use {Init, Config};


/// Low-level interface to a single carbon connection
pub struct Proto<T> {
    io: IoBuf<T>,
    channel: Receiver,
    config: Arc<Config>,
    timeo: Timeout,
    handle: Handle,
}

impl Init {
    /// Wrap existing connection into a future that implements carbon protocol
    pub fn from_connection<T>(self, conn: T, handle: &Handle)
        -> Proto<T>
    {
        Proto {
            io: IoBuf::new(conn),
            channel: self.chan,
            timeo: Timeout::new(self.config.write_timeout, &handle)
                .expect("can always set a timeout"),
            handle: handle.clone(),
            config: self.config,
        }
    }
}

impl<T: AsyncRead+AsyncWrite> Future for Proto<T> {
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
        if self.io.out_buf.len() >= self.config.watermarks.0 {
            self.flush_output().map_err(|_| ())?;
            if self.io.out_buf.len() >= self.config.watermarks.0 {
                return Ok(Async::NotReady);
            }
        }
        while let Async::Ready(Some(metric)) = self.channel.poll()?  {
            self.io.out_buf.extend(&metric.0);
            if self.io.out_buf.len() >= self.config.watermarks.0 {
                break;
            }
        }
        self.flush_output().map_err(|_| ())?;
        if self.channel.is_done() && self.io.out_buf.len() == 0 {
            return Ok(Async::Ready(()));
        }
        return Ok(Async::NotReady);
    }
}

impl<T: AsyncWrite> Proto<T> {

    fn flush_output(&mut self) -> io::Result<()> {
        let old_out = self.io.out_buf.len();
        if old_out > 0 {
            self.io.flush()?;
            let new_out = self.io.out_buf.len();
            if new_out != old_out {
                if new_out != 0 {
                    self.timeo = Timeout::new(
                        self.config.write_timeout, &self.handle)?;
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

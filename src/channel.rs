//! The channel here is similar to `futures::sync::mpsc::channel` but allows
//! non-blocking send (and looses message when buffer is full)

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{Stream, Async};
use futures::stream::{Fuse};
use futures::sync::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};

use element::{Metric};

#[derive(Clone)]
pub struct Sender {
    channel: UnboundedSender<Metric>,
    buffered: Arc<AtomicUsize>,
    max_metrics_buffered: usize,
}

pub struct Receiver {
    channel: Fuse<UnboundedReceiver<Metric>>,
    buffered: Arc<AtomicUsize>,
}

pub fn channel(max_metrics_buffered: usize) -> (Sender, Receiver) {
    let (tx, rx) = unbounded();
    let counter = Arc::new(AtomicUsize::new(0));
    (Sender {
        channel: tx,
        buffered: counter.clone(),
        max_metrics_buffered: max_metrics_buffered,
    }, Receiver {
        channel: rx.fuse(),
        buffered: counter.clone(),
    })
}

impl Sender {
    pub fn send(&self, metric: Metric) {
        let max = self.max_metrics_buffered;
        if self.buffered.load(Ordering::Relaxed) > max {
            trace!("Warning can't send metric {}, buffer is full",
                String::from_utf8_lossy(&metric.0));
            return;
        }
        self.buffered.fetch_add(1, Ordering::Relaxed);
        self.channel.send(metric)
        // This shouldn't happen actually
        .map_err(|_| debug!("Can't send metric, \
            connection has been shut down"))
        // but we don't want it to be fatal
        .ok();
    }
    pub fn buffered(&self) -> (usize, usize) {
        return (
            self.buffered.load(Ordering::Relaxed),
            self.max_metrics_buffered,
        )
    }
}

impl Stream for Receiver {
    type Item = Metric;
    type Error = ();   // Void
    fn poll(&mut self) -> Result<Async<Option<Metric>>, ()> {
        match self.channel.poll() {
            Ok(Async::Ready(Some(x))) => {
                self.buffered.fetch_sub(1, Ordering::Relaxed);
                Ok(Async::Ready(Some(x)))
            }
            y => y,
        }
    }
}

impl Receiver {
    pub fn is_done(&self) -> bool {
        self.channel.is_done()
    }
}

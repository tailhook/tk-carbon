use std::fmt::{self, Display};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use num_traits::Num;

use element::{Metric};
use channel::{channel, Sender};
use {Init, Config};

/// A structure that is used to submit values to carbon
///
/// Internally it uses a state machine to communicate to the underlying
/// network connection(s)
#[derive(Clone)]
pub struct Carbon {
    chan: Sender,
}

impl Carbon {
    /// This creates an instance of the Carbon public interface and `Init`
    /// structure that can be used to initialize a Proto instance
    pub fn new(config: &Arc<Config>) -> (Carbon, Init) {
        let (tx, rx) = channel(config.max_metrics_buffered);
        return (
            Carbon {
                chan: tx,
            },
            Init {
                chan: rx,
                config: config.clone(),
            }
        )
    }
    /// Add any numeric value for carbon with current timestamp
    ///
    /// # Example
    ///
    /// ```ignore
    /// carbon.add_value("my.metric", 1);
    /// carbon.add_value(
    ///     format_args!("metrics.{host}.cpu", host),
    ///     27);
    /// ```
    ///
    /// # Panics
    ///
    /// * When either name or value can't be formatted (Display'd)
    /// * When formatted name contains a whitespace or a newline
    pub fn add_value<N, V>(&self, name:N, value: V)
        where N: Display, V: Num + Display
    {
        self.add_value_at(name, value, SystemTime::now());
    }

    /// Add any numeric value for carbon with specific timestamp
    ///
    /// # Example
    ///
    /// ```ignore
    /// let timestamp = SystemTime::now();
    /// carbon.add_value_at("my.metric", 1, timestamp);
    /// carbon.add_value_at(
    ///     format_args!("metrics.{host}.cpu", host),
    ///     27, timestamp);
    /// ```
    ///
    /// # Panics
    ///
    /// * When either name or value can't be formatted (Display'd)
    /// * When formatted name contains a whitespace or a newline
    /// * If timestamp is smaller than UNIX_EPOCH
    pub fn add_value_at<N, V>(&self, name: N, value: V, ts: SystemTime)
        where N: Display, V: Num + Display
    {
        let mut buf = Vec::with_capacity(100);
        let tm = ts.duration_since(UNIX_EPOCH)
            .expect("time is larger than epoch");
        writeln!(&mut buf, "{} {} {}", name, value, tm.as_secs())
            .expect("writing to buffer always succeed");
        assert!(buf.iter()
            .filter(|&&x| x == b' ' || x == b'\n' || x == b'\n')
            .count() == 3, // exactly two spaces and a newline at the end
            "Metric should not contain any spaces or newlines inside");
        self.chan.send(Metric(buf));
    }
}

impl fmt::Debug for Carbon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (a, b) = self.chan.buffered();
        write!(f, "Carbon({}/{})", a, b)
    }
}

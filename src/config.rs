use std::time::Duration;

use {Config};

pub fn to_ms(dur: Duration) -> u64 {
    return dur.as_secs() + (dur.subsec_nanos() / 1000000) as u64;
}

pub fn from_ms(ms: u64) -> Duration {
    return Duration::new(ms / 1000, (ms % 1000) as u32 * 1000000);
}


impl Config {
    /// Create the config builder with all defaults
    pub fn new() -> Config {
        Config {
            write_timeout: Duration::new(10, 0),
            watermarks: (60_000, 1_048_576),
            max_metrics_buffered: 10000,

            reconnect_delay: (50, 150),
        }
    }

    /// Set the reconnect delay
    ///
    /// Actual delay is chosen each time as a random millisecond
    /// from 0.5 of this value to 1.5 for this value.
    ///
    /// Note: reconnect delay is calculated since previous connection attempt.
    /// I.e. if connection is broken after a minute of normal work it will
    /// reconnect immediately.
    pub fn reconnect_delay(&mut self, delay: Duration) -> &mut Self {
        let ms = to_ms(delay);
        self.reconnect_delay = (delay/2, delay*3/2);
        self
    }

    /// Set the reconnect delay bounds to more specific values
    ///
    /// Actual delay is chosen each time as a random millisecond
    /// from the range of `min_delay..max_delay`. Usually it's enough to set
    /// `reconnect_delay` but you can use this method if you need more tight
    /// control.
    ///
    /// Note: reconnect delay is calculated since previous connection attempt.
    /// I.e. if connection is broken after a minute of normal work it will
    /// reconnect immediately.
    pub fn reconnect_delay_min_max(&mut self,
        min_delay: Duration, max_delay: Duration)
        -> &mut Self
    {
        self.reconnect_delay = (to_ms(min_delay), to_ms(max_delay));
        self
    }

    /// Timeout of writing at least some byte when there are any bytes in the
    /// outgoing buffer
    pub fn write_timeout(&mut self, dur: Duration) -> &mut Self {
        self.write_timeout = dur;
        self
    }

    /// Buffer limits or watermarks
    ///
    /// The rules of thumb to not to loose any metrics:
    ///
    /// * Low watermark is efficient buffer size for sending to network
    /// * High watermark is maximum buffer size, should be much larger to
    ///   compensate when individual host becomes slower
    ///
    /// # Details
    ///
    /// When number of bytes buffered reaches low watermark (in all
    /// connections) we stop pulling metrics from the internal channel.
    /// The latter will start to drop messages after `max_metrics_buffered`
    /// is reached.
    ///
    /// When number of bytes buffered reaches high watermark in any specific
    /// connection we drop connection entirely, as this means connection can't
    /// keep up with the traffic (but `write_timeout` has not reached for
    /// some reason, for example it accepts bytes in small chunks).
    ///
    /// High water mark should be at least one metric larger than low watermark
    /// or connection will be dropped whenever watermark is reached. For
    /// single connection high watermark should still be higher, but otherwise
    /// there is no chance it will be reached beyond single metric.
    ///
    /// # Panics
    ///
    /// Panics if high watermark is smaller than low watermark or watermark
    /// is zero.
    pub fn watermarks(&mut self, low: usize, high: usize) -> &mut Self {
        assert!(low > 0);
        assert!(high >= low);
        self.watermarks = (low, high);
        self
    }

    /// Maximum metrics buffered in a channel
    ///
    /// The rule of thumb: this channel should contain as much metrics as might
    /// be sent within the period of the downtime + reconnect time for the
    /// carbon backend (in order to not to loose any values). Probably 10
    /// seconds or 30 seconds worth of metrics at least.
    ///
    /// This buffer is common for all the underlying channel between `Carbon`
    /// instance and the actual `Proto` or `Pool`. This channel is single
    /// one for all underlying connections.
    pub fn max_metrics_buffered(&mut self, metrics: usize) -> &mut Self {
        self.max_metrics_buffered = metrics;
        self
    }

    /// Create a Arc'd config clone to pass to the constructor
    ///
    /// This is just a convenience method.
    pub fn done(&mut self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}

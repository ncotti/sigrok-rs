//! Packets

use std::os::raw::c_void;
use std::time::Duration;

use libsigrok_sys::sigrok::{self as sr};

use std::fmt;

/// Header of a sigrok data feed.
///
/// The header is the first packet ever received for a data feed.
#[derive(Debug, Clone, Copy)]
pub struct HeaderPacket {
    /// Data feed version
    feed_version: u32,
    /// Time when the data feed started
    time: Duration,
}

impl From<*const c_void> for HeaderPacket {
    fn from(payload: *const c_void) -> Self {
        let p_feed_version: *const u32 = payload.cast();
        let feed_version: u32 = unsafe { p_feed_version.read() };

        let p_time_seconds: *const u32 = unsafe { payload.byte_offset(4).cast() };
        let time_seconds: u64 = unsafe { p_time_seconds.read() } as u64;

        let p_time_us: *const u32 = unsafe { payload.byte_offset(8).cast() };
        let time_us: u64 = unsafe { p_time_us.read() } as u64;

        HeaderPacket {
            feed_version: feed_version,
            time: Duration::from_micros(time_us + time_seconds * 1_000_000),
        }
    }
}

impl fmt::Display for HeaderPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Packet header v{} @ {}s",
            self.feed_version,
            self.time.as_secs_f32()
        )
    }
}

/// Given "N" logic channels, where `1 <= N <= 16`, the `data` vector holds in
/// each bit the logical state of each channel, ordered by channel index,
/// as such:
///
///         D7 D6 D5 D4 D3 D2 D1 D0
// 0x0E      0  0  0  0  1  1  1  0
// 0xF6      1  1  1  1  0  1  1  0
// 0xD2      1  1  0  1  0  0  1  0
pub struct LogicPacket {
    /// Logic data from each logic channel, up to 16 channels
    data: Vec<u8>,
}

impl From<*const c_void> for LogicPacket {
    fn from(payload: *const c_void) -> Self {
        let logic = unsafe { &*(payload as *const sr::sr_datafeed_logic) };

        // How many samples have been taken from the logic channels
        let length: u64 = logic.length;

        // The size in bytes of the sample, depends on the number of
        // channels the device (up to 8 channels -> 1 byte).
        let unit_size: u16 = logic.unitsize;

        // Samples from the device.
        let mut p_data: *const u8 = logic.data as *const u8;

        let mut data: Vec<u8> = Vec::new();

        for _i in 0..length {
            let channel_values: u8 = unsafe { p_data.read() };
            data.push(channel_values);
            p_data = unsafe { p_data.byte_offset(unit_size as isize) };
        }

        LogicPacket { data: data }
    }
}

impl LogicPacket {
    /// Returns all the values corresponding to a single channel. Channels
    /// are identified by an index, from 0 (LSB) to 7 (MSB).
    pub fn get_channel_data(&self, index: u8) -> Vec<bool> {
        let mut channel_data: Vec<bool> = Vec::new();
        for value in &self.data {
            channel_data.push(value >> index & 0b1 != 0);
        }
        channel_data
    }
}

impl fmt::Display for LogicPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel_data: [Vec<bool>; 8] = [
            self.get_channel_data(0),
            self.get_channel_data(1),
            self.get_channel_data(2),
            self.get_channel_data(3),
            self.get_channel_data(4),
            self.get_channel_data(5),
            self.get_channel_data(6),
            self.get_channel_data(7),
        ];

        for start in (0..channel_data[0].len()).step_by(80) {
            for i in 0..8 {
                write!(f, "D{} ", i)?;

                let end = (start + 80).min(channel_data[i].len());

                for &value in &channel_data[i][start..end] {
                    write!(f, "{}", if value { '1' } else { '0' })?;
                }

                writeln!(f)?;
            }

            writeln!(f)?;
        }
        Ok(())
    }
}

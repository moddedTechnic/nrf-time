#![no_std]

mod duration;
mod instant;
mod tick;
mod time_driver;

pub use duration::Duration;
pub use instant::Instant;
use tick::*;
pub use time_driver::RtcDriver;

extern "Rust" {
    fn _nrf_time_now() -> u64;
}

pub fn now() -> u64 {
    unsafe { _nrf_time_now() }
}

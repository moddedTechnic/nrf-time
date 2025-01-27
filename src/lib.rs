#![no_std]

mod duration;
mod instant;
mod tick;
mod time_driver;
mod timer;

pub use duration::Duration;
pub use instant::Instant;
use tick::*;
pub use time_driver::RtcDriver;
pub use timer::{Ticker, Timer, WithTimeout};

extern "Rust" {
    fn _nrf_time_now() -> u64;
}

pub fn now() -> u64 {
    unsafe { _nrf_time_now() }
}

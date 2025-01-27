use core::sync::atomic::{compiler_fence, AtomicU32, Ordering};
use nrf52833_hal::{rtc, Rtc};
use nrf52833_hal::pac::NVIC;

/// Calculate the timestamp from the period count and the tick count.
///
/// The RTC counter is 24 bit. Ticking at 32768hz, it overflows every ~8 minutes. This is
/// too short. We must make it "never" overflow.
///
/// The obvious way would be to count overflow periods. Every time the counter overflows,
/// increase a `periods` variable. `now()` simply does `periods << 24 + counter`. So, the logic
/// around an overflow would look like this:
///
/// ```not_rust
/// periods = 1, counter = 0xFF_FFFE --> now = 0x1FF_FFFE
/// periods = 1, counter = 0xFF_FFFF --> now = 0x1FF_FFFF
/// **OVERFLOW**
/// periods = 2, counter = 0x00_0000 --> now = 0x200_0000
/// periods = 2, counter = 0x00_0001 --> now = 0x200_0001
/// ```
///
/// The problem is this is vulnerable to race conditions if `now()` runs at the exact time an
/// overflow happens.
///
/// If `now()` reads `periods` first and `counter` later, and overflow happens between the reads,
/// it would return a wrong value:
///
/// ```not_rust
/// periods = 1 (OLD), counter = 0x00_0000 (NEW) --> now = 0x100_0000 -> WRONG
/// ```
///
/// It fails similarly if it reads `counter` first and `periods` second.
///
/// To fix this, we define a "period" to be 2^23 ticks (instead of 2^24). One "overflow cycle" is 2 periods.
///
/// - `period` is incremented on overflow (at counter value 0)
/// - `period` is incremented "midway" between overflows (at counter value 0x80_0000)
///
/// Therefore, when `period` is even, counter is in 0..0x7f_ffff. When odd, counter is in 0x80_0000..0xFF_FFFF
/// This allows for now() to return the correct value even if it races an overflow.
///
/// To get `now()`, `period` is read first, then `counter` is read. If the counter value matches
/// the expected range for the `period` parity, we're done. If it doesn't, this means that
/// a new period start has raced us between reading `period` and `counter`, so we assume the `counter` value
/// corresponds to the next period.
///
/// `period` is a 32bit integer, so It overflows on 2^32 * 2^23 / 32768 seconds of uptime, which is 34865
/// years. For comparison, flash memory like the one containing your firmware is usually rated to retain
/// data for only 10-20 years. 34865 years is long enough!
fn calc_now(period: u32, counter: u32) -> u64 {
    ((period as u64) << 23) + ((counter ^ ((period & 1) << 23)) as u64)
}

#[macro_export]
macro_rules! time_init {
    // Take the name of the RTC peripheral
    ($name:ident: $RTC:ident) => {
        mod $name {
            use ::core::cell::RefCell;
            use ::cortex_m::interrupt::Mutex;
            use ::nrf52833_hal::{pac, Rtc};
            use $crate::RtcDriver;

            static DRIVER: Mutex<RefCell<Option<RtcDriver<pac::$RTC>>>> = Mutex::new(RefCell::new(None));

            pub fn init(rtc: Rtc<pac::$RTC>, nvic: &mut pac::NVIC) {
                cortex_m::interrupt::free(|cs| {
                    DRIVER.borrow(cs).replace(Some(RtcDriver::new(rtc, nvic)));
                });
            }

            pub fn now() -> u64 {
                cortex_m::interrupt::free(|cs| {
                    DRIVER.borrow(cs)
                        .borrow()
                        .as_ref()
                        .expect("Time driver not initialized")
                        .now()
                })
            }
        }
    };
}

pub struct RtcDriver<RTC: rtc::Instance> {
    rtc: Rtc<RTC>,
    /// Number of 2^23 periods elapsed since boot.
    period: AtomicU32,
}

impl<RTC: rtc::Instance> RtcDriver<RTC> {
    pub fn new(rtc: Rtc<RTC>, nvic: &mut NVIC) -> Self {
        let mut this = Self {
            rtc,
            period: AtomicU32::new(0),
        };
        this.init(nvic);
        this
    }

    fn init(&mut self, nvic: &mut NVIC) {
        self.rtc.set_compare(rtc::RtcCompareReg::Compare2, 0x800000).unwrap();

        self.rtc.clear_counter();
        self.rtc.enable_counter();

        // Wait for clear
        while self.rtc.get_counter() != 0 {}

        self.rtc.enable_interrupt(rtc::RtcInterrupt::Overflow, Some(nvic));
        self.rtc.enable_interrupt(rtc::RtcInterrupt::Compare2, Some(nvic));
    }

    pub fn on_interrupt(&self) {
        if self.rtc.is_event_triggered(rtc::RtcInterrupt::Overflow) {
            self.rtc.reset_event(rtc::RtcInterrupt::Overflow);
            self.next_period();
        }
        if self.rtc.is_event_triggered(rtc::RtcInterrupt::Compare2) {
            self.rtc.reset_event(rtc::RtcInterrupt::Compare2);
            self.next_period();
        }
    }

    fn next_period(&self) {
        let period = self.period.load(Ordering::Relaxed) + 1;
        self.period.store(period, Ordering::Relaxed);
    }

    pub fn now(&self) -> u64 {
        // `period` MUST be read before `counter`, see comment at the top for details.
        let period = self.period.load(Ordering::Relaxed);
        compiler_fence(Ordering::Acquire);
        let counter = self.rtc.get_counter();
        calc_now(period, counter)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calc_now() {
        assert_eq!(calc_now(0, 0x000000), 0x0_000000);
        assert_eq!(calc_now(0, 0x000001), 0x0_000001);
        assert_eq!(calc_now(0, 0x7FFFFF), 0x0_7FFFFF);
        assert_eq!(calc_now(1, 0x7FFFFF), 0x1_7FFFFF);
        assert_eq!(calc_now(0, 0x800000), 0x0_800000);
        assert_eq!(calc_now(1, 0x800000), 0x0_800000);
        assert_eq!(calc_now(1, 0x800001), 0x0_800001);
        assert_eq!(calc_now(1, 0xFFFFFF), 0x0_FFFFFF);
        assert_eq!(calc_now(2, 0xFFFFFF), 0x1_FFFFFF);
        assert_eq!(calc_now(1, 0x000000), 0x1_000000);
        assert_eq!(calc_now(2, 0x000000), 0x1_000000);
    }
}

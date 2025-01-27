use super::Timer;

/// Type implementing async delays
///
/// The delays are implemented in a "best-effort" way, meaning that the cpu will block for at least
/// the amount provided, but accuracy can be affected by many factors, including interrupt usage.
/// Make sure to use a suitable tick rate for your use case. The tick rate is defined by the currently
/// active driver.
#[derive(Clone)]
pub struct Delay;

impl embedded_hal_async::delay::DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        Timer::after_nanos(ns as _).await
    }

    async fn delay_us(&mut self, us: u32) {
        Timer::after_micros(us as _).await
    }

    async fn delay_ms(&mut self, ms: u32) {
        Timer::after_millis(ms as _).await
    }
}
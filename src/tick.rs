const fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub(crate) const TICK_HZ: u64 = 32_768;
pub(crate) const GCD_1K: u64 = gcd(TICK_HZ, 1_000);
pub(crate) const GCD_1M: u64 = gcd(TICK_HZ, 1_000_000);
pub(crate) const GCD_1G: u64 = gcd(TICK_HZ, 1_000_000_000);
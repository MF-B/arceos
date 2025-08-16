use lazyinit::LazyInit;
use loongArch64::time::Time;

static NANOS_PER_TICK: LazyInit<u64> = LazyInit::new();

/// RTC wall time offset in nanoseconds at monotonic time base.
static mut RTC_EPOCHOFFSET_NANOS: u64 = 0;

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn current_ticks() -> u64 {
    Time::read() as _
}

/// Return epoch offset in nanoseconds (wall time offset to monotonic clock start).
#[inline]
pub fn epochoffset_nanos() -> u64 {
    unsafe { RTC_EPOCHOFFSET_NANOS }
}

/// Converts hardware ticks to nanoseconds.
#[inline]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    ticks * *NANOS_PER_TICK
}

/// Converts nanoseconds to hardware ticks.
#[inline]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    nanos / *NANOS_PER_TICK
}

/// Set a one-shot timer.
///
/// A timer interrupt will be triggered at the given deadline (in nanoseconds).
pub fn set_oneshot_timer(deadline_ns: u64) {
    let current_ns = crate::time::monotonic_time_nanos();
    if deadline_ns <= current_ns {
        loongArch64::register::tcfg::set_init_val(1); // trigger immediately
    } else {
        let ticks = nanos_to_ticks(deadline_ns - current_ns) as usize;
        loongArch64::register::tcfg::set_init_val(ticks);
    }
    loongArch64::register::tcfg::set_periodic(false);
    loongArch64::register::tcfg::set_en(true);
}

pub(super) fn init_primary() {
    NANOS_PER_TICK
        .init_once(crate::time::NANOS_PER_SEC / loongArch64::time::get_timer_freq() as u64);
}

pub(super) fn init_percpu() {
    // Use the arch-common timer setup
    use loongArch64::register::tcfg;
    tcfg::set_en(true);
    super::irq::set_enable(super::irq::TIMER_IRQ_NUM, true);
}

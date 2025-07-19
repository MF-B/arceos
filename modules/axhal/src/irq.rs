//! Interrupt management.

use core::sync::atomic::{AtomicU64, Ordering};
use handler_table::HandlerTable;

use crate::platform::irq::{MAX_IRQ_COUNT, dispatch_irq};
use crate::trap::{IRQ, register_trap_handler};

pub use crate::platform::irq::{register_handler, set_enable};

/// The type if an IRQ handler.
pub type IrqHandler = handler_table::Handler;

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

static IRQ_COUNTERS: [AtomicU64; MAX_IRQ_COUNT] = {
    const INIT: AtomicU64 = AtomicU64::new(0);
    [INIT; MAX_IRQ_COUNT]
};

fn increment_irq_count(irq_num: usize) {
    #[cfg(target_arch = "riscv64")]
    let actual_irq = if irq_num >= (1 << (usize::BITS - 1)) {
        irq_num - (1 << (usize::BITS - 1))
    } else {
        irq_num
    };

    #[cfg(not(target_arch = "riscv64"))]
    let actual_irq = irq_num;

    if actual_irq < MAX_IRQ_COUNT {
        IRQ_COUNTERS[actual_irq].fetch_add(1, Ordering::Relaxed);
    }
}

/// Platform-independent IRQ dispatching.
#[allow(dead_code)]
pub(crate) fn dispatch_irq_common(irq_num: usize) {
    trace!("IRQ {}", irq_num);
    if !IRQ_HANDLER_TABLE.handle(irq_num) {
        warn!("Unhandled IRQ {}", irq_num);
    }
}

/// Platform-independent IRQ handler registration.
///
/// It also enables the IRQ if the registration succeeds. It returns `false` if
/// the registration failed.
#[allow(dead_code)]
pub(crate) fn register_handler_common(irq_num: usize, handler: IrqHandler) -> bool {
    if irq_num < MAX_IRQ_COUNT && IRQ_HANDLER_TABLE.register_handler(irq_num, handler) {
        set_enable(irq_num, true);
        return true;
    }
    warn!("register handler for IRQ {} failed", irq_num);
    false
}

#[register_trap_handler(IRQ)]
fn handler_irq(irq_num: usize) -> bool {
    let guard = kernel_guard::NoPreempt::new();
    increment_irq_count(irq_num);
    dispatch_irq(irq_num);
    drop(guard); // rescheduling may occur when preemption is re-enabled.
    true
}

/// Get the count of all IRQs.
pub fn get_all_irq_counts() -> [u64; MAX_IRQ_COUNT] {
    let mut counts = [0u64; MAX_IRQ_COUNT];
    for (i, counter) in IRQ_COUNTERS.iter().enumerate() {
        counts[i] = counter.load(Ordering::Relaxed);
    }
    counts
}

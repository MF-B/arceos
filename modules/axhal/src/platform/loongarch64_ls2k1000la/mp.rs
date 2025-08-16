use crate::mem::phys_to_virt;

pub static mut SMP_BOOT_STACK_TOP: usize = 0;

/// Returns the ID of the current CPU.
pub fn cpu_id() -> usize {
    loongArch64::register::cpuid::read().core_id() as usize
}

/// Returns the maximum number of CPUs.
pub fn max_cpu_count() -> usize {
    axconfig::SMP as usize
}

/// Starts the given secondary CPU with its boot stack.
pub fn start_secondary_cpu(cpu_id: usize, stack_top: crate::mem::PhysAddr) {
    unsafe extern "C" {
        fn _start_secondary();
    }
    let stack_top_virt_addr = phys_to_virt(stack_top).as_usize();
    unsafe {
        SMP_BOOT_STACK_TOP = stack_top_virt_addr;
    }
    // TODO: Implement LS2K1000LA specific CPU startup
    axlog::ax_println!("Starting CPU {} (stack at {:#x})...", cpu_id, stack_top_virt_addr);
}
/// Terminate the current thread.
pub fn terminate() -> ! {
    axlog::ax_println!("Shutting down...");
    loop {
        crate::arch::halt();
    }
}

/// Reboot the system.
pub fn reboot() -> ! {
    axlog::ax_println!("Rebooting...");
    loop {
        crate::arch::halt();
    }
}
use kspin::SpinNoIrq;
use lazyinit::LazyInit;
use ns16550a::Uart;

// const UART_BASE: PhysAddr = pa!(axconfig::devices::UART_PADDR);

static UART: LazyInit<SpinNoIrq<Uart>> = LazyInit::new();

/// Writes bytes to the console from input u8 slice.
pub fn write_bytes(bytes: &[u8]) {
    let uart = UART.lock();
    bytes.iter().for_each(|&c| {
        if c == b'\n' {
            // Send carriage return before newline
            while uart.put(b'\r') == None {}
        }
        while uart.put(c) == None {}
    });
}

/// Reads bytes from the console into the given mutable slice.
/// Returns the number of bytes read.
pub fn read_bytes(bytes: &mut [u8]) -> usize {
    for (i, byte) in bytes.iter_mut().enumerate() {
        match UART.lock().get() {
            Some(c) => *byte = c,
            None => return i,
        }
    }
    bytes.len()
}

/// Early stage initialization for ns16550a
pub(super) fn init_early() {
//    let vaddr = phys_to_virt(UART_BASE);
    let vaddr = va!(0x8000_0000_1fe2_0000);
    //    let vaddr = va!(0x9000_0000_afe2_0000);
    let uart = Uart::new(vaddr.as_usize());
    UART.init_once(SpinNoIrq::new(uart));
}

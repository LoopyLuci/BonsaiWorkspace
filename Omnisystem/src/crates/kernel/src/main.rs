#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod vga_buffer;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    vga_buffer::clear_screen();
    vga_buffer::print_str("USOS Co-OS Kernel v0.1\n");
    vga_buffer::print_str("Bonsai Ecosystem ready.\n");
    vga_buffer::print_str("CPU: Ryzen 9 5900X\n");
    vga_buffer::print_str("RAM: 64 GB\n");
    vga_buffer::print_str("GPU: RX 7900 XTX 24GB\n");
    vga_buffer::print_str("\nKernel initialized.\n");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    vga_buffer::print_str("Kernel panic!\n");
    loop {}
}

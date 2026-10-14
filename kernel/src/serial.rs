//! Minimal 16550 UART driver for COM1.
//!
//! This is the kernel's only output device until the framebuffer can render
//! text, and it stays the primary debugging channel afterwards: on real
//! hardware there is no debugger attached, so a serial cable is the only way to
//! see what the machine was doing before it stopped.

use core::arch::asm;
use core::fmt::{self, Write};

use spin::Mutex;

/// I/O port of the first serial controller on the PC platform.
const COM1: u16 = 0x3f8;

/// Divisor applied to the UART's 115200 Hz clock. 3 gives 38400 baud.
const BAUD_DIVISOR: u16 = 3;

// Register offsets from the port base. Several registers are multiplexed on the
// same offset depending on the DLAB bit in the line control register.
const DATA: u16 = 0;
const INTERRUPT_ENABLE: u16 = 1;
const DIVISOR_LOW: u16 = 0;
const DIVISOR_HIGH: u16 = 1;
const FIFO_CONTROL: u16 = 2;
const LINE_CONTROL: u16 = 3;
const MODEM_CONTROL: u16 = 4;
const LINE_STATUS: u16 = 5;

/// Line control bit that remaps offsets 0 and 1 onto the divisor registers.
const LCR_DLAB: u8 = 0x80;
/// 8 data bits, no parity, one stop bit.
const LCR_8N1: u8 = 0x03;
/// Line status bit set when the transmit holding register is free.
const LSR_TRANSMIT_EMPTY: u8 = 0x20;

/// # Safety
/// The caller must ensure `port` addresses a device that tolerates this write.
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

/// # Safety
/// The caller must ensure `port` addresses a device that tolerates this read.
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    const fn new(base: u16) -> Self {
        Self { base }
    }

    fn init(&mut self) {
        unsafe {
            outb(self.base + INTERRUPT_ENABLE, 0x00);
            outb(self.base + LINE_CONTROL, LCR_DLAB);
            outb(self.base + DIVISOR_LOW, BAUD_DIVISOR as u8);
            outb(self.base + DIVISOR_HIGH, (BAUD_DIVISOR >> 8) as u8);
            outb(self.base + LINE_CONTROL, LCR_8N1);
            // Enable the FIFOs, clear them, and set a 14-byte trigger level.
            outb(self.base + FIFO_CONTROL, 0xc7);
            // Data terminal ready, request to send, auxiliary output 2.
            outb(self.base + MODEM_CONTROL, 0x0b);
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { inb(self.base + LINE_STATUS) & LSR_TRANSMIT_EMPTY != 0 }
    }

    fn write_byte(&mut self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        unsafe { outb(self.base + DATA, byte) }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            // Terminals expect CRLF; the rest of the kernel only emits '\n'.
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

static PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1));

pub fn init() {
    PORT.lock().init();
}

/// Release the serial lock without owning it.
///
/// # Safety
/// Only sound when no other execution context can be mid-write. The panic
/// handler calls this because a panic may have interrupted a thread that held
/// the lock, and a deadlocked panic handler prints nothing at all.
pub unsafe fn force_unlock() {
    unsafe { PORT.force_unlock() }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let _ = PORT.lock().write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

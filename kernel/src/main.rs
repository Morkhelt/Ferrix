//! The Ferrix kernel.
//!
//! Boots under the Limine protocol, brings up serial output, and paints the
//! framebuffer. Nothing else is set up yet: no interrupts, no paging of our
//! own, no allocator.

#![no_std]
#![no_main]

mod framebuffer;
mod serial;

use core::panic::PanicInfo;

use limine::memmap::MEMMAP_USABLE;
use limine::request::{BootloaderInfoRequest, FramebufferRequest, MemmapRequest, StackSizeRequest};
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker};

/// Stack Limine should allocate for the boot processor before calling `kmain`.
const STACK_SIZE: u64 = 128 * 1024;

// Limine scans the loaded image for these structures and writes response
// pointers back into them, which is why they are `#[used]` (nothing in the
// kernel reads them at link time) and why they sit in `.requests` (a writable
// section the linker script places between the two markers).

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static STACK: StackSizeRequest = StackSizeRequest::new(STACK_SIZE);

#[used]
#[unsafe(link_section = ".requests")]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMMAP: MemmapRequest = MemmapRequest::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

/// Entry point. Named in `linker.ld`; Limine jumps here with interrupts
/// disabled, the kernel mapped, and a stack already set up.
#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    serial::init();

    println!();
    println!("Ferrix 0.1.0");

    // A bootloader that does not speak our base revision may have laid memory
    // out differently than this kernel assumes, so trusting the other responses
    // would be guesswork.
    if !BASE_REVISION.is_supported() {
        println!(
            "fatal: bootloader does not support Limine base revision {}, it offers {:?}",
            BaseRevision::MAX_SUPPORTED,
            BASE_REVISION.actual_revision(),
        );
        halt();
    }

    if let Some(info) = BOOTLOADER_INFO.response() {
        println!("booted by {} {}", info.name(), info.version());
    }

    report_memory();
    paint_screen();

    println!("kernel reached the end of kmain, halting");
    halt();
}

/// Total the usable regions of Limine's memory map.
///
/// This is only a report for now, but the same map is what the physical frame
/// allocator will consume next.
fn report_memory() {
    let Some(memmap) = MEMMAP.response() else {
        println!("warning: no memory map; cannot size physical memory");
        return;
    };

    let entries = memmap.entries();
    let usable: u64 = entries
        .iter()
        .filter(|entry| entry.type_ == MEMMAP_USABLE)
        .map(|entry| entry.length)
        .sum();

    println!(
        "memory: {} MiB usable across {} regions ({} map entries)",
        usable / (1024 * 1024),
        entries.iter().filter(|e| e.type_ == MEMMAP_USABLE).count(),
        entries.len(),
    );
}

/// Paint the first framebuffer Limine gave us, if there is one.
fn paint_screen() {
    let Some(response) = FRAMEBUFFER.response() else {
        println!("warning: no framebuffer response; running headless");
        return;
    };

    let Some(fb) = response.framebuffers().first() else {
        println!("warning: bootloader reported zero framebuffers");
        return;
    };

    println!(
        "framebuffer: {}x{} at {} bpp, pitch {}",
        fb.width, fb.height, fb.bpp, fb.pitch
    );

    if fb.bpp != framebuffer::SUPPORTED_BPP {
        println!(
            "warning: only {} bpp is implemented, leaving the screen alone",
            framebuffer::SUPPORTED_BPP
        );
        return;
    }

    framebuffer::draw_test_pattern(fb);
    println!("framebuffer: test pattern drawn");
}

/// Stop this processor for good, with interrupts masked so nothing wakes it.
fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack)) };
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // The panic may have interrupted code that held the serial lock. A
    // deadlocked panic handler prints nothing, which is strictly worse than
    // interleaved output.
    unsafe { serial::force_unlock() };

    println!();
    println!("*** KERNEL PANIC ***");
    println!("{info}");
    halt();
}

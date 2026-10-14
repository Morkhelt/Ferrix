//! Direct writes to the linear framebuffer Limine hands us.
//!
//! Limine sets the video mode before the kernel runs, so there is no mode
//! setting to do here: the response carries a pointer to the pixel buffer and
//! enough geometry to address it. Everything in this module is temporary
//! scaffolding to prove the framebuffer is real; the eventual compositor will
//! replace it.

use limine::framebuffer::Framebuffer;

/// Pixel formats other than 32 bits per pixel need a narrower store than the
/// `u32` write below, so they are reported rather than corrupted.
pub const SUPPORTED_BPP: u16 = 32;

/// Scale an 8-bit colour component into this framebuffer's channel width and
/// shift it into position.
fn channel(value: u8, mask_size: u8, mask_shift: u8) -> u32 {
    let max = (1u32 << mask_size) - 1;
    ((u32::from(value) * max) / 255) << mask_shift
}

/// Pack 8-bit RGB into one pixel in the framebuffer's own format.
fn pack(fb: &Framebuffer, r: u8, g: u8, b: u8) -> u32 {
    channel(r, fb.red_mask_size, fb.red_mask_shift)
        | channel(g, fb.green_mask_size, fb.green_mask_shift)
        | channel(b, fb.blue_mask_size, fb.blue_mask_shift)
}

/// Fill the screen with a two-axis gradient.
///
/// A gradient rather than a flat fill because it proves more at a glance: a
/// wrong pitch shears the image, a wrong channel mask tints it, and a wrong
/// base address shows nothing at all.
pub fn draw_test_pattern(fb: &Framebuffer) {
    let width = fb.width as usize;
    let height = fb.height as usize;
    let pitch = fb.pitch as usize;
    let base = fb.address().cast::<u8>();

    if width == 0 || height == 0 {
        return;
    }

    for y in 0..height {
        let row = unsafe { base.add(y * pitch) };
        let green = (y * 255 / height) as u8;
        for x in 0..width {
            let red = (x * 255 / width) as u8;
            let pixel = pack(fb, red, green, 0x60);
            // Safety: `x` and `y` are inside the geometry Limine reported, so
            // this address lies within the mapped framebuffer. Volatile because
            // the compiler must not coalesce or drop writes to device memory.
            unsafe { row.add(x * 4).cast::<u32>().write_volatile(pixel) };
        }
    }
}

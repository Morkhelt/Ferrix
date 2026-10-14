# Ferrix

A UNIX-like operating system for x86_64, written in Rust.

Ferrix boots on real hardware under the [Limine](https://codeberg.org/Limine/Limine)
boot protocol. Right now it brings up serial logging, reads the firmware memory
map and paints the framebuffer; everything past that is still to come.

## Status

Boots under both BIOS and UEFI and reaches Rust code. On a successful boot it
logs:

```
Ferrix 0.1.0
booted by Limine 11.4.1
memory: 510 MiB usable across 3 regions (17 map entries)
framebuffer: 1280x800 at 32 bpp, pitch 5120
framebuffer: test pattern drawn
```

There are no interrupts, no page tables of our own, no heap and no userspace
yet.

## Building

You need a POSIX-ish host with `rustup`, `make`, `git`, `xorriso`, `qemu` and,
for the UEFI path, `ovmf`. On Debian or Ubuntu:

```sh
sudo apt install build-essential xorriso qemu-system-x86 ovmf
```

The Rust toolchain pins itself: `rust-toolchain.toml` names the exact nightly
and the `x86_64-unknown-none` target, so `rustup` installs both on first build
and a `rustup update` cannot silently change the compiler under you.

```sh
make            # build the kernel and ferrix.iso
make run        # boot it in QEMU under BIOS, kernel log on your terminal
make run-uefi   # the same under UEFI firmware, which is what real hardware uses
make test       # boot headless under both and check the kernel log
make check      # clippy and the boot test, what CI runs
```

`make` clones the Limine binary release into `limine/` on first run. It is not
vendored, and it is not in version control.

### Running it on real hardware

`ferrix.iso` is a hybrid image: write it to a USB stick with `dd` and boot from
it. Connect a serial cable and watch COM1 at 38400 baud, 8N1 — until the kernel
can render text to the screen, that is the only way to see what it is doing, and
it stays the most useful one afterwards.

## Layout

```
kernel/
  linker.ld        image layout: higher half, one segment per permission set
  build.rs         hands the linker script to rustc
  src/main.rs      Limine requests, entry point, panic handler
  src/serial.rs    16550 UART driver for COM1
  src/framebuffer.rs  direct pixel writes
scripts/
  boot-test.py     boots the ISO in QEMU and asserts on the kernel log
limine.conf        boot menu entry
```

## Design decisions

These are settled, with the reasoning written up separately:

- **Rust, not C.** Both compile through LLVM so performance is a wash. Rust's
  type system rejects the use-after-free, aliasing and data-race bugs that are
  hardest to find on hardware with no debugger attached.
- **UNIX-like, not DOS-like.** A DOS-like system means writing every program
  that will ever run on it. A UNIX-like one means porting a libc and then
  porting everything else.
- **Monolithic, not a microkernel.** Rust already gives us most of what driver
  isolation would buy, without the cost of designing and tuning IPC. Individual
  subsystems can move to userspace later if they earn it.
- **POSIX-shaped, not POSIX-compliant.** Syscalls look like POSIX where that is
  free, because it is what ported software expects. Full signal semantics and
  System V IPC can wait.
- **Limine, not GRUB or a hand-rolled UEFI stub.** It boots both firmwares from
  one image and hands over an initialised framebuffer, the memory map, a
  higher-half mapping and secondary CPUs already started.

## Next

1. GDT, IDT and exception handlers that print a useful register dump.
2. The local APIC timer as a tick source.
3. A physical frame allocator over the Limine memory map, then our own page
   tables, then a heap so `alloc` becomes available.
4. A bitmap font on the framebuffer, then PS/2 input, then a terminal.

## License

MIT. See [LICENSE](LICENSE).

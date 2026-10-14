# Ferrix

A UNIX-like operating system for x86_64, written in Rust, booting on real
hardware under the Limine protocol. This file is the standing context for any
Claude session working in this tree.

## Where the project is

The kernel boots under BIOS and UEFI, logs over COM1, reads the firmware memory
map and paints the framebuffer. That is all. There are **no interrupts, no page
tables of our own, no heap and no userspace yet.**

A successful boot prints:

```
Ferrix 0.1.0
booted by Limine 11.4.1
memory: 510 MiB usable across 3 regions (17 map entries)
framebuffer: 1280x800 at 32 bpp, pitch 5120
framebuffer: test pattern drawn
kernel reached the end of kmain, halting
```

`PROMPTS.md` holds the planned milestones in order. The next one is descriptor
tables and exception handlers.

## Decisions already made — do not reopen these

- **Rust, not C.** Both compile through LLVM, so performance was not the
  deciding factor. Rust's type system rejects the use-after-free, aliasing and
  data-race bugs that are hardest to find on hardware with no debugger.
- **UNIX-like, not DOS-like.** A DOS-like system means writing every program
  that will ever run on it; a UNIX-like one means porting a libc and then
  porting everything else.
- **Monolithic, not a microkernel.** Rust already supplies most of what driver
  isolation would buy, without the cost of designing and tuning IPC. A
  subsystem can move to userspace later if it earns it.
- **POSIX-shaped, not POSIX-compliant.** Syscalls look like POSIX where that is
  free. Full signal semantics and System V IPC can wait.
- **`posix_spawn`-style process creation, not `fork()`,** at least at first.
- **Limine, not GRUB or a hand-rolled UEFI stub.**

The eventual goals are a terminal mode, then a Windows 95 / NeXTSTEP-styled GUI,
and much later a browser and a compatibility layer for Windows binaries. None of
that changes the kernel decisions above.

## Verifying your work

```sh
make          # kernel + ferrix.iso
make test     # boot headless under BIOS and UEFI, assert on the kernel log
make check    # clippy and the boot test — run this before calling anything done
make run      # boot in QEMU with the kernel log on your terminal
```

**Compiling proves almost nothing here; booting proves a lot.** `make test` is
the real regression test. Never finish a change without it passing, and never
leave the kernel unbootable between milestones.

`scripts/boot-test.py --screenshot DIR` captures what the guest had on screen,
which is the only way to check framebuffer work.

## Constraints

- **The toolchain pin in `rust-toolchain.toml` is deliberate.** Do not change
  it. Nightly is required because the `limine` crate uses `ptr_metadata`, and
  `extern "x86-interrupt"` will need `abi_x86_interrupt` shortly.
- **`x86_64-unknown-none` ships precompiled `core` and `alloc`,** so there is no
  cross-compiler to build and no `build-std`. Keep it that way.
- **This tree may be an offline bundle** — see `OFFLINE.md`. If `vendor/` exists
  and `.cargo/config.toml` sets `net.offline`, you cannot add dependencies or
  fetch anything. Work with what is vendored.
- `make distclean` deletes `limine/`, which an offline copy cannot re-fetch.

## Things that cost a debug cycle to learn

- The Limine `.requests` section **must be in a writable segment**. The
  bootloader writes response pointers back into those structures; a read-only
  section faults. The start and end markers must bracket them.
- Without `relocation-model=static`, `code-model=kernel` and `-no-pie` in
  `.cargo/config.toml`, the image comes out PIE with `.rela.dyn` and `.got`.
- `kernel/build.rs` passes the linker script by absolute path on purpose, so it
  resolves wherever cargo is invoked from.
- `[[bin]] test = false, bench = false` in `kernel/Cargo.toml` is required, or
  `cargo clippy --all-targets` fails with "can't find crate for `test`".
- The serial port is behind a spin lock, and the panic handler force-unlocks it
  before printing. Once interrupts exist, a handler that logs while the main
  thread holds that lock will deadlock on a single CPU. Decide deliberately.

## Conventions

- Comments explain **why**, not what. The existing code is the reference for
  density and tone; match it.
- Every `unsafe` block gets the invariant that makes it sound stated above it.
  Wrap hardware access in a safe interface once rather than scattering `unsafe`.
- Serial output stays working no matter what else is added. It is the channel
  that works when the screen does not.
- Prefer a small, verified change over a large speculative one.

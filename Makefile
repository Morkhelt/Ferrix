# Ferrix build and boot.
#
#   make         build the kernel and a bootable ISO
#   make run     boot the ISO in QEMU under BIOS, serial on stdout
#   make run-uefi  the same under UEFI firmware, which is what real hardware uses
#   make test    boot headless under both firmwares and check the kernel log
#   make clean   drop build output, keep the fetched bootloader

LIMINE_BRANCH := v11.x-binary
KERNEL        := target/x86_64-unknown-none/release/ferrix-kernel
ISO           := ferrix.iso
OVMF_CODE     := /usr/share/OVMF/OVMF_CODE_4M.fd

# -serial stdio puts the kernel's log on the terminal. -no-reboot and
# -no-shutdown keep a triple fault visible instead of silently rebooting.
QEMUFLAGS ?= -m 512M -serial stdio -no-reboot -no-shutdown

.PHONY: all kernel run run-uefi test fmt clippy check clean distclean

all: $(ISO)

kernel:
	cargo build --release

# Phony prerequisite: cargo decides whether anything needs rebuilding.
$(KERNEL): kernel

limine:
	git clone https://github.com/limine-bootloader/limine.git \
		--branch=$(LIMINE_BRANCH) --depth=1 limine
	$(MAKE) -C limine

$(ISO): $(KERNEL) limine limine.conf
	rm -rf iso_root
	mkdir -p iso_root/boot/limine iso_root/EFI/BOOT
	cp $(KERNEL) iso_root/boot/ferrix
	cp limine.conf iso_root/boot/limine/
	cp limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin \
		iso_root/boot/limine/
	cp limine/BOOTX64.EFI limine/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -R -r -J \
		-b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(ISO)
	./limine/limine bios-install $(ISO)
	rm -rf iso_root

run: $(ISO)
	qemu-system-x86_64 $(QEMUFLAGS) -cdrom $(ISO)

run-uefi: $(ISO)
	qemu-system-x86_64 $(QEMUFLAGS) -drive if=pflash,unit=0,format=raw,readonly=on,file=$(OVMF_CODE) -cdrom $(ISO)

test: $(ISO)
	python3 scripts/boot-test.py $(ISO)

fmt:
	cargo fmt --all

clippy:
	cargo clippy --release --all-targets -- -D warnings

check: clippy test

clean:
	cargo clean
	rm -rf iso_root $(ISO)

distclean: clean
	rm -rf limine

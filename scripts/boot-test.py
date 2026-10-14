#!/usr/bin/env python3
"""Boot the Ferrix ISO in QEMU and check that the kernel got where it should.

A kernel that compiles tells you very little, so this is the test that matters:
it boots the real image under both firmware paths and asserts on what the
kernel logged over its serial port.

Usage: scripts/boot-test.py ferrix.iso [--mode bios|uefi|both] [--screenshot DIR]
"""

import argparse
import os
import socket
import subprocess
import sys
import tempfile
import time

# Every line the kernel must emit for the boot to count as successful.
EXPECTED = [
    "Ferrix",
    "framebuffer: test pattern drawn",
    "kernel reached the end of kmain",
]

# The line that tells us the kernel is done and the screen is worth capturing.
DONE_MARKER = EXPECTED[-1]

OVMF_CANDIDATES = [
    "/usr/share/OVMF/OVMF_CODE_4M.fd",
    "/usr/share/OVMF/OVMF_CODE.fd",
    "/usr/share/ovmf/OVMF.fd",
]

BOOT_TIMEOUT_SECONDS = 90


def find_ovmf():
    for path in OVMF_CANDIDATES:
        if os.path.exists(path):
            return path
    return None


def run_once(iso, mode, workdir, screenshot_dir):
    """Boot the ISO once and return (ok, serial_output)."""
    monitor = os.path.join(workdir, f"monitor-{mode}.sock")
    serial = os.path.join(workdir, f"serial-{mode}.log")

    command = [
        "qemu-system-x86_64",
        "-m", "512M",
        "-cdrom", iso,
        "-display", "none",
        "-no-reboot",
        "-no-shutdown",
        "-serial", f"file:{serial}",
        "-monitor", f"unix:{monitor},server,nowait",
    ]

    if mode == "uefi":
        firmware = find_ovmf()
        if firmware is None:
            print(f"[{mode}] skipped: no OVMF firmware installed")
            return True, ""
        command += [
            "-drive",
            f"if=pflash,unit=0,format=raw,readonly=on,file={firmware}",
        ]

    qemu = subprocess.Popen(
        command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE
    )

    def serial_text():
        if not os.path.exists(serial):
            return ""
        with open(serial, errors="replace") as handle:
            return handle.read()

    deadline = time.time() + BOOT_TIMEOUT_SECONDS
    finished = False
    while time.time() < deadline:
        if qemu.poll() is not None:
            break
        if DONE_MARKER in serial_text():
            finished = True
            break
        time.sleep(0.5)

    if finished and screenshot_dir and os.path.exists(monitor):
        capture_screen(monitor, os.path.join(screenshot_dir, f"screen-{mode}.ppm"))

    qemu.terminate()
    try:
        qemu.wait(timeout=10)
    except subprocess.TimeoutExpired:
        qemu.kill()

    output = serial_text()
    missing = [line for line in EXPECTED if line not in output]

    print(f"--- {mode} ---")
    print(output if output else "(no serial output)")
    if missing:
        print(f"[{mode}] FAILED, kernel never logged: {missing}")
        stderr = qemu.stderr.read().decode(errors="replace").strip()
        if stderr:
            print(f"[{mode}] qemu stderr:\n{stderr}")
        return False, output

    print(f"[{mode}] ok")
    return True, output


def capture_screen(monitor, destination):
    """Ask the QEMU monitor for a screenshot of the guest display."""
    try:
        with socket.socket(socket.AF_UNIX) as sock:
            sock.connect(monitor)
            time.sleep(0.5)
            sock.sendall(f"screendump {destination}\n".encode())
            time.sleep(2)
    except OSError as error:
        print(f"warning: could not capture the screen: {error}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("iso")
    parser.add_argument("--mode", choices=["bios", "uefi", "both"], default="both")
    parser.add_argument(
        "--screenshot",
        metavar="DIR",
        help="write a screenshot of the guest display into DIR",
    )
    args = parser.parse_args()

    if not os.path.exists(args.iso):
        sys.exit(f"no such image: {args.iso}")

    if args.screenshot:
        os.makedirs(args.screenshot, exist_ok=True)

    modes = ["bios", "uefi"] if args.mode == "both" else [args.mode]

    with tempfile.TemporaryDirectory() as workdir:
        results = [
            run_once(args.iso, mode, workdir, args.screenshot) for mode in modes
        ]

    sys.exit(0 if all(ok for ok, _ in results) else 1)


if __name__ == "__main__":
    main()

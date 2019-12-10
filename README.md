# sigen - standalone EFI executable generator

This utility automates the process of creating self-contained EFI executables for the Linux kernel using systemd-boot's EFI stub.

# Installation

Arch Linux users can use the [AUR package](https://aur.archlinux.org/packages/sigen).

Users of other distros should manually package/install from source (see below). This isn't a huge issue since sigen is just a single executable file.

# Building

A reasonably up-to-date [Rust](https://rust-lang.org) compiler is required.

To build, run `cargo build --release`. The resulting binary will be located at `target/release/sigen`.

# Usage

    sigen [FLAGS] [OPTIONS] --cmdline <cmdline> --kernel <kernel> --output <output>

    FLAGS:
        -h, --help       Prints help information
        -f, --force      Overwrite output file if it already exists
        -V, --version    Prints version information

    OPTIONS:
        -b, --backup <backup>       Make a backup of the previous output if it exists
        -c, --cmdline <cmdline>     Path to file containing the default command line arguments
        -i, --initrd <initrd>...    Path to the initramfs file(s) to include
        -k, --kernel <kernel>       Path to the kernel image
        -o, --output <output>       Path to the output file
        -s, --sign <sign> <sign>    Path to the .key and .crt files (in this order) to sign the executable with

# Example

    sigen -c /boot/cmdline -k /boot/vmlinuz-linux -i /boot/amd-ucode.img -i /boot/initramfs-linux.img -o /boot/efi/linux-signed.efi -s /etc/efi-keys/db.key /etc/efi-keys/db.crt -f

# Automation

To automatically regenerate the EFI executable after each kernel update, you can, for example, use systemd path triggers.

The `sigen.service.example` and `sigen.path.example` files are examples on how to implement this.

---

Copyright © 2019 Joaquim Monteiro

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

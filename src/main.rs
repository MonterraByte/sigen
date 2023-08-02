// Copyright © 2019-2020 Joaquim Monteiro
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, IsTerminal, Write};
use std::iter::Iterator;
use std::path::PathBuf;
use std::process::Command;

use structopt::StructOpt;
use tempfile::NamedTempFile;

macro_rules! os {
    ($s:tt) => {
        std::ffi::OsStr::new($s)
    };
}

/// Creates standalone EFI executables from Linux kernel images
///
/// WARNING: This software is deprecated. Consider using ukify, dracut or mkinitcpio instead.
#[derive(StructOpt)]
#[structopt(author)]
struct Args {
    /// Path to the kernel image
    #[structopt(short, long)]
    kernel: PathBuf,
    /// Path to file containing the default command line arguments
    #[structopt(short, long)]
    cmdline: PathBuf,
    /// Path to the output file
    #[structopt(short, long)]
    output: PathBuf,
    /// Path to the initramfs file(s) to include
    #[structopt(short, long)]
    initrd: Vec<PathBuf>,
    /// Path to the systemd-boot stub file
    #[cfg(target_arch = "aarch64")]
    #[structopt(short = "S", long, default_value = "/usr/lib/systemd/boot/efi/linuxaa64.efi.stub")]
    stub: PathBuf,
    /// Path to the systemd-boot stub file
    #[cfg(target_arch = "arm")]
    #[structopt(short = "S", long, default_value = "/usr/lib/systemd/boot/efi/linuxarm.efi.stub")]
    stub: PathBuf,
    /// Path to the systemd-boot stub file
    #[cfg(target_arch = "x86")]
    #[structopt(short = "S", long, default_value = "/usr/lib/systemd/boot/efi/linuxia32.efi.stub")]
    stub: PathBuf,
    /// Path to the systemd-boot stub file
    #[cfg(target_arch = "x86_64")]
    #[structopt(short = "S", long, default_value = "/usr/lib/systemd/boot/efi/linuxx64.efi.stub")]
    stub: PathBuf,
    /// Path to the systemd-boot stub file
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64")))]
    #[structopt(short = "S", long)]
    stub: PathBuf,
    /// Make a backup of the previous output if it exists
    #[structopt(short, long)]
    backup: Option<PathBuf>,
    /// Path to the .key and .crt files (in this order) to sign the executable with
    #[structopt(short, long, number_of_values = 2)]
    sign: Option<Vec<PathBuf>>,
    /// Overwrite output file if it already exists
    #[structopt(short = "f", long = "force")]
    overwrite: bool,
}

#[paw::main]
fn main(args: Args) -> io::Result<()> {
    println!("sigen {}", option_env!("CARGO_PKG_VERSION").unwrap_or(""));
    if io::stdout().is_terminal() {
        print!("\x1b[31;1mWARNING: \x1b[0m");
    } else {
        print!("WARNING: ");
    }
    println!("This software is deprecated. Consider using ukify, dracut or mkinitcpio instead.");

    if !args.stub.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to find stub {}", args.stub.display()),
        ));
    }

    if let Some(ref v) = args.sign {
        let key = &v[0];
        let crt = &v[1];

        if !key.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to find key {}", key.display()),
            ));
        }

        if !crt.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to find crt {}", crt.display()),
            ));
        }

        Command::new("sbsign").arg("-V").status()?;
    }

    Command::new("objcopy").arg("-V").status()?;

    if !args.kernel.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to find kernel image {}", args.kernel.display()),
        ));
    }

    print!("\nCreating combined initramfs...");
    io::stdout().flush()?;

    let mut merged_initrd = NamedTempFile::new()?;

    for path in &args.initrd {
        match File::open(path) {
            Ok(mut file) => {
                io::copy(&mut file, &mut merged_initrd)?;
            }
            Err(err) => {
                eprintln!("Failed to find initramfs image {}", path.display());
                return Err(err);
            }
        }
    }

    println!(" done");

    merged_initrd.as_file_mut().sync_all()?;
    let merged_initrd = merged_initrd.into_temp_path();
    let merged_initrd_path = merged_initrd.keep()?;

    if args.output.is_file() {
        match args.backup {
            Some(path) => {
                if path.is_file() && !args.overwrite {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "Backup file already exists, pass -f to overwrite",
                    ));
                }

                fs::copy(&args.output, &path)?;
                fs::remove_file(&args.output)?;
            }
            None => {
                if args.overwrite {
                    fs::remove_file(&args.output)?;
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "Output file already exists, pass -f to overwrite",
                    ));
                }
            }
        }
    }

    print!("Creating standalone executable...");
    io::stdout().flush()?;

    let mut cmdline_arg = OsString::new();
    cmdline_arg.push(".cmdline=");
    cmdline_arg.push(args.cmdline);

    let mut kernel_arg = OsString::new();
    kernel_arg.push(".linux=");
    kernel_arg.push(args.kernel);

    let mut initrd_arg = OsString::new();
    initrd_arg.push(".initrd=");
    initrd_arg.push(&merged_initrd_path);

    let mut command = Command::new("objcopy");
    command.args(&[
        os!("--add-section"),
        os!(".osrel=/etc/os-release"),
        os!("--change-section-vma"),
        os!(".osrel=0x20000"),

        os!("--add-section"),
        &cmdline_arg,
        os!("--change-section-vma"),
        os!(".cmdline=0x30000"),

        os!("--add-section"),
        os!(".splash=/dev/null"),
        os!("--change-section-vma"),
        os!(".splash=0x40000"),

        os!("--add-section"),
        &kernel_arg,
        os!("--change-section-vma"),
        os!(".linux=0x2000000"),

        os!("--add-section"),
        &initrd_arg,
        os!("--change-section-vma"),
        os!(".initrd=0x3000000"),

        args.stub.as_os_str(),
        args.output.as_os_str(),
    ]);

    match command.status() {
        Ok(status) => {
            if !status.success() {
                match status.code() {
                    Some(code) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("objcopy terminated with code {}", code),
                        ))
                    }
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "objcopy terminated by signal",
                        ))
                    }
                }
            }
        }
        Err(err) => return Err(err),
    }

    println!(" done");
    fs::remove_file(merged_initrd_path)?;

    if let Some(v) = args.sign {
        print!("Signing executable...");
        io::stdout().flush()?;

        let key = &v[0];
        let crt = &v[1];

        let mut sign_command = Command::new("sbsign");
        sign_command.args(&[
            os!("--key"),
            key.as_os_str(),

            os!("--cert"),
            crt.as_os_str(),

            os!("--output"),
            args.output.as_os_str(),

            args.output.as_os_str(),
        ]);

        match sign_command.status() {
            Ok(status) => {
                if !status.success() {
                    match status.code() {
                        Some(code) => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("sbsign terminated with code {}", code),
                            ))
                        }
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "sbsign terminated by signal",
                            ))
                        }
                    }
                }
            }
            Err(err) => return Err(err),
        }

        println!(" done");
    }

    Ok(())
}

// Copyright © 2019 Joaquim Monteiro
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
use std::io::{self, Write};
use std::iter::Iterator;
use std::path::PathBuf;
use std::process::Command;

use structopt::StructOpt;
use tempfile::NamedTempFile;

#[cfg(target_arch = "x86_64")]
const STUB_PATH: &str = "/usr/lib/systemd/boot/efi/linuxx64.efi.stub";

#[cfg(target_arch = "x86")]
const STUB_PATH: &str = "/usr/lib/systemd/boot/efi/linuxia32.efi.stub";

macro_rules! os {
    ($s:tt) => {
        std::ffi::OsStr::new($s)
    };
}

#[derive(StructOpt)]
#[structopt(about, author)]
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
    /// Overwrite output file if it already exists
    #[structopt(short = "f", long = "force")]
    overwrite: bool,
}

#[paw::main]
fn main(args: Args) -> io::Result<()> {
    println!("sigen {}", option_env!("CARGO_PKG_VERSION").unwrap_or(""));

    if !PathBuf::from(STUB_PATH).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to find stub {}", STUB_PATH),
        ));
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

    if args.output.exists() {
        if args.overwrite {
            fs::remove_file(&args.output)?
        } else {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Output file already exists, pass -f to overwrite",
            ));
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

        os!(STUB_PATH),
        args.output.as_os_str(),
    ]);

    match command.status() {
        Ok(status) => {
            if status.success() {
                println!(" done\nExecutable creation successful.");
                fs::remove_file(merged_initrd_path)
            } else {
                match status.code() {
                    Some(code) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("objcopy terminated with code {}", code),
                    )),
                    None => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "objcopy terminated by signal",
                    )),
                }
            }
        }
        Err(err) => Err(err),
    }
}

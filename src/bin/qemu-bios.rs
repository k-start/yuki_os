use std::{
    env,
    process::{self, Command},
};

fn main() {
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={}", env!("BIOS_IMAGE")));
    qemu.arg("-drive");
    qemu.arg("format=raw,file=fat32.img,bus=1");
    // qemu.arg("-d");
    // qemu.arg("cpu_reset");
    qemu.arg("-serial");
    qemu.arg("mon:stdio");
    // qemu.arg("-monitor");
    // qemu.arg("stdio");
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}

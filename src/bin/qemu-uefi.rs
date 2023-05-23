use std::{
    env,
    process::{self, Command},
};

fn main() {
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={}", env!("UEFI_IMAGE")));
    qemu.arg("-drive");
    qemu.arg("format=raw,file=user_disk.img,bus=1");
    qemu.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
    qemu.arg("-serial");
    qemu.arg("mon:stdio");
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}

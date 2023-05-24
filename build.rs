use bootloader::DiskImageBuilder;
use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

extern crate fatfs;

fn main() {
    // set by cargo for the kernel artifact dependency
    let kernel_path = env::var("CARGO_BIN_FILE_KERNEL").unwrap();
    let disk_builder = DiskImageBuilder::new(PathBuf::from(kernel_path));

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let uefi_path = out_dir.join("blog_os-uefi.img");
    let bios_path = out_dir.join("blog_os-bios.img");

    // create the disk images
    disk_builder.create_uefi_image(&uefi_path).unwrap();
    disk_builder.create_bios_image(&bios_path).unwrap();

    // Build the user disk image
    // if it doesnt exist, use qemu-img to create one
    if !Path::new("user_disk.img").exists() {
        let mut qemu = Command::new("qemu-img");
        qemu.arg("create");
        qemu.arg("user_disk.img");
        qemu.arg("100M");
        let _exit_status = qemu.status().unwrap();

        let img_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("user_disk.img")
            .unwrap();

        // format as fat32
        fatfs::format_volume(
            img_file,
            fatfs::FormatVolumeOptions::new().fat_type(fatfs::FatType::Fat32),
        )
        .unwrap();
    }

    // load disk and copy user programs onto it
    let img_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("user_disk.img")
        .unwrap();
    let fs = fatfs::FileSystem::new(img_file, fatfs::FsOptions::new()).unwrap();
    let root_dir = fs.root_dir();

    // get all env vars for user binaries -kernel binary
    let bin_vars: Vec<(String, String)> = env::vars()
        .filter(|(x, _)| {
            x.starts_with("CARGO_BIN_FILE_") && !x.starts_with("CARGO_BIN_FILE_KERNEL")
        })
        .collect();

    // loop through binaries
    // there are 2 env vars for each binary in format:
    // CARGO_BIN_FILE_{BINARY_NAME}
    // CARGO_BIN_FILE_{BINARY_NAME}_{binary_name}
    // use this to get the true binary name in lowercase
    for i in (0..bin_vars.len()).step_by(2) {
        let var1 = bin_vars.get(i).unwrap();
        let var2 = bin_vars.get(i + 1).unwrap();
        let binary_path = var1.1.clone();
        let binary_name = var2.0.replace(&format!("{}_", var1.0).to_string(), "");

        // load binary and put it into the .img
        let mut file_in_img: fatfs::File<File> = root_dir.create_file(&binary_name).unwrap();

        let mut binary_file = File::open(binary_path).unwrap();
        let mut data: Vec<u8> = Vec::new();
        let _ = binary_file.read_to_end(&mut data);

        file_in_img.write_all(&data).unwrap();
    }

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_path.display());
}

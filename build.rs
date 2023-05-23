use bootloader::DiskImageBuilder;
use std::{
    env,
    fs::{self, File},
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
        let _format = fatfs::format_volume(
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

    // loop through all files and load them
    let paths = fs::read_dir("user-drive/").unwrap();

    for path in paths {
        let os_str_filename = path.unwrap().file_name();
        let filename = os_str_filename.to_str().unwrap();
        println!("{filename}");
        let mut file = root_dir.create_file(filename).unwrap();

        let mut test_binary = File::open(format!("user-drive/{filename}")).unwrap();
        let mut data: Vec<u8> = Vec::new();
        let _ = test_binary.read_to_end(&mut data);

        file.write_all(&data).unwrap();
    }

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_path.display());
}

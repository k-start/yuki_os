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
    let mut disk_builder = DiskImageBuilder::new(PathBuf::from(kernel_path));

    build_ramdisk();
    build_userdisk();

    let test = disk_builder.set_ramdisk("initrd.img".into());

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let uefi_path = out_dir.join("yuki_os-uefi.img");
    let bios_path = out_dir.join("yuki_os-bios.img");

    // create the disk images
    test.create_uefi_image(&uefi_path).unwrap();
    test.create_bios_image(&bios_path).unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_path.display());
}

#[derive(Debug, Clone)]
struct FileData {
    filename: String,
    content: Vec<u8>,
}

#[allow(dead_code)]
struct RdFile {
    filename: [u8; 32],
    size: usize,
    offset: usize,
}

fn build_ramdisk() {
    let mut files: Vec<FileData> = vec![];

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

        // load binary
        let mut binary_file = File::open(binary_path).unwrap();
        let mut data: Vec<u8> = Vec::new();
        let _ = binary_file.read_to_end(&mut data);

        files.push(FileData {
            filename: binary_name,
            content: data,
        });
    }

    let mut img_data: Vec<u8> = vec![];

    img_data.push(files.len() as u8);
    let mut offset: usize = 0;

    for i in files.clone() {
        let mut rd_file = RdFile {
            filename: [0; 32],
            size: i.content.len(),
            offset,
        };
        offset += i.content.len();

        let mut filename_buf = &mut rd_file.filename[..];
        filename_buf.write_all(i.filename.as_bytes()).unwrap();

        let mut bytes = unsafe { any_as_u8_slice(&rd_file) }.to_vec();
        img_data.append(&mut bytes);
    }

    for i in files {
        let mut content = i.content.clone();
        img_data.append(&mut content);
    }

    let mut f = File::create("initrd.img").unwrap();
    f.write_all(img_data.as_slice()).unwrap();
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

#[allow(dead_code)]
fn build_userdisk() {
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
}

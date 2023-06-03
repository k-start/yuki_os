use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use block_device::BlockDevice;

use crate::fs::filesystem::File;

use super::filesystem::FileSystem;

const BUFFER_SIZE: usize = 512;

#[derive(Debug)]
pub struct Fat32<T>
where
    T: BlockDevice + Clone + Copy,
    <T as BlockDevice>::Error: core::fmt::Debug,
{
    device: T,
    bpb: BiosParameterBlock,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BiosParameterBlock {
    _offset: [u8; 0xB],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fats: u8,
    root_entries: u16,
    total_sectors_16: u16,
    media: u8,
    sectors_per_fat_16: u16,
    sectors_per_track: u16,
    heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,

    // Extended BIOS Paramter Block
    sectors_per_fat_32: u32,
    extended_flags: u16,
    fs_version: u16,
    root_dir_first_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    reserved_0: [u8; 12],
    drive_num: u8,
    ext_sig: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type_label: [u8; 8],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DirEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attribute: u8,
    nt_reserve: u8,
    creation_time_tenth: u8,
    creation_time: u16,
    creation_date: u16,
    last_access_date: u16,
    first_cluster_high: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_low: u16,
    file_size: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LongFileName {
    order: u8,
    name: [u8; 10],
    attribute: u8,
    r#type: u8,
    checksum: u8,
    name_2: [u8; 12],
    first_cluster_low: u16,
    name_3: [u8; 4],
}

impl<T> Fat32<T>
where
    T: BlockDevice + Clone + Copy,
    <T as BlockDevice>::Error: core::fmt::Debug,
{
    pub fn new(device: T) -> Fat32<T> {
        let mut buf = [0; BUFFER_SIZE];
        device.read(&mut buf, 0, 1).unwrap();

        let (_head, bpb, _tail) = unsafe { buf.align_to::<BiosParameterBlock>() };
        let bpb = bpb[0];

        let fs_type = core::str::from_utf8(&bpb.fs_type_label).unwrap_or("");

        if !fs_type.contains("FAT32") {
            panic!("not fat32");
        }

        // println!("{:?}", bpb);

        Fat32 { device, bpb }
    }

    pub fn sector_from_cluster(&self, cluster: u32) -> usize {
        let bpb = self.bpb;

        (bpb.reserved_sectors as usize
            + (bpb.fats as usize * bpb.sectors_per_fat_32 as usize)
            + ((cluster - 2) as usize * bpb.sectors_per_cluster as usize))
            * bpb.bytes_per_sector as usize
    }

    pub fn root_dir(&self) {
        let bpb = self.bpb;

        let root_sector = self.sector_from_cluster(bpb.root_dir_first_cluster);

        let mut buf = [0; BUFFER_SIZE];
        self.device.read(&mut buf, root_sector, 1).unwrap();

        let (_head, dir_entries, _tail) = unsafe { buf.align_to::<DirEntry>() };

        for dir in dir_entries {
            // long file name
            if dir.attribute == 15 {
                let bytes: &[u8] = unsafe {
                    core::slice::from_raw_parts(
                        (dir as *const DirEntry) as *const u8,
                        core::mem::size_of::<DirEntry>(),
                    )
                };

                let (_head, lfns, _tail) = unsafe { bytes.align_to::<LongFileName>() };
                let lfn = lfns[0];

                println!("{:?}", lfn);

                let mut name: [u8; 13] = [0; 13];

                for i in (0..lfn.name.len()).step_by(2) {
                    name[i / 2] = lfn.name[i];
                }
                for i in (0..lfn.name_2.len()).step_by(2) {
                    name[i / 2 + 5] = lfn.name_2[i];
                }
                for i in (0..lfn.name_3.len()).step_by(2) {
                    if lfn.name_3[i] == 255 || lfn.name_3[i] == 0 {
                        break;
                    }
                    name[i / 2 + 5 + 6] = lfn.name_3[i];
                }

                let name = core::str::from_utf8(&name).unwrap_or("oops");
                println!("{:?}", name);
            }
        }

        // println!("{:#?}", dir_entries);
    }
}

impl<T> FileSystem for Fat32<T>
where
    T: BlockDevice + Clone + Copy,
    <T as BlockDevice>::Error: core::fmt::Debug,
{
    fn dir_entries(&self, _dir: &str) -> Vec<File> {
        let mut ret = Vec::new();

        let bpb = self.bpb;
        let root_sector = self.sector_from_cluster(bpb.root_dir_first_cluster);

        let mut buf = [0; BUFFER_SIZE];
        self.device.read(&mut buf, root_sector, 1).unwrap();

        let (_head, dir_entries, _tail) = unsafe { buf.align_to::<DirEntry>() };

        let mut long_file_name: String = String::new();

        for (i, dir) in dir_entries.iter().enumerate() {
            if dir.name == [0; 8] {
                continue;
            }
            if (dir.attribute & 0x15) != 0 {
                // long file name
                let bytes: [u8; 0x20] = buf[i * 0x20..(i + 1) * 0x20]
                    .try_into()
                    .expect("incorrect len");
                let lfn = read_long_file_name(&bytes);
                let mut name: [u8; 13] = [0; 13];

                for (i, c) in lfn.name.iter().enumerate().step_by(2) {
                    name[i / 2] = *c;
                }
                for (i, c) in lfn.name_2.iter().enumerate().step_by(2) {
                    name[i / 2 + 5] = *c;
                }
                for (i, c) in lfn.name_3.iter().enumerate().step_by(2) {
                    if lfn.name_3[i] == 255 {
                        break;
                    }
                    name[i / 2 + 11] = *c;
                }
                long_file_name = String::from_utf8((&name).to_vec())
                    .unwrap()
                    .trim()
                    .replace('\0', "")
                    .to_owned();
                continue;
            }
            let r#type = match dir.attribute & 0x10 {
                0 => "file",
                _ => "dir",
            };
            let name = core::str::from_utf8(&dir.name).unwrap_or("").trim();
            let ext = core::str::from_utf8(&dir.ext).unwrap_or("").trim();
            let name = match ext {
                "" => format!("{name}"),
                _ => format!("{name}.{ext}"),
            };

            let file = File {
                lfn: long_file_name,
                name,
                r#type: r#type.to_owned(),
                size: dir.file_size,
                first_cluster_high: dir.first_cluster_high,
                first_cluster_low: dir.first_cluster_low,
            };
            ret.push(file);
            long_file_name = String::new();
        }

        ret
    }

    fn open(&self, path: &str) -> Option<File> {
        let split: Vec<&str> = path.split("/").collect();
        let file_name = *split.last().unwrap_or(&"");
        let dir = self.dir_entries(path);
        for file in dir {
            if file.name.to_lowercase() == file_name.to_lowercase()
                || file.lfn.to_lowercase() == file_name.to_lowercase()
            {
                return Some(file);
            }
        }
        None
    }

    fn read(&self, file: &File, buffer: &mut [u8]) {
        let mut buf = [0; BUFFER_SIZE];
        self.device
            .read(&mut buf, self.bpb.reserved_sectors as usize, 1)
            .unwrap();

        let mut buf_u32: [u32; 128] = [0; 128];

        for i in (0..512).step_by(4) {
            buf_u32[i / 4] = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
        }

        println!("{:x?}", buf_u32);

        todo!()
    }
}

fn read_long_file_name(bytes: &[u8; 0x20]) -> LongFileName {
    let (_head, lfns, _tail) = unsafe { bytes.align_to::<LongFileName>() };
    lfns[0]
}

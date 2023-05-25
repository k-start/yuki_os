use block_device::BlockDevice;

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

    pub fn root_dir(&self) {
        let bpb = self.bpb;

        let root_sector = (bpb.reserved_sectors as u32 + bpb.fats as u32 * bpb.sectors_per_fat_32)
            * bpb.bytes_per_sector as u32;

        let mut buf = [0; BUFFER_SIZE];
        self.device.read(&mut buf, root_sector as usize, 1).unwrap();

        let (_head, dir, _tail) = unsafe { buf.align_to::<DirEntry>() };

        let name = core::str::from_utf8(&dir[0].name).unwrap_or("oops");

        println!("{} {:?}", name, dir[0]);
    }
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
    name: [u8; 11],
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

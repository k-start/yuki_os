use alloc::vec::Vec;
use bit_field::BitField;
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

pub static BUSES: Mutex<Vec<Bus>> = Mutex::new(Vec::new());

#[allow(dead_code)]
#[repr(u16)]
enum Command {
    Read = 0x20,
    Write = 0x30,
    Identify = 0xEC,
}

#[allow(dead_code)]
#[repr(usize)]
enum Status {
    Err = 0,
    Idx = 1,
    Corr = 2,
    Drq = 3,
    Srv = 4,
    Df = 5,
    Rdy = 6,
    Bsy = 7,
}

#[allow(dead_code)]
pub struct Bus {
    bus: u16,
    data_register: Port<u16>,
    error_register: PortReadOnly<u8>,
    features_register: PortWriteOnly<u8>,
    sector_count_register: Port<u8>,
    lba0_register: Port<u8>,
    lba1_register: Port<u8>,
    lba2_register: Port<u8>,
    drive_register: Port<u8>,
    status_register: PortReadOnly<u8>,
    command_register: PortWriteOnly<u8>,
    control_register: PortWriteOnly<u8>,
    alternate_status_register: PortReadOnly<u8>,
}

impl Bus {
    pub fn new(base_port: u16) -> Bus {
        Bus {
            bus: base_port,
            data_register: Port::new(base_port),
            error_register: PortReadOnly::new(base_port + 1),
            features_register: PortWriteOnly::new(base_port + 1),
            sector_count_register: Port::new(base_port + 2),
            lba0_register: Port::new(base_port + 3),
            lba1_register: Port::new(base_port + 4),
            lba2_register: Port::new(base_port + 5),
            drive_register: Port::new(base_port + 6),
            status_register: PortReadOnly::new(base_port + 7),
            command_register: PortWriteOnly::new(base_port + 7),
            alternate_status_register: PortReadOnly::new(base_port + 0xC),
            control_register: PortWriteOnly::new(base_port + 0xC),
        }
    }

    pub fn init(&mut self) -> u32 {
        println!("[ATA] Initializing IDE on bus {:#x}", self.bus);

        unsafe {
            self.features_register.write(1);
        }
        crate::outb(self.bus + 0x306, 0);

        self.ata_select();
        self.ata_io_wait();

        self.write_command(Command::Identify);

        self.ata_io_wait();

        let status: u8 = self.status();
        println!("[ATA] Status = {:#x}", status);

        self.wait_ready();

        let mut buf: [u8; 512] = [0; 512];

        for i in 0..256 {
            unsafe {
                let short: u16 = self.data_register.read();
                // buf[i] = self.data_register.read();
                buf[i * 2] = short.to_be_bytes()[0];
                buf[i * 2 + 1] = short.to_be_bytes()[1];
            }
        }

        let model = core::str::from_utf8(&buf[54..94]).unwrap().trim();
        let serial = core::str::from_utf8(&buf[20..40]).unwrap().trim();
        let sectors = u32::from_be_bytes(buf[120..124].try_into().unwrap()).rotate_left(16);

        println!("[ATA] Device - {}", model);
        println!("[ATA] Serial - {}", serial);
        println!("[ATA] Size = {}MB", sectors / 2 / 1024);

        unsafe {
            self.command_register.write(0x02);
        }

        sectors
    }

    pub fn read(&mut self, block: u32, buf: &mut [u8]) {
        // assert!(buf.len() == 512);
        self.setup(0, block);
        self.write_command(Command::Read);
        self.wait_ready();
        for i in 0..(buf.len() + 1) / 2 {
            let data = self.read_data();
            buf[i * 2] = data.get_bits(0..8) as u8;
            // fix me - allow for better shorter reads;
            if i * 2 + 1 < buf.len() {
                buf[i * 2 + 1] = data.get_bits(8..16) as u8;
            }
        }
    }

    pub fn write(&mut self, block: u32, buf: &[u8]) {
        assert!(buf.len() == 512);
        self.setup(0, block);
        self.write_command(Command::Write);
        self.wait_ready();
        for i in 0..256 {
            let mut data = 0;
            data.set_bits(0..8, buf[i * 2] as u16);
            data.set_bits(8..16, buf[i * 2 + 1] as u16);
            self.write_data(data);
        }
        self.wait_ready();
    }

    fn setup(&mut self, drive: u8, block: u32) {
        let drive_id = 0xE0 | (drive << 4);
        unsafe {
            self.drive_register
                .write(drive_id | ((block.get_bits(24..28) as u8) & 0x0F));
            self.sector_count_register.write(1);
            self.lba0_register.write(block.get_bits(0..8) as u8);
            self.lba1_register.write(block.get_bits(8..16) as u8);
            self.lba2_register.write(block.get_bits(16..24) as u8);
        }
    }

    fn ata_select(&mut self) {
        let device_port: u8 = 0xA0;
        // if self.bus == 0x1F0 {
        //     device_port = 0xA0;
        // } else {
        //     device_port = 0xB0;
        // }

        unsafe {
            self.drive_register.write(device_port);
        }
    }

    fn ata_io_wait(&mut self) {
        unsafe {
            self.alternate_status_register.read();
            self.alternate_status_register.read();
            self.alternate_status_register.read();
            self.alternate_status_register.read();
        }
    }

    fn wait_ready(&mut self) {
        loop {
            if !self.is_busy() {
                break;
            }
        }
    }

    fn is_busy(&mut self) -> bool {
        self.status().get_bit(Status::Bsy as usize)
    }

    fn status(&mut self) -> u8 {
        unsafe { self.status_register.read() }
    }

    fn read_data(&mut self) -> u16 {
        unsafe { self.data_register.read() }
    }

    fn write_data(&mut self, data: u16) {
        unsafe { self.data_register.write(data) }
    }

    fn write_command(&mut self, cmd: Command) {
        unsafe {
            self.command_register.write(cmd as u8);
        }
    }
}

pub fn init() {
    let mut buses = BUSES.lock();
    // buses.push(Bus::new(0x1F0));
    buses.push(Bus::new(0x170));

    for i in 0..buses.len() {
        buses[i].init();
    }
}

pub fn read(bus: u8, block: u32, buf: &mut [u8]) {
    let mut buses = BUSES.lock();
    buses[bus as usize].read(block, buf);
}

pub fn write(bus: u8, block: u32, buf: &[u8]) {
    let mut buses = BUSES.lock();
    buses[bus as usize].write(block, buf);
}

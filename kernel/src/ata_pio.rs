use alloc::vec::Vec;
use bit_field::BitField;
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

pub static BUSES: Mutex<Vec<Bus>> = Mutex::new(Vec::new());

#[allow(dead_code)]
#[repr(u16)]
pub enum Command {
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
    // Control block registers are at a separate base address
    control_register: PortWriteOnly<u8>,
    alternate_status_register: PortReadOnly<u8>,
}

impl Bus {
    pub fn new(cmd_base: u16, ctl_base: u16) -> Bus {
        Bus {
            bus: cmd_base,
            data_register: Port::new(cmd_base),
            error_register: PortReadOnly::new(cmd_base + 1),
            features_register: PortWriteOnly::new(cmd_base + 1),
            sector_count_register: Port::new(cmd_base + 2),
            lba0_register: Port::new(cmd_base + 3),
            lba1_register: Port::new(cmd_base + 4),
            lba2_register: Port::new(cmd_base + 5),
            drive_register: Port::new(cmd_base + 6),
            status_register: PortReadOnly::new(cmd_base + 7),
            command_register: PortWriteOnly::new(cmd_base + 7),
            alternate_status_register: PortReadOnly::new(ctl_base),
            control_register: PortWriteOnly::new(ctl_base),
        }
    }

    pub fn init(&mut self) -> Option<u32> {
        println!("[ATA PIO] Initializing IDE on bus {:#x}", self.bus);

        // Select drive 0
        self.select_drive(0);
        // Disable interrupts for this bus
        unsafe { self.control_register.write(2) };

        self.write_command(Command::Identify);
        self.ata_io_wait();

        if self.status() == 0 {
            println!("[ATA PIO] No device found on bus {:#x}", self.bus);
            return None;
        }

        self.wait_ready();

        // Read the IDENTIFY data
        let mut identify_buf = [0u16; 256];
        for i in 0..256 {
            identify_buf[i] = self.read_data();
        }

        // Parse model and serial, which are stored as byte-swapped strings
        let mut model_bytes = [0u8; 40];
        for (i, &word) in identify_buf[27..47].iter().enumerate() {
            model_bytes[i * 2..i * 2 + 2].copy_from_slice(&word.to_be_bytes());
        }
        let model = core::str::from_utf8(&model_bytes).unwrap_or("").trim();

        // LBA28 sector count is at words 60-61
        let sectors = u32::from(identify_buf[60]) | (u32::from(identify_buf[61]) << 16);

        println!("[ATA PIO] Device: {}", model);
        println!("[ATA PIO] Size: {} MB", sectors / 2 / 1024);

        Some(sectors)
    }

    pub fn read(&mut self, block: u32, buf: &mut [u8]) {
        self.setup(0, block, 1);
        self.write_command(Command::Read);
        self.wait_ready();

        let mut word_buf = &mut buf[..];
        while word_buf.len() >= 2 {
            let data = self.read_data().to_le_bytes();
            word_buf[0] = data[0];
            word_buf[1] = data[1];
            word_buf = &mut word_buf[2..];
        }
        if !word_buf.is_empty() {
            let data = self.read_data().to_le_bytes();
            word_buf[0] = data[0];
        }
    }

    pub fn write(&mut self, block: u32, buf: &[u8]) {
        assert!(buf.len() == 512);
        self.setup(0, block, 1);
        self.write_command(Command::Write);
        self.wait_ready();
        for i in 0..256 {
            let data = u16::from_le_bytes([buf[i * 2], buf[i * 2 + 1]]);
            self.write_data(data);
        }
        self.wait_ready();
    }

    fn setup(&mut self, drive: u8, block: u32, sector_count: u8) {
        let drive_id = 0xE0 | (drive << 4);
        unsafe {
            self.drive_register
                .write(drive_id | ((block.get_bits(24..28) as u8) & 0x0F));
            self.sector_count_register.write(sector_count);
            self.lba0_register.write(block.get_bits(0..8) as u8);
            self.lba1_register.write(block.get_bits(8..16) as u8);
            self.lba2_register.write(block.get_bits(16..24) as u8);
        }
    }

    fn select_drive(&mut self, drive: u8) {
        // drive 0: 0xA0, drive 1: 0xB0
        let device_port: u8 = 0xA0 | (drive << 4);
        unsafe {
            self.drive_register.write(device_port);
        }
    }

    fn ata_io_wait(&mut self) {
        // Wait 400ns by reading the alternate status register 4 times
        unsafe {
            self.alternate_status_register.read();
            self.alternate_status_register.read();
            self.alternate_status_register.read();
            self.alternate_status_register.read();
        }
    }

    fn wait_ready(&mut self) {
        while self.is_busy() {}
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
    buses.push(Bus::new(0x1F0, 0x3F6)); // Primary bus
    buses.push(Bus::new(0x170, 0x376)); // Secondary bus

    for bus in buses.iter_mut() {
        bus.init();
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

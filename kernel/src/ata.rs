// pub mod ata_dma; // This can be enabled when the DMA driver is ready

use crate::ata_pio;

pub fn init() {
    // For now, we initialize the PIO driver by default
    ata_pio::init();
}

pub fn read(bus: u8, block: u32, buf: &mut [u8]) {
    // Default to PIO read
    ata_pio::read(bus, block, buf);
}

pub fn write(bus: u8, block: u32, buf: &[u8]) {
    // Default to PIO write
    ata_pio::write(bus, block, buf);
}

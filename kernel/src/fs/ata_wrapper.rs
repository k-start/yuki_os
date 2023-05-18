use block_device::BlockDevice;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    ReadError,
    WriteError,
}

#[derive(Clone, Copy)]
pub struct AtaWrapper {
    pub ata_bus: i32,
}

impl BlockDevice for AtaWrapper {
    type Error = Error;

    fn read(
        &self,
        buf: &mut [u8],
        address: usize,
        number_of_blocks: usize,
    ) -> Result<(), Self::Error> {
        for i in 0..number_of_blocks {
            let mut buf_2: [u8; 512] = [0; 512];
            crate::ata::read(
                self.ata_bus as u8,
                address as u32 / 512 + i as u32,
                &mut buf_2,
            );

            for j in 0..512 {
                buf[(i * 512) + j] = buf_2[j];
            }
        }
        Ok(())
    }

    fn write(
        &self,
        buf: &[u8],
        address: usize,
        number_of_blocks: usize,
    ) -> Result<(), Self::Error> {
        for i in 0..number_of_blocks {
            crate::ata::write(
                self.ata_bus as u8,
                address as u32 / 512,
                &buf[i * 512..(i + 1) * 512],
            );
        }
        Ok(())
    }
}

impl AtaWrapper {
    pub fn new(ata_bus: i32) -> AtaWrapper {
        AtaWrapper { ata_bus }
    }
}

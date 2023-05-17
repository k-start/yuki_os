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
        if number_of_blocks == 1 {
            crate::ata::read(self.ata_bus as u8, address as u32 / 512, buf);
        } else {
            return Err(Error::ReadError);
        }
        Ok(())
    }

    fn write(
        &self,
        buf: &[u8],
        address: usize,
        number_of_blocks: usize,
    ) -> Result<(), Self::Error> {
        if number_of_blocks == 1 {
            crate::ata::write(self.ata_bus as u8, address as u32 / 512, buf);
        } else {
            return Err(Error::WriteError);
        }
        Ok(())
    }
}

impl AtaWrapper {
    pub fn new(ata_bus: i32) -> AtaWrapper {
        AtaWrapper { ata_bus }
    }
}

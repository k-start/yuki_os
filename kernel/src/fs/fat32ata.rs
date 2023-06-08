use fatfs::{IoBase, IoError, Read, Seek, Write};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    ReadError,
    WriteError,
    SeekError,
}

impl IoError for Error {
    fn is_interrupted(&self) -> bool {
        true
    }

    fn new_unexpected_eof_error() -> Self {
        println!("eof error");
        Self::ReadError
    }

    fn new_write_zero_error() -> Self {
        println!("write zero error");
        Self::WriteError
    }
}

#[derive(Clone, Copy)]
pub struct Fat32Ata {
    pub ata_bus: i32,
    pub pos: u64,
}

impl IoBase for Fat32Ata {
    type Error = Error;
}

impl Read for Fat32Ata {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.len() > 512 {
            panic!("longer than 512");
        }
        // println!("reading pos: {} len: {}", self.pos, buf.len());

        let block = self.pos / 512;
        let block_offset = self.pos % 512;
        let mut buf_2 = [0; 512];

        crate::ata::read(self.ata_bus as u8, block as u32, &mut buf_2);

        for i in 0..buf.len() {
            buf[i] = buf_2[block_offset as usize + i];
        }

        self.pos = self.pos + buf.len() as u64;

        Ok(buf.len())
    }

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        if buf.is_empty() {
            Ok(())
        } else {
            println!("failed to fill whole buffer in read_exact");
            Err(Self::Error::new_unexpected_eof_error())
        }
    }
}

impl Write for Fat32Ata {
    fn write(&mut self, _buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    println!("failed to write whole buffer in write_all");
                    return Err(Self::Error::new_write_zero_error());
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl Seek for Fat32Ata {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        // println!("{:?}", pos);

        match pos {
            fatfs::SeekFrom::Start(x) => self.pos = x,
            fatfs::SeekFrom::End(_x) => todo!(),
            fatfs::SeekFrom::Current(x) => {
                let i = (self.pos as i64) + x;
                if i < 0 {
                    return Err(Self::Error::SeekError);
                }
                self.pos = i as u64;
            }
        }

        Ok(self.pos)
    }
}

impl Fat32Ata {
    pub fn new(ata_bus: i32) -> Fat32Ata {
        Fat32Ata { ata_bus, pos: 0 }
    }
}

use elfloader::*;
use x86_64::{
    structures::paging::{PageTable, PageTableFlags},
    VirtAddr,
};

use crate::memory::allocate_pages;

pub(crate) struct UserspaceElfLoader {
    pub(crate) vbase: u64,
    pub(crate) user_page_table_ptr: *mut PageTable,
}

impl ElfLoader for UserspaceElfLoader {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), ElfLoaderErr> {
        for header in load_headers {
            println!(
                "allocate base = {:#x} size = {:#x} flags = {}",
                header.virtual_addr(),
                header.mem_size(),
                header.flags()
            );

            unsafe {
                allocate_pages(
                    self.user_page_table_ptr,
                    VirtAddr::new(self.vbase + header.virtual_addr()),
                    header.mem_size(),
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::USER_ACCESSIBLE,
                )
                .expect("Could not allocate memory");
            }
        }
        Ok(())
    }

    fn relocate(&mut self, entry: RelocationEntry) -> Result<(), ElfLoaderErr> {
        use elfloader::arch::x86_64::RelocationTypes::*;
        use RelocationType::x86_64;

        let addr: *mut u64 = (self.vbase + entry.offset) as *mut u64;

        match entry.rtype {
            x86_64(R_AMD64_RELATIVE) => {
                // This type requires addend to be present
                let addend = entry
                    .addend
                    .ok_or(ElfLoaderErr::UnsupportedRelocationEntry)?;

                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                println!("R_RELATIVE *{:p} = {:#x}", addr, self.vbase + addend);

                unsafe {
                    core::ptr::write(addr, self.vbase + addend);
                }

                Ok(())
            }
            _ => Ok((/* not implemented */)),
        }
    }

    fn load(&mut self, _flags: Flags, base: VAddr, region: &[u8]) -> Result<(), ElfLoaderErr> {
        let start = self.vbase + base;
        let end = self.vbase + base + region.len() as u64;
        println!("load region into = {:#x} -- {:#x}", start, end);

        let dest_ptr: *const u8 = VirtAddr::new(start).as_ptr();
        let mut i = 0;
        for value in region {
            unsafe {
                let ptr: *mut u8 = dest_ptr.add(i).cast_mut();
                core::ptr::write(ptr, *value);
            }
            i += 1;
        }

        Ok(())
    }

    fn tls(
        &mut self,
        tdata_start: VAddr,
        _tdata_length: u64,
        total_size: u64,
        _align: u64,
    ) -> Result<(), ElfLoaderErr> {
        let tls_end = tdata_start + total_size;
        println!(
            "Initial TLS region is at = {:#x} -- {:#x}",
            tdata_start, tls_end
        );
        Ok(())
    }
}

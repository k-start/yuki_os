pub mod allocator;
pub mod slab_alloc;

use core::arch::asm;

use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub struct MemoryInfo {
    pub phys_mem_offset: VirtAddr,
    frame_allocator: BootInfoFrameAllocator,
    kernel_l4_table: &'static mut PageTable,
}

// NOTE: mutable but changed only once during initialization
pub static mut MEMORY_INFO: Option<MemoryInfo> = None;

pub fn init(physical_memory_offset: Option<u64>, memory_regions: &'static MemoryRegions) {
    let phys_mem_offset = VirtAddr::new(physical_memory_offset.unwrap());
    let mut mapper = unsafe { init_page_table(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(memory_regions) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let level_4_table = unsafe { active_level_4_table(phys_mem_offset) };

    unsafe {
        MEMORY_INFO = Some(MemoryInfo {
            phys_mem_offset,
            frame_allocator,
            kernel_l4_table: level_4_table,
        })
    };

    let total_mem: u64 = memory_regions.iter().map(|x| x.end - x.start).sum();
    println!("Ram detected: {}MB", total_mem / 1024 / 1024 + 1);
}

/// Initialize a new OffsetPageTable.
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init_page_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::page_table::FrameError;

    // read the active level 4 frame from the CR3 register
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi-level page table
    for &index in &table_indexes {
        // convert the frame into a page table reference
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        // read the page table entry and update `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

pub fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn create_empty_pagetable() -> (*mut PageTable, u64) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    // Get a frame to store the level 4 table
    let level_4_table_frame = memory_info.frame_allocator.allocate_frame().unwrap();
    let phys = level_4_table_frame.start_address(); // Physical address
    let virt = memory_info.phys_mem_offset + phys.as_u64(); // Kernel virtual address
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // Clear all entries in the page table
    unsafe {
        (*page_table_ptr).zero();
    }

    (page_table_ptr, phys.as_u64())
}

fn copy_pagetables(level_4_table: &PageTable) -> (*mut PageTable, u64) {
    // Create a new level 4 pagetable
    let (table_ptr, table_physaddr) = create_empty_pagetable();
    let table = unsafe { &mut *table_ptr };

    fn copy_pages_rec(
        physical_memory_offset: VirtAddr,
        from_table: &PageTable,
        to_table: &mut PageTable,
        level: u16,
    ) {
        for (i, entry) in from_table.iter().enumerate() {
            if !entry.is_unused() {
                if (level == 1) || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                    // Maps a frame, not a page table
                    to_table[i].set_addr(entry.addr(), entry.flags());
                } else {
                    // Create a new table at level - 1
                    let (new_table_ptr, new_table_physaddr) = create_empty_pagetable();
                    let to_table_m1 = unsafe { &mut *new_table_ptr };

                    // Point the entry to the new table
                    to_table[i].set_addr(PhysAddr::new(new_table_physaddr), entry.flags());

                    // Get reference to the input level-1 table
                    let from_table_m1 = {
                        let virt = physical_memory_offset + entry.addr().as_u64();
                        unsafe { &*virt.as_ptr() }
                    };

                    // Copy level-1 entries
                    copy_pages_rec(
                        physical_memory_offset,
                        from_table_m1,
                        to_table_m1,
                        level - 1,
                    );
                }
            }
        }
    }

    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    copy_pages_rec(memory_info.phys_mem_offset, level_4_table, table, 4);

    (table_ptr, table_physaddr)
}

pub fn create_new_user_pagetable() -> (*mut PageTable, u64) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    // Copy kernel pages
    let (user_page_table_ptr, user_page_table_physaddr) =
        copy_pagetables(memory_info.kernel_l4_table);

    (user_page_table_ptr, user_page_table_physaddr)
}

pub fn switch_to_pagetable(physaddr: u64) {
    unsafe {
        asm!("mov cr3, {addr}",
             addr = in(reg) physaddr);
    }
}

pub fn switch_to_kernel_pagetable() {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let phys_addr = (memory_info.kernel_l4_table as *mut PageTable as u64)
        - memory_info.phys_mem_offset.as_u64();
    switch_to_pagetable(phys_addr);
}

pub fn active_pagetable_physaddr() -> u64 {
    let mut physaddr: u64;
    unsafe {
        asm!("mov {addr}, cr3",
             addr = out(reg) physaddr);
    }
    physaddr
}

/// Allocates pages in the level_4_table supplied
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// passed `level_4_table` must point to the level 4 page table of a valid
/// page table hierarchy. Otherwise this function might break memory safety,
/// e.g. by writing to an illegal memory location.
pub unsafe fn allocate_pages(
    level_4_table: *mut PageTable,
    start_addr: VirtAddr,
    size: u64,
    flags: PageTableFlags,
) -> Result<(), MapToError<Size4KiB>> {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    let mut mapper =
        unsafe { OffsetPageTable::new(&mut *level_4_table, memory_info.phys_mem_offset) };

    let page_range = {
        let end_addr = start_addr + size - 1u64;
        let start_page = Page::containing_address(start_addr);
        let end_page = Page::containing_address(end_addr);
        Page::range_inclusive(start_page, end_page)
    };

    for page in page_range {
        let frame = memory_info
            .frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut memory_info.frame_allocator)?
                .flush()
        };
    }

    Ok(())
}

// ---------------------------------------------------------------------------------------------

/// A FrameAllocator that always returns `None`.
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

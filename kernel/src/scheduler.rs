use crate::{
    elf,
    fs::{self, filesystem::FileDescriptor},
    gdt, memory,
    process::{Context, Process, ProcessState},
};
use alloc::{boxed::Box, string::String, vec::Vec};
use spin::RwLock;
use x86_64::{
    registers::rflags::RFlags,
    structures::paging::{PageTable, PageTableFlags},
    VirtAddr,
};

pub static SCHEDULER: RwLock<Scheduler> = RwLock::new(Scheduler::new());
static STACK_START: usize = 0x800000;
static STACK_SIZE: usize = 0x100000;
static HEAP_START: usize = 0x5000_0000_0000;
static HEAP_SIZE: usize = 0x100000;

// --- ELF Header Constants ---
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_2_LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const ET_DYN: u16 = 3; // Position-Independent Executable
const PT_LOAD: u32 = 1;

// --- ELF Header Structs ---

/// Represents the main ELF header at the start of the file
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ElfHeader {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// Represents a program header entry, which describes a segment
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

// --- Dynamic Segment & Relocation Structs and Constants ---
const PT_DYNAMIC: u32 = 2;
const DT_NULL: u64 = 0;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_RELAENT: u64 = 9;
const DT_JMPREL: u64 = 23;
const DT_PLTRELSZ: u64 = 2;

// Relocation type for x86_64
const R_X86_64_RELATIVE: u32 = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Dyn {
    d_tag: u64,
    d_val: u64, // d_un.d_val or d_un.d_ptr
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

pub struct Scheduler {
    processes: RwLock<Vec<Box<Process>>>,
    cur_process: RwLock<Option<usize>>,
    allocated_ids: RwLock<Vec<usize>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub const fn new() -> Scheduler {
        Scheduler {
            processes: RwLock::new(Vec::new()),
            cur_process: RwLock::new(None), // so that next process is 0
            allocated_ids: RwLock::new(Vec::new()),
        }
    }

    // Private helper to find a PID without taking a lock
    // This should only be called when the `allocated_ids` lock is already held
    fn get_available_pid_unlocked(&self, allocated_ids: &[usize]) -> usize {
        for i in 1..1000 {
            if !allocated_ids.contains(&i) {
                return i;
            }
        }
        panic!("No available PIDs left");
    }

    pub fn get_available_pid(&self) -> usize {
        // This function is now a simple wrapper that takes the lock
        // and calls the unlocked version
        let allocated = self.allocated_ids.read();
        self.get_available_pid_unlocked(&allocated)
    }

    pub fn schedule(&self, file: FileDescriptor) {
        let (_current_page_table_ptr, current_page_table_physaddr) = memory::active_page_table();
        let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

        memory::switch_to_pagetable(user_page_table_physaddr);

        let entry_point = self
            .load_elf(&file, user_page_table_ptr)
            .expect("Failed to load ELF for new process");

        memory::switch_to_pagetable(current_page_table_physaddr);

        let mut process = Process::new(
            VirtAddr::new(entry_point),
            VirtAddr::new((STACK_START + STACK_SIZE) as u64),
            user_page_table_physaddr,
            0,
        );

        // Acquire locks in the canonical order to prevent deadlocks:
        // processes -> allocated_ids
        let mut processes = self.processes.write();
        let mut allocated_ids = self.allocated_ids.write();

        process.process_id = self.get_available_pid_unlocked(&allocated_ids);
        allocated_ids.push(process.process_id);
        processes.push(Box::new(process));
    }

    /// Initialize a new OffsetPageTable.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it derefences the context provided
    /// This should only be called by the interrupt that is switching
    /// contexts.
    pub unsafe fn save_current_context(&self, context: *const Context) {
        // This function should only save the context of the current process.
        // Lock in a consistent order to prevent deadlocks: processes -> cur_process
        let mut processes = self.processes.write();
        if let Some(cur_process_idx) = *self.cur_process.read() {
            if cur_process_idx < processes.len() {
                // Only save the context if the process is not already exiting
                // If it's exiting, its context is frozen and will be removed in run_next
                if processes[cur_process_idx].state != ProcessState::Exiting() {
                    let ctx = (*context).clone();
                    processes[cur_process_idx].state = ProcessState::SavedContext(ctx);
                }
            }
        }
    }

    pub fn run_next(&self) -> *const Context {
        // We take a write lock on processes because we might need to initialize a new process
        // by transitioning it from `StartingInfo` to `SavedContext`
        let mut processes = self.processes.write();
        let processes_len = processes.len();

        if processes_len == 0 {
            return core::ptr::null(); // No processes to run
        }

        // Reap any exited "zombie" processes. This is the safe place to do it,
        // as we are in the scheduler and not running in the context of any process
        // that might be reaped.
        let pids_to_reap: Vec<usize> = processes
            .iter()
            .filter(|p| p.state == ProcessState::Exiting())
            .map(|p| {
                println!("Reaping process #{}", p.process_id);
                p.process_id
            })
            .collect();

        if !pids_to_reap.is_empty() {
            // Lock allocated_ids only after we've collected the PIDs to reap
            // This maintains the `processes` -> `allocated_ids` lock order
            let mut allocated = self.allocated_ids.write();
            allocated.retain(|pid| !pids_to_reap.contains(pid));

            processes.retain(|p| p.state != ProcessState::Exiting());
        }
        let processes_len = processes.len();

        // Look for the next non-exiting process to run
        for _ in 0..processes_len {
            // Determine and update the current process index
            let next_idx = {
                let mut cur_process_opt = self.cur_process.write();
                let next = cur_process_opt.map_or(0, |current| (current + 1) % processes_len);
                *cur_process_opt = Some(next);
                next
            };

            let process = &mut processes[next_idx];

            // If the process is runnable, prepare and return its context
            if process.state != ProcessState::Exiting() {
                // println!("Switching to process #{}", process.process_id);

                memory::switch_to_pagetable(process.page_table_phys);

                // If the process is new, it's in a `StartingInfo` state
                // We must transition it to `SavedContext` to run it
                if let ProcessState::StartingInfo(entry_point, stack_top) = process.state {
                    // This is the first time we're running this process. We need to create a context that
                    // will jump to the program's entry point in user mode
                    let (code_selector, data_selector) = gdt::get_usermode_segments();

                    let mut context = Context::default();
                    context.rip = entry_point.as_u64() as usize;
                    context.rsp = stack_top.as_u64() as usize;
                    // CRITICAL: Bit 1 of RFLAGS is reserved and must be 1.
                    context.rflags =
                        (RFlags::INTERRUPT_FLAG | RFlags::from_bits_truncate(0x2)).bits() as usize;
                    context.cs = code_selector.0 as usize;
                    context.ss = data_selector.0 as usize;

                    // Transition the process to a runnable state with the new context
                    process.state = ProcessState::SavedContext(context);
                }

                // Now we can be sure the state is `SavedContext`
                // We then return the context to the calling interrupt to return to that context
                let context_ptr = match &process.state {
                    ProcessState::SavedContext(context) => context as *const Context,
                    _ => unreachable!(),
                };

                return context_ptr;
            }
            // If the process was exiting, the loop continues to the next one
        }

        // TODO: Spin up a default process if all are exited
        core::ptr::null()
    }

    pub fn exit_current(&self) {
        // This function is called from a syscall when a process wants to exit
        // It marks the current process' state as `Exiting`
        // The caller (syscall handler) forces a context switch
        // immediately after this function returns

        // Lock in the established order: processes -> cur_process to avoid deadlocks
        let mut processes = self.processes.write();
        let cur_process_opt = self.cur_process.read();

        if let Some(cur_process_idx) = *cur_process_opt {
            if cur_process_idx < processes.len() {
                println!(
                    "Process #{} is exiting",
                    processes[cur_process_idx].process_id
                );
                processes[cur_process_idx].state = ProcessState::Exiting();
            }
        }
    }

    pub fn fork_current(&self, context: Context) -> usize {
        // This function needs to read the current process and write to the process list
        // and PID list. To avoid deadlocks, we must acquire all necessary locks
        // up-front in the canonical order: processes -> cur_process -> allocated_ids
        let mut processes = self.processes.write();
        let cur_process_opt = self.cur_process.read();
        let (current_page_table_ptr, current_page_table_physaddr) = memory::active_page_table();
        unsafe {
            // FIX ME - implement copy on write later
            memory::allocate_pages(
                current_page_table_ptr,
                VirtAddr::new((STACK_START + STACK_SIZE) as u64),
                STACK_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");

            let new_stack: &mut [u8] = core::slice::from_raw_parts_mut(
                VirtAddr::new((STACK_START + STACK_SIZE) as u64).as_mut_ptr(),
                STACK_SIZE,
            );
            let old_stack: &[u8] =
                core::slice::from_raw_parts(VirtAddr::new(STACK_START as u64).as_ptr(), STACK_SIZE);

            new_stack.copy_from_slice(&old_stack[..STACK_SIZE]);
        }

        if let Some(cur_process_idx) = *cur_process_opt {
            if cur_process_idx < processes.len() {
                let mut allocated_ids = self.allocated_ids.write();
                let cur_process = &processes[cur_process_idx];
                let (code_selector, data_selector) = crate::gdt::get_usermode_segments();
                let mut ctx = context.clone();

                ctx.rax = 0;
                ctx.rsp += STACK_SIZE;
                ctx.cs = code_selector.0 as usize;
                ctx.ss = data_selector.0 as usize;
                let pid = self.get_available_pid_unlocked(&allocated_ids);

                allocated_ids.push(pid);

                let child_process = Process {
                    process_id: pid,
                    state: ProcessState::SavedContext(ctx),
                    page_table_phys: current_page_table_physaddr, // Use same address space
                    file_descriptors: cur_process.file_descriptors.clone(),
                };
                processes.push(Box::new(child_process));
                return pid;
            }
        }

        0 // Return 0 if fork failed
    }

    // exec sys_call function. Differs from `schedule` as it executes on the currently running process
    // rather than creating a new process and executing on that
    pub fn exec(&self, context: &mut Context, filename: String) -> usize {
        println!("{:?}", filename);
        let file = fs::vfs::open(&filename).unwrap();

        let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

        memory::switch_to_pagetable(user_page_table_physaddr);

        let entry_point = self
            .load_elf(&file, user_page_table_ptr)
            .expect("Failed to load ELF for exec");

        context.rsp = STACK_START + STACK_SIZE;
        context.rip = entry_point as usize;
        context.rcx = entry_point as usize;
        let (code_selector, data_selector) = crate::gdt::get_usermode_segments();
        context.cs = code_selector.0 as usize;
        context.ss = data_selector.0 as usize;

        self.cur_process.read().map(|cur_process_idx| {
            self.processes.write()[cur_process_idx].page_table_phys = user_page_table_physaddr;
        });
        0
    }

    /// Loads an ELF file by reading its segments directly into the provided page table.
    /// This function assumes that the provided `user_page_table_ptr` is active.
    fn load_elf(
        &self,
        file: &FileDescriptor,
        user_page_table_ptr: *mut PageTable,
    ) -> Result<u64, &'static str> {
        // NOTE: This implementation requires a `pread` (or `read_at`) function in your VFS
        // that can read from an arbitrary offset in a file. For this example, we'll
        // simulate it by cloning the file descriptor and setting the offset for each read

        let vbase = 0x400000;

        let mut header_fd = file.clone();
        header_fd.file.offset = 0;
        let mut elf_header_buf = [0u8; core::mem::size_of::<ElfHeader>()];
        fs::vfs::read(&header_fd, &mut elf_header_buf).map_err(|_| "Failed to read ELF header")?;
        let elf_header: ElfHeader = unsafe { core::mem::transmute(elf_header_buf) };

        // --- Validate ELF Header ---
        if elf_header.e_ident[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic number");
        }
        if elf_header.e_ident[EI_CLASS] != ELF_CLASS_64 {
            return Err("Not a 64-bit ELF file");
        }
        if elf_header.e_ident[EI_DATA] != ELF_DATA_2_LSB {
            return Err("Not a little-endian ELF file");
        }
        if elf_header.e_type != ET_EXEC && elf_header.e_type != ET_DYN {
            return Err("Not an executable or PIE file");
        }
        if elf_header.e_machine != EM_X86_64 {
            return Err("Not an x86_64 executable");
        }

        // --- Read Program Headers ---
        let ph_offset = elf_header.e_phoff;
        let ph_entsize = elf_header.e_phentsize as usize;
        let ph_num = elf_header.e_phnum as usize;
        let ph_table_size = ph_entsize * ph_num;

        let mut ph_table_buf = Vec::with_capacity(ph_table_size);
        ph_table_buf.resize(ph_table_size, 0);
        let mut ph_fd = file.clone();
        ph_fd.file.offset = ph_offset;
        fs::vfs::read(&ph_fd, &mut ph_table_buf).map_err(|_| "Failed to read program headers")?;

        let mut dynamic_vaddr = 0;

        // --- Load Segments and find PT_DYNAMIC ---
        for i in 0..ph_num {
            let ph_buf_start = i * ph_entsize;
            let ph_buf_end = ph_buf_start + ph_entsize;
            let ph_entry_buf = &ph_table_buf[ph_buf_start..ph_buf_end];
            let prog_header: ProgramHeader =
                unsafe { core::ptr::read(ph_entry_buf.as_ptr() as *const _) };

            match prog_header.p_type {
                PT_LOAD => {
                    let virt_addr = VirtAddr::new(vbase + prog_header.p_vaddr);
                    let mem_size = prog_header.p_memsz;
                    let file_size = prog_header.p_filesz;
                    let file_offset = prog_header.p_offset;

                    // Determine page flags from segment flags
                    let mut page_flags = PageTableFlags::PRESENT
                        | PageTableFlags::USER_ACCESSIBLE
                        | PageTableFlags::WRITABLE;
                    if (prog_header.p_flags & 0x2) != 0 {
                        // PF_W: Writable
                        page_flags |= PageTableFlags::WRITABLE;
                    }
                    if (prog_header.p_flags & 0x1) == 0 {
                        // PF_X: Executable (invert for NO_EXECUTE)
                        page_flags |= PageTableFlags::NO_EXECUTE;
                    }

                    // Allocate virtual memory for the segment
                    unsafe {
                        memory::allocate_pages(
                            user_page_table_ptr,
                            virt_addr,
                            mem_size,
                            page_flags,
                        )
                        .map_err(|_| "Failed to allocate pages for segment")?;
                    }

                    // Read segment data from file directly into the new memory
                    let segment_slice = unsafe {
                        core::slice::from_raw_parts_mut(virt_addr.as_mut_ptr(), file_size as usize)
                    };
                    let mut segment_fd = file.clone();
                    segment_fd.file.offset = file_offset;
                    fs::vfs::read(&segment_fd, segment_slice)
                        .map_err(|_| "Failed to read segment data")?;
                }
                PT_DYNAMIC => {
                    // This vaddr is the address of the _DYNAMIC array
                    dynamic_vaddr = vbase + prog_header.p_vaddr;
                }
                _ => {}
            }
        }

        // --- Allocate Stack and Heap ---
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(STACK_START as u64),
                STACK_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .map_err(|_| "Could not allocate user stack")?;
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(HEAP_START as u64),
                HEAP_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .map_err(|_| "Could not allocate user heap")?;
        }

        // --- Process Relocations ---
        if dynamic_vaddr != 0 {
            let mut rela_addr = 0;
            let mut rela_size = 0;
            let mut rela_ent = 0;
            let mut jmprel_addr = 0;
            let mut pltrel_size = 0;

            let mut dyn_ptr = dynamic_vaddr as *const Dyn;
            unsafe {
                while (*dyn_ptr).d_tag != DT_NULL {
                    match (*dyn_ptr).d_tag {
                        DT_RELA => rela_addr = (*dyn_ptr).d_val,
                        DT_RELASZ => rela_size = (*dyn_ptr).d_val,
                        DT_RELAENT => rela_ent = (*dyn_ptr).d_val,
                        DT_JMPREL => jmprel_addr = (*dyn_ptr).d_val,
                        DT_PLTRELSZ => pltrel_size = (*dyn_ptr).d_val,
                        _ => {}
                    }
                    dyn_ptr = dyn_ptr.add(1);
                }
            }

            // Process .rela.dyn relocations
            if rela_addr != 0 {
                self.perform_relocations(vbase, rela_addr, rela_size, rela_ent)?;
            }
            // Process .rela.plt relocations
            if jmprel_addr != 0 {
                // The size of a PLT relocation entry is always the size of Rela, so we don't need DT_PLTREL
                self.perform_relocations(
                    vbase,
                    jmprel_addr,
                    pltrel_size,
                    core::mem::size_of::<Rela>() as u64,
                )?;
            }
        }

        Ok(vbase + elf_header.e_entry)
    }

    fn perform_relocations(
        &self,
        vbase: u64,
        rela_addr: u64,
        rela_size: u64,
        rela_ent_size: u64,
    ) -> Result<(), &'static str> {
        if rela_ent_size as usize != core::mem::size_of::<Rela>() {
            return Err("Unsupported RELA entry size");
        }
        let rela_count = rela_size / rela_ent_size;
        let rela_table = unsafe {
            core::slice::from_raw_parts((vbase + rela_addr) as *const Rela, rela_count as usize)
        };

        for rela in rela_table {
            let r_type = (rela.r_info & 0xFFFFFFFF) as u32;
            match r_type {
                R_X86_64_RELATIVE => {
                    // This is a relative relocation, add the load address (vbase)
                    // The location to patch is at `vbase + r_offset`
                    // The value to write is `vbase + r_addend`
                    unsafe {
                        let location = (vbase + rela.r_offset) as *mut u64;
                        *location = (vbase as i64 + rela.r_addend) as u64;
                    }
                }
                typ => {
                    // Other relocation types like GLOB_DAT and JUMP_SLOT require symbol lookups,
                    // which is a much larger feature (dynamic linking). For now we will ignore them.
                    println!("Ignoring unsupported relocation type: {}", typ);
                }
            }
        }
        Ok(())
    }

    pub fn push_stdin(&self, key: u8) {
        let processes = self.processes.read();
        // for i in 0..processes.len() {
        let _ = fs::vfs::write(processes[0].file_descriptors.get(&0).unwrap(), &[key]);
        // }
    }

    pub fn write_file_descriptor(&self, id: u32, buf: &[u8]) {
        self.cur_process.read().map(|cur_process_idx| {
            let _ = fs::vfs::write(
                self.processes.read()[cur_process_idx]
                    .file_descriptors
                    .get(&id)
                    .unwrap(),
                buf,
            );
        });
    }

    pub fn read_file_descriptor(&self, id: u32, buf: &mut [u8]) -> isize {
        self.cur_process
            .read()
            .map(|cur_process_idx| {
                fs::vfs::read(
                    self.processes.read()[cur_process_idx]
                        .file_descriptors
                        .get(&id)
                        .unwrap(),
                    buf,
                )
                .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    pub fn add_file_descriptor(&self, fd: &FileDescriptor) -> usize {
        self.cur_process
            .read()
            .map(|cur_process_idx| {
                let fd_idx: usize = self.processes.read()[cur_process_idx]
                    .file_descriptors
                    .len();
                self.processes.write()[cur_process_idx]
                    .file_descriptors
                    .insert(fd_idx as u32, fd.clone());
                fd_idx
            })
            .unwrap()
    }

    pub fn ioctl(&self, fd: usize, cmd: u32, args: usize) {
        self.cur_process.read().map(|cur_process_idx| {
            let _ = fs::vfs::ioctl(
                self.processes.read()[cur_process_idx]
                    .file_descriptors
                    .get(&(fd as u32))
                    .unwrap(),
                cmd,
                args,
            );
        });
    }

    pub fn get_cur_pid(&self) -> usize {
        self.processes.read()[self.cur_process.read().unwrap_or(0)].process_id
    }
}

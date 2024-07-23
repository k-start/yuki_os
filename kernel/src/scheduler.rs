use crate::{
    elf,
    fs::{self, filesystem::FileDescriptor},
    gdt, memory,
    process::{Context, Process, ProcessState},
};
use alloc::{string::String, vec::Vec};
use elfloader::ElfBinary;
use spin::RwLock;
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

pub static SCHEDULER: RwLock<Scheduler> = RwLock::new(Scheduler::new());
static STACK_START: usize = 0x800000;
static STACK_SIZE: usize = 0x100000;
static HEAP_START: usize = 0x5000_0000_0000;
static HEAP_SIZE: usize = 0x100000;

pub struct Scheduler {
    processes: RwLock<Vec<Process>>,
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

    pub fn schedule(&self, file: FileDescriptor) {
        let (_current_page_table_ptr, current_page_table_physaddr) = memory::active_page_table();
        let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

        memory::switch_to_pagetable(user_page_table_physaddr);

        // let file_buf = if let Some(ptr) = file.ptr {
        //     unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, file.size as usize) }
        // } else {
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(0x500000000000),
                file.file.size,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");
        }

        // fix me - terrible loading
        let file_buf: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(0x500000000000 as *mut u8, file.file.size as usize)
        };
        let _ = fs::vfs::read(&file, file_buf);
        //     file_buf
        // };

        let binary = ElfBinary::new(file_buf).unwrap();
        let mut loader = elf::loader::UserspaceElfLoader {
            vbase: 0x400000,
            user_page_table_ptr,
        };
        binary.load(&mut loader).expect("Can't load the binary");

        let entry_point = loader.vbase + binary.entry_point();

        unsafe {
            memory::deallocate_pages(
                user_page_table_ptr,
                VirtAddr::new(0x500000000000),
                file.file.size,
            )
            .expect("Could not deallocate memory");
        }

        // user stack
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(STACK_START as u64),
                STACK_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate user stack");
        }
        // user heap
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(HEAP_START as u64),
                HEAP_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate user heap");
        }

        memory::switch_to_pagetable(current_page_table_physaddr);

        let process = Process::new(
            VirtAddr::new(entry_point),
            VirtAddr::new((STACK_START + STACK_SIZE) as u64),
            user_page_table_physaddr,
            0,
        );

        self.allocated_ids.write().push(process.process_id);
        self.processes.write().push(process);
    }

    /// Initialize a new OffsetPageTable.
    ///
    /// # Safety
    ///
    /// This function is unsafe as it derefences the context provided
    /// This should only be called by the interrupt that is switching
    /// contexts.
    pub unsafe fn save_current_context(&self, context: *const Context) {
        self.cur_process.read().map(|cur_process_idx| {
            if self.processes.read()[cur_process_idx].state == ProcessState::Exiting() {
                let pid = self.processes.read()[cur_process_idx].process_id;
                self.allocated_ids.write().remove(pid);
                self.processes.write().remove(cur_process_idx);
                println!("Exited process #{}", pid);
            } else {
                let ctx = (*context).clone();
                self.processes.write()[cur_process_idx].state = ProcessState::SavedContext(ctx);
            }
        });
    }

    pub fn run_next(&self) -> Context {
        let processes_len = self.processes.read().len(); // how many processes are available
        if processes_len > 0 {
            let process_state = {
                let mut cur_process_opt = self.cur_process.write(); // lock the current process index

                let next_process = if cur_process_opt.is_none() {
                    // properly start at process 0
                    0
                } else {
                    let cur_process = cur_process_opt.get_or_insert(0); // default to 0
                    (*cur_process + 1) % processes_len // next process index
                };

                let cur_process = cur_process_opt.get_or_insert(processes_len);
                *cur_process = next_process;
                let process = &self.processes.read()[next_process]; // get the next process

                // println!("Switching to process #{} ({})", cur_process, process);

                memory::switch_to_pagetable(process.page_table_phys);

                process.state.clone() // clone process state information
            }; // release held locks
            match process_state {
                ProcessState::SavedContext(context) => {
                    return context; // either restore the saved context
                }
                ProcessState::StartingInfo(exec_base, stack_end) => {
                    jmp_to_usermode(exec_base, stack_end); // or initialize the process with the given instruction, stack pointers
                    todo!();
                }
                ProcessState::Exiting() => {
                    todo!();
                }
            }
        }

        todo!();
    }

    pub fn exit_current(&self) {
        // FIX ME - janky exiting due to TSS not set up for syscall
        self.cur_process.read().map(|cur_process_idx| {
            println!(
                "Exiting process #{}",
                self.processes.read()[cur_process_idx].process_id
            );
            self.processes.write()[cur_process_idx].state = ProcessState::Exiting();
        });

        // let next_process = (*cur_process + 1) % processes_len;
        // *cur_process = next_process;
        // unsafe {
        //     self.run_next();
        // };
        // x86_64::instructions::interrupts::enable();
        // hlt_loop();
        // unsafe {
        //     core::arch::asm!("sti", "2:", "hlt", "jmp 2b");
        // }
    }

    pub fn fork_current(&self, context: Context) -> usize {
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

        let mut pid = 0;

        let child_process = self.cur_process.read().map(|cur_process_idx| {
            let cur_process = &self.processes.read()[cur_process_idx];
            let (code_selector, data_selector) = crate::gdt::get_usermode_segments();
            let mut ctx = context.clone();

            ctx.rax = 0;
            ctx.rsp += STACK_SIZE;
            ctx.cs = code_selector.0 as usize;
            ctx.ss = data_selector.0 as usize;
            pid = self.get_available_pid();

            self.allocated_ids.write().push(pid);

            Process {
                process_id: pid,
                state: ProcessState::SavedContext(ctx),
                page_table_phys: current_page_table_physaddr, // Use same address space
                file_descriptors: cur_process.file_descriptors.clone(),
            }
        });

        self.processes.write().push(child_process.unwrap());

        pid
    }

    // exec sys_call function. Differs from `schedule` as it executes on the currently running process
    // rather than creating a new process and executing on that
    pub fn exec(&self, context: &mut Context, filename: String) -> usize {
        println!("{:?}", filename);
        let file = fs::vfs::open(&filename).unwrap();

        let (_current_page_table_ptr, _current_page_table_physaddr) = memory::active_page_table();
        let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

        memory::switch_to_pagetable(user_page_table_physaddr);

        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(0x500000000000),
                file.file.size as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");
        }

        let file_buf: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(0x500000000000 as *mut u8, file.file.size as usize)
        };
        let _ = fs::vfs::read(&file, file_buf);

        let binary = ElfBinary::new(file_buf).unwrap();
        let mut loader = elf::loader::UserspaceElfLoader {
            vbase: 0x400000,
            user_page_table_ptr,
        };
        binary.load(&mut loader).expect("Can't load the binary");

        let entry_point = loader.vbase + binary.entry_point();

        unsafe {
            memory::deallocate_pages(
                user_page_table_ptr,
                VirtAddr::new(0x500000000000),
                file.file.size as u64,
            )
            .expect("Could not deallocate memory");
        }

        // user heap
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(STACK_START as u64),
                STACK_SIZE as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");
        }

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

    pub fn get_available_pid(&self) -> usize {
        for i in 1..1000 {
            // FIX ME - terrible search for PID
            if !self.allocated_ids.read().contains(&i) {
                return i;
            }
        }
        return 0;
    }
}

#[inline(never)]
pub fn jmp_to_usermode(code: VirtAddr, stack_end: VirtAddr) {
    unsafe {
        let (cs_idx, ds_idx) = gdt::set_usermode_segments();
        x86_64::instructions::tlb::flush_all(); // flush the TLB after address-space switch

        core::arch::asm!(
            "cli",        // Disable interrupts
            "push {:r}",  // Stack segment (SS)
            "push {:r}",  // Stack pointer (RSP)
            "push 0x200", // RFLAGS with interrupts enabled
            "push {:r}",  // Code segment (CS)
            "push {:r}",  // Instruction pointer (RIP)
            "iretq",
            in(reg) ds_idx,
            in(reg) stack_end.as_u64(),
            in(reg) cs_idx,
            in(reg) code.as_u64(),
        );
    }
}

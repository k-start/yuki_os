use crate::{
    elf,
    fs::{self, filesystem::FileDescriptor},
    gdt, memory,
    process::{Context, Process, ProcessState},
};
use alloc::{boxed::Box, string::String, vec::Vec};
use elfloader::ElfBinary;
use spin::RwLock;
use x86_64::{registers::rflags::RFlags, structures::paging::PageTableFlags, VirtAddr};

pub static SCHEDULER: RwLock<Scheduler> = RwLock::new(Scheduler::new());
static STACK_START: usize = 0x800000;
static STACK_SIZE: usize = 0x100000;
static HEAP_START: usize = 0x5000_0000_0000;
static HEAP_SIZE: usize = 0x100000;

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
                println!("Switching to process #{}", process.process_id);

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
}

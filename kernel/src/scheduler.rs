use crate::{
    elf,
    fs::{self, filesystem::FileDescriptor},
    gdt, memory,
    process::{Context, Process, ProcessState},
};
use alloc::vec::Vec;
use elfloader::ElfBinary;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

pub struct Scheduler {
    processes: Mutex<Vec<Process>>,
    cur_process: Mutex<Option<usize>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            processes: Mutex::new(Vec::new()),
            cur_process: Mutex::new(None), // so that next process is 0
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
                file.file.size as u64,
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
                file.file.size as u64,
            )
            .expect("Could not deallocate memory");
        }

        // user heap
        unsafe {
            memory::allocate_pages(
                user_page_table_ptr,
                VirtAddr::new(0x800000),
                0x1000_u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");
        }

        memory::switch_to_pagetable(current_page_table_physaddr);

        let process = Process::new(
            VirtAddr::new(entry_point),
            VirtAddr::new(0x801000),
            user_page_table_physaddr,
        );

        self.processes.lock().push(process);
    }

    pub fn save_current_context(&self, context: *const Context) {
        self.cur_process.lock().map(|cur_process_idx| {
            if self.processes.lock()[cur_process_idx].state == ProcessState::Exiting() {
                self.processes.lock().remove(cur_process_idx);
                println!("Exited process #{}", cur_process_idx);
            } else {
                let ctx = unsafe { (*context).clone() };
                self.processes.lock()[cur_process_idx].state = ProcessState::SavedContext(ctx);
            }
        });
    }

    pub unsafe fn run_next(&self) -> Context {
        let processes_len = self.processes.lock().len(); // how many processes are available
        if processes_len > 0 {
            let process_state = {
                let mut cur_process_opt = self.cur_process.lock(); // lock the current process index

                let next_process = if cur_process_opt.is_none() {
                    // properly start at process 0
                    0
                } else {
                    let cur_process = cur_process_opt.get_or_insert(0); // default to 0
                    (*cur_process + 1) % processes_len // next process index
                };

                let cur_process = cur_process_opt.get_or_insert(processes_len);
                *cur_process = next_process;
                let process = &self.processes.lock()[next_process]; // get the next process

                // println!("Switching to process #{} ({})", next_process, process);

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
        self.cur_process.lock().map(|cur_process_idx| {
            println!("Exiting process #{}", cur_process_idx);
            self.processes.lock()[cur_process_idx].state = ProcessState::Exiting();
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

    pub fn push_stdin(&self, key: u8) {
        let mut processes = self.processes.lock();
        for i in 0..processes.len() {
            let _ = processes[i].vfs.write_stdin(&[key]);
        }
    }

    pub fn pop_stdin(&self, buf: &mut [u8]) {
        self.cur_process.lock().map(|cur_process_idx| {
            let _ = &self.processes.lock()[cur_process_idx].vfs.read_stdin(buf);
        });
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

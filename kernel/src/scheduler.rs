use crate::{
    elf,
    fs::{self, filesystem::File},
    gdt, memory,
    process::{Context, Task, TaskState},
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
    tasks: Mutex<Vec<Task>>,
    cur_task: Mutex<Option<usize>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            tasks: Mutex::new(Vec::new()),
            cur_task: Mutex::new(None), // so that next task is 0
        }
    }

    pub fn schedule(&self, file: File) {
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
                file.size as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");
        }

        // fix me - terrible loading
        let file_buf: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(0x500000000000 as *mut u8, file.size as usize)
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
                file.size as u64,
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

        let task = Task::new(
            VirtAddr::new(entry_point),
            VirtAddr::new(0x801000),
            user_page_table_physaddr,
        );

        self.tasks.lock().push(task);
    }

    pub fn save_current_context(&self, context: *const Context) {
        self.cur_task.lock().map(|cur_task_idx| {
            if self.tasks.lock()[cur_task_idx].state == TaskState::Exiting() {
                self.tasks.lock().remove(cur_task_idx);
                println!("Exited task #{}", cur_task_idx);
            } else {
                let ctx = unsafe { (*context).clone() };
                self.tasks.lock()[cur_task_idx].state = TaskState::SavedContext(ctx);
            }
        });
    }

    pub unsafe fn run_next(&self) -> Context {
        let tasks_len = self.tasks.lock().len(); // how many tasks are available
        if tasks_len > 0 {
            let task_state = {
                let mut cur_task_opt = self.cur_task.lock(); // lock the current task index

                let next_task = if cur_task_opt.is_none() {
                    // properly start at task 0
                    0
                } else {
                    let cur_task = cur_task_opt.get_or_insert(0); // default to 0
                    (*cur_task + 1) % tasks_len // next task index
                };

                let cur_task = cur_task_opt.get_or_insert(tasks_len);
                *cur_task = next_task;
                let task = &self.tasks.lock()[next_task]; // get the next task

                println!("Switching to task #{} ({})", next_task, task);

                memory::switch_to_pagetable(task.page_table_phys);

                task.state.clone() // clone task state information
            }; // release held locks
            match task_state {
                TaskState::SavedContext(context) => {
                    return context; // either restore the saved context
                }
                TaskState::StartingInfo(exec_base, stack_end) => {
                    jmp_to_usermode(exec_base, stack_end); // or initialize the task with the given instruction, stack pointers
                    todo!();
                }
                TaskState::Exiting() => {
                    todo!();
                }
            }
        }

        todo!();
    }

    pub fn exit_current(&self) {
        // FIX ME - janky exiting due to TSS not set up for syscall
        self.cur_task.lock().map(|cur_task_idx| {
            println!("Exiting task #{}", cur_task_idx);
            self.tasks.lock()[cur_task_idx].state = TaskState::Exiting();
        });

        // let next_task = (*cur_task + 1) % tasks_len;
        // *cur_task = next_task;
        // unsafe {
        //     self.run_next();
        // };
        // x86_64::instructions::interrupts::enable();
        // hlt_loop();
        // unsafe {
        //     core::arch::asm!("sti", "2:", "hlt", "jmp 2b");
        // }
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

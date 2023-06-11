use crate::{
    elf,
    fs::{self, filesystem::File},
    gdt, memory,
};
use alloc::vec::Vec;
use core::fmt::Display;
use elfloader::ElfBinary;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{
    structures::paging::{PageTable, PageTableFlags},
    VirtAddr,
};

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
        println!("{}", self.tasks.lock().len());
    }

    pub fn save_current_context(&self, context: *const Context) {
        self.cur_task.lock().map(|cur_task_idx| {
            let ctx = unsafe { (*context).clone() };
            self.tasks.lock()[cur_task_idx].state = TaskState::SavedContext(ctx);
        });
    }

    pub unsafe fn run_next(&self) -> Context {
        let tasks_len = self.tasks.lock().len(); // how many tasks are available
        if tasks_len > 0 {
            let task_state = {
                let mut cur_task_opt = self.cur_task.lock(); // lock the current task index
                let cur_task = cur_task_opt.get_or_insert(0); // default to 0
                let next_task = (*cur_task + 1) % tasks_len; // next task index
                *cur_task = next_task;
                let task = &self.tasks.lock()[next_task]; // get the next task

                println!("Switching to task #{} ({})", next_task, task);

                memory::switch_to_pagetable(task.page_table_phys);

                task.state.clone() // clone task state information
            }; // release held locks
            match task_state {
                TaskState::SavedContext(context) => {
                    // restore_context(&ctx) // either restore the saved context
                    return context;
                }
                TaskState::StartingInfo(exec_base, stack_end) => {
                    jmp_to_usermode(exec_base, stack_end); // or initialize the task with the given instruction, stack pointers
                    todo!();
                }
            }
        }
        todo!();
    }
}

#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct Context {
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[derive(Clone, Debug)]
enum TaskState {
    // a task's state can either be
    SavedContext(Context),            // a saved context
    StartingInfo(VirtAddr, VirtAddr), // or a starting instruction and stack pointer
}

struct Task {
    state: TaskState,     // the current state of the task
    page_table_phys: u64, // the page table for this task
}

impl Task {
    pub fn new(exec_base: VirtAddr, stack_end: VirtAddr, page_table_phys: u64) -> Task {
        Task {
            state: TaskState::StartingInfo(exec_base, stack_end),
            page_table_phys,
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PT: {}, Context: {:x?}",
            self.page_table_phys, self.state
        )
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

pub unsafe fn get_context() -> *const Context {
    let context: *const Context;
    core::arch::asm!(
        "cli",
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push r11",
        "push r10",
        "push r9",
        "push r8",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rbx",
        "push rax",
        "push rbp",
        "mov {}, rsp",
        "sub rsp, 0x400",
        out(reg) context
    );
    context
}

// pub unsafe fn restore_context(context: &Context) {
//     core::arch::asm!(
//         "mov rsp, {:r}",
//         "pop rbp",
//         "pop rax",
//         "pop rbx",
//         "pop rcx",
//         "pop rdx",
//         "pop rsi",
//         "pop rdi",
//         "pop r8",
//         "pop r9",
//         "pop r10",
//         "pop r11",
//         "pop r12",
//         "pop r13",
//         "pop r14",
//         "pop r15",
//         "sti",
//         "iretq",
//         in(reg) context
//     );
// }

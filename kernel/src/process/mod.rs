use crate::{
    fs::{self, file::FileDescriptor},
    scheduler,
};
use alloc::{collections::BTreeMap, format, sync::Arc};
use core::fmt::Display;
use x86_64::{PhysAddr, VirtAddr};

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    // a task's state can either be
    SavedContext(Context),            // a saved context
    StartingInfo(VirtAddr, VirtAddr), // or a starting instruction and stack pointer
    Exiting(),
}

pub struct Process {
    pub process_id: usize,
    pub state: ProcessState,       // the current state of the process
    pub page_table_phys: PhysAddr, // the page table for this process
    pub file_descriptors: BTreeMap<u32, Arc<FileDescriptor>>, // file descriptors for Stdio
}

impl Process {
    pub fn new(
        exec_base: VirtAddr,
        stack_end: VirtAddr,
        page_table_phys: PhysAddr,
        parent_id: usize,
    ) -> Process {
        let id = if parent_id == 0 {
            scheduler::SCHEDULER.read().get_available_pid()
        } else {
            parent_id
        };

        let mut file_descriptors = BTreeMap::new();
        // For a new process, create file descriptors for stdin and stdout
        // These will point to the stdio virtual filesystem
        // if let Ok(stdin) = fs::vfs::open(&format!("/stdio/{id}/stdin")) {
        //     file_descriptors.insert(0, stdin);
        // }
        // if let Ok(stdout) = fs::vfs::open(&format!("/stdio/{id}/stdout")) {
        //     // In a real implementation, you'd also open stderr, for now, just clone stdout
        //     if let Some(stdout_fd) = stdout.clone().into_iter().next() {
        //         file_descriptors.insert(2, stdout_fd);
        //     }
        //     file_descriptors.insert(1, stdout);
        // }

        Process {
            process_id: id,
            state: ProcessState::StartingInfo(exec_base, stack_end),
            page_table_phys,
            file_descriptors,
        }
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PT: {}, Context: {:#x?}",
            self.page_table_phys.as_u64(),
            self.state
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
#[repr(C, packed)]
pub struct Context {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rbp: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
}

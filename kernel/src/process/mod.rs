use crate::fs::{self, filesystem::FileDescriptor};
use alloc::{collections::BTreeMap, format};
use core::fmt::Display;
use x86_64::VirtAddr;

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    // a task's state can either be
    SavedContext(Context),            // a saved context
    StartingInfo(VirtAddr, VirtAddr), // or a starting instruction and stack pointer
    Exiting(),
}

pub struct Process {
    pub state: ProcessState,  // the current state of the process
    pub page_table_phys: u64, // the page table for this process
    pub file_descriptors: BTreeMap<u32, FileDescriptor>, // file descriptors for Stdio
}

impl Process {
    pub fn new(exec_base: VirtAddr, stack_end: VirtAddr, page_table_phys: u64, id: u32) -> Process {
        let mut file_descriptors = BTreeMap::new();
        file_descriptors.insert(0, fs::vfs::open(&format!("/stdio/{id}/stdin")).unwrap());
        file_descriptors.insert(1, fs::vfs::open(&format!("/stdio/{id}/stdout")).unwrap());

        Process {
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
            "PT: {}, Context: {:x?}",
            self.page_table_phys, self.state
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

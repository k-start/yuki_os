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

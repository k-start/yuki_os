use crate::{gdt, inb, keyboard, outb, process::Context, scheduler};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use ps2_mouse::{Mouse, MouseState};
use spin::{self, Mutex};
use x86_64::{
    instructions::port::Port,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Mouse = PIC_1_OFFSET + 12,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

            idt.page_fault
                .set_handler_fn(page_fault_handler)
                .set_stack_index(gdt::PAGE_FAULT_IST_INDEX);
            idt.general_protection_fault
                .set_handler_fn(general_protection_fault_handler)
                .set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST_INDEX);
            idt[InterruptIndex::Timer.as_usize()]
                .set_handler_fn(timer_handler_naked)
                .set_stack_index(gdt::TIMER_INTERRUPT_INDEX);
        }
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);
        idt
    };
}

pub fn init() {
    IDT.load();
    unsafe { PICS.lock().initialize() };

    // Initialize mouse
    // Address the 2nd device
    outb(0x64, 0xd4);
    // Write it
    outb(0x60, 0xF4);
    // Read back
    while inb(0x60) != 0xFA {} /* Wait for ACK from mouse... */
    outb(0x64, 0x20); /* Write 0x20 to port 0x64 */
    let mut config = inb(0x60); /* Read byte from port 0x60 */
    config = config | 1 << 1; /* Set bit 1*/
    outb(0x64, 0x60); /* Write 0x60 to port 0x64 */
    outb(0x60, config); /* Write config byte to port 0x60 */

    // x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("Stack Frame: {:#?}", stack_frame);
    println!("Error: {:?}", error_code);
    crate::hlt_loop();
}

extern "C" fn timer_interrupt_handler(context_addr: *const Context) -> *const Context {
    unsafe {
        scheduler::SCHEDULER
            .read()
            .save_current_context(context_addr)
    };

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }

    let next = scheduler::SCHEDULER.read().run_next();
    &next as *const Context
}

pub static MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());

// Initialize the mouse and set the on complete event.
pub fn init_mouse() {
    MOUSE.lock().init().unwrap();
    MOUSE.lock().set_on_complete(on_complete);
}

// This will be fired when a packet is finished being processed.
fn on_complete(mouse_state: MouseState) {
    println!("{:?}", mouse_state);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    keyboard::handle_key(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let packet: u8 = unsafe { port.read() };
    MOUSE.lock().process_packet(packet);

    // println!("[Kernel] {:?}", packet);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

#[macro_export]
macro_rules! interrupt_wrap {
    ($func: ident => $wrapper:ident) => {
        #[naked]
        pub extern "x86-interrupt" fn $wrapper (_stack_frame: InterruptStackFrame) {
            // Naked functions must consist of a single asm! block
            unsafe{
                core::arch::asm!(
                    // Disable interrupts
                    "cli",
                    // Push registers
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",

                    "push rdi",
                    "push rsi",
                    "push rbp",
                    "push r8",

                    "push r9",
                    "push r10",
                    "push r11",
                    "push r12",

                    "push r13",
                    "push r14",
                    "push r15",

                    // First argument in rdi with C calling convention
                    "mov rdi, rsp",
                    // Call the hander function
                    "call {handler}",

                    // New stack pointer is in RAX
                    // (C calling convention return value)
                    "cmp rax, 0",
                    "je 2f", // If RAX is zero, keep stack
                    "mov rsp, rax",
                     "2:",

                    // Pop scratch registers from new stack
                    "pop r15",
                    "pop r14",
                    "pop r13",

                    "pop r12",
                    "pop r11",
                    "pop r10",
                    "pop r9",

                    "pop r8",
                    "pop rbp",
                    "pop rsi",
                    "pop rdi",

                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
                    // Enable interrupts
                    "sti",
                    // Interrupt return
                    "iretq",
                    // Note: Getting the handler pointer here using `sym` operand, because
                    // an `in` operand would clobber a register that we need to save, and we
                    // can't have two asm blocks
                    handler = sym $func,
                    options(noreturn)
                );
            }
        }
    };
}

interrupt_wrap!(timer_interrupt_handler => timer_handler_naked);

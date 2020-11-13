#![no_std]
#![no_main]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(global_asm)]
#![feature(alloc_error_handler)]
#![feature(never_type)]
#![feature(const_generics)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_btree_new)]
#![feature(crate_visibility_modifier)]
#![feature(panic_info_message)]
#![feature(concat_idents)]
#![allow(incomplete_features)]
#![feature(new_uninit)]
#![feature(const_fn)]
#![feature(slice_ptr_len)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate num_derive;
extern crate static_assertions;

#[macro_use]
pub mod drivers;

pub mod aarch64;
pub mod boot;
pub mod interupts;
pub mod io;
pub mod memory;
pub mod scheduler;
pub mod syscall;

pub mod config;
pub mod sync;
pub mod userspace;
pub mod vfs;

pub mod utils;

use core::panic::PanicInfo;

use aarch64::*;
use utils::binary_info;

#[cfg(not(feature = "raspi4"))]
const MMIO_BASE: usize = 0x3F00_0000;
#[cfg(feature = "raspi4")]
const MMIO_BASE: usize = 0xFE00_0000;

const INTERRUPT_CONTROLLER_BASE: usize = MMIO_BASE + 0xB200;
const KERNEL_OFFSET: usize = !((1usize << 36) - 1);

use drivers::traits::console::*;
use drivers::traits::Init;

use drivers::rpi3_interrupt_controller::Rpi3InterruptController;

use drivers::arm_timer::ArmTimer;
use drivers::traits::time::Timer;

fn kernel_entry() -> ! {
    let uart = drivers::UART.lock();
    match uart.init() {
        Ok(_) => println!("\x1B[2J\x1B[2;1H\x1B[2J\x1B[2;1H[ Ok ] UART is live!"),
        Err(_) => halt(), // If UART fails, abort early
    }
    drop(uart);
    let binary_info = binary_info::BinaryInfo::get();
    println!("{}", binary_info);
    println!("{:?}", crate::memory::allocator::kernel_heap_range());
    unsafe {
        interupts::init_exceptions(binary_info.exception_vector);
    }

    println!("Enabling ARM Timer");

    let _controller = Rpi3InterruptController::new(INTERRUPT_CONTROLLER_BASE);

    println!("TEST mmu");
    unsafe {
        if let Err(msg) = memory::armv8::mmu::init_mmu() {
            panic!(msg);
        }

        jump_to_kernel_space(echo);
    }
}
unsafe fn jump_to_kernel_space(f: fn() -> !) -> ! {
    let address = f as *const () as u64;
    llvm_asm!("brk 0" : : "{x0}"(address) : : "volatile");

    loop {}
}
fn echo() -> ! {
    println!("Echoing input.");

    unsafe {
        let t_string: &'static str = "Hello String";
        let ptr = t_string.as_ptr();
        let size = t_string.bytes().len();

        let user_ptr = ((!KERNEL_OFFSET) & ptr as usize) as *const u8;
        let user_str = core::str::from_utf8_unchecked(core::slice::from_raw_parts(user_ptr, size));

        crate::println!("ORGINAL {:#018x}: {}", t_string.as_ptr() as usize, t_string);
        crate::println!("USER    {:#018x}: {}", user_str.as_ptr() as usize, user_str);

        use crate::memory::memory_controler::{
            map_kernel_memory, Granule, RangeDescriptor, Translation, KERNEL_RW_,
        };

        let pages_containing = |pointer: *const u8, size: usize| {
            let start_address = pointer.add(pointer.align_offset(4096)).offset(-4096) as usize;
            let end_address = pointer.add(size).add(pointer.align_offset(4096)) as usize;
            start_address..end_address
        };
        let p_range = pages_containing(user_ptr, size);
        let v_range = p_range.start | 0x1_0000_0000..p_range.end | 0x1_0000_0000;

        crate::println!("memory {:x} - {:x}", p_range.start, p_range.end);
        crate::println!("memory {:x} - {:x}", v_range.start, v_range.end);

        config::set_debug_mmu(true);

        map_kernel_memory("moved_string", v_range, p_range.start, true);
        // crate::memory::armv8::mmu::add_translation(
        //     user_ptr as usize,
        //     user_ptr as usize | 0x1_0000_0000,
        // );

        let moved_ptr = (ptr as u64 | 0x1_0000_0000) as *const u8;
        let moved_str =
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(moved_ptr, size));

        crate::println!(
            "MOVED   {:#018x}: {}",
            moved_str.as_ptr() as usize,
            moved_str
        );
    }

    let task1 = scheduler::task_context::TaskContext::new(scheduler::first_task, false)
        .expect("Error creating task 1 context");
    // let task2 = scheduler::task_context::TaskContext::new(userspace::task_two, false)
        // .expect("Error creating task 2 context");
    scheduler::add_task(task1).expect("Error adding task 1");
    // scheduler::add_task(task2).expect("Error adding task 2");
  
    unsafe {
        interupts::init_exceptions(
            utils::binary_info::BinaryInfo::get().exception_vector | KERNEL_OFFSET,
        );
    }

    // config::set_debug_alloc(true);

    interupts::enable_irqs();
    
    {
        let timer = ArmTimer {};

        timer.interupt_after(scheduler::get_time_quant());
        timer.enable();
    }
    println!("Kernel Initialization complete.");

    syscall::start_scheduling();

    let mut uart = drivers::UART.lock();
    uart.move_uart();
    uart.puts("string\n\n\n");
    println!("{:x}", uart.get_base_address());

    let mut uart = drivers::UART.lock();
    uart.base_address |= KERNEL_OFFSET;
    let echo_loop = || -> Result<!, &str> {
        loop {
            uart.putc(uart.getc()?);
        }
    };
    loop {
        let value = echo_loop().unwrap_err();
        println!("{}", value);
    }
}

entry!(kernel_entry);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(args) = info.message() {
        eprintln!("\nKernel panic: {}", args);
    } else {
        eprintln!("\nKernel panic!");
    }

    halt();
}

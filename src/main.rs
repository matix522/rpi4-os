#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(alloc_error_handler)]

// extern crate spin;
extern crate alloc;

pub mod gpio;
pub mod mbox;
pub mod uart;
pub mod io;
pub mod time;
pub mod interupt;
pub mod sync;
pub mod memory;

use aarch64::*;

#[cfg(not(feature = "raspi4"))]
const MMIO_BASE: u32 = 0x3F00_0000;
#[cfg(feature = "raspi4")]
const MMIO_BASE: u32 = 0xFE00_0000;


use alloc::vec::Vec;

extern "C" {
    pub fn _boot_cores() -> !;
    pub static __exception_vectors_start: u64;
    static mut __binary_end : u64;
}

fn kernel_entry() -> ! {
    let mut mbox = mbox::Mbox::new();
    let uart = uart::Uart::new();

    match uart.init(&mut mbox) {
        Ok(_) =>  println!("\x1B[2J\x1B[2;1H[ Ok ] UART is live!"),
        Err(_) => halt()  // If UART fails, abort early
    }

    println!("Exception Level: {:?}", boot::mode::ExceptionLevel::get_current());

    println!("Binary loaded at: {:x} - {:x}", _boot_cores as *const () as u64,unsafe { &__binary_end as *const u64 as u64});
    println!("Init Task Stack: {:x} - {:x}", _boot_cores as *const () as u64, 0);
    println!("Main Heap: {:x} - {:x}", memory::allocator::heap_start(), memory::allocator::heap_end());

    print!("Initializing Interupt Vector Table: ");
    unsafe { 
        
        let exception_vectors_start: u64 = &__exception_vectors_start as *const _ as u64;
        println!("{:x}", exception_vectors_start);
        interupt::set_vector_table_pointer(exception_vectors_start); 
    }
    // use interupt::timer::ArmQemuTimer as Timer;
    // interupt::daif_clr(2);
    // Timer::interupt_after(Timer::get_frequency());
    // Timer::enable();

    println!("Kernel Initialization complete.");

    let mut vector = Vec::new();
    for i in 0..20 {
        vector.push(i);
    }
    for i in &vector {
        print!("{} ", i);
    }
    println!("");
    core::mem::drop(vector);
    // echo everything back
    loop { 
        uart.send(uart.getc());
    }
}

boot::entry!(kernel_entry);
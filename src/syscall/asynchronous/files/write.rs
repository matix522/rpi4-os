use super::*;
use crate::syscall::asynchronous::async_returned_values::*;
use crate::syscall::asynchronous::async_syscall::*;
use crate::utils::circullar_buffer::*;
use crate::vfs;

pub struct AsyncWriteSyscallData {
    pub afd: usize,
    pub message: &'static str,
}

impl AsyncWriteSyscallData {
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { crate::utils::struct_to_slice::any_as_u8_slice(self) }
    }
}

pub fn write(
    afd: &AsyncFileDescriptor,
    message: &'static str,
    id: usize,
    submission_buffer: &mut CircullarBuffer,
) {
    let data = AsyncWriteSyscallData {
        afd: afd.to_usize(),
        message,
    };

    let bytes = data.as_bytes();

    let a: AsyncSyscall = AsyncSyscall {
        data: bytes,
        id,
        data_size: bytes.len(),
        syscall_type: AsyncSyscalls::WriteFile,
    };

    crate::syscall::asynchronous::async_syscall::send_async_syscall(submission_buffer, a);
}

pub fn handle_async_write(
    ptr: *const u8,
    len: usize,
    returned_values: &mut AsyncReturnedValues,
) -> usize {
    let syscall_data: &AsyncWriteSyscallData = unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        crate::utils::struct_to_slice::u8_slice_to_any(slice)
    };

    let fd = match AsyncFileDescriptor::from_usize(syscall_data.afd) {
        AsyncFileDescriptor::FileDescriptor(val) => val,
        AsyncFileDescriptor::AsyncSyscallReturnValue(val) => match returned_values.map.get(&val) {
            None => return ONLY_MSB_OF_USIZE | vfs::FileError::ReadOnClosedFile as usize,
            Some((syscall_type, returned_value)) => {
                if let AsyncSyscalls::OpenFile = syscall_type {
                    if *returned_value & ONLY_MSB_OF_USIZE > 0 {
                        return *returned_value;
                    } else {
                        *returned_value
                    }
                } else {
                    return ONLY_MSB_OF_USIZE | vfs::FileError::ReadOnClosedFile as usize;
                }
            }
        },
    };

    let current_task = crate::scheduler::get_current_task_context();
    let fd_table = unsafe { &mut (*current_task).file_descriptor_table };
    let opened_file = fd_table.get_file_mut(fd).unwrap();
    match vfs::write(opened_file, syscall_data.message) {
        Ok(_) => 0,
        Err(err) => ONLY_MSB_OF_USIZE | err as usize,
    }
}

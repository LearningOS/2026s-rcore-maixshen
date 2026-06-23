//! Process management syscalls

use crate::config::SYSCALL_COUNT;
use crate::mm::translated_byte_buffer;
use crate::mm::PTEFlags;
use crate::mm::PageTable;
use crate::mm::VirtAddr;
use crate::task::current_user_token;
use crate::task::get_syscall_times;
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
use crate::task::{do_task_mmap, do_task_munmap};
use crate::timer::get_time_us;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffers =
        translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let vpn = VirtAddr::from(id).floor();

    match trace_request {
        0 => {
            // Read
            let pte = page_table.translate(vpn);
            if pte.is_none()
                || !pte.unwrap().readable()
                || !pte.unwrap().flags().contains(PTEFlags::U)
            {
                return -1;
            }
            let buffers = translated_byte_buffer(token, id as *const u8, 1);
            if buffers.is_empty() {
                -1
            } else {
                buffers[0][0] as isize
            }
        }
        1 => {
            // Write
            let pte = page_table.translate(vpn);
            if pte.is_none()
                || !pte.unwrap().writable()
                || !pte.unwrap().flags().contains(PTEFlags::U)
            {
                return -1;
            }
            let mut buffers = translated_byte_buffer(token, id as *mut u8, 1);
            if buffers.is_empty() {
                return -1;
            }
            buffers[0][0] = (data & 0xFF) as u8;
            0
        }
        2 => {
            // Query ...
            if id >= SYSCALL_COUNT {
                return -1;
            }
            get_syscall_times(id) as isize
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // trace!("kernel: sys_mmap {:#x} ~ {:#x}", _start, _start + _len);
    let len = (len + 0xfff) & !0xfff;
    if port & 0x7 == 0 {
        return -1; // Useless Mapping Page
    }
    if start & 0xfff != 0 || port & !(0b111 as usize) != 0 {
        return -1; // Tutorial Request
    }
    if len == 0 {
        return 0; //{ISSUE}
    }

    if do_task_mmap(start, len, port) {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _len == 0 {
        return 0; //{ISSUE}
    }
    let _len = (_len + 0xfff) & !0xfff;
    //info!(">>>>>>>>>{:#x}!!{:#x}", _start, _start + _len);// any info/error! here will stuck system
    if do_task_munmap(_start, _len) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

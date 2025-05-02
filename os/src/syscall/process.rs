//! Process management syscalls
use crate::mm::{MapPermission, VirtAddr};
use crate::task::{
    change_program_brk, exit_current_and_run_next, get_current_task, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

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

/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is split by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let uspace = &get_current_task().memory_set;
    let Some(ts) = uspace.translate_user_addr(VirtAddr(ts as usize), MapPermission::W) else {
        return -1;
    };
    let us = get_time_us();
    unsafe {
        *(ts.0 as *mut TimeVal) = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            let uspace = &get_current_task().memory_set;
            let Some(ptr) = uspace.translate_user_addr(VirtAddr(id), MapPermission::R) else {
                return -1;
            };
            unsafe { core::ptr::read_volatile(ptr.0 as *const u8) as isize }
        }
        1 => {
            let uspace = &get_current_task().memory_set;
            let Some(ptr) = uspace.translate_user_addr(VirtAddr(id), MapPermission::W) else {
                return -1;
            };
            unsafe {
                core::ptr::write_volatile(ptr.0 as *mut u8, data as u8);
                0
            }
        }
        2 => {
            let trace_infos = super::TRACE_INFOS.exclusive_access();
            let task_id = crate::task::get_current_task_id();
            trace_infos
                .get(&task_id)
                .and_then(|t| t.get(&id))
                .copied()
                .unwrap_or(0) as isize
        }
        _ => -1,
    }
}

// TODO: YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// TODO: YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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

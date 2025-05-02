//! Process management syscalls
use crate::mm::{available_memory, MapPermission, VirtAddr};
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

bitflags! {
    struct MmapFlags: u8 {
        const R = 1 << 0;
        const W = 1 << 1;
        const X = 1 << 2;
    }
}

pub fn sys_mmap(start: usize, len: usize, flags: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    if len > available_memory() {
        return -1;
    }
    if flags == 0 {
        return -1;
    }
    let Some(flags) = MmapFlags::from_bits(flags as u8) else {
        return -1;
    };

    let mut tcb = get_current_task();
    let uspace = &mut tcb.memory_set;
    let end_va = VirtAddr(start + len);
    if uspace.contains_any(start_va, end_va) {
        return -1;
    }

    uspace.insert_framed_area(
        start_va,
        end_va,
        MapPermission::from_bits(flags.bits() << 1).unwrap() | MapPermission::U,
    );
    0
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    let mut tcb = get_current_task();
    let uspace = &mut tcb.memory_set;
    if !uspace.find_area(start_va).map_or(false, |a| a.len() >= len) {
        return -1;
    }
    uspace.remove_area(start_va);
    0
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

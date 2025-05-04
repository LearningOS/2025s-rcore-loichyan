//! Process management syscalls
use crate::mm::{available_memory, MapPermission, VirtAddr};
use crate::timer::get_time_us;
use crate::{
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};
use alloc::sync::Arc;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is split by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let task = current_task().unwrap();
    let uspace = &task.inner_exclusive_access().memory_set;
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

    let task = current_task().unwrap();
    let uspace = &mut task.inner_exclusive_access().memory_set;
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
    let task = current_task().unwrap();
    let uspace = &mut task.inner_exclusive_access().memory_set;
    if !uspace.find_area(start_va).map_or(false, |a| a.len() >= len) {
        return -1;
    }
    uspace.remove_area(start_va);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    let current_task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    let Some(inode) = open_file(path.as_str(), OpenFlags::RDONLY) else {
        return -1;
    };
    let data = inode.read_all();
    let new_task = current_task.spawn(path.as_str(), &data);
    add_task(new_task.clone());
    new_task.pid.0 as isize
}

pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if prio < 2 {
        return -1;
    }
    let current_task = current_task().unwrap();
    current_task.inner_exclusive_access().priority = prio as usize;
    prio
}

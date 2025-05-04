//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let i = self.select_next_task()?;
        let task = self.ready_queue.remove(i).unwrap();
        let mut task_inner = task.inner_exclusive_access();
        task_inner.stride = task_inner
            .stride
            .overflowing_add(usize::MAX / task_inner.priority)
            .0;
        drop(task_inner);
        Some(task)
    }

    fn select_next_task(&mut self) -> Option<usize> {
        use core::cmp::Ordering;
        #[derive(Eq)]
        struct Stride(usize);
        impl PartialOrd for Stride {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Stride {
            fn cmp(&self, other: &Self) -> Ordering {
                match self.0.cmp(&other.0) {
                    Ordering::Less if (other.0 - self.0) > (usize::MAX / 2) => Ordering::Greater,
                    Ordering::Greater if (self.0 - other.0) > (usize::MAX / 2) => Ordering::Less,
                    other => other,
                }
            }
        }
        impl PartialEq for Stride {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        self.ready_queue
            .iter()
            .enumerate()
            .min_by_key(|(_, task)| Stride(task.inner_exclusive_access().stride))
            .map(|(i, _)| i)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

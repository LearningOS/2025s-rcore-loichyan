use alloc::vec::Vec;

/// Deadlock detector based on Banker's Algorithm.
#[derive(Debug, Default)]
pub struct DeadlockDetector {
    resources: Vec<ResourceState>,
    threads: Vec<Vec<ThreadState>>,
}

#[derive(Debug, Default)]
struct ResourceState {
    available: usize,
}

#[derive(Debug, Default)]
struct ThreadState {
    allocated: usize,
    // Assume there is no limit on each thread's needs.
    needs: usize,
}

impl DeadlockDetector {
    /// Resets the internal state.
    pub fn reset(&mut self) {
        self.resources.clear();
        self.threads.clear();
    }

    /// Registers a new resource with its available count.
    pub fn add_resource(&mut self, id: usize, available: usize) {
        adjust_vec_with(&mut self.resources, id, ResourceState::default);
        self.resources[id].available = available;
    }

    /// Registers a new thread.
    pub fn add_thread(&mut self, id: usize) {
        adjust_vec_with(&mut self.threads, id, Vec::new);
    }

    /// Allocates one specified resource to a thread. Returns false if the
    /// request will lead to a deadlock.
    pub fn acquire(&mut self, thread: usize, resource: usize) -> bool {
        let thread_res = &mut self.threads[thread];
        // Adjust the thread's resource vector to contain the requested resource.
        adjust_vec_with(thread_res, resource, ThreadState::default);

        let rstate = &mut self.resources[resource];
        let tstate = &mut thread_res[resource];

        if rstate.available == 0 {
            // Resource unavailable, increase the thread's needs.
            tstate.needs += 1;
        } else {
            // Otherwise, allocate one to the thread.
            tstate.allocated += 1;
            rstate.available -= 1;
        }

        self.check()
    }

    /// Releases the specified resource allocated to a thread.
    pub fn release(&mut self, thread: usize, resource: usize) {
        let thread_res = &mut self.threads[thread];
        // Adjust the thread's resource vector to contain the requested resource.
        adjust_vec_with(thread_res, resource, ThreadState::default);

        let rstate = &mut self.resources[resource];
        let tstate = &mut thread_res[resource];

        rstate.available += tstate.allocated;
        *tstate = ThreadState::default();
    }

    /// Releases the specified resource allocated to a thread. If no resource is
    /// released, the count of available resources will be increased by 1.
    pub fn release_or_increase(&mut self, thread: usize, resource: usize) {
        let thread_res = &mut self.threads[thread];
        // Adjust the thread's resource vector to contain the requested resource.
        adjust_vec_with(thread_res, resource, ThreadState::default);

        let rstate = &mut self.resources[resource];
        let tstate = &mut thread_res[resource];

        rstate.available += if tstate.allocated == 0 {
            1
        } else {
            tstate.allocated
        };
        *tstate = ThreadState::default();
    }

    fn check(&self) -> bool {
        let mut working = self
            .resources
            .iter()
            .map(|r| r.available)
            .collect::<Vec<_>>();
        let mut can_finish = self.threads.iter().map(|_| false).collect::<Vec<_>>();

        'outer: loop {
            // Main check loop
            for (t, thread_res) in self.threads.iter().enumerate() {
                let can_finish = &mut can_finish[t];
                if !*can_finish
                    && thread_res
                        .iter()
                        .enumerate()
                        .all(|(r, t)| t.needs <= working[r])
                {
                    // Current resources fit the thread's needs, so it can finish
                    // eventually and we should recycle thread's allocations.
                    thread_res
                        .iter()
                        .enumerate()
                        .for_each(|(r, t)| working[r] += t.allocated);
                    *can_finish = true;
                    // Restart the check loop so that the recycled resources can
                    // be allocated to other threads.
                    continue 'outer;
                } else {
                    // Otherwise, either the thread has been checked or there are
                    // resources not recycled yet, we continue the check loop.
                }
            }
            // Once we finish the check loop, it means no new resources are recycled.
            // So if some threads still cannot finish, there must be a deadlock.
            break can_finish.iter().all(|t| *t);
        }
    }
}

fn adjust_vec_with<T>(vec: &mut Vec<T>, id: usize, f: impl FnMut() -> T) {
    if vec.len() <= id {
        vec.resize_with(id + 1, f);
    }
}

use parking_lot::{Condvar, Mutex, ReentrantMutex, ReentrantMutexGuard};
use std::{sync::Arc, time::Duration};

struct LockState {
    parallels: u32,
}

struct LockData {
    mutex: Mutex<LockState>,
    serial: ReentrantMutex<()>,
    condvar: Condvar,
}

#[derive(Clone)]
pub(crate) struct Locks {
    arc: Arc<LockData>,
}

pub(crate) struct MutexGuardWrapper<'a> {
    #[allow(dead_code)] // need it around to get dropped
    mutex_guard: ReentrantMutexGuard<'a, ()>,
    locks: Locks,
}

impl<'a> Drop for MutexGuardWrapper<'a> {
    fn drop(&mut self) {
        self.locks.arc.condvar.notify_one();
    }
}

impl Locks {
    pub fn new() -> Locks {
        Locks {
            arc: Arc::new(LockData {
                mutex: Mutex::new(LockState { parallels: 0 }),
                condvar: Condvar::new(),
                serial: Default::default(),
            }),
        }
    }

    #[cfg(test)]
    pub fn is_locked(&self) -> bool {
        self.arc.serial.is_locked()
    }

    pub fn serial(&self) -> MutexGuardWrapper {
        let mut lock_state = self.arc.mutex.lock();
        loop {
            // If all the things we want are true, try to lock out serial
            if lock_state.parallels == 0 {
                let possible_serial_lock = self.arc.serial.try_lock();
                if let Some(serial_lock) = possible_serial_lock {
                    return MutexGuardWrapper {
                        mutex_guard: serial_lock,
                        locks: self.clone(),
                    };
                }
            }

            self.arc
                .condvar
                .wait_for(&mut lock_state, Duration::from_secs(1));
        }
    }

    pub fn start_parallel(&self) {
        let mut lock_state = self.arc.mutex.lock();
        loop {
            if lock_state.parallels > 0 {
                // fast path, as someone else already has it locked
                lock_state.parallels += 1;
                return;
            }

            let possible_serial_lock = self.arc.serial.try_lock();
            if possible_serial_lock.is_some() {
                // We now know no-one else has the serial lock, so we can add to parallel
                lock_state.parallels = 1; // Had to have been 0 before, as otherwise we'd have hit the fast path
                return;
            }

            self.arc
                .condvar
                .wait_for(&mut lock_state, Duration::from_secs(1));
        }
    }

    pub fn end_parallel(&self) {
        let mut lock_state = self.arc.mutex.lock();
        assert!(lock_state.parallels > 0);
        lock_state.parallels -= 1;
        drop(lock_state);
        self.arc.condvar.notify_one();
    }

    #[cfg(test)]
    pub fn parallel_count(&self) -> u32 {
        let lock_state = self.arc.mutex.lock();
        lock_state.parallels
    }
}

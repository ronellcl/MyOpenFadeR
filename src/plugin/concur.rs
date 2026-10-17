//! A cap on parallel plugin work: every plugin event hook and every source scan gets a thread, and each plugin call
//! pairs with children and their reader/writer threads, so a burst (a plugin firing events in a loop) could flood the
//! process. A [`Gate`] holds a generous number of slots; when it is full, the newest request is dropped — an event
//! hook misses one occurrence and a source read retries on the next rebuild, which is exactly how those already
//! behave when a plugin is slow.

use parking_lot::Mutex;

/// Some `cap` slots, taken and released without blocking.
pub struct Gate {
    free: Mutex<usize>,
}

impl Gate {
    pub const fn new(cap: usize) -> Self {
        Self {
            free: Mutex::new(cap),
        }
    }

    /// A slot for as long as the [`Slot`] lives, or `None` when the gate is full.
    pub fn slot(&self) -> Option<Slot<'_>> {
        let mut free = self.free.lock();
        if *free == 0 {
            return None;
        }
        *free -= 1;
        Some(Slot(self))
    }
}

/// One taken slot; released on drop.
pub struct Slot<'g>(&'g Gate);

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        *self.0.free.lock() += 1;
    }
}

/// Caps of the two kinds of plugin work (plugin scripts and children count against them too, as they run on those
/// threads): generous enough that no real user session reaches them, low enough that a runaway plugin cannot make
/// hundreds of threads.
pub static EVENTS: Gate = Gate::new(16);
pub static SOURCES: Gate = Gate::new(16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gate_is_a_nonblocking_counting_semaphore() {
        let g = Gate::new(2);
        let a = g.slot().expect("first");
        let _b = g.slot().expect("second");
        assert!(g.slot().is_none(), "the third is refused, never waited on");
        drop(_b);
        drop(a);
        let c = g.slot().expect("a released slot is free again");
        drop(c);
        assert_eq!(*g.free.lock(), 2, "every slot comes back");
    }

    #[test]
    fn a_dropped_slot_is_released() {
        let g = Gate::new(1);
        let s = g.slot().unwrap();
        assert!(g.slot().is_none(), "taken until released");
        drop(s);
        assert!(g.slot().is_some(), "the scope took it back");
    }
}

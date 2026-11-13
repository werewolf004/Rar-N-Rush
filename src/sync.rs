//! Small synchronisation helpers.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks a mutex, recovering the data if another thread panicked while holding it.
///
/// The shared state only holds plain data (no invariants spanning several fields
/// that a panic could leave half-updated), so continuing with it is safe and
/// better than taking the whole application down.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::lock;
    use std::sync::{Arc, Mutex};

    #[test]
    fn recovers_from_poisoned_mutex() {
        let m = Arc::new(Mutex::new(1));
        let m2 = Arc::clone(&m);
        let _ = std::thread::spawn(move || {
            let _guard = m2.lock().unwrap();
            panic!("poison");
        })
        .join();
        assert!(m.is_poisoned());
        *lock(&m) += 1;
        assert_eq!(*lock(&m), 2);
    }
}

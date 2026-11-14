//! Memory locking support (Windows only).
//!
//! SQLCipher tries to `VirtualLock` the pages that hold key material so the
//! key never lands in the swap file. `VirtualLock` counts against the
//! process working-set quota, and the default minimum working set is too
//! small, so the lock fails with `ERROR_WORKING_SET_QUOTA` (1453) and the
//! key pages stay swappable. Raising the floor once at startup makes the
//! lock succeed.
//!
//! The functions are declared by hand instead of pulling in a Windows
//! binding crate: it is two symbols in kernel32.

#[cfg(windows)]
mod inner {
    use std::sync::Once;

    static RAISED: Once = Once::new();

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetProcessWorkingSetSizeEx(
            process: isize,
            minimum_working_set_size: usize,
            maximum_working_set_size: usize,
            flags: u32,
        ) -> i32;
    }

    /// Raise the process minimum working set so SQLCipher's `VirtualLock`
    /// of the key pages fits inside the quota. Idempotent per process;
    /// failures are non-fatal (the db still works, the pages are just
    /// swappable — exactly the state before the fix).
    pub fn raise_working_set_floor() {
        RAISED.call_once(|| {
            // SQLCipher locks a handful of small buffers; 8 MiB of floor is
            // a comfortable margin and still a rounding error next to a
            // Tauri webview process.
            const FLOOR: usize = 8 * 1024 * 1024;
            unsafe {
                let process = GetCurrentProcess();
                if process == 0 {
                    return;
                }
                // min == max, no flags: no hard/adaptive policy change.
                let ok = SetProcessWorkingSetSizeEx(process, FLOOR, FLOOR, 0);
                if ok == 0 {
                    eprintln!(
                        "[glimmerx] could not raise the working-set floor; key pages stay swappable"
                    );
                }
            }
        });
    }
}

#[cfg(not(windows))]
mod inner {
    /// No-op off Windows: `mlock` needs no quota handling there and SQLCipher
    /// uses it directly.
    pub fn raise_working_set_floor() {}
}

pub use inner::raise_working_set_floor;

#[cfg(test)]
mod tests {
    use super::raise_working_set_floor;

    #[test]
    fn test_idempotent_and_non_fatal() {
        // Second call must be a no-op and never panic, even on odd systems.
        raise_working_set_floor();
        raise_working_set_floor();
    }
}

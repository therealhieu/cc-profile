//! Serializes tests that read or mutate process environment variables shared across crates.

use std::sync::{Mutex, MutexGuard};

static CC_PROFILE_TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Lock for `CC_PROFILE_NO_UPDATE_CHECK` and related env used by passive update checks.
pub fn lock_cc_profile_update_check_env() -> MutexGuard<'static, ()> {
    CC_PROFILE_TEST_ENV_LOCK
        .lock()
        .expect("update check env lock poisoned")
}

/// Restores `CC_PROFILE_NO_UPDATE_CHECK` when dropped (including on panic).
pub struct CcProfileNoUpdateCheckGuard {
    previous: Option<String>,
}

impl CcProfileNoUpdateCheckGuard {
    pub fn set(value: &str) -> Self {
        let previous = std::env::var("CC_PROFILE_NO_UPDATE_CHECK").ok();
        // SAFETY: caller must hold `lock_cc_profile_update_check_env()`.
        unsafe {
            std::env::set_var("CC_PROFILE_NO_UPDATE_CHECK", value);
        }
        Self { previous }
    }

    pub fn clear() -> Self {
        let previous = std::env::var("CC_PROFILE_NO_UPDATE_CHECK").ok();
        // SAFETY: caller must hold `lock_cc_profile_update_check_env()`.
        unsafe {
            std::env::remove_var("CC_PROFILE_NO_UPDATE_CHECK");
        }
        Self { previous }
    }
}

impl Drop for CcProfileNoUpdateCheckGuard {
    fn drop(&mut self) {
        // SAFETY: only used while the env lock is held.
        unsafe {
            match &self.previous {
                Some(v) => std::env::set_var("CC_PROFILE_NO_UPDATE_CHECK", v),
                None => std::env::remove_var("CC_PROFILE_NO_UPDATE_CHECK"),
            }
        }
    }
}

//! Helpers shared by unit tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// A fresh directory under the system temporary directory, removed with its contents on drop.
pub struct TestDir(pub PathBuf);

impl TestDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ccline-test-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

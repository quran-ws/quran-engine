//! What the data-gated tests share: where the data release is, and how a test skips without it.
//!
//! Each test file compiles this module on its own and uses part of it.
#![allow(dead_code)]
use qvp_core::Page;
use std::path::{Path, PathBuf};

/// A file of the data release, under `dist/` at the repository root.
pub fn dist_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist").join(name)
}

/// Whether a file of the data release is there. A missing file is a skip with a notice, or a
/// panic when `QVP_REQUIRE_DATA` is set, as it is in CI.
pub fn present(name: &str) -> bool {
    if dist_path(name).exists() {
        return true;
    }
    if std::env::var("QVP_REQUIRE_DATA").is_ok() {
        panic!("dist/{name} is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
    }
    eprintln!("skip: dist/{name} is missing; run scripts/sync-test-data.sh to enable this test");
    false
}

/// The bytes of a file of the data release, or `None` when the release is not synced.
pub fn data(name: &str) -> Option<Vec<u8>> {
    present(name).then(|| std::fs::read(dist_path(name)).unwrap())
}

/// Page `n` of the data release, or `None` when the release is not synced.
pub fn page(n: u32) -> Option<Page> {
    data(&format!("pages/{n:03}.qvp")).map(|bytes| Page::load(&bytes).unwrap())
}

/// The directory of the release's pages, or `None` when the release is not synced.
pub fn pages_dir() -> Option<PathBuf> {
    present("pages/001.qvp").then(|| dist_path("pages"))
}

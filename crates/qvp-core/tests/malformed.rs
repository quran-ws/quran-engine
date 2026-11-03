//! A damaged page, atlas or ornament file is an error, never a panic: every prefix of a
//! real page, and thousands of single-byte mutations of the real files, go through the
//! decoders and the page loader.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::{Atlas, OrnamentSet, Page};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

fn data_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../dist/{name}"))
}

/// The bytes of a data file, or `None` with a skip notice when the release is not synced.
fn data(name: &str) -> Option<Vec<u8>> {
    let path = data_path(name);
    if !path.exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/{name} is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/{name} is missing; run scripts/sync-test-data.sh to enable this test");
        return None;
    }
    Some(std::fs::read(path).unwrap())
}

/// A small linear congruential generator: the same seed gives the same mutations on
/// every machine, so a failure names a reproducible offset and byte.
struct Lcg(u64);
impl Lcg {
    fn step(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

/// Run `load` on `rounds` single-byte mutations of `clean`, each from the same seed, and
/// fail on the first panic with the offset and byte that caused it.
fn mutate(what: &str, clean: &[u8], rounds: usize, load: impl Fn(&[u8]) -> bool) {
    let mut rng = Lcg(0x5eed);
    let mut accepted = 0usize;
    for _ in 0..rounds {
        let mut bytes = clean.to_vec();
        let at = rng.step() as usize % bytes.len();
        let value = rng.step() as u8;
        bytes[at] = value;
        match catch_unwind(AssertUnwindSafe(|| load(&bytes))) {
            Ok(ok) => accepted += ok as usize,
            Err(_) => panic!("{what}: byte {at} set to {value:#04x} panicked"),
        }
    }
    eprintln!("{what}: {accepted} of {rounds} mutations still load");
}

#[test]
fn every_prefix_of_a_real_page_is_an_error() {
    let Some(bytes) = data("pages/042.qvp") else { return };
    for n in 0..bytes.len() {
        match catch_unwind(|| Page::load(&bytes[..n]).is_err()) {
            Ok(true) => {}
            Ok(false) => panic!("a prefix of {n} bytes loaded"),
            Err(_) => panic!("a prefix of {n} bytes panicked"),
        }
    }
    assert!(Page::load(&bytes).is_ok());
}

#[test]
fn single_byte_mutations_of_a_real_page_load_or_fail_without_panic() {
    let Some(bytes) = data("pages/042.qvp") else { return };
    mutate("page 042", &bytes, 4000, |b| Page::load(b).is_ok());
}

#[test]
fn single_byte_mutations_of_the_atlas_decode_or_fail_without_panic() {
    let Some(bytes) = data("pages/atlas.qva") else { return };
    mutate("atlas", &bytes, 4000, |b| Atlas::decode(b).is_ok());
    for n in 0..bytes.len().min(4096) {
        assert!(Atlas::decode(&bytes[..n]).is_err(), "a prefix of {n} bytes decoded");
    }
}

#[test]
fn single_byte_mutations_of_the_ornament_set_decode_or_fail_without_panic() {
    // The set is in the data release from data-v0.5.0 on; an older release has none.
    let path = data_path("pages/ornaments.qvo");
    if !path.exists() {
        eprintln!("skip: dist/pages/ornaments.qvo is missing; a newer data release carries it");
        return;
    }
    let bytes = std::fs::read(path).unwrap();
    mutate("ornaments", &bytes, 4000, |b| OrnamentSet::decode(b).is_ok());
}

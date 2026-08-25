//! Shared machinery for the test suites.

use std::path::{Path, PathBuf};

/// The repository root, found from where this crate lives.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("the repository should be where the crate says it is")
}

pub fn cases(folder: &str) -> Vec<PathBuf> {
    let dir = root().join("tests").join(folder);
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "polite").unwrap_or(false))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no cases in {}", dir.display());
    out
}

pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// Compare against the file next door, or write it when `POLITE_BLESS` is set.
pub fn compare(case: &Path, suffix: &str, got: &str) {
    let expected_path = case.with_extension(suffix);
    if std::env::var("POLITE_BLESS").is_ok() {
        std::fs::write(&expected_path, got).expect("could not write the expected output");
        return;
    }
    let want = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| {
            panic!(
                "{} is missing.\nRun the tests once with POLITE_BLESS=1 to write it, then read it \
                 carefully before committing.\n\nWhat happened was:\n{got}",
                expected_path.display()
            )
        })
        .replace("\r\n", "\n");

    if want.trim_end() != got.trim_end() {
        panic!(
            "{} does not match.\n\n--- expected ---\n{want}\n--- what happened ---\n{got}",
            case.display()
        );
    }
}

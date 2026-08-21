//! What a container run leaves behind for a person to look at.
//!
//! A run inside the container writes where nobody can open it, so
//! anything meant to be read afterwards goes to a host directory bridged
//! in over /out (`linux`).

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// A host directory for what a run means to be looked at afterwards, or
/// None when the command leaves nothing. verify-ui writes its screenshot
/// and the settings it ran with into --shot-dir; inside a container that is
/// a place nobody can open, and the whole verdict is a PNG.
pub(crate) fn keepsakes(command: &[String]) -> Result<Option<PathBuf>, String> {
    if !command.iter().any(|word| word == "verify-ui") {
        return Ok(None);
    }
    if command.iter().any(|word| word == "--shot-dir") {
        // Named by the caller, who then owns where it lands.
        return Ok(None);
    }
    keepsake_dir("shots").map(Some)
}

pub(crate) fn keepsake_dir(kind: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let dir = std::env::temp_dir()
        .join("pg-linux")
        .join(format!("{kind}-{nanos}"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to make {}: {e}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::keepsakes;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn only_a_verify_run_without_a_directory_of_its_own_gets_one() {
        assert!(
            keepsakes(&words("cargo xtask verify-ui commit"))
                .expect("temp dir")
                .is_some()
        );
        assert_eq!(
            keepsakes(&words("cargo xtask verify-ui commit --shot-dir /somewhere")).expect("none"),
            None
        );
        assert_eq!(
            keepsakes(&words("cargo test -p platitude-core")).expect("none"),
            None
        );
    }
}

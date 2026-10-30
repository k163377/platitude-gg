//! Inputs supplied by the host rather than the Git tree. A step's source key cannot stand for a different Qt,
//! font installation or inherited runtime setting. Read once per plan; never print environment values.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::plan::{Side, Step};

pub(super) struct Environment {
    inherited: String,
    qt: Option<Result<String, String>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            inherited: inherited(std::env::vars_os().map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })),
            qt: None,
        }
    }

    pub fn identity(&mut self, step: &Step) -> Result<String, String> {
        // The launcher supplies the container's environment; it does not inherit the host's Qt or fonts.
        if step.side == Side::Linux {
            return Ok("container".into());
        }
        let qt = step.release
            || step.inputs.iter().any(|path| {
                path.starts_with("crates/platitude-app") || path == "crates/xtask/src/qt.rs"
            });
        let mut identity = format!(
            "{}:{}:{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.inherited
        );
        if qt {
            let installation = self
                .qt
                .get_or_insert_with(qt_installation)
                .as_ref()
                .map_err(Clone::clone)?;
            identity.push_str(installation);
        }
        Ok(identity)
    }
}

fn inherited(values: impl Iterator<Item = (String, String)>) -> String {
    let mut values: Vec<_> = values
        .filter(|(name, _)| {
            let name = name.to_ascii_uppercase();
            [
                "QT_",
                "QML",
                "QV4_",
                "QSG_",
                "QTEST",
                "QQUICK",
                "FONTCONFIG",
                "DYLD_",
                "LC_",
            ]
            .iter()
            .any(|prefix| name.starts_with(prefix))
                || matches!(
                    name.as_str(),
                    "PATH"
                        | "QMAKE"
                        | "LANG"
                        | "LD_LIBRARY_PATH"
                        | "RUSTFLAGS"
                        | "RUSTDOCFLAGS"
                        | "CARGO_ENCODED_RUSTFLAGS"
                        | "RUSTUP_TOOLCHAIN"
                        | "XDG_CONFIG_HOME"
                        | "XDG_DATA_HOME"
                        | "HOME"
                        | "USERPROFILE"
                )
        })
        .collect();
    values.sort();
    fingerprint(
        values
            .iter()
            .flat_map(|(name, value)| name.bytes().chain([0]).chain(value.bytes()).chain([0])),
    )
}

fn qt_installation() -> Result<String, String> {
    let mut roots = font_roots();
    let chosen = match crate::qt::chosen() {
        Ok(Some(qt)) => {
            roots.extend([qt.bin.clone(), qt.runtime.clone()]);
            roots.extend(
                ["include", "plugins", "qml", "mkspecs"]
                    .map(|name| Path::new(&qt.prefix).join(name)),
            );
            qt.describe()
        }
        // A machine without that Qt cannot reuse a green from a machine that had it. The step itself reports
        // why it cannot run; source-only miniature runners need no installed Qt.
        Ok(None) => "Qt absent".into(),
        Err(why) => format!("Qt unavailable: {why}"),
    };
    Ok(format!("{chosen}:{}", files(&roots)?))
}

fn font_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        return vec![PathBuf::from("C:/Windows/Fonts")];
    }
    let mut roots: Vec<PathBuf> = if cfg!(target_os = "macos") {
        ["/System/Library/Fonts", "/Library/Fonts"]
            .map(PathBuf::from)
            .into()
    } else {
        ["/usr/share/fonts", "/usr/local/share/fonts", "/etc/fonts"]
            .map(PathBuf::from)
            .into()
    };
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.extend(
            [
                ".fonts",
                ".local/share/fonts",
                ".config/fontconfig",
                "Library/Fonts",
            ]
            .map(|path| home.join(path)),
        );
    }
    for (variable, suffix) in [
        ("XDG_DATA_HOME", "fonts"),
        ("XDG_CONFIG_HOME", "fontconfig"),
    ] {
        if let Some(path) = std::env::var_os(variable) {
            roots.push(PathBuf::from(path).join(suffix));
        }
    }
    roots
}

/// Installation identity: names, link targets, file sizes and modification times. Scanning metadata once is
/// independent of the number of steps. Missing roots are inputs too; a failed inspection permits no cached green.
fn files(roots: &[PathBuf]) -> Result<String, String> {
    let mut pending = roots.to_vec();
    let mut seen = BTreeSet::new();
    let mut records = BTreeSet::new();
    while let Some(path) = pending.pop() {
        let metadata = match std::fs::metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                records.insert(format!("{}:absent", path.display()));
                continue;
            }
            Err(error) => {
                return Err(format!(
                    "host input {} cannot be inspected: {error}",
                    path.display()
                ));
            }
        };
        let real = path
            .canonicalize()
            .map_err(|error| format!("host input {}: {error}", path.display()))?;
        records.insert(format!("{}=>{}", path.display(), real.display()));
        if !seen.insert(real.clone()) {
            continue;
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(&real)
                .map_err(|error| format!("host directory {}: {error}", real.display()))?
            {
                pending.push(
                    entry
                        .map_err(|error| format!("host directory {}: {error}", real.display()))?
                        .path(),
                );
            }
        } else {
            let modified = metadata
                .modified()
                .map_err(|error| format!("host input {}: {error}", real.display()))?;
            records.insert(format!(
                "{}:{}:{modified:?}",
                real.display(),
                metadata.len()
            ));
        }
    }
    Ok(fingerprint(
        records.iter().flat_map(|line| line.bytes().chain([0])),
    ))
}

fn fingerprint(bytes: impl Iterator<Item = u8>) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{files, inherited};

    #[test]
    fn runtime_overrides_change_identity_but_activity_marks_do_not() {
        let base = vec![("PATH".into(), "qt/bin".into())];
        let original = inherited(base.clone().into_iter());
        for variable in [
            "QT_BIN",
            "QML_IMPORT_PATH",
            "QT_SCALE_FACTOR",
            "FONTCONFIG_FILE",
            "LANG",
        ] {
            let mut changed = base.clone();
            changed.push((variable.into(), "another".into()));
            assert_ne!(original, inherited(changed.into_iter()), "{variable}");
        }
        let mut activity = base;
        activity.push(("CODEX_THREAD_ID".into(), "another-seat".into()));
        assert_eq!(original, inherited(activity.into_iter()));
    }

    #[test]
    fn installation_changes_and_failed_inspection_do_not_reuse_an_identity() {
        let root = crate::yard::Yard::new("gate-host-inputs");
        let font = root.join("fonts/font.ttf");
        let roots = vec![root.join("fonts")];
        let missing = files(&roots).unwrap();
        std::fs::create_dir_all(font.parent().unwrap()).unwrap();
        std::fs::write(&font, "font one").unwrap();
        let installed = files(&roots).unwrap();
        assert_ne!(missing, installed);
        assert_eq!(installed, files(&roots).unwrap());
        std::fs::write(&font, "another font").unwrap();
        assert_ne!(installed, files(&roots).unwrap());
        assert!(files(&[root.join("\0")]).is_err());
    }
}

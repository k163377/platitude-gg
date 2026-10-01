//! What a build of the app is made of beyond its source: the Qt it links,
//! the compilers, cargo's configuration and the build flags in the
//! environment. A shelved exe answers for a build only when all of it
//! matches (`perf::rig`), and the evidence lists it (`build.txt`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::qt::Qt;

/// The build's parts, one `key=value` per line.
pub(crate) struct Identity {
    pub(crate) text: String,
    /// Whether every part was read: a part that was not cannot be matched,
    /// so no shelved build answers for this one.
    pub(crate) complete: bool,
}

impl Identity {
    /// A fingerprint of [`Identity::text`], for a shelf name.
    pub(crate) fn hash(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in self.text.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}

/// The build of `source` with `features` against `qt`, with cargo's
/// configuration `configs` (see [`configs`]).
pub(crate) fn of(
    source: &str,
    features: &str,
    qt: &Qt,
    configs: &[(PathBuf, Vec<u8>)],
) -> Identity {
    let rustc = rustc();
    let cxx = cxx();
    let mut text = format!(
        "source={source}\nfeatures={features}\nqt={} {}\nrustc={}\ncxx={}\n",
        qt.version,
        qt.prefix,
        rustc.as_deref().unwrap_or("unknown"),
        cxx.as_deref().unwrap_or("unknown"),
    );
    for (path, contents) in configs {
        text.push_str(&format!(
            "cargo_config={} {}\n",
            path.display(),
            Identity {
                text: String::from_utf8_lossy(contents).into_owned(),
                complete: true,
            }
            .hash()
        ));
    }
    for (key, value) in build_env(std::env::vars()) {
        text.push_str(&format!("env {key}={value}\n"));
    }
    Identity {
        text,
        complete: rustc.is_some() && cxx.is_some(),
    }
}

/// cargo's configuration files for a build in `tree`, deepest first, as
/// cargo reads them: the tree's own, every directory above it, then
/// `CARGO_HOME`'s. `at` reads the tree's own at a commit (the rig, before
/// it is switched to it).
pub(crate) fn configs(tree: &Path, at: Option<(&str, &str)>) -> Vec<(PathBuf, Vec<u8>)> {
    let names = [".cargo/config.toml", ".cargo/config"];
    let mut found = Vec::new();
    for name in names {
        let own = match at {
            Some((repo, commit)) => {
                crate::subprocess::git_query(repo, &["show", &format!("{commit}:{name}")])
                    .map(|text| format!("{text}\n").into_bytes())
            }
            None => std::fs::read(tree.join(name)).ok(),
        };
        if let Some(contents) = own {
            found.push((tree.join(name), contents));
        }
    }
    for dir in tree.ancestors().skip(1) {
        for name in names {
            if let Ok(contents) = std::fs::read(dir.join(name)) {
                found.push((dir.join(name), contents));
            }
        }
    }
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    if let Some(home) = home {
        for name in ["config.toml", "config"] {
            if let Ok(contents) = std::fs::read(home.join(name)) {
                found.push((home.join(name), contents));
            }
        }
    }
    found
}

/// The environment a build reads its flags and tools from, sorted.
fn build_env(vars: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    const EXACT: [&str; 7] = [
        "CC",
        "CXX",
        "AR",
        "RUSTC_WRAPPER",
        "QT_VERSION_MAJOR",
        "VCINSTALLDIR",
        "VCToolsVersion",
    ];
    const PREFIXES: [&str; 8] = [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_",
        "CARGO_PROFILE_",
        "CFLAGS",
        "CXXFLAGS",
        "CC_",
        "CXX_",
    ];
    let mut picked: Vec<(String, String)> = vars
        .filter(|(key, _)| {
            EXACT.contains(&key.as_str()) || PREFIXES.iter().any(|prefix| key.starts_with(prefix))
        })
        .collect();
    picked.sort();
    picked
}

/// `rustc -vV`'s release, commit, host and LLVM: the toolchain the build
/// below this process runs (rustup hands it the same toolchain).
fn rustc() -> Option<String> {
    let output = Command::new("rustc").arg("-vV").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .map(str::trim)
            .unwrap_or("-")
            .to_string()
    };
    Some(format!(
        "{} {} {} llvm {}",
        field("release:"),
        field("commit-hash:"),
        field("host:"),
        field("LLVM version:")
    ))
}

/// The C++ compiler a build script's `cc` takes: on Windows the MSVC
/// toolset of the newest Visual Studio with the C++ tools (its default
/// toolset file), unless a developer prompt names one; elsewhere `cc` and
/// `c++` as they answer.
fn cxx() -> Option<String> {
    if cfg!(windows) {
        if let Ok(version) = std::env::var("VCToolsVersion") {
            return Some(format!("msvc {version} (VCToolsVersion)"));
        }
        let installer = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?)
            .join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe");
        let output = Command::new(installer)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .ok()?;
        let install = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if install.is_empty() {
            return None;
        }
        let version = std::fs::read_to_string(
            Path::new(&install)
                .join("VC")
                .join("Auxiliary")
                .join("Build")
                .join("Microsoft.VCToolsVersion.default.txt"),
        )
        .ok()?;
        return Some(format!("msvc {} ({install})", version.trim()));
    }
    let first_line = |tool: &str| {
        let output = Command::new(tool).arg("--version").output().ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::to_string)
    };
    Some(format!(
        "cc {} / c++ {}",
        first_line("cc")?,
        first_line("c++")?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qt(version: &str) -> Qt {
        Qt {
            bin: PathBuf::from(format!("C:/Qt/{version}/m/bin")),
            qmake: PathBuf::from(format!("C:/Qt/{version}/m/bin/qmake.exe")),
            version: version.into(),
            prefix: format!("C:/Qt/{version}/m"),
            runtime: PathBuf::from(format!("C:/Qt/{version}/m/bin")),
        }
    }

    /// Each part moves the fingerprint: a shelf hit is a match on all of
    /// them.
    #[test]
    fn every_part_of_a_build_moves_its_fingerprint() {
        let config = vec![(PathBuf::from("t/.cargo/config.toml"), b"[env]\n".to_vec())];
        let base = of("abc", "automation", &qt("6.12.0"), &config).hash();
        assert_eq!(base, of("abc", "automation", &qt("6.12.0"), &config).hash());
        assert_ne!(base, of("abd", "automation", &qt("6.12.0"), &config).hash());
        assert_ne!(base, of("abc", "none", &qt("6.12.0"), &config).hash());
        assert_ne!(base, of("abc", "automation", &qt("6.10.3"), &config).hash());
        let flagged = vec![(
            PathBuf::from("t/.cargo/config.toml"),
            b"[env]\nCFLAGS = \"-DX\"\n".to_vec(),
        )];
        assert_ne!(
            base,
            of("abc", "automation", &qt("6.12.0"), &flagged).hash()
        );
    }

    #[test]
    fn the_build_flags_are_read_and_nothing_else() {
        let vars = [
            ("PATH", "x"),
            ("CFLAGS", "-DX"),
            ("CXXFLAGS_x86_64_pc_windows_msvc", "/utf-8"),
            ("CARGO_MANIFEST_DIR", "y"),
            ("RUSTFLAGS", "-Cz"),
            ("CC", "clang"),
            ("CCACHE_DIR", "z"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let keys: Vec<String> = build_env(vars.into_iter())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            keys,
            [
                "CC",
                "CFLAGS",
                "CXXFLAGS_x86_64_pc_windows_msvc",
                "RUSTFLAGS"
            ]
        );
    }

    /// The tree's own first, then the directories above it.
    #[test]
    fn cargo_reads_the_tree_s_configuration_and_every_one_above_it() {
        let root = crate::yard::Yard::new("cargo-configs");
        let tree = root.join("outer").join("tree");
        std::fs::create_dir_all(tree.join(".cargo")).expect("dir");
        std::fs::create_dir_all(root.join("outer").join(".cargo")).expect("dir");
        std::fs::write(tree.join(".cargo/config.toml"), "inner").expect("inner");
        std::fs::write(root.join("outer/.cargo/config.toml"), "outer").expect("outer");
        let found: Vec<Vec<u8>> = configs(&tree, None)
            .into_iter()
            .filter(|(path, _)| path.starts_with(&*root))
            .map(|(_, contents)| contents)
            .collect();
        assert_eq!(found, [b"inner".to_vec(), b"outer".to_vec()]);
    }
}

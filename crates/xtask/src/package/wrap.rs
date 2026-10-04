//! What the bundle is handed over in: a zip anyone opens, or a DMG only its
//! password opens — the form a run of the public repository keeps, where
//! whoever can read the run can take the file (P5-確認事項 §3.5).

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::bundle;
use super::run::{said, tool};

/// The name the DMG's volume mounts under.
const VOLUME: &str = "Platitude GG snapshot";

/// What the bundle goes out in.
pub(super) enum Wrap {
    Zip,
    /// Encrypted under the password, which is read off a variable and never
    /// put on a command line.
    Dmg {
        password: String,
    },
}

impl Wrap {
    /// The DMG under the password `variable` holds, or a zip with none
    /// named. Read before the build, so a missing password costs nothing.
    pub(super) fn from_variable(variable: Option<&str>) -> Result<Self, String> {
        let Some(variable) = variable else {
            return Ok(Self::Zip);
        };
        match std::env::var(variable) {
            Ok(password) if !password.is_empty() => Ok(Self::Dmg { password }),
            _ => Err(format!(
                "{variable} holds no password — on CI it is the repository secret of that name"
            )),
        }
    }
}

/// The bundle wrapped under the name a snapshot goes by, beside it in `out`.
pub(super) fn wrap(root: &Path, out: &Path, app: &Path, how: &Wrap) -> Result<PathBuf, String> {
    let here = root.display().to_string();
    let day = crate::subprocess::git_query(
        &here,
        &["log", "-1", "--format=%cd", "--date=format:%Y%m%d"],
    )
    .ok_or("git could not say the day of HEAD's commit")?;
    let commit = crate::subprocess::git_query(&here, &["rev-parse", "--short=8", "HEAD"])
        .ok_or("git could not name HEAD")?;
    let stem = bundle::archive_stem(&day, &commit);
    match how {
        Wrap::Zip => zip(app, &out.join(format!("{stem}.zip"))),
        Wrap::Dmg { password } => dmg(out, app, &out.join(format!("{stem}.dmg")), password),
    }
}

/// ditto, not zip: the frameworks' symlinks and the signature's extended
/// attributes travel as they are.
fn zip(app: &Path, archive: &Path) -> Result<PathBuf, String> {
    replace(archive)?;
    tool(
        Path::new("ditto"),
        &[
            OsStr::new("-c"),
            OsStr::new("-k"),
            OsStr::new("--sequesterRsrc"),
            OsStr::new("--keepParent"),
            app.as_os_str(),
            archive.as_os_str(),
        ],
    )?;
    Ok(archive.to_path_buf())
}

/// The bundle, and a link to /Applications to drag it onto, in an AES-256
/// image. Then two witnesses that the image is what it should be: hdiutil
/// calls it encrypted, and the password opens it onto a bundle whose
/// signature still verifies.
fn dmg(out: &Path, app: &Path, image: &Path, password: &str) -> Result<PathBuf, String> {
    let staging = fresh(&out.join("dmg"))?;
    tool(
        Path::new("ditto"),
        &[app.as_os_str(), staging.join(bundle::APP).as_os_str()],
    )?;
    tool(
        Path::new("ln"),
        &[
            OsStr::new("-s"),
            OsStr::new("/Applications"),
            staging.join("Applications").as_os_str(),
        ],
    )?;
    replace(image)?;
    with_password(
        "hdiutil",
        &[
            OsStr::new("create"),
            OsStr::new("-encryption"),
            OsStr::new("AES-256"),
            OsStr::new("-stdinpass"),
            OsStr::new("-srcfolder"),
            staging.as_os_str(),
            OsStr::new("-volname"),
            OsStr::new(VOLUME),
            OsStr::new("-format"),
            OsStr::new("UDZO"),
            image.as_os_str(),
        ],
        password,
    )?;
    std::fs::remove_dir_all(&staging)
        .map_err(|e| format!("could not clear {}: {e}", staging.display()))?;

    let encrypted = said("hdiutil", &[OsStr::new("isencrypted"), image.as_os_str()])?;
    if !encrypted
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("encrypted: yes"))
    {
        return Err(format!(
            "hdiutil does not call {} encrypted:\n{encrypted}",
            image.display()
        ));
    }
    let mount = fresh(&out.join("dmg-opened"))?;
    with_password(
        "hdiutil",
        &[
            OsStr::new("attach"),
            OsStr::new("-stdinpass"),
            OsStr::new("-nobrowse"),
            OsStr::new("-readonly"),
            OsStr::new("-noautoopen"),
            OsStr::new("-mountpoint"),
            mount.as_os_str(),
            image.as_os_str(),
        ],
        password,
    )?;
    let opened = tool(
        Path::new("codesign"),
        &[
            OsStr::new("--verify"),
            OsStr::new("--deep"),
            OsStr::new("--strict"),
            mount.join(bundle::APP).as_os_str(),
        ],
    );
    // Detached whatever the verdict: a mounted image outlives the run. By
    // force — the mount is read-only and nothing of this run's still reads
    // it, where macOS's own indexing can hold a plain detach busy.
    let detached = tool(
        Path::new("hdiutil"),
        &[
            OsStr::new("detach"),
            OsStr::new("-force"),
            mount.as_os_str(),
        ],
    );
    opened?;
    detached?;
    println!("the image is encrypted, and its password opens it onto the bundle");
    Ok(image.to_path_buf())
}

/// Runs an hdiutil verb that reads the password, NUL-ended, off its stdin
/// (`-stdinpass`). A failure is red without the password in the sentence.
fn with_password(program: &str, args: &[&OsStr], password: &str) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    let written = child
        .stdin
        .take()
        .ok_or_else(|| format!("{program} took no stdin"))
        .and_then(|mut stdin| {
            stdin
                .write_all(format!("{password}\0").as_bytes())
                .map_err(|e| format!("could not hand {program} the password: {e}"))
        });
    let status = child
        .wait()
        .map_err(|e| format!("{program} did not end: {e}"))?;
    written?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {args:?} failed ({status})"))
    }
}

/// An empty directory at `dir`, whatever stood there.
fn fresh(dir: &Path) -> Result<PathBuf, String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(|e| format!("could not clear {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    Ok(dir.to_path_buf())
}

/// No file at `path`, so what is written there is this run's.
fn replace(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| format!("could not replace {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Wrap;

    /// A variable nobody sets: what a missing secret looks like to the run.
    #[test]
    fn no_variable_is_a_zip_and_an_unset_one_is_refused() {
        assert!(matches!(Wrap::from_variable(None), Ok(Wrap::Zip)));
        let refused = Wrap::from_variable(Some("XTASK_PACKAGE_TEST_UNSET_PASSWORD"))
            .err()
            .expect("refused");
        assert!(
            refused.contains("XTASK_PACKAGE_TEST_UNSET_PASSWORD holds no password"),
            "{refused}"
        );
    }
}

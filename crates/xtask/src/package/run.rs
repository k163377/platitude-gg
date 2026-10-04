//! The steps, in the order each needs the one before: the build, the
//! bundle's skeleton, Qt's deployment (`macdeployqt`), the run paths, the
//! signature, a stand from the bundle alone, and the archive. Every tool
//! after the build is Qt's or the Mac's own (Xcode's command line tools).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::bundle;
use super::wrap::{Wrap, wrap};
use crate::subprocess::run_captured;

/// The helper the app starts for an interactive rebase, from beside its own
/// binary (platitude-core's `sequencer::run::HELPER_NAME`). Built with the
/// app, as a bin of the core crate.
const HELPER: &str = "pgg-todo-editor";

/// Where the bundle's own binaries find their frameworks.
const FROM_MACOS: &str = "@executable_path/../Frameworks";

/// Where a platform plugin, two levels under `Contents/PlugIns`, finds them.
const FROM_PLUGINS: &str = "@loader_path/../../Frameworks";

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut build = true;
    let mut out = None;
    let mut password_from = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-build" => build = false,
            "--out" => {
                out = Some(PathBuf::from(
                    args.next()
                        .ok_or("--out needs the directory to write into")?,
                ));
            }
            "--password-from" => {
                password_from = Some(
                    args.next()
                        .ok_or("--password-from needs the variable that holds it")?,
                );
            }
            other => return Err(format!("package does not take {other:?}")),
        }
    }
    let how = Wrap::from_variable(password_from.map(String::as_str))?;
    // The password's variable reaches no code of the build's or the app's:
    // a build script that printed it would print it into a public log.
    let hidden: Vec<String> = password_from.cloned().into_iter().collect();
    if !cfg!(target_os = "macos") {
        return Err(
            "package makes a macOS bundle, and only on a Mac: macdeployqt, \
                    install_name_tool, codesign and ditto are the Mac's tools — CI's macOS \
                    runner is one"
                .into(),
        );
    }
    let root = crate::tree::workspace_root();
    let out = out.unwrap_or_else(|| root.join("target").join("package"));
    // Counted like any compile, whether a workflow asked for it or a session.
    let _room = crate::budget::standalone(
        &root,
        if build {
            crate::budget::COMPILE
        } else {
            crate::budget::LIGHT
        },
        crate::budget::Rank::Normal,
        "package",
    )?;
    let qt = crate::qt::this_tree()?;
    let path = crate::qt::path_with_qt()?;
    let exe = crate::app_build::shipped_exe(
        &root,
        &path,
        crate::app_build::BuildEnv {
            qmake: None,
            unset: &hidden,
        },
        build,
    )?;

    std::fs::create_dir_all(&out).map_err(|e| format!("could not make {}: {e}", out.display()))?;
    let out = std::fs::canonicalize(&out)
        .map_err(|e| format!("could not resolve {}: {e}", out.display()))?;
    let app = out.join(bundle::APP);
    let main = skeleton(&root, &exe, &app)?;
    deploy(&qt, &root, &app, &main)?;
    sign(&app)?;
    super::stand::stand(&app, &main, password_from.map(String::as_str))?;
    let wrapped = wrap(&root, &out, &app, &how)?;
    let size = std::fs::metadata(&wrapped)
        .map(|m| m.len())
        .map_err(|e| format!("could not read {}: {e}", wrapped.display()))?;
    println!(
        "PASS: {} ({:.1} MB), from {}",
        wrapped.display(),
        size as f64 / 1_000_000.0,
        app.display()
    );
    Ok(())
}

/// Qt into the bundle: what macdeployqt finds the app needs — frameworks,
/// plugins, and the QML modules `ui/` imports — then the offscreen
/// platform, and the run paths of every binary in it kept inside.
///
/// macdeployqt resolves the app's `@rpath` names against the binary's own
/// run paths and its own Qt's library directory, deletes the run paths it
/// used and adds `@executable_path/../Frameworks` in their place. It does
/// not touch the helper (no dylib, no Qt), nor a plugin copied in after it.
fn deploy(qt: &crate::qt::Qt, root: &Path, app: &Path, main: &Path) -> Result<(), String> {
    println!("deploying Qt {} into the bundle…", qt.version);
    let ui = root.join("crates/platitude-app/src/ui");
    tool(
        &qt.bin.join("macdeployqt"),
        &[
            app.as_os_str(),
            OsStr::new(&format!("-qmldir={}", ui.display())),
            OsStr::new("-verbose=2"),
        ],
    )?;
    let offscreen = offscreen(qt, app)?;
    keep_inside(main, Some(FROM_MACOS))?;
    keep_inside(&main.with_file_name(HELPER), None)?;
    keep_inside(&offscreen, Some(FROM_PLUGINS))?;
    println!("{}", said("otool", &[OsStr::new("-L"), main.as_os_str()])?);
    Ok(())
}

/// Ad hoc: Apple silicon runs nothing unsigned, and every tool before this
/// one rewrote what the linker and Qt had signed. Then read back, strictly.
fn sign(app: &Path) -> Result<(), String> {
    tool(
        Path::new("codesign"),
        &[
            OsStr::new("--force"),
            OsStr::new("--deep"),
            OsStr::new("--sign"),
            OsStr::new("-"),
            app.as_os_str(),
        ],
    )?;
    tool(
        Path::new("codesign"),
        &[
            OsStr::new("--verify"),
            OsStr::new("--deep"),
            OsStr::new("--strict"),
            app.as_os_str(),
        ],
    )
}

/// The bundle as the build leaves it, before Qt: the two binaries, the
/// icon and the Info.plist. Answers the app's binary inside it.
fn skeleton(root: &Path, exe: &Path, app: &Path) -> Result<PathBuf, String> {
    if app.exists() {
        std::fs::remove_dir_all(app)
            .map_err(|e| format!("could not clear {}: {e}", app.display()))?;
    }
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    for dir in [&macos, &resources] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    }
    let main = macos.join(crate::app_build::exe_name());
    copy(exe, &main)?;
    copy(&exe.with_file_name(HELPER), &macos.join(HELPER))?;
    copy(
        &root.join("crates/platitude-app/assets/platitude.icns"),
        &resources.join("platitude.icns"),
    )?;
    let template_path = root.join("packaging/macos/Info.plist");
    let template = std::fs::read_to_string(&template_path)
        .map_err(|e| format!("could not read {}: {e}", template_path.display()))?;
    let plist = bundle::info_plist(&template, env!("CARGO_PKG_VERSION"), &bundle::snapshot_id())?;
    let plist_path = contents.join("Info.plist");
    std::fs::write(&plist_path, plist)
        .map_err(|e| format!("could not write {}: {e}", plist_path.display()))?;
    Ok(main)
}

/// The offscreen platform the stand runs on, which macdeployqt does not
/// deploy (it takes Cocoa's): copied in beside Cocoa's. It stays — a
/// snapshot can be started headless the way every run here is.
fn offscreen(qt: &crate::qt::Qt, app: &Path) -> Result<PathBuf, String> {
    let from = Path::new(&qt.prefix).join("plugins/platforms/libqoffscreen.dylib");
    let dir = app.join("Contents/PlugIns/platforms");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let to = dir.join("libqoffscreen.dylib");
    copy(&from, &to)?;
    Ok(to)
}

/// Leaves `binary` with no run path outside the bundle (the build names the
/// Qt it links against as one, on every binary it links), and with
/// `inside` among those left where it loads frameworks.
fn keep_inside(binary: &Path, inside: Option<&str>) -> Result<(), String> {
    let listed = said("otool", &[OsStr::new("-l"), binary.as_os_str()])?;
    let paths = bundle::rpaths(&listed);
    for path in paths.iter().filter(|path| !path.starts_with('@')) {
        tool(
            Path::new("install_name_tool"),
            &[
                OsStr::new("-delete_rpath"),
                OsStr::new(path),
                binary.as_os_str(),
            ],
        )?;
    }
    if let Some(inside) = inside.filter(|inside| !paths.iter().any(|path| path == inside)) {
        tool(
            Path::new("install_name_tool"),
            &[
                OsStr::new("-add_rpath"),
                OsStr::new(inside),
                binary.as_os_str(),
            ],
        )?;
    }
    Ok(())
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("could not copy {} to {}: {e}", from.display(), to.display()))
}

/// Runs a tool to its end, its account shown as it goes; a failure is red.
pub(super) fn tool(program: &Path, args: &[&OsStr]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("could not run {}: {e}", program.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} {:?} failed ({status})",
            program.display(),
            args
        ))
    }
}

/// What a tool printed, for reading; a failure is red with its stderr.
pub(super) fn said(program: &str, args: &[&OsStr]) -> Result<String, String> {
    let output = run_captured(Command::new(program).args(args))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "{program} {args:?} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

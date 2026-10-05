//! The steps, in the order each needs the one before: the app's imports
//! held to the bundle's lists, the build, the bundle's skeleton, Qt — the
//! plugins and QML modules the lists name, then the frameworks they and the
//! app load (`macdeployqt`) — the run paths, the bundle held to the lists,
//! the signature, a stand from the bundle alone, and the archive. Every
//! tool after the build is Qt's or the Mac's own (Xcode's command line
//! tools).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::bundle;
use super::contents;
use super::wrap::{Wrap, wrap};
use crate::subprocess::run_captured;

/// The helper the app starts for an interactive rebase, from beside its own
/// binary (platitude-core's `sequencer::run::HELPER_NAME`). Built with the
/// app, as a bin of the core crate.
const HELPER: &str = "pgg-todo-editor";

/// Where the bundle's own binaries find their frameworks.
const FROM_MACOS: &str = "@executable_path/../Frameworks";

/// Where a plugin, two levels under `Contents/PlugIns`, finds them.
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
    // Before the build: an import the bundle would not carry costs nothing
    // to find.
    imports(&qt, &root)?;
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
    deploy(&qt, &app, &main)?;
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

/// The app's imports as Qt's own scanner reads them, held to the lists:
/// once as `ui/` writes them, where each must be carried, and once followed
/// through Qt's modules, where each one reached is carried or left out on
/// purpose.
fn imports(qt: &crate::qt::Qt, root: &Path) -> Result<(), String> {
    let scanner = format!("{}/libexec/qmlimportscanner", qt.prefix);
    let ui = root.join("crates/platitude-app/src/ui");
    let qml = Path::new(&qt.prefix).join("qml");
    let root_path = OsStr::new("-rootPath");
    let import_path = OsStr::new("-importPath");
    let written = scan(&scanner, &[root_path, ui.as_os_str()])?;
    let reached = scan(
        &scanner,
        &[root_path, ui.as_os_str(), import_path, qml.as_os_str()],
    )?;
    let mut parted = contents::written(&contents::scanned(&written)?);
    parted.extend(contents::reached(&contents::scanned(&reached)?));
    if parted.is_empty() {
        println!(
            "ui/ imports only modules the bundle carries, and each module they reach is carried \
             or left out on purpose"
        );
        return Ok(());
    }
    Err(format!(
        "ui/'s imports and the lists in crates/xtask/src/package/contents.rs part — an import \
         ui/ took up, or a Qt that moved what a module imports:\n  {}",
        parted.join("\n  ")
    ))
}

/// qmlimportscanner's answer. A file it cannot read it names on stderr,
/// still answering for the rest with nothing of that file's — so anything
/// on stderr is red: the imports it hides are the ones the lists would miss.
fn scan(scanner: &str, args: &[&OsStr]) -> Result<String, String> {
    let output = run_captured(Command::new(scanner).args(args))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stderr.trim().is_empty() {
        return Err(format!(
            "{scanner} {args:?} did not read every file ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Qt into the bundle: the plugins and QML modules the lists name, then the
/// frameworks they and the app load, every run path kept inside, `qt.conf`,
/// and the whole held to the lists.
///
/// macdeployqt deploys for every dylib already in the bundle beside the app
/// (shared.cpp's `findAppLibraries`), so the plugins go in first. With
/// `-no-plugins` it adds no plugin and with no `-qmldir` no module — but it
/// looks for imports itself where the directory it runs in holds a `.qml`,
/// so it runs in the bundle's own. It resolves the `@rpath` names against
/// the app's run paths and its own Qt's library directory, and swaps any
/// run path it used for `@executable_path/../Frameworks` — the app's; a
/// plugin's, relative to where Qt keeps the plugin, resolves to none it
/// used, and [`keep_inside`] sets it.
fn deploy(qt: &crate::qt::Qt, app: &Path, main: &Path) -> Result<(), String> {
    println!("deploying Qt {} into the bundle…", qt.version);
    let libraries = place(qt, app)?;
    finished(
        Command::new(qt.bin.join("macdeployqt"))
            .args([
                app.as_os_str(),
                OsStr::new("-no-plugins"),
                OsStr::new("-verbose=2"),
            ])
            .current_dir(app),
    )?;
    keep_inside(main, Some(FROM_MACOS))?;
    keep_inside(&main.with_file_name(HELPER), None)?;
    for library in &libraries {
        keep_inside(library, Some(FROM_PLUGINS))?;
    }
    let conf = app.join("Contents/Resources/qt.conf");
    std::fs::write(&conf, contents::QT_CONF)
        .map_err(|e| format!("could not write {}: {e}", conf.display()))?;
    held(app)?;
    println!("{}", said("otool", &[OsStr::new("-L"), main.as_os_str()])?);
    Ok(())
}

/// The lists' plugins and modules, put where macdeployqt puts its own: a
/// plugin under `Contents/PlugIns`; a module's files under
/// `Contents/Resources/qml`, but for its plugin, which goes in
/// `Contents/PlugIns/quick` with a link to it left in its place — codesign
/// signs code under PlugIns and refuses it among resources. Answers every
/// library put in.
fn place(qt: &crate::qt::Qt, app: &Path) -> Result<Vec<PathBuf>, String> {
    let prefix = Path::new(&qt.prefix);
    let inside = app.join("Contents");
    let mut libraries = Vec::new();
    for plugin in contents::QT_PLUGINS {
        let to = inside.join("PlugIns").join(plugin);
        make_dir(to.parent().unwrap_or(inside.as_path()))?;
        copy(&prefix.join("plugins").join(plugin), &to)?;
        libraries.push(to);
    }
    let quick = inside.join("PlugIns/quick");
    make_dir(&quick)?;
    for module in contents::QML_MODULES {
        let dir = contents::module_dir(module);
        let from = prefix.join("qml").join(&dir);
        let to = inside.join("Resources/qml").join(&dir);
        make_dir(&to)?;
        let unreadable = |e: std::io::Error| format!("could not read {}: {e}", from.display());
        for entry in std::fs::read_dir(&from).map_err(unreadable)? {
            let entry = entry.map_err(unreadable)?;
            let source = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            // A directory under a module's is another module, or Qt Design
            // Studio's.
            if !source.is_file() {
                continue;
            }
            match contents::goes(&name) {
                contents::Goes::WithModule => copy(&source, &to.join(&name))?,
                contents::Goes::Quick => {
                    let library = quick.join(&name);
                    copy(&source, &library)?;
                    link(&contents::plugin_link(module, &name), &to.join(&name))?;
                    libraries.push(library);
                }
                contents::Goes::Nowhere => {}
            }
        }
    }
    // As macdeployqt strips what it copies itself.
    for library in &libraries {
        tool(Path::new("strip"), &[OsStr::new("-x"), library.as_os_str()])?;
    }
    println!(
        "put in {} Qt plugins and {} QML modules, by the lists",
        contents::QT_PLUGINS.len(),
        contents::QML_MODULES.len()
    );
    Ok(libraries)
}

/// The bundle held to the lists before it is signed: a Qt that moved what a
/// plugin or a module brings is red here, by name.
fn held(app: &Path) -> Result<(), String> {
    let parted = contents::parted(&contents::carried(app)?);
    if parted.is_empty() {
        println!(
            "the bundle carries what the lists name: {} frameworks, {} Qt plugins, {} QML modules",
            contents::FRAMEWORKS.len(),
            contents::QT_PLUGINS.len(),
            contents::QML_MODULES.len()
        );
        return Ok(());
    }
    Err(format!(
        "the bundle and the lists in crates/xtask/src/package/contents.rs part — a Qt that moved \
         what a plugin or a module brings:\n  {}",
        parted.join("\n  ")
    ))
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

/// Leaves `binary` with `inside` for its one run path, or with none: the
/// build names the Qt it links against as one on every binary it links, and
/// Qt's plugins name the way back to where Qt keeps its libraries.
fn keep_inside(binary: &Path, inside: Option<&str>) -> Result<(), String> {
    let listed = said("otool", &[OsStr::new("-l"), binary.as_os_str()])?;
    let paths = bundle::rpaths(&listed);
    for path in paths.iter().filter(|path| Some(path.as_str()) != inside) {
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

fn make_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))
}

/// A link at `at` to `target`, relative to where it stands.
#[cfg(unix)]
fn link(target: &str, at: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, at)
        .map_err(|e| format!("could not link {} to {target}: {e}", at.display()))
}

#[cfg(not(unix))]
fn link(target: &str, at: &Path) -> Result<(), String> {
    Err(format!(
        "could not link {} to {target}: a bundle is made on a Mac",
        at.display()
    ))
}

/// Runs a tool to its end, its account shown as it goes; a failure is red.
pub(super) fn tool(program: &Path, args: &[&OsStr]) -> Result<(), String> {
    finished(Command::new(program).args(args))
}

/// [`tool`], for a command made ready already.
fn finished(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|e| format!("could not run {command:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed ({status})"))
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

//! `film shoot`: one side of a scene, headless — the release this tree
//! builds with the film verb in, a copy of the scene's repository, a
//! settings dir of its own, and `verify-ui film` playing the scene into
//! the side's directory.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::Side;
use super::scene::{self, Repo, Scene};

/// The scene's repository as built once: every side copies it. Named so no
/// file a preset tracks carries it (`demo::copy_root` rebinds the name).
pub(super) const BASE: &str = "pgg-film-base";

/// Which preset the base was built from, beside it.
const BASE_PRESET: &str = "base-preset.txt";

/// The scene's text as each side was shot from it, so `bake` can refuse
/// two sides of different scenes.
pub(super) const SCENE_COPY: &str = "scene.txt";

pub(super) const CAPTIONS: &str = "captions.txt";

pub(super) struct Options {
    pub(super) scenes: Vec<PathBuf>,
    pub(super) side: &'static Side,
    pub(super) build: bool,
    pub(super) fresh_base: bool,
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        scenes: Vec::new(),
        side: super::side("now")?,
        build: true,
        fresh_base: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--side" => opts.side = super::side(it.next().ok_or("--side needs a side")?)?,
            "--no-build" => opts.build = false,
            "--fresh-base" => opts.fresh_base = true,
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            scene => opts.scenes.push(PathBuf::from(scene)),
        }
    }
    if opts.scenes.is_empty() {
        return Err("film shoot needs a scene file".into());
    }
    Ok(opts)
}

pub(super) fn run(tree: &Path, opts: &Options) -> Result<(), String> {
    if !super::inject::injected(tree) {
        return Err(format!(
            "the film verb is not in this tree: `{}` first",
            super::IN.line()
        ));
    }
    let refreshed = super::inject::refreshed(tree)?;
    for (index, path) in opts.scenes.iter().enumerate() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        let scene = scene::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let dir = super::scene_dir(tree, path)?;
        // One build for the lot: the scenes after the first run on it. A
        // component just rewritten is built whatever --no-build says.
        let build = (opts.build || refreshed) && index == 0;
        shoot(&dir, &scene, &text, opts.side, build, opts.fresh_base)?;
    }
    Ok(())
}

fn shoot(
    dir: &Path,
    scene: &Scene,
    text: &str,
    side: &Side,
    build: bool,
    fresh_base: bool,
) -> Result<(), String> {
    let work = repository(dir, scene, side, fresh_base)?;
    let root = work
        .parent()
        .ok_or("a copied repository stands in a root")?;
    prep(root, &work, &scene.prep)?;

    let config = fresh(&dir.join(format!("cfg-{}", side.name)))?;
    std::fs::write(config.join("state.toml"), state(scene))
        .map_err(|e| format!("writing the state: {e}"))?;
    let shots = fresh(&dir.join(side.name))?;
    std::fs::write(shots.join(SCENE_COPY), text).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(CAPTIONS), scene.captions.join("\n") + "\n")
        .map_err(|e| e.to_string())?;

    let exe = std::env::current_exe().map_err(|e| format!("where this runner is: {e}"))?;
    let mut command = Command::new(exe);
    command
        .arg("verify-ui")
        .arg("film")
        .arg(scene::arg(scene, side.tag, side.colour))
        .arg("--repo")
        .arg(&work)
        .arg("--config-dir")
        .arg(&config)
        .arg("--shot-dir")
        .arg(&shots)
        .args(["--no-board", "--watchdog-ms"])
        .arg(ceiling_ms(scene).to_string());
    if !build {
        command.arg("--no-build");
    }
    let status = command
        .status()
        .map_err(|e| format!("starting verify-ui: {e}"))?;
    let frames = std::fs::read_to_string(shots.join("frames.tsv")).map_err(|_| {
        format!(
            "verify-ui ({status}) left no frames.tsv in {}: the run's own lines above say why",
            shots.display()
        )
    })?;
    println!(
        "film: {} — {} frames of side {} in {}",
        dir.file_name().unwrap_or_default().to_string_lossy(),
        frames
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        side.name,
        shots.display()
    );
    // The frames stay for `changes` to read; what the run judged is not
    // passed over.
    if !status.success() {
        return Err(format!(
            "verify-ui did not pass ({status}): the run's own lines above say why"
        ));
    }
    Ok(())
}

/// What the run is given: its start, then the scene's own waits twice
/// over — a loaded machine slows the frames, not the scene's clock.
fn ceiling_ms(scene: &Scene) -> u64 {
    (60_000 + 2 * scene::length_ms(scene)).max(120_000)
}

/// The side's own copy of the scene's base, built first if it is not
/// standing (or another preset's, or `fresh` asks for it).
fn repository(dir: &Path, scene: &Scene, side: &Side, fresh: bool) -> Result<PathBuf, String> {
    let base = dir.join(BASE);
    match &scene.repo {
        Repo::Preset(preset) => {
            let note = dir.join(BASE_PRESET);
            let standing = std::fs::read_to_string(&note).is_ok_and(|built| built.trim() == preset);
            if fresh || !standing || !base.join("repo").is_dir() {
                if base.exists() {
                    std::fs::remove_dir_all(&base)
                        .map_err(|e| format!("clearing {}: {e}", base.display()))?;
                }
                crate::demo::create(preset, Some(base.clone()))?;
                std::fs::write(&note, format!("{preset}\n")).map_err(|e| e.to_string())?;
                println!("film: built {preset} at {} for every side", base.display());
            }
        }
        Repo::Made if !base.join("repo").is_dir() => {
            return Err(format!(
                "`repo made` copies {}, and no repository stands there: build it with `{} --at \
                 {}` and change it",
                base.join("repo").display(),
                crate::demo::DEMO_REPO.line(),
                base.display()
            ));
        }
        Repo::Made => {}
    }
    let root = dir.join(format!("pgg-film-{}", side.name));
    crate::demo::copy_root(&base, &root)?;
    let work = root.join("repo");
    own_repository(&work)?;
    Ok(work)
}

/// `work` is a repository of its own: a copy that went wrong would hand
/// every git below to the tree this runner stands in.
fn own_repository(work: &Path) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(work)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("starting git: {e}"))?;
    let top = String::from_utf8_lossy(&out.stdout)
        .trim()
        .replace('\\', "/");
    let want = work.to_string_lossy().replace('\\', "/");
    if !out.status.success() || !top.eq_ignore_ascii_case(&want) {
        return Err(format!(
            "{} is not a repository of its own (git answers {top:?})",
            work.display()
        ));
    }
    Ok(())
}

/// The scene's git, before the window opens: under the copy's own
/// settings, as the preset was built.
fn prep(root: &Path, work: &Path, prep: &[Vec<String>]) -> Result<(), String> {
    for args in prep {
        let out = Command::new("git")
            .args(args)
            .current_dir(work)
            .env("GIT_CONFIG_GLOBAL", root.join("no-global-config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| format!("starting git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "prep `git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    Ok(())
}

/// A directory emptied for this run.
fn fresh(dir: &Path) -> Result<PathBuf, String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("clearing {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    Ok(dir.to_path_buf())
}

/// The window every side opens in: one size, the panels the films were
/// framed against, and the scene's own layout keys over them.
fn state(scene: &Scene) -> String {
    let mut layout: Vec<(String, String)> = [
        ("commands_height", "280"),
        ("details_tree", "true"),
        ("details_width", "400"),
        ("diff_split", "false"),
        ("graph_labels_width", "-1"),
        ("graph_lanes_width", "-1"),
        ("sidebar_collapsed", "false"),
        ("sidebar_width", "260"),
        ("tags_shown", "true"),
        ("wip_tree", "true"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    for (key, value) in &scene.layout {
        match layout.iter_mut().find(|(known, _)| known == key) {
            Some(entry) => entry.1.clone_from(value),
            None => layout.push((key.clone(), value.clone())),
        }
    }
    let mut text = String::from("version = 1\n\n[layout]\n");
    for (key, value) in layout {
        text.push_str(&format!("{key} = {value}\n"));
    }
    text.push_str(
        "\n[layout.sections]\nbranches = true\nremotes = true\nstashes = true\ntags = true\n\
         worktree = true\n\n[window]\nheight = 900\nmaximized = false\nwidth = 1440\n",
    );
    text
}

#[cfg(test)]
mod tests {
    use super::state;
    use crate::film::scene::parse;

    #[test]
    fn the_scene_s_layout_keys_go_over_the_defaults() {
        let scene = parse("repo deep\nlayout sidebar_width 300\nview 0 0 9 9\ncap -\ncap x\n")
            .expect("parses");
        let text = state(&scene);
        assert!(text.contains("\nsidebar_width = 300\n"));
        assert_eq!(text.matches("sidebar_width").count(), 1);
        assert!(text.contains("width = 1440"));
    }
}

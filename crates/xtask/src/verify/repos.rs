//! What a verb needs made for it before the app opens: the repositories
//! its tabs stand on, and the folders it is handed.

use std::path::PathBuf;
use std::process::Command;

/// Names of a spread of lengths, for the verbs whose subject is the tab
/// strip: the short ones keep their width and the long ones give way
/// together (デザイン規約 §ウィンドウの縁), which a row of `repo`s cannot show.
const TAB_NAMES: [&str; 16] = [
    "ui",
    "platitude-gg",
    "core",
    "sealed-class-enumizer",
    "notes",
    "a-repository-with-a-rather-long-name",
    "docs",
    "another-repository-with-a-long-name",
    "spike",
    "yet-another-long-repository-name",
    "assets",
    "the-longest-repository-name-in-the-row",
    "ci",
    "one-more-long-repository-name-here",
    "tools",
    "a-final-repository-with-a-long-name",
];

/// The strip the `tab-widths` verb is run against: repositories named off
/// the ladder above, as many as the state the run asked for takes.
///
/// The argument names the state, not a count
/// (rules-refs/app-ui.md「枚数ではなく状態を名乗らせ」): the counts are the
/// ladder's two ends, and the state reached is judged
/// (`TabStrip.tabNamesCut`). A bare number still works, for choosing
/// those counts again on a new band; it claims only that no name was
/// crushed.
pub(super) fn tab_width_repos(arg: &str) -> Result<Vec<PathBuf>, String> {
    let count = match arg {
        "" | "ample" => 6,
        "short" => TAB_NAMES.len(),
        _ => strip_count("tab-widths", arg)?,
    };
    named_strip(count, "basic", "tab-widths")
}

/// How many tabs a strip verb was asked for. Only a `badges` argument
/// ending at the `:` names none.
fn strip_count(verb: &str, arg: &str) -> Result<usize, String> {
    if arg.is_empty() {
        return Ok(8);
    }
    let count: usize = arg
        .parse()
        .map_err(|_| format!("{verb} takes a number of tabs; got {arg:?}"))?;
    if count == 0 || count > TAB_NAMES.len() {
        return Err(format!("{verb} takes 1..={} tabs", TAB_NAMES.len()));
    }
    Ok(count)
}

/// `count` repositories off the ladder, the last of them built to
/// `front`: the tab opened last is the one in front, the page in the
/// picture and the only one the band shows a state for
/// (`BandStateGroup.stateWt`). The rest are `empty` — the subject is the
/// strip above them.
fn named_strip(count: usize, front: &str, verb: &str) -> Result<Vec<PathBuf>, String> {
    let mut made = Vec::with_capacity(count);
    for (position, name) in TAB_NAMES.iter().take(count).enumerate() {
        let preset = if position + 1 == count {
            front
        } else {
            "empty"
        };
        let repo = crate::demo::create_named(preset, None, name)?;
        made.push(repo);
    }
    println!("demo repos ({verb}): {count} named after their length, {front} in front");
    Ok(made)
}

/// How many tabs a `badges` run asked to stand its group beside, off the
/// second half of its argument (`<width>:<tabs>`), or `None` where it
/// named none and the ordinary one-repository fixture stands.
///
/// The band's shortfall is shared between the tab strip and the state
/// group (規約 §ウィンドウの縁), so where the group gives up its words
/// depends on the tabs beside it; against one repository it reaches the
/// window's floor still wearing them.
pub(super) fn band_tab_count(arg: &str) -> Result<Option<usize>, String> {
    match arg.split_once(':') {
        Some((_, tabs)) => Ok(Some(strip_count("badges", tabs)?)),
        None => Ok(None),
    }
}

/// The strip `badges` stands its group beside: `count` tabs off the same
/// ladder, with the run's one preset on the tab in front — the only one
/// whose state the band shows.
pub(super) fn band_state_repos(count: usize, presets: &[String]) -> Result<Vec<PathBuf>, String> {
    let front = match presets {
        [] => "basic",
        [one] => one.as_str(),
        _ => {
            return Err(
                "badges takes one --preset: the tab in front is the only one the band shows a \
                 state for"
                    .into(),
            );
        }
    };
    named_strip(count, front, "badges")
}

/// The preset a run stands on where it names none. Also read by the
/// census line (`options::census_line`, [`preset_is_the_default`]):
/// naming it and leaving it off are one fixture, and two spellings would
/// be two lines every gate runs twice.
pub(super) const DEFAULT_PRESET: &str = "basic";

/// One fresh demo repository per preset, in the order they were asked
/// for — which is the order the tabs come up in. Each is called `repo`,
/// or after its preset where `named` ([`Route::NamedPresets`]).
fn preset_repos(presets: &[String], named: bool) -> Result<Vec<PathBuf>, String> {
    let default = [DEFAULT_PRESET.to_string()];
    let presets = if presets.is_empty() {
        &default[..]
    } else {
        presets
    };
    let mut made = Vec::with_capacity(presets.len());
    for preset in presets {
        let name = if named { preset.as_str() } else { "repo" };
        let repo = crate::demo::create_named(preset, None, name)?;
        println!("demo repo ({preset}): {}", repo.display());
        made.push(repo);
    }
    Ok(made)
}

/// The linked working copy the `nested-copy` preset keeps below its
/// root — the one fixture whose opened folder is not the repository's
/// own.
fn nested_copy_repo() -> Result<Vec<PathBuf>, String> {
    let work = crate::demo::create("nested-copy", None)?;
    let root = work
        .parent()
        .ok_or_else(|| format!("no root above {}", work.display()))?;
    let copy = root.join(crate::demo::NESTED_COPY);
    println!(
        "demo repo (nested-copy): {} in {}",
        copy.display(),
        work.display()
    );
    Ok(vec![copy])
}

/// The strip the `tab-name` verb is run against (デザイン規約 §タブの所作):
/// `foo/repo` and `bar/repo` grow a parent each, the two under a `deep`
/// have to grow past it, and `solo`, shared with nobody, never moves.
///
/// All under one root so the paths differ only where the names do, and a
/// root of this run's own (`demo::claim_root`) so two runs do not build
/// into one. `solo` comes last and carries the history: the tab opened
/// last is the page in the picture.
pub(super) fn tab_name_repos() -> Result<Vec<PathBuf>, String> {
    let root = crate::demo::claim_root("tab-name")?;
    let mut made = Vec::with_capacity(5);
    for (under, name) in [
        (Some("foo"), "repo"),
        (Some("bar"), "repo"),
        (Some("1/deep"), "repo"),
        (Some("2/deep"), "repo"),
        (None, "solo"),
    ] {
        let at = under.map_or_else(|| root.clone(), |dir| root.join(dir));
        let preset = if name == "solo" { "basic" } else { "empty" };
        made.push(crate::demo::create_named(preset, Some(at), name)?);
    }
    println!("demo repos (tab-name): four called `repo`, and one nobody shares");
    Ok(made)
}

/// The description a verb needs typed for it, when the run brought none:
/// the verbs that pull the commit editor open need text taller than the
/// pane, so the pull lands on the bound.
pub(super) fn body_for(verb: &str) -> Option<String> {
    match verb {
        "wip-grow" | "wip-grow-squeeze" => Some(crate::demo::pasted(4000)),
        _ => None,
    }
}

/// The one merge tool the candidate list is guaranteed to hold:
/// `git mergetool --tool-help` lists the machine's, and a test container
/// has no windowed one.
const SEEDED_TOOL: &str = "demo-editor";

/// What the seeded tool does once launched: block, long enough to be
/// waited on. `sleep` works on all three OSes because git runs `cmd`
/// through its own shell (Git for Windows' `sh` on Windows).
///
/// The picture is taken after the tool exits (`AutoActDriver.writeBarrier`
/// and `treeBarrier`), so the length buys only a wait a person watching
/// can see, and every run pays it.
const TOOL_CMD: &str = "sleep 2";

/// Names a tool in the repository's own config, for the verbs whose
/// picture is the tool list and the one that hands a file over.
///
/// The candidates arrive in two waves — names in config, then the
/// installed sweep — and an empty list closes the card
/// (`AppCombo.hasList`), leaving the verb waiting on a popup that never
/// opens. The run's own row in the first wave carries the picture on
/// every machine.
///
/// The settings verbs get the command only: `merge.guitool` would land in
/// the dialog field they photograph. `open-mergetool` also needs
/// `merge.guitool` (without it `conflict::configured_tool` has no answer
/// and the menu row opens the settings instead) and `trustExitCode`
/// (without it git asks a closed stdin whether the merge went well and
/// calls the file failed).
///
/// A `--repo` of one's own is written to as well: the run owns it for its
/// length (`claim_resource`), and a merge tool named in it is overwritten.
pub(super) fn seed_merge_tool(
    verb: &str,
    repos: &[PathBuf],
    path: &std::ffi::OsStr,
) -> Result<(), String> {
    let launched = match verb {
        "settings-tools" | "settings-tools-loading" => false,
        "open-mergetool" => true,
        _ => return Ok(()),
    };
    let mut keys = vec![(format!("mergetool.{SEEDED_TOOL}.cmd"), TOOL_CMD.to_string())];
    if launched {
        keys.push((
            format!("mergetool.{SEEDED_TOOL}.trustExitCode"),
            "true".to_string(),
        ));
        keys.push(("merge.guitool".to_string(), SEEDED_TOOL.to_string()));
    }
    for repo in repos {
        for (key, value) in &keys {
            let status = Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(["config", key, value])
                .env("PATH", path)
                .status()
                .map_err(|e| format!("failed to run git: {e}"))?;
            if !status.success() {
                return Err(format!("could not name a merge tool in {}", repo.display()));
            }
        }
    }
    if launched {
        println!("merge editor seeded: {SEEDED_TOOL} runs {TOOL_CMD:?}, and git would launch it");
    } else {
        println!("merge editor named in config: {SEEDED_TOOL} (the list's first wave)");
    }
    Ok(())
}

/// The path a verb needs handed to it, made on the spot; empty for every
/// other verb.
///
/// The open-refused verbs' subject is a folder no preset can be. Both
/// sit one level down, so the folder the second try opens at is a real
/// one with the failed pick inside it.
pub(super) fn folder_for(
    verb: &str,
    shot_dir: &std::path::Path,
    path: &std::ffi::OsStr,
) -> Result<String, String> {
    let made = shot_dir.join("picked");
    match verb {
        "open-not-a-repo"
        | "open-not-a-repo-retry"
        | "open-not-a-repo-cancel"
        | "open-dialog-sweep"
        | "open-fail-tab"
        | "open-fail-tab-log"
        | "open-fail-sweep" => {
            let dir = made.join("notes");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(dir.display().to_string())
        }
        "open-bare" | "open-fail-tab-bare" => {
            let dir = made.join("origin.git");
            std::fs::create_dir_all(&made).map_err(|e| e.to_string())?;
            let status = Command::new("git")
                .args(["init", "--bare", "--quiet"])
                .arg(&dir)
                .env("PATH", path)
                .status()
                .map_err(|e| format!("failed to run git: {e}"))?;
            if !status.success() {
                return Err("git init --bare failed".into());
            }
            Ok(dir.display().to_string())
        }
        // The thirteenth off the ladder, to arrive among the twelve. Built
        // here because [`for_run`]'s list is opened at startup.
        "tab-open-go" | "tab-open-moved-on" => {
            let repo = crate::demo::create_named("basic", None, TAB_NAMES[12])?;
            Ok(repo.display().to_string())
        }
        // A picture made here keeps the avatar verbs' line free of one
        // machine's path, so the census can record it. Its name holds what
        // a URL spells differently from a path — a space, a non-ASCII
        // letter, `#`, a `%` before hex digits and, where a name may hold
        // one, a backslash — and the card's verbs wait on the picture
        // loading; `avatar-assign` says whether it was filed.
        "avatar-assign" | "avatar-settings" | "avatar-combo" | "avatar-row-lit"
        | "avatar-remove" => {
            std::fs::create_dir_all(&made).map_err(|e| e.to_string())?;
            let name = match cfg!(windows) {
                true => "face 顔 #1 %41.png",
                false => "face 顔 #1 %41\\x.png",
            };
            let picture = made.join(name);
            std::fs::write(&picture, tiny_png()).map_err(|e| e.to_string())?;
            Ok(picture.display().to_string())
        }
        _ => Ok(String::new()),
    }
}

/// A 2×2 opaque PNG, enough for the avatar store to decode and file.
fn tiny_png() -> Vec<u8> {
    crate::png::rgba(2, 2, |x, y| {
        if (x + y) % 2 == 0 {
            [0x2a, 0x9d, 0x8f, 0xff]
        } else {
            [0xe9, 0xc4, 0x6a, 0xff]
        }
    })
}

/// The repositories one run opens, in the order their tabs come up: the
/// `--repo`s, else what the verb's `Route` builds.
pub(super) fn for_run(opts: &super::options::Options) -> Result<Vec<PathBuf>, String> {
    if !opts.repo.is_empty() && !opts.preset.is_empty() {
        return Err("--repo and --preset name different fixtures: pass one of them".into());
    }
    let badges = matches!(
        opts.verb.as_str(),
        "badges" | "badges-hover" | "badges-hover-early" | "badges-all" | "badges-all-hover"
    );
    // A `--repo` beside a tab count: `--repo` wins the list, while
    // `WindowBadgeActs` waits for a band carrying the count, and the run
    // hangs silently to its ceiling.
    if badges && !opts.repo.is_empty() && band_tab_count(&opts.arg)?.is_some() {
        return Err(
            "badges builds its own strip from the count in its argument: pass one of \
             --repo and `<width>:<tabs>`"
                .into(),
        );
    }
    let repos = if !opts.repo.is_empty() {
        opts.repo.clone()
    } else {
        match route_of(&opts.verb, &opts.arg)? {
            Route::Strip(None) => tab_width_repos(&opts.arg)?,
            Route::Strip(Some(count)) => tab_width_repos(count)?,
            Route::Colliding => tab_name_repos()?,
            Route::BandStrip(count) => band_state_repos(count, &opts.preset)?,
            Route::NestedCopy => nested_copy_repo()?,
            Route::Presets => preset_repos(&opts.preset, false)?,
            Route::NamedPresets => preset_repos(&opts.preset, true)?,
        }
    };
    Ok(repos)
}

/// Which fixture a run of this verb stands on. Read by both the builder
/// above and the census line ([`preset_is_the_default`]); a second copy
/// of "does this verb read `--preset`" would drift, recording one run
/// under two lines.
enum Route {
    /// A strip off the ladder of names: `None` takes the count from the
    /// run's argument, `Some` is the verb's own. `--preset` is not read —
    /// the shape is the fixture.
    Strip(Option<&'static str>),
    /// The one strip whose names collide, which the ladder deliberately
    /// has none of.
    Colliding,
    /// A strip with the run's own preset on the tab in front.
    BandStrip(usize),
    /// The `nested-copy` preset's linked working copy, the one fixture
    /// copy that does not sit beside its repository. `--preset` is not
    /// read — the shape is the fixture.
    NestedCopy,
    /// One fresh demo repository per preset, [`DEFAULT_PRESET`] where the
    /// run names none.
    Presets,
    /// The same, each folder named after its preset: the run reads the
    /// name the operation panel says per tab, and with every tab called
    /// `repo` a panel still naming the tab it left would read as right.
    NamedPresets,
}

fn route_of(verb: &str, arg: &str) -> Result<Route, String> {
    Ok(match verb {
        // The folder the picker comes up in is the repository's, which
        // only a copy away from it can tell.
        "open-picker" if arg == "copy" => Route::NestedCopy,
        "tab-widths" => Route::Strip(None),
        // The argument names the tab the hand is on, so the count is
        // fixed here rather than taken from `tab-widths`' state names.
        "tab-mark" => Route::Strip(Some("8")),
        "tab-name" => Route::Colliding,
        // The order is the subject; four distinct names make it readable.
        // The argument names two of them.
        "tab-drag" | "tab-hold" => Route::Strip(Some("4")),
        // A strip that has to overflow a window at its floor. Twelve:
        // tabs above their own floor divide the run exactly and overflow
        // by nothing — eight do on the wider Linux band (no grab run or
        // window buttons), and `tab-edge` / `tab-pin` then photograph a
        // strip that never moved. `tab-open-go` / `tab-open-moved-on`
        // open a thirteenth, built by [`folder_for`]: a tab already in
        // the strip cannot arrive in it.
        "tab-edge" | "tab-pin" | "tab-pin-go" | "tab-open-go" | "tab-open-moved-on" => {
            Route::Strip(Some("12"))
        }
        // A strip and a page with a state at once: the count rides in the
        // argument beside the width (`band_tab_count`); with none, the
        // ordinary one repository.
        "badges" | "badges-hover" | "badges-hover-early" | "badges-all" | "badges-all-hover" => {
            match band_tab_count(arg)? {
                Some(count) => Route::BandStrip(count),
                None => Route::Presets,
            }
        }
        "ops-repo-pick" => Route::NamedPresets,
        _ => Route::Presets,
    })
}

/// Whether the `--preset` flag as typed builds exactly what leaving it
/// off would. Only [`Route::Presets`] and [`Route::NamedPresets`] have
/// that default; on every other route a run keeps the words it was typed
/// with.
pub(super) fn preset_is_the_default(verb: &str, arg: &str, presets: &[String]) -> bool {
    matches!(presets, [one] if one == DEFAULT_PRESET)
        && matches!(
            route_of(verb, arg),
            Ok(Route::Presets | Route::NamedPresets)
        )
}

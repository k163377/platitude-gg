//! What a verb needs made for it before the app opens: the repositories
//! its tabs stand on, and the folders it is handed.

use std::path::PathBuf;
use std::process::Command;

/// Names of a spread of lengths, for the one verb whose subject is the
/// tab strip itself.
///
/// Both halves of the rule need seeing at once (デザイン規約 §ウィンドウの縁):
/// the short names keep their own width however crowded the strip gets,
/// and the long ones give way together. A row of repositories all called
/// `repo` can show neither.
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
/// **The argument names the state, not a count.** Which count crowds this
/// band is the window's furniture and the platform's font talking — the
/// same six tabs leave Windows sharing a run it has to cut and Linux with
/// room to spare (rules-refs/app-ui.md) — so a run that named a count
/// would be claiming a state nobody had checked it reached. What the
/// counts here are is the two ends of the ladder: the fewest that share a
/// run at all, and the most it holds, which crowds every band this app
/// has been run on. The state itself is read off the strip and judged
/// (`TabStrip.tabNamesCut`), so a count that stopped reaching it fails
/// rather than passing quietly.
///
/// A bare number still works, for choosing those counts again on a band
/// nobody has measured — it makes the strip and reports what it came to,
/// and claims only that no name was crushed.
pub(super) fn tab_width_repos(arg: &str) -> Result<Vec<PathBuf>, String> {
    let count = match arg {
        "" | "ample" => 6,
        "short" => TAB_NAMES.len(),
        _ => strip_count("tab-widths", arg)?,
    };
    named_strip(count, "basic", "tab-widths")
}

/// How many tabs a strip verb was asked for. Eight where it named none —
/// which is only `badges`, whose own argument may end before the `:`.
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
/// `front`.
///
/// That last one is the one carrying a real history — the tab opened
/// last is the one left in front, so that is the page under the strip in
/// the picture, and the only page the band is showing a state for
/// (`BandStateGroup.stateWt` reads the tab in front and no other). The
/// rest are bare of commits: what is being looked at is above them, and
/// building sixteen histories to photograph one band would be paying for
/// the wrong thing.
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
/// group (規約 §ウィンドウの縁), so the width at which the group gives up
/// its words moves with how many tabs are beside it and how long their
/// names are. A run against one repository photographs none of that: the
/// group there is narrowed by the window alone, and reaches the floor a
/// hand can drag to still wearing its words.
pub(super) fn band_tab_count(arg: &str) -> Result<Option<usize>, String> {
    match arg.split_once(':') {
        Some((_, tabs)) => Ok(Some(strip_count("badges", tabs)?)),
        None => Ok(None),
    }
}

/// The strip `badges` stands its group beside: `count` tabs off the same
/// ladder, with the run's own preset on the one in front.
///
/// One preset, because the tab in front is the only one whose state the
/// band shows — a second would name a fixture no picture here reaches.
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

/// The one repository a run stands on where it names no preset.
///
/// **Two readers, one value.** The other is the line the census files a
/// passing run under: naming this preset and leaving it off build the
/// same fixture, so two spellings of it there would be two lines, and
/// every gate that owed the verb would run the same thing twice on each
/// side (`options::census_line`, [`preset_is_the_default`]).
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
/// root, which is what the run opens — the repository's own copy is
/// what every other fixture hands over, and a tab standing in that one
/// has no second folder to tell apart.
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

/// The strip the `tab-name` verb is run against: repositories that share
/// a folder name, and one that shares nothing with anybody.
///
/// Both halves of that rule need seeing at once too (デザイン規約 §タブの所作):
/// `foo/repo` and `bar/repo` grow a parent each, the two `repo`s under a
/// `deep` of their own have to grow past that parent as well, and `solo`
/// stands there proving that a name nobody shares never moves. A ladder
/// of distinct names — the one `tab-widths` runs on — can show none of
/// it, because there is nothing there to tell apart.
///
/// Built under one root so the paths differ only where the names do: an
/// answer that came out right because each repository sat in a directory
/// of its own would say nothing about the rule. That root is this run's
/// own (`demo::claim_root`) — a shared one hands two runs the same five
/// repositories to build and then open.
///
/// `solo` comes last and carries the history, for the reason the ladder's
/// last one does: the tab opened last is the one in front, so that is the
/// page under the strip in the picture.
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

/// The description a verb needs typed for it, when the run brought none.
///
/// The commit editor starts empty, so the verbs that pull its box open
/// have nothing to open it for unless something is in it — and something
/// longer than the pane is tall, so the pull lands on the bound this is
/// about. The details pane needs no such thing: its messages come out of
/// a repository.
pub(super) fn body_for(verb: &str) -> Option<String> {
    match verb {
        "wip-grow" | "wip-grow-squeeze" => Some(crate::demo::pasted(4000)),
        _ => None,
    }
}

/// The one merge tool the candidate list is guaranteed to hold.
///
/// Named here: `git mergetool --tool-help` is an inventory of the
/// machine, and a container built to run tests has no windowed merge
/// tool on it at all.
const SEEDED_TOOL: &str = "demo-editor";

/// What the seeded tool does once it is launched for real: nothing, out
/// loud, for long enough to be waited on.
///
/// `sleep` is the blocking command all three machines agree on — git runs
/// a tool's `cmd` through its own shell, which on Windows is Git for
/// Windows' `sh` and its `/usr/bin/sleep` (measured).
///
/// Two seconds. The verb is a write act, so its one picture is taken
/// behind `AutoActDriver.writeBarrier` and the `treeBarrier` after it
/// — after the tool has exited and the file it resolved has left the
/// conflicted bucket — and no length buys a frame of the wait. What is
/// left for the number to be is a wait a person watching a windowed
/// run can see, against time every `cargo xtask check` pays on both
/// sides.
const TOOL_CMD: &str = "sleep 2";

/// Puts a tool in the repository's own config, for the verbs whose
/// picture is the list of them and the one that hands a file over.
///
/// The candidates arrive in two waves — names written in config, then
/// the installed sweep — and only the first is the run's to decide. Where
/// the second answers with nothing the list is empty, and an empty list
/// is an answer: the card closes itself (`AppCombo.hasList`), leaving the
/// verb waiting on a popup that will not open again. So the run brings
/// its own row, and the first wave carries the picture on every machine.
///
/// For the settings verbs that is the whole of it: the command is never
/// run, and no tool is named as the one to launch — `merge.guitool` would
/// land in the dialog's own field, which is the thing they photograph.
///
/// `open-mergetool` needs the name launchable, so it gets that key and
/// one more. Without `merge.guitool` there is nothing for
/// `conflict::configured_tool` to answer with, the menu row becomes the
/// door to the settings instead, and the verb queues no write. Without
/// `trustExitCode` git asks a closed stdin whether the merge went well,
/// reads EOF, and calls the file failed (measured).
///
/// Whatever repositories the run is about are written to, a `--repo` of
/// one's own included: the run owns them for its length
/// (`claim_resource`), and a merge tool named in one is overwritten.
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

/// The folder a verb needs handed to it, made on the spot.
///
/// The open-refused verbs are the only ones whose subject is a folder no
/// demo repository can be — being one is the whole point — so there is
/// nothing to name with `--preset`. Both sit one level down, so the
/// folder the second try opens at is a real one with the failed pick
/// inside it. Empty for every other verb: nothing is invented for a verb
/// that was simply run without its argument.
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
        // The one repository these verbs' strip has not got: the thirteenth
        // off the same ladder, so the tab that arrives is named like the
        // twelve it arrives among. Built here, because [`for_run`]'s
        // list is what the run opens at startup.
        "tab-open-go" | "tab-open-moved-on" => {
            let repo = crate::demo::create_named("basic", None, TAB_NAMES[12])?;
            Ok(repo.display().to_string())
        }
        // The avatar card's verbs file a picture on their way in, and a
        // path typed by hand names one machine — the census could never
        // record such a line as one to run again. A picture made here is
        // the same run everywhere.
        "avatar-settings" | "avatar-combo" | "avatar-row-lit" | "avatar-remove" => {
            std::fs::create_dir_all(&made).map_err(|e| e.to_string())?;
            let picture = made.join("avatar.png");
            std::fs::write(&picture, tiny_png()).map_err(|e| e.to_string())?;
            Ok(picture.display().to_string())
        }
        _ => Ok(String::new()),
    }
}

/// A 2×2 opaque PNG — teal, gold / gold, teal — enough for the avatar
/// store to decode and file.
fn tiny_png() -> Vec<u8> {
    crate::png::rgba(2, 2, |x, y| {
        if (x + y) % 2 == 0 {
            [0x2a, 0x9d, 0x8f, 0xff]
        } else {
            [0xe9, 0xc4, 0x6a, 0xff]
        }
    })
}

/// The repositories one run opens, in the order their tabs come up.
///
/// Named repositories win outright; otherwise one fresh demo repository
/// per preset. Six verbs bring a whole strip of their own: what they
/// are about is how a strip of that shape lays out, so the shape is
/// the fixture.
pub(super) fn for_run(opts: &super::options::Options) -> Result<Vec<PathBuf>, String> {
    // A named repository and a preset ask for two different fixtures at
    // once, so the run stops here and says which two.
    if !opts.repo.is_empty() && !opts.preset.is_empty() {
        return Err("--repo and --preset name different fixtures: pass one of them".into());
    }
    let badges =
        opts.verb == "badges" || opts.verb == "badges-hover" || opts.verb == "badges-hover-early";
    // And a named repository beside a tab count is the same contradiction
    // with a worse ending: `--repo` wins the list below, so the strip
    // comes up with the repositories that were named — while the act goes
    // on waiting for the count it was given (`WindowBadgeActs` measures
    // nothing until the band carries it). Nobody sees that: the run says
    // nothing at all until the watchdog ends it a whole ceiling later
    // (measured).
    if badges && !opts.repo.is_empty() && band_tab_count(&opts.arg)?.is_some() {
        return Err(
            "badges builds its own strip from the count in its argument: pass one of \
             --repo and `<width>:<tabs>`"
                .into(),
        );
    }
    // Named repositories win outright; otherwise the route decides, and
    // it is the route that knows whether the preset flag is read at all.
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

/// Which fixture a run of this verb stands on.
///
/// **Decided once and read twice** — by the builder above, and by the
/// line the census files the run under, which leaves a `--preset` off
/// only where the route would have built the same thing without it
/// ([`preset_is_the_default`]). A second reading of "does this verb read
/// the flag" is the one that drifts, and what it would cost is a verb
/// recorded under two lines and run twice by every gate that owes it.
enum Route {
    /// A strip off the ladder of names — `None` for as many tabs as the
    /// run's own argument asks for, `Some` where the verb writes the
    /// count out. **The `--preset` flag is not read**: what these verbs
    /// are about is how a strip of that shape lays out, so the shape is
    /// the fixture.
    Strip(Option<&'static str>),
    /// The one strip whose names collide, which the ladder deliberately
    /// has none of.
    Colliding,
    /// A strip with the run's own preset on the tab in front.
    BandStrip(usize),
    /// The one fixture a run opens a **linked** working copy of: the
    /// `nested-copy` preset's copy, which is the only one in these
    /// fixtures that does not sit beside its repository. **The
    /// `--preset` flag is not read**: what the verb is about is the
    /// difference between the two folders, so the shape is the fixture.
    NestedCopy,
    /// One fresh demo repository per preset, [`DEFAULT_PRESET`] where the
    /// run names none.
    Presets,
    /// The same, **each folder named after its preset**: the run is about
    /// the names the operation panel says as the window moves between the
    /// tabs, and repositories all called `repo` say one name on every tab
    /// — a panel still saying the tab it left would read as right.
    NamedPresets,
}

fn route_of(verb: &str, arg: &str) -> Result<Route, String> {
    Ok(match verb {
        // The picker's second subject, and the only run that opens a
        // linked working copy: the folder the picker comes up in is the
        // repository's, which is a claim no fixture whose copies sit
        // beside the repository can be judged on (`Route::NestedCopy`).
        "open-picker" if arg == "copy" => Route::NestedCopy,
        "tab-widths" => Route::Strip(None),
        // Same ladder, eight of them; the argument here names the tab the
        // hand is on, so the count is written out rather than taken from
        // the other verb's default — which is a state name, and the
        // strip this one photographs has no business moving with it.
        "tab-mark" => Route::Strip(Some("8")),
        // A different strip entirely: names that collide, which the
        // ladder above deliberately has none of.
        "tab-name" => Route::Colliding,
        // Same ladder of names, four of them: the order is what these are
        // about, and four differently named tabs say an order a picture
        // can be read for. The argument names two of them.
        "tab-drag" | "tab-hold" => Route::Strip(Some("4")),
        // A strip that has to overflow, against a window these put down
        // on its floor. Twelve, because tabs give their names up only as
        // far as their own floor and stand at whatever the run divides
        // into until then: a count below the one that puts them on that
        // floor fills the run exactly and overflows it by nothing at all
        // (measured, eight tabs on the wider Linux band: `content=627`
        // against `view=627`, where Windows was over by 65). The
        // Linux band is the wider one because it carries neither a grab
        // run nor window buttons. `tab-edge` has nowhere to travel to
        // there, and what `tab-pin` photographs is a stand-in with the
        // strip running underneath it — the stand-in stood either way,
        // `must_say` says so, but the picture was of a strip that had
        // not moved. `tab-open-go` and `tab-open-moved-on` open one more
        // on top of these, and that one is built by [`folder_for`]:
        // everything handed over here is opened at startup, and a tab
        // already in the strip cannot arrive in it.
        "tab-edge" | "tab-pin" | "tab-pin-go" | "tab-open-go" | "tab-open-moved-on" => {
            Route::Strip(Some("12"))
        }
        // The one verb whose fixture is both at once: a strip of named
        // tabs *and* a page with a state on it. Which of the group's
        // three shapes the band lands in is what the two of them settle
        // between them, so the count rides in the argument beside the
        // width (`band_tab_count`) and a run that names none stands on
        // the ordinary one repository.
        "badges" | "badges-hover" | "badges-hover-early" => match band_tab_count(arg)? {
            Some(count) => Route::BandStrip(count),
            None => Route::Presets,
        },
        "ops-repo-pick" => Route::NamedPresets,
        _ => Route::Presets,
    })
}

/// Whether the `--preset` flag as typed asks for exactly the fixture
/// leaving it off would have built.
///
/// Only [`Route::Presets`] and [`Route::NamedPresets`] have that default
/// to compare against, so the answer is the route's; every other route
/// either ignores the flag or reads it for something that has no default
/// of its own, and a run on one of them keeps the words it was typed with.
pub(super) fn preset_is_the_default(verb: &str, arg: &str, presets: &[String]) -> bool {
    matches!(presets, [one] if one == DEFAULT_PRESET)
        && matches!(
            route_of(verb, arg),
            Ok(Route::Presets | Route::NamedPresets)
        )
}

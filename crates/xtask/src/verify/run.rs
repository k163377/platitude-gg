//! The run itself, in the order it happens: what the verb needs made for
//! it, what it is allowed to own while it runs, and then the app
//! ([`super::child`]) and the verdict ([`super::outcome`]).

use std::collections::BTreeSet;

use super::options::parse;
use super::ownership::{claim_dir, claim_resource, fresh_shot_dir};
use super::repos::{body_for, folder_for, seed_merge_tool};
use super::shim::stage_shim;
use super::{child, outcome, repos, seed};
use crate::app_build::{BuildEnv, app_exe};

/// The verbs that bring an old git of their own, so the line is just
/// `verify-ui old-git`: the band's `OLD GIT`, alone or among the rest.
const OLD_GIT_VERBS: [&str; 5] = [
    "old-git",
    "old-git-card",
    "old-git-fold",
    "badges-all",
    "badges-all-hover",
];
/// Likewise a git without Git LFS: a desk's Git for Windows carries it,
/// and the band's `NO LFS` stands only where git cannot run it.
const NO_LFS_VERBS: [&str; 4] = ["no-lfs", "no-lfs-card", "badges-all", "badges-all-hover"];

/// Which gits a run is staged with.
struct Gits<'a> {
    /// What the staged copy answers `--version` with (`--old-git` on
    /// PATH, `--other-git` beside the pictures); empty when neither.
    version: &'a str,
    /// Whether the copy on PATH answers `git lfs` as missing.
    no_lfs: bool,
    /// The PATH the app is handed.
    child_path: std::ffi::OsString,
    /// Where a second git stands.
    other: Option<std::path::PathBuf>,
}

/// The gits a run is staged with. `--old-git` / `--no-lfs` stand one copy
/// on PATH; `--other-git` stands one beside it, reading the same
/// variables — so a run takes one or the other.
fn gits_for<'a>(
    opts: &'a super::options::Options,
    shot_dir: &std::path::Path,
    path: &std::ffi::OsString,
) -> Result<Gits<'a>, String> {
    // Below any minimum this app will ever have.
    let old_git = match opts.old_git.as_str() {
        "" if OLD_GIT_VERBS.contains(&opts.verb.as_str()) => "2.42.0",
        asked => asked,
    };
    let no_lfs = opts.no_lfs || NO_LFS_VERBS.contains(&opts.verb.as_str());
    let on_path = !old_git.is_empty() || no_lfs;
    if on_path && !opts.other_git.is_empty() {
        return Err(
            "--old-git / --no-lfs and --other-git stage one copy of git each; a run takes one"
                .into(),
        );
    }
    if on_path {
        let staged = stage_shim(shot_dir, path)?;
        println!(
            "git for this run: {}{} (real git behind it)",
            if old_git.is_empty() {
                "this machine's"
            } else {
                old_git
            },
            if no_lfs { ", without Git LFS" } else { "" }
        );
        return Ok(Gits {
            version: old_git,
            no_lfs,
            child_path: staged,
            other: None,
        });
    }
    // Likewise the second-git runs, which typed without one would wait
    // out the watchdog for a path never staged. Any version at or above
    // the supported minimum; if the minimum passes it, the shots turn red.
    let asks_for_one = opts.verb == "settings-git-leave"
        || (opts.verb == "settings-git-path" && opts.arg == "other");
    let other = match (opts.other_git.as_str(), asks_for_one) {
        ("", true) => "2.55.0",
        (asked, _) => asked,
    };
    if other.is_empty() {
        return Ok(Gits {
            version: "",
            no_lfs: false,
            child_path: path.clone(),
            other: None,
        });
    }
    let staged = super::shim::stage_other_git(shot_dir)?;
    println!(
        "a second git for this run: {} answering {other}",
        staged.display()
    );
    Ok(Gits {
        version: other,
        no_lfs: false,
        child_path: path.clone(),
        other: Some(staged),
    })
}

/// What this run takes of the machine for as long as it runs — an app
/// and its git, or a release build when it builds (`crate::budget`).
/// Taken before the measurement is announced, by every run: a verb
/// beside another seat's gate is load like any other. Under a gate, the
/// gate's ticket covers it.
fn room_for(opts: &super::options::Options) -> Result<crate::budget::Admitted, String> {
    crate::budget::standalone(
        &crate::tree::workspace_root(),
        if opts.build {
            crate::budget::COMPILE
        } else {
            crate::budget::LIGHT
        },
        // A headless verb is a test, whoever started it: the rank above
        // it is for a window somebody is waiting at (`gui::launch`).
        crate::budget::Rank::Normal,
        &format!("verify-ui {}", opts.verb),
    )
}

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let _room = room_for(&opts)?;
    // waits(measured): the run's own wall clock, said on the `spent` line and judged by nothing
    let whole = std::time::Instant::now();
    let (root, _busy) = crate::still::announced("verify-ui")?;
    let path = crate::qt::path_with_qt()?;
    // waits(measured): what the fixture took, said on the `spent` line and judged by nothing
    let at = std::time::Instant::now();
    let repos = repos::for_run(&opts)?;
    let fixture = at.elapsed();

    // A `--repo` belongs to one verify-ui process at a time: two would mix
    // git writes and can make a false PASS. Fresh presets need no claim —
    // their directory is already this run's.
    let mut claimed = BTreeSet::new();
    let mut resource_claims = Vec::new();
    if !opts.repo.is_empty() {
        for repo in &repos {
            if let Some(claim) = claim_resource(repo, "repository", &mut claimed)? {
                resource_claims.push(claim);
            }
        }
    }

    // Written after the claim above, so a repository this run was handed
    // is owned before it is added to.
    seed_merge_tool(&opts.verb, &repos, &path)?;

    // The build's PATH: the git shim below goes onto the child's PATH
    // only, so the build never sees it.
    // waits(measured): what the build took, said on the `spent` line and judged by nothing
    let at = std::time::Instant::now();
    let exe = app_exe(&root, &path, BuildEnv::default(), opts.build, &[])?;
    let build = at.elapsed();

    // Inside a container this claim guards nothing
    // (rules-refs/app-ui.md「コンテナの run が持つのはその裏のホストのディレクトリ」);
    // the host side claims the mount (`keepsakes::bridge`).
    let shot_dir = match &opts.shot_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            if let Some(claim) = claim_resource(dir, "shot directory", &mut claimed)? {
                resource_claims.push(claim);
            }
            dir.clone()
        }
        None => fresh_shot_dir(&opts.verb)?,
    };

    // The run's own unless one is named; naming one lets a run read what
    // the last one wrote.
    let config_dir = match &opts.config_dir {
        Some(dir) => dir.clone(),
        None => shot_dir.join("config"),
    };
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    if opts.config_dir.is_some()
        && let Some(claim) = claim_resource(&config_dir, "config directory", &mut claimed)?
    {
        resource_claims.push(claim);
    }
    println!("config dir: {}", config_dir.display());
    seed::config(&config_dir, &opts.verb, &opts.arg, &opts.preset)?;

    let arg = match opts.arg.is_empty() {
        true => match body_for(&opts.verb) {
            Some(body) => body,
            None => folder_for(&opts.verb, &shot_dir, &path)?,
        },
        // The seed is the whole of this one (`seed::config`): the field
        // is never typed into.
        false if opts.verb == "settings-git-path" && opts.arg == "stored-missing" => String::new(),
        false => opts.arg.clone(),
    };

    let opened = repos
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "running: {} (arg: {}) against {}",
        opts.verb,
        if arg.is_empty() { "-" } else { &arg },
        if opts.restore {
            "the tabs it remembers"
        } else {
            &opened
        }
    );
    let gits = gits_for(&opts, &shot_dir, &path)?;

    // Git reads "who is sitting here" from a config file and from the
    // directory it runs in, so every run gets both of its own: a gitconfig
    // (`shim::global_seed`, which the identity verbs write to) and a
    // working directory outside every checkout. Without the directory, a
    // container's checkout `.git` names a Windows path git calls fatal.
    let config = gitconfig_home(
        std::env::var_os(crate::linux::IN_CONTAINER).is_some(),
        &shot_dir,
    )?
    .join("gitconfig");
    std::fs::write(&config, super::shim::global_seed(&opts.verb)).map_err(|e| e.to_string())?;
    println!("git config for this run: {}", config.display());

    // Who was already running, taken at the last moment before the app
    // exists (`ownership::give_back_claimed`).
    let beside = crate::reap::others_in_this_group();

    let ran = child::run_app(&child::Start {
        exe: &exe,
        shot_dir: &shot_dir,
        config_dir: &config_dir,
        config: &config,
        child_path: &gits.child_path,
        path: &path,
        arg: &arg,
        shim_version: gits.version,
        shim_no_lfs: gits.no_lfs,
        other_git: gits.other.as_deref(),
        repos: &repos,
        opts: &opts,
    })?;
    let verdict = outcome::judge(&opts, &ran, &config);
    let census = if verdict.passed() {
        settle_a_pass(&root, &opts, &ran, beside.as_deref())
    } else {
        None
    };
    say_what_it_spent(whole, fixture, build, ran.elapsed);
    outcome::announce(&opts, &shot_dir, &ran, &verdict, census.as_deref())
}

/// What a passed run still owes: its census line, then the scaffolding
/// it stood up. Answers why the line was not written, which fails the
/// run — else a gate would choose off a stale census.
fn settle_a_pass(
    root: &std::path::Path,
    opts: &super::options::Options,
    ran: &child::Ran,
    beside: Option<&[u32]>,
) -> Option<String> {
    if let Err(why) = tell_the_census(root, opts.census_line(), &ran.err_lines) {
        return Some(why);
    }
    let gone = super::ownership::give_back_claimed(beside);
    if gone > 0 {
        println!("demo repositories given back: {gone}");
    }
    None
}

/// The three parts of a run worth telling apart, and the rest — one
/// line the gate's ledger reads back off this log (`gate::record`).
///
/// Separate because a block of verbs shares one release, built by
/// whichever uncached verb came first (`gate::sides::verbs`), and a
/// preset's first run builds its template (`demo::template`): by wall
/// clock those verbs would rank as the expensive ones. `app` is what the
/// verb itself costs. `rest` (seeds, shim, judging, census, pictures) is
/// not split — nobody measured its parts.
fn say_what_it_spent(
    whole: std::time::Instant,
    fixture: std::time::Duration,
    build: std::time::Duration,
    app: std::time::Duration,
) {
    let whole = whole.elapsed();
    let rest = whole
        .saturating_sub(fixture)
        .saturating_sub(build)
        .saturating_sub(app);
    println!(
        "spent fixture={}ms build={}ms app={}ms rest={}ms whole={}ms",
        fixture.as_millis(),
        build.as_millis(),
        app.as_millis(),
        rest.as_millis(),
        whole.as_millis(),
    );
}

/// A passing run tells the census what it showed, so the gate picks this
/// line when one of those components changes (`gate::census`). Only a
/// run somebody can type again is recorded (`line`, from
/// `Options::census_line`); only one whose page had stopped arriving
/// writes its line, the rest add to it.
///
/// An error is a line owed and not written: the run never said what it
/// showed, or the file could not be read, held or written. A run that
/// owes no line (not typable again, or a twin) answers `Ok` whatever it
/// said.
fn tell_the_census(
    root: &std::path::Path,
    line: Option<String>,
    said: &[String],
) -> Result<(), String> {
    let Some(line) = line else {
        return Ok(());
    };
    // No gate owes a twin; recording it would bring it back for `verbs`
    // to refuse (`gate::tiers`).
    if let Some(of) = crate::gate::Tiers::load(root)?.twin_of(&line) {
        println!("census: {line} — not recorded, the tier table says it is the same run as {of}");
        return Ok(());
    }
    // Every run that ends on its picture walks the window first
    // (`AutoShotDriver.waitsForCensus`), so a pass with no names is a
    // harness that stopped saying them.
    let names =
        crate::gate::names_in(said).ok_or_else(|| format!("{line}: never said `census=`"))?;
    let page_settled = crate::gate::page_settled_in(said);
    let (count, shift) = crate::gate::record(root, &line, &names, page_settled)
        .map_err(|why| format!("{line}: {why}"))?;
    // The names the write moved: when a component came or went the
    // file's diff is every line, so the run says which.
    if page_settled {
        println!(
            "census: {line} — {count} component(s) recorded{}",
            shift.said_for(&line)
        );
    } else {
        println!(
            "census: {line} — {count} component(s), added to: the page was still arriving{}",
            shift.said_for(&line)
        );
    }
    Ok(())
}

/// Where a run's gitconfig stands: beside its pictures, except inside a
/// container, where it goes to a leaf of the container's `/tmp`.
///
/// A rename on the `/out` mount is not atomic: git replaces its config by
/// renaming a lock over it, and a reader past `access(2)` then opens
/// nothing — `repo_read_config` dies with `unknown error occurred while
/// reading the configuration files`. The identity verbs rewrite this file
/// while `status` and `for-each-ref` read it. Pictures and settings stay
/// on `/out`: only the app writes those, and it reads back what it
/// finished writing.
fn gitconfig_home(
    in_container: bool,
    shot_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    if !in_container {
        return Ok(shot_dir.to_path_buf());
    }
    claim_dir(&std::env::temp_dir().join("pgg-verify"), "gitconfig")
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_gitconfig_leaves_the_mount_inside_a_container() {
        let shot_dir = std::path::Path::new("/out/shots-1");
        assert_eq!(
            super::gitconfig_home(false, shot_dir).expect("beside the pictures"),
            shot_dir
        );
        let own = super::gitconfig_home(true, shot_dir).expect("a leaf of its own");
        assert!(
            !own.starts_with(shot_dir),
            "{} is under the mount",
            own.display()
        );
        assert!(own.starts_with(std::env::temp_dir().join("pgg-verify")));
        let _ = std::fs::remove_dir_all(&own);
    }

    /// A tree of one test's own, holding one component and no census yet.
    fn tree() -> std::path::PathBuf {
        let root = super::claim_dir(&std::env::temp_dir().join("pgg-census"), "told")
            .expect("a root of this test's own");
        let ui = root.join("crates/platitude-app/src/ui");
        std::fs::create_dir_all(&ui).expect("ui dir");
        std::fs::create_dir_all(root.join("crates/xtask")).expect("xtask dir");
        std::fs::write(ui.join("WipPane.qml"), "Item {}\n").expect("component");
        // A table with no rows: a tree without one is refused (`gate::tiers`).
        std::fs::write(root.join("crates/xtask/verb-tiers.txt"), "# no rows\n")
            .expect("the tier table");
        root
    }

    /// What a run that walked the window says at its end.
    fn walked() -> Vec<String> {
        [
            "INFO bench: census page=settled",
            "INFO bench: census=WipPane",
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn a_line_owed_and_not_written_is_an_error_and_a_run_that_owes_none_is_not() {
        let root = tree();
        let census = root.join(crate::gate::CENSUS_FILE);
        let silent = ["INFO bench: auto_act complete=wip".to_string()];
        let Err(why) = super::tell_the_census(&root, Some("wip".into()), &silent) else {
            panic!("a run that never said what it showed owed its line nothing");
        };
        assert!(why.contains("census="), "{why}");
        assert!(!census.exists(), "nothing was written");
        // `--no-census`, the container, a `--repo`: no line anybody can
        // type again.
        super::tell_the_census(&root, None, &silent).expect("a run that owes no line");

        let staging = root.join("target").join("verb-census.txt.part");
        std::fs::create_dir_all(&staging).expect("a directory where the staging file goes");
        let Err(why) = super::tell_the_census(&root, Some("wip".into()), &walked()) else {
            panic!("a line with nowhere to stage it was written");
        };
        assert!(why.contains("verb-census.txt.part"), "{why}");
        assert!(!census.exists(), "nothing was written");

        std::fs::remove_dir(&staging).expect("the directory gone");
        super::tell_the_census(&root, Some("wip".into()), &walked()).expect("the line written");
        let text = std::fs::read_to_string(&census).expect("the census");
        assert!(text.contains("\nwip\tWipPane\n"), "{text}");
    }

    #[test]
    fn a_twin_owes_no_line() {
        let root = tree();
        std::fs::write(
            root.join("crates/xtask/verb-tiers.txt"),
            "twin\twip again\twip\tthe same run as wip\n",
        )
        .expect("the tier table");
        super::tell_the_census(&root, Some("wip again".into()), &[]).expect("said nothing");
        super::tell_the_census(&root, Some("wip again".into()), &walked()).expect("said it");
        assert!(!root.join(crate::gate::CENSUS_FILE).exists());
    }
}

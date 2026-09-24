//! The run itself, in the order it happens: what the verb needs made for
//! it, what it is allowed to own while it runs, and then the app
//! ([`super::child`]) and the verdict ([`super::outcome`]).

use std::collections::BTreeSet;

use super::options::parse;
use super::ownership::{claim_dir, claim_resource, fresh_shot_dir};
use super::repos::{body_for, folder_for, seed_merge_tool};
use super::shim::stage_old_git;
use super::{child, outcome, repos, seed};

/// Which gits a run is staged with: the version an old one answers, the
/// PATH the app is handed, and where a second one stands.
///
/// **A run asks for one of the two** — the copy that stands in for git
/// reads one pair of variables to know what it is, and two of them on
/// one app would be the same copy told two things.
fn gits_for<'a>(
    opts: &'a super::options::Options,
    shot_dir: &std::path::Path,
    path: &std::ffi::OsString,
) -> Result<(&'a str, std::ffi::OsString, Option<std::path::PathBuf>), String> {
    // The three verbs named for it bring their own version, so that the
    // run reads `verify-ui old-git` and nothing else. Below any minimum
    // this app will ever have: minimums only go up.
    let old_git = match (opts.old_git.as_str(), opts.verb.as_str()) {
        ("", "old-git" | "old-git-card" | "old-git-fold") => "2.42.0",
        (asked, _) => asked,
    };
    if !old_git.is_empty() && !opts.other_git.is_empty() {
        return Err("--old-git and --other-git are one git each; a run takes one".into());
    }
    if !old_git.is_empty() {
        let staged = stage_old_git(shot_dir, path)?;
        println!("git for this run: {old_git} (real git behind it)");
        return Ok((old_git, staged, None));
    }
    // The runs whose subject is a second git bring their own version, so
    // that the line reads `verify-ui settings-git-leave` and nothing else
    // — and so a run typed without one waits out its watchdog for a path
    // that was never staged. A plain modern number: the shim only prints
    // it, and all it has to be is at or above the supported minimum. If a
    // minimum ever passes it the shots turn red and this moves up.
    let asks_for_one = opts.verb == "settings-git-leave"
        || (opts.verb == "settings-git-path" && opts.arg == "other");
    let other = match (opts.other_git.as_str(), asks_for_one) {
        ("", true) => "2.55.0",
        (asked, _) => asked,
    };
    if other.is_empty() {
        return Ok((old_git, path.clone(), None));
    }
    // A second git, standing where nothing resolves to it, so a run can
    // point the settings box at a git that answers and is not the one it
    // is running.
    let staged = super::shim::stage_other_git(shot_dir)?;
    println!(
        "a second git for this run: {} answering {other}",
        staged.display()
    );
    Ok((other, path.clone(), Some(staged)))
}

/// What this run takes of the machine, held for as long as it runs: one
/// verb is an app, its offscreen raster and the git it spawns — or a
/// release build, when it is the one that builds (`crate::budget`).
///
/// Taken before the measurement is announced, which is the order the
/// whole runner keeps, and taken by a run of any kind: a verb a
/// session runs beside another seat's gate is load like any other. A
/// verb the gate started is under the gate's own ticket and takes
/// none.
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

    // Explicit resources survive across sequential runs, and belong to
    // one verify-ui process at a time. Two at once would mix Git
    // writes, settings, or PNGs and can manufacture a false PASS.
    // Fresh preset repositories need no cross-process claim: their creator
    // already gave this run a private directory.
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
    let exe = crate::tree::app_exe(&root, &path, opts.build, &[])?;
    let build = at.elapsed();

    // The claim below answers for this machine only, and inside a
    // container that machine is one run wide: `/out` is the same path in
    // every one of them and the lock goes to a `/tmp` nobody else can
    // see. What two container runs collide over is the host directory
    // behind the mount, which is claimed out there (`keepsakes::bridge`).
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

    // A run of its own unless told otherwise. The app would refuse the
    // real files anyway once it sees a PGG_* variable, but naming a
    // directory is what lets one run read what the last one wrote — and
    // what lets anyone look at the two files afterwards.
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
        // The seed is the whole of this one: the field is never typed
        // into, so the app is handed the same nothing a bare run gets
        // and photographs what the store put on screen (`seed::config`).
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
    let (shim_version, child_path, other_git) = gits_for(&opts, &shot_dir, &path)?;

    // The picture is the same whatever machine it was taken on, and git
    // takes its answer to "who is sitting here" from two places the run
    // would otherwise inherit: a configuration file, and the directory it
    // resolves one from. So every run is handed both of its own — a
    // gitconfig carrying the fixture identity (`shim::global_seed`), and a
    // working directory outside every checkout to read it in.
    //
    // Without the file a machine with no global `user.*` — which is every
    // container — puts the first-run modal over the window on the way in,
    // and the two popups it brings are counted by verbs that have nothing
    // to do with an identity. Without the directory the tree this ran from
    // gets a say, and inside a container that tree's `.git` names a
    // Windows path git calls fatal rather than absent.
    //
    // The identity verbs are the ones whose write lands in this
    // gitconfig; they bring their own seed and answer the screen it
    // raises, and go through the same door as everyone else.
    let config = gitconfig_home(
        std::env::var_os(crate::linux::IN_CONTAINER).is_some(),
        &shot_dir,
    )?
    .join("gitconfig");
    std::fs::write(&config, super::shim::global_seed(&opts.verb)).map_err(|e| e.to_string())?;
    println!("git config for this run: {}", config.display());

    // Who was already running beside this run, so that what the app
    // adds can be told from what was there
    // (`ownership::give_back_claimed`). Taken here, which is the last
    // moment before the app exists.
    let beside = crate::reap::others_in_this_group();

    let ran = child::run_app(&child::Start {
        exe: &exe,
        shot_dir: &shot_dir,
        config_dir: &config_dir,
        config: &config,
        child_path: &child_path,
        path: &path,
        arg: &arg,
        shim_version,
        other_git: other_git.as_deref(),
        repos: &repos,
        opts: &opts,
    })?;
    let verdict = outcome::judge(&opts, &ran, &config);
    if verdict.passed() {
        tell_the_census(&root, &opts, &ran);
        // The scaffolding goes back where the run stood it up, and only
        // on the road where nobody will want to look at it again
        // (`ownership::give_back_claimed`). A failing run keeps its
        // tree: in a container that volume is the only place the scene
        // survives.
        let gone = super::ownership::give_back_claimed(beside.as_deref());
        if gone > 0 {
            println!("demo repositories given back: {gone}");
        }
    }
    say_what_it_spent(whole, fixture, build, ran.elapsed);
    outcome::announce(&opts, &shot_dir, &ran, &verdict)
}

/// The three parts of a run that are worth telling apart, and what is
/// left over — said in one line the gate's ledger reads back off this
/// log (`gate::record`).
///
/// **Why they are separate.** A block of verbs shares one release: the
/// first uncached verb builds it and the rest are handed `--no-build`
/// (`gate::verbs`), so a verb's wall clock is that build plus the verb
/// wherever it happened to come first. Ranked by wall clock, the verb
/// that built would read as the expensive one, and the thing to look at
/// would be whichever verb the ordering put in front. `fixture` is the
/// repositories the run stands on — built once per preset and copied
/// after that (`demo::template`), so the first run of a preset carries
/// the building of it. `app` is the window itself, which is what the
/// verb costs when nothing else is on its bill.
///
/// **`rest` is named, not split.** It is the seeds, the git shim, the
/// judging, the census write and the pictures — a handful of small
/// things, and calling any one of them out would be claiming a
/// measurement nobody took.
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

/// A passing run tells the census what it showed, so the gate can pick
/// this line by itself the next time one of those components changes
/// (`gate::census`). Only a run somebody can type again is recorded, and
/// only one whose page had stopped arriving writes its line — the rest
/// add to it, having seen whichever rows the reads had brought.
fn tell_the_census(root: &std::path::Path, opts: &super::options::Options, ran: &child::Ran) {
    let (Some(line), Some(names)) = (opts.census_line(), crate::gate::names_in(&ran.err_lines))
    else {
        return;
    };
    let page_settled = crate::gate::page_settled_in(&ran.err_lines);
    match crate::gate::record(root, &line, &names, page_settled) {
        // The names the write moved, beside the count: the file's own
        // diff is every line when a component came or went, and this is
        // the run saying which name that was and whether the line it was
        // about is the one that moved.
        Ok((count, shift)) if page_settled => println!(
            "census: {line} — {count} component(s) recorded{}",
            shift.said_for(&line)
        ),
        Ok((count, shift)) => println!(
            "census: {line} — {count} component(s), added to: the page was still arriving{}",
            shift.said_for(&line)
        ),
        Err(why) => println!("census: not recorded ({why})"),
    }
}

/// Where a run's gitconfig stands: beside its pictures, except inside a
/// container, where the pictures' directory is the host's.
///
/// **A rename on the mount is not a rename.** `/out` is a directory of
/// the Windows host, and git replaces the configuration file it writes
/// by renaming its lock over it. On a disk of the container's own that
/// rename is atomic, and a reader that has just passed `access(2)` opens
/// the file it was promised. On the mount there is a moment with no file
/// at that name (measured: one read in every few dozen on `/out` opens
/// nothing after `access` said yes, and none on `/tmp`), and git's reading
/// side takes that `ENOENT` for a file that was never there — a `-1`
/// from a read it had checked, which `repo_read_config` dies on with
/// `unknown error occurred while reading the configuration files`. The
/// identity verbs rewrite this file while the session's own `status` and
/// `for-each-ref` read it (measured: a `verify-linux identity-tip` whose
/// tab-open `status` and `refs` both died on it, and which then waited
/// out its whole ceiling for a page that never came).
///
/// So inside a container the file stands on the container's `/tmp`, in a
/// leaf of this run's own. The pictures and the settings stay under
/// `/out`: the app is the one process writing those, and it reads back
/// what it finished writing.
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
    /// Outside a container the gitconfig stands beside the pictures;
    /// inside one it stands on the container's own disk, never under the
    /// mount the pictures go out through.
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
}

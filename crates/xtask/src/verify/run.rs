//! The run itself, in the order it happens: what the verb needs made for
//! it, what it is allowed to own while it runs, and then the app
//! ([`super::child`]) and the verdict ([`super::outcome`]).

use std::collections::BTreeSet;

use super::options::parse;
use super::ownership::{claim_resource, fresh_shot_dir};
use super::repos::{body_for, folder_for, seed_merge_tool};
use super::shim::stage_old_git;
use super::{child, outcome, repos, seed};

/// Which gits a run is staged with: the version an old one answers, the
/// PATH the app is handed, and where a second one stands.
///
/// **A run asks for one of the two, never both** — the copy that stands in
/// for git reads one pair of variables to know what it is, and two of them
/// on one app would be the same copy told two things.
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

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let (root, _busy) = crate::still::announced("verify-ui")?;
    let path = crate::qt::path_with_qt()?;
    let repos = repos::for_run(&opts)?;

    // Explicit resources are allowed to survive across sequential runs,
    // but never to be owned by two verify-ui processes at once. That would
    // mix Git writes, settings, or PNGs and can manufacture a false PASS.
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

    // The build's PATH, not the run's: the git shim below goes onto the
    // child's PATH only, so the build never sees it.
    let exe = crate::tree::app_exe(&root, &path, opts.build, &[])?;

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
    // real files anyway once it sees a PG_* variable, but naming a
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
    seed::config(&config_dir, &opts.verb)?;

    let arg = match opts.arg.is_empty() {
        true => match body_for(&opts.verb) {
            Some(body) => body,
            None => folder_for(&opts.verb, &shot_dir, &path)?,
        },
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

    // Which machine a run happens on must not reach the picture, and git
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
    // The identity verbs are the ones whose write would land here rather
    // than in a demo repository; they bring their own seed and answer the
    // screen it raises, and go through the same door as everyone else.
    let config = shot_dir.join("gitconfig");
    std::fs::write(&config, super::shim::global_seed(&opts.verb)).map_err(|e| e.to_string())?;
    println!("git config for this run: {}", config.display());

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
    let verdict = outcome::judge(&opts, &ran);
    // A passing run tells the census what it showed, so the gate can pick
    // this line by itself the next time one of those components changes
    // (gate::census). Only a run somebody can type again is recorded, and
    // only one whose page had stopped arriving writes its line — the rest
    // add to it, having seen whichever rows the reads had brought.
    if verdict.passed()
        && let Some(line) = opts.census_line()
        && let Some(names) = crate::gate::names_in(&ran.err_lines)
    {
        let page_settled = crate::gate::page_settled_in(&ran.err_lines);
        match crate::gate::record(&root, &line, &names, page_settled) {
            Ok(count) if page_settled => println!("census: {line} — {count} component(s) recorded"),
            Ok(count) => println!(
                "census: {line} — {count} component(s), added to: the page was still arriving"
            ),
            Err(why) => println!("census: not recorded ({why})"),
        }
    }
    outcome::announce(&opts, &shot_dir, &ran, &verdict)
}

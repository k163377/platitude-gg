//! The run itself, in the order it happens: what the verb needs made for
//! it, what it is allowed to own while it runs, and then the app
//! ([`super::child`]) and the verdict ([`super::outcome`]).

use std::collections::BTreeSet;

use super::options::parse;
use super::ownership::{claim_resource, fresh_shot_dir};
use super::repos::{body_for, folder_for, seed_merge_tool};
use super::shim::stage_old_git;
use super::{child, outcome, repos, seed};

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = crate::tree::workspace_root();
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
    // The three verbs named for it bring their own version, so that the
    // run reads `verify-ui old-git` and nothing else. Below any minimum
    // this app will ever have: minimums only go up.
    let old_git = match (opts.old_git.as_str(), opts.verb.as_str()) {
        ("", "old-git" | "old-git-card" | "old-git-fold") => "2.42.0",
        (asked, _) => asked,
    };
    let child_path = if old_git.is_empty() {
        path.clone()
    } else {
        let staged = stage_old_git(&shot_dir, &path)?;
        println!("git for this run: {old_git} (real git behind it)");
        staged
    };

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
        old_git,
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

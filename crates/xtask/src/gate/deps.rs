//! `cargo xtask gate deps` — the dependency graph, asked directly: what a
//! change reaches, how a file got into that reach, and what the last N
//! commits would have owed. The diagnostics behind the gate, for when a
//! selection looks wrong.

use std::collections::BTreeSet;

use super::graph::{self, Carried, Graph};
use crate::subprocess::git_query;

struct Options {
    history: Option<usize>,
    show: bool,
    files: Vec<String>,
    why: Vec<String>,
    main_ref: String,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        history: None,
        show: false,
        files: Vec::new(),
        why: Vec::new(),
        main_ref: "main".to_string(),
    };
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--history" => {
                at += 1;
                opts.history = Some(
                    args.get(at)
                        .ok_or("--history needs a count")?
                        .parse()
                        .map_err(|e| format!("--history: {e}"))?,
                );
            }
            "--file" => {
                at += 1;
                opts.files
                    .push(args.get(at).ok_or("--file needs a path")?.clone());
            }
            "--why" => {
                at += 1;
                opts.why
                    .push(args.get(at).ok_or("--why needs a path")?.clone());
            }
            "--main" => {
                at += 1;
                opts.main_ref = args.get(at).ok_or("--main needs a ref")?.clone();
            }
            "--show" => opts.show = true,
            other => {
                return Err(format!(
                    "unknown option {other:?} (deps takes --file <path>…, --why <path>…, \
                     --history <n>, --show, --main <ref>)"
                ));
            }
        }
        at += 1;
    }
    Ok(opts)
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let opts = options(args)?;
    let root = crate::tree::workspace_root();
    let here = root.display().to_string();
    // waits(measured): the graph's build time, printed beside its size
    let started = std::time::Instant::now();
    let g = graph::build(&root)?;
    let edges: usize = g.deps.values().map(BTreeSet::len).sum();
    println!(
        "graph: {} rust modules, {} nodes, {} edges, {} unresolved paths ({} ms)",
        g.modules.len(),
        g.deps.len(),
        edges,
        g.unresolved.len(),
        started.elapsed().as_millis()
    );
    for (file, path) in g.unresolved.iter().take(20) {
        println!("  unresolved: {file}: {path}");
    }
    if let Some(count) = opts.history {
        return history(&here, &g, &opts.main_ref, count);
    }
    let changed = if opts.files.is_empty() {
        branch_changes(&here, &opts.main_ref)?
    } else {
        opts.files
    };
    probe(&g, &changed, &opts.why, opts.show);
    Ok(())
}

/// The files `base..head` touched, quotepath off so a non-ASCII name
/// comes back as itself.
fn changed_between(here: &str, base: &str, head: &str) -> Vec<String> {
    git_query(
        here,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-only",
            base,
            head,
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter(|l| !l.is_empty())
    .map(str::to_string)
    .collect()
}

/// The branch's own diff against main.
fn branch_changes(here: &str, main_ref: &str) -> Result<Vec<String>, String> {
    let head = git_query(here, &["rev-parse", "HEAD"]).ok_or("no HEAD")?;
    let main = git_query(here, &["rev-parse", main_ref]).ok_or("no main")?;
    let base = git_query(here, &["merge-base", &main, &head]).ok_or("no merge base")?;
    Ok(changed_between(here, &base, &head))
}

/// One line per commit of the last `count` on main: how much it changed
/// and how far that reached, in today's graph.
fn history(here: &str, g: &Graph, main_ref: &str, count: usize) -> Result<(), String> {
    let listing = git_query(here, &["rev-list", "-n", &count.to_string(), main_ref])
        .ok_or("git rev-list failed")?;
    println!("{:<10} {:>4} {:>5}  subject", "commit", "chg", "reach");
    for sha in listing.lines() {
        let changed = changed_between(here, &format!("{sha}~1"), sha);
        let subject = git_query(here, &["log", "-1", "--format=%s", sha]).unwrap_or_default();
        let reach = g.reach(&changed);
        println!(
            "{:<10} {:>4} {:>5}  {}",
            &sha[..10],
            changed.len(),
            reach.len(),
            subject.chars().take(60).collect::<String>()
        );
    }
    Ok(())
}

/// What `changed` reaches, how each `why` file got there, and — shown —
/// every file of the reach with the changed ones' readers and reads.
fn probe(g: &Graph, changed: &[String], why: &[String], show: bool) {
    let reach = g.reach(changed);
    println!(
        "changed: {} files; reach: {} files",
        changed.len(),
        reach.len()
    );
    // A file a change reaches as data did not change (`graph::Carried`);
    // the plan selects by the difference.
    let as_data = |file: &String| match reach.get(file) {
        Some(Carried::AsProductFile) => " (as a product file the tool reads)",
        Some(Carried::AsData) => " (as data its reader takes)",
        _ => "",
    };
    for target in why {
        match g.why(changed, target) {
            Some(chain) => println!("  why {target}: {}{}", chain.join(" -> "), as_data(target)),
            None => println!("  why {target}: not in reach"),
        }
    }
    if !show {
        return;
    }
    let names = |set: Option<&BTreeSet<String>>| -> String {
        set.map(|s| s.iter().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default()
    };
    for file in changed {
        println!("  {file} is read by: {}", names(g.rdeps.get(file)));
        println!("  {file} reads: {}", names(g.deps.get(file)));
    }
    for file in reach.keys() {
        println!(
            "  {}{file}{}",
            if changed.contains(file) { "* " } else { "  " },
            as_data(file)
        );
    }
}

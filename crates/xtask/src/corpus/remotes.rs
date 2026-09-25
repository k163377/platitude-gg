//! The remotes the corpus is configured with, as bare repositories
//! inside it.
//!
//! **Without a remote, half the session never runs**: the application
//! skips its fetch (`session::auto_fetch::known_to_have_no_remote`), so
//! `session::RemoteTagIndex` — megabytes at this size
//! (ci/baseline/code-costs-windows-x64.md §メモリの形) — is built from
//! nothing and no branch has an upstream.
//!
//! A remote is a bare repository holding refs only, reading the corpus's
//! objects through `objects/info/alternates`, so `ls-remote` and `fetch`
//! answer offline. Each mirror holds exactly the refs the corpus already
//! tracks for it, so the application's `fetch --prune` writes nothing: a
//! benchmark repository must not drift under its own measurement, which
//! `proven` checks.

use std::path::{Path, PathBuf};

use super::{git, shape};

/// Under the corpus's `.git`: out of `git status`'s sight, and cleared
/// with the corpus.
const MIRRORS: &str = "pgg-remotes";

/// Builds one bare mirror per remote, configures the corpus to use them,
/// and puts `main` ahead of the first.
pub(super) fn configure(at: &Path) -> Result<(), String> {
    let listing = git(at, &["show-ref"])?;
    for name in shape::REMOTES {
        let mirror = mirror_path(at, name);
        build_mirror(&mirror, at, name, &listing)?;
        git(at, &["remote", "add", name, &shown(&mirror)])?;
    }
    upstream(at)?;
    default_heads(at)?;
    proven(at)
}

/// The `refs/remotes/<name>/HEAD` a clone carries. Written here because
/// otherwise the first fetch against a remote with a symbolic HEAD
/// writes it — during the measurement itself.
fn default_heads(at: &Path) -> Result<(), String> {
    for name in shape::REMOTES {
        git(at, &["remote", "set-head", name, "--auto"])?;
    }
    Ok(())
}

/// A path as git takes it, whichever way this platform spells one.
fn shown(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn mirror_path(at: &Path, name: &str) -> PathBuf {
    at.join(".git").join(MIRRORS).join(format!("{name}.git"))
}

fn build_mirror(mirror: &Path, at: &Path, name: &str, listing: &str) -> Result<(), String> {
    std::fs::create_dir_all(mirror)
        .map_err(|e| format!("could not make {}: {e}", mirror.display()))?;
    // Pinned rather than taken from the machine's `init.*` config: it has
    // to be one corpus on every machine.
    git(
        mirror,
        &[
            "init",
            "--bare",
            "--quiet",
            "--object-format=sha1",
            "--initial-branch=main",
        ],
    )?;
    // Borrowed, not copied: a copy doubles the pack that is most of the
    // corpus's weight.
    let alternates = mirror.join("objects").join("info").join("alternates");
    let objects = shown(&at.join(".git").join("objects"));
    std::fs::write(&alternates, format!("{objects}\n")).map_err(|e| {
        format!(
            "could not point {} at the corpus: {e}",
            alternates.display()
        )
    })?;
    let refs = carried(name, listing);
    let packed = mirror.join("packed-refs");
    std::fs::write(&packed, &refs)
        .map_err(|e| format!("could not write {}: {e}", packed.display()))?;
    // HEAD has to name a branch the mirror holds: `remote set-head
    // --auto` cannot answer for a remote whose HEAD dangles.
    let first = refs
        .lines()
        .find_map(|line| line.split_once(" refs/heads/"))
        .map(|(_, branch)| branch.to_string())
        .ok_or_else(|| format!("the {name} mirror holds no branch to point HEAD at"))?;
    git(
        mirror,
        &["symbolic-ref", "HEAD", &format!("refs/heads/{first}")],
    )?;
    Ok(())
}

/// What a mirror answers with, as a `packed-refs` file: every branch the
/// corpus tracks for this remote under the remote's own name for it, and
/// — on the first remote only, as in the reference repository — the tags.
///
/// Written as one file: `update-ref --stdin` over fifty thousand names
/// makes as many loose refs for `pack-refs` to read back.
fn carried(name: &str, listing: &str) -> String {
    let tracked = format!("refs/remotes/{name}/");
    let mut lines: Vec<String> = Vec::new();
    for line in listing.lines() {
        let Some((oid, refname)) = line.split_once(' ') else {
            continue;
        };
        if let Some(branch) = refname.strip_prefix(&tracked) {
            // Carried back, the tracking `HEAD` would advertise as
            // `refs/heads/HEAD`.
            if branch == "HEAD" {
                continue;
            }
            lines.push(format!("{oid} refs/heads/{branch}"));
        } else if name == shape::REMOTES[0] && refname.starts_with("refs/tags/") {
            lines.push(format!("{oid} {refname}"));
        }
    }
    // git reads packed-refs in order. No trait line: with no peel lines,
    // git peels on demand.
    lines.sort_by(|a, b| a[41..].cmp(&b[41..]));
    let mut packed = String::with_capacity(lines.len() * 64);
    for line in lines {
        packed.push_str(&line);
        packed.push('\n');
    }
    packed
}

/// `main` tracks the first remote, and sits `shape::AHEAD_OF_UPSTREAM`
/// ahead of it.
fn upstream(at: &Path) -> Result<(), String> {
    let remote = shape::REMOTES[0];
    let behind = format!("main~{}", shape::AHEAD_OF_UPSTREAM);
    let oid = git(at, &["rev-parse", &behind])?;
    let oid = oid.trim();
    let tracking = format!("refs/remotes/{remote}/main");
    git(at, &["update-ref", &tracking, oid])?;
    let mirror = mirror_path(at, remote);
    git(&mirror, &["update-ref", "refs/heads/main", oid])?;
    // A fork's HEAD names `main`, and `remote set-head --auto` reads it.
    git(&mirror, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    git(at, &["config", "branch.main.remote", remote])?;
    git(at, &["config", "branch.main.merge", "refs/heads/main"])?;
    Ok(())
}

/// Proves at build time that the remotes answer — an unreadable mirror
/// fails silently in the application (`read_remote_tags` logs and moves
/// on with no readings) — and that the application's own fetch moves no
/// ref.
fn proven(at: &Path) -> Result<(), String> {
    // waits(measured): the phase times this proof says beside its verdict, judged by
    // nothing
    let mut began = std::time::Instant::now();
    let mut took = |what: &str| {
        // waits(measured): the phase's end, for the same line
        let now = std::time::Instant::now();
        let seconds = (now - began).as_secs_f64();
        began = now;
        format!("{what} {seconds:.0}s")
    };
    let before = git(at, &["show-ref"])?;
    let listing = took("show-ref");
    let listed = git(at, &["ls-remote", "--tags", "--", shape::REMOTES[0]])?;
    let advertising = took("ls-remote");
    // An annotated tag is advertised twice, as itself and peeled; the
    // count is of names.
    let mut names = std::collections::BTreeSet::new();
    let mut peeled = 0;
    for line in listed.lines().filter(|line| !line.is_empty()) {
        let Some((_, refname)) = line.split_once('\t') else {
            continue;
        };
        match refname.strip_suffix("^{}") {
            Some(base) => {
                peeled += 1;
                names.insert(base.to_string());
            }
            None => {
                names.insert(refname.to_string());
            }
        }
    }
    let answered = names.len() as u64;
    if answered != shape::TAGS {
        return Err(format!(
            "the {} mirror answered {answered} tags, where the corpus carries {} — the index the \
             application builds from this would be the wrong size",
            shape::REMOTES[0],
            shape::TAGS
        ));
    }
    if peeled * 1_000 / shape::TAGS < shape::ANNOTATED_SHARE - 5 {
        return Err(format!(
            "only {peeled} of {} tags advertised a peeled line — an annotated tag costs a peel \
             and a lightweight one does not, and the reference repository's are 99.3% annotated",
            shape::TAGS
        ));
    }
    // `--all`, as the application runs it.
    git(at, &["fetch", "--prune", "--all"])?;
    let fetching = took("fetch");
    let after = git(at, &["show-ref"])?;
    if before != after {
        // Named: which refs moved is the diagnosis.
        let was: std::collections::BTreeSet<&str> = before.lines().collect();
        let now: std::collections::BTreeSet<&str> = after.lines().collect();
        let moved: Vec<&str> = now
            .symmetric_difference(&was)
            .map(|line| line.split_once(' ').map_or(*line, |(_, name)| name))
            .collect();
        return Err(format!(
            "fetching the corpus moved {} of its refs ({}{}) — the application fetches on its \
             own, so this corpus would not hold still under a measurement",
            moved.len(),
            moved.iter().take(4).copied().collect::<Vec<_>>().join(", "),
            if moved.len() > 4 { ", ..." } else { "" },
        ));
    }
    println!(
        "  {} remotes, {answered} tags answered offline ({peeled} peeled), main {} ahead — and \
         fetching moved nothing [{listing} | {advertising} | {fetching}]",
        shape::REMOTES.len(),
        shape::AHEAD_OF_UPSTREAM
    );
    Ok(())
}

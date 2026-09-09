//! The remotes the corpus is configured with, as bare repositories
//! inside it.
//!
//! **Without one, a whole half of the session is unreachable.** The
//! application fetches on its own — `fetch --prune --all` at open and on
//! an interval — and skips it only where no remote is configured
//! (`session::auto_fetch::known_to_have_no_remote`). A corpus with no
//! remote therefore never reaches `remote::list_tags`, so
//! `session::RemoteTagIndex` is built from nothing, no tag carries a
//! remote reading, and no branch has an upstream. Against a repository
//! this size that index is tens of thousands of entries, and the record
//! notes a run where it cost 4.7MB — bytes the corpus could not show.
//!
//! **Nothing here can reach a network.** A remote is a directory: a bare
//! repository holding refs and no objects of its own, which reads the
//! corpus's through `objects/info/alternates`. `ls-remote` and `fetch`
//! answer from the filesystem, offline, on a machine that has never had
//! a network.
//!
//! **And nothing here can move.** Each mirror holds exactly the refs the
//! corpus already tracks for it, derived from those refs, so the
//! `--prune` the application runs finds every one up to date and writes
//! nothing — down to the `refs/remotes/<name>/HEAD` a first fetch would
//! otherwise write for itself. A benchmark repository that drifts under
//! its own measurement is the disease this whole corpus exists to cure,
//! which is why `proven` asks rather than assumes.

use std::path::{Path, PathBuf};

use super::{git, shape};

/// Where the mirrors live: under the corpus's `.git`, so they are not in
/// its working tree for `git status` to find, and so that clearing the
/// corpus clears them with it.
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

/// The `refs/remotes/<name>/HEAD` a clone carries.
///
/// **Written here because otherwise a fetch writes it.** A mirror
/// advertises a symbolic HEAD, and the first fetch against a remote that
/// has one records the branch it names. A corpus built without these
/// would therefore gain two refs the first time the application opened
/// it — during the measurement, not before it.
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

/// One remote: a bare repository whose refs are the corpus's own
/// readings of it, and whose objects are the corpus's.
fn build_mirror(mirror: &Path, at: &Path, name: &str, listing: &str) -> Result<(), String> {
    std::fs::create_dir_all(mirror)
        .map_err(|e| format!("could not make {}: {e}", mirror.display()))?;
    // `--initial-branch` as well as the hash: a bare repository takes
    // `init.defaultBranch` from whatever the machine has configured,
    // and its HEAD is a ref the application's fetch would bring across
    // — so the corpus would gain a remote-tracking ref on some machines
    // and not others, which is the one thing it may not do.
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
    // The objects are the corpus's. Writing them twice would double the
    // pack that is most of what the corpus weighs, and a mirror holding
    // its own could answer for a commit the corpus does not have.
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
    // **HEAD has to name a branch the mirror holds.** `init` points it
    // at `init.defaultBranch`, which for a mirror built out of somebody
    // else's branch names is a ref that does not exist — and a remote
    // whose HEAD dangles is one `remote set-head --auto` cannot answer
    // for, so the corpus would carry a tracking HEAD for one remote and
    // not the other.
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
/// corpus tracks for this remote, under the name the remote itself would
/// use, and the tags.
///
/// **Written, not created one at a time.** `update-ref --stdin` over
/// fifty thousand names makes fifty thousand loose refs and then
/// `pack-refs` reads them all back — six minutes for the two mirrors,
/// against a file whose content is already known. The format is one
/// `<oid> <refname>` a line, sorted by name.
///
/// **The tags are the first remote's only.** `ls-remote --tags` is what
/// fills `RemoteTagIndex`, and the reference repository's second remote
/// carries a handful of branches rather than a mirror of the first.
fn carried(name: &str, listing: &str) -> String {
    let tracked = format!("refs/remotes/{name}/");
    let mut lines: Vec<String> = Vec::new();
    for line in listing.lines() {
        let Some((oid, refname)) = line.split_once(' ') else {
            continue;
        };
        if let Some(branch) = refname.strip_prefix(&tracked) {
            // `HEAD` is the reading of where the remote points, not a
            // branch the remote has; carrying it back would make the
            // mirror advertise `refs/heads/HEAD`.
            if branch == "HEAD" {
                continue;
            }
            lines.push(format!("{oid} refs/heads/{branch}"));
        } else if name == shape::REMOTES[0] && refname.starts_with("refs/tags/") {
            lines.push(format!("{oid} {refname}"));
        }
    }
    // git reads a packed-refs file in order and will not have it out of
    // one; the trait line is left off so it peels on demand rather than
    // claiming peels this does not carry.
    lines.sort_by(|a, b| a[41..].cmp(&b[41..]));
    let mut packed = String::with_capacity(lines.len() * 64);
    for line in lines {
        packed.push_str(&line);
        packed.push('\n');
    }
    packed
}

/// `main` tracks the first remote, and sits ahead of it.
///
/// **Ahead, because the reference repository's is.** A branch level with
/// its upstream draws no ahead-behind badge and answers the count with a
/// walk that stops at once; one that is ahead is what the badge, the
/// push button and the range all read.
fn upstream(at: &Path) -> Result<(), String> {
    let remote = shape::REMOTES[0];
    let behind = format!("main~{}", shape::AHEAD_OF_UPSTREAM);
    let oid = git(at, &["rev-parse", &behind])?;
    let oid = oid.trim();
    let tracking = format!("refs/remotes/{remote}/main");
    git(at, &["update-ref", &tracking, oid])?;
    let mirror = mirror_path(at, remote);
    git(&mirror, &["update-ref", "refs/heads/main", oid])?;
    // The mirror has `main` now, and a fork's HEAD names it — which is
    // the reading `remote set-head --auto` takes.
    git(&mirror, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    git(at, &["config", "branch.main.remote", remote])?;
    git(at, &["config", "branch.main.merge", "refs/heads/main"])?;
    Ok(())
}

/// Proves both properties the corpus rests on, while it is being built.
///
/// **That the remotes answer.** A mirror that cannot be read leaves
/// `RemoteTagIndex` built from nothing all over again, and it does so
/// silently: `read_remote_tags` logs the failure and moves on with the
/// readings it had, which is none.
///
/// **And that fetching changes nothing.** The application fetches on its
/// own, so the corpus is asked this question whether or not anybody
/// meant to ask it, and a benchmark repository that moves under its own
/// measurement is the disease the generated one exists to cure. Asked
/// here rather than trusted, because the answer is what everything
/// measured against this corpus assumes.
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
    // **An annotated tag is advertised twice**: once as the tag object
    // and once peeled to the commit under it, which is the pairing
    // `remote::parse_ls_remote_tags` exists to do. Names, then, not
    // lines.
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
    // Both of them, which is what the application runs. It is cheap
    // once nothing repacks behind it (`gc.auto`), and the whole point
    // is that the command the application uses moves nothing.
    git(at, &["fetch", "--prune", "--all"])?;
    let fetching = took("fetch");
    let after = git(at, &["show-ref"])?;
    if before != after {
        // Named, because which ones is the whole diagnosis and the
        // fact alone points nowhere: a first fetch against a remote
        // with a symbolic HEAD writes a tracking `HEAD`, a mirror
        // holding a ref the corpus does not writes that, and the two
        // read identically from here.
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

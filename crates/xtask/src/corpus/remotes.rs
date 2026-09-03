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
//! nothing. A benchmark repository that drifts under its own
//! measurement is the disease this whole corpus exists to cure.

use std::path::{Path, PathBuf};

use super::{git, shape};

/// Where the mirrors live: under the corpus's `.git`, so they are not in
/// its working tree for `git status` to find, and so that clearing the
/// corpus clears them with it.
const MIRRORS: &str = "pg-remotes";

/// Builds one bare mirror per remote, configures the corpus to use them,
/// and puts `main` ahead of the first.
pub(super) fn configure(at: &Path) -> Result<(), String> {
    let listing = git(at, &["show-ref"])?;
    for name in shape::REMOTES {
        let mirror = mirror_path(at, name);
        build_mirror(&mirror, at, name, &listing)?;
        git(at, &["remote", "add", name, &shown(&mirror)])?;
    }
    upstream(at)
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
    git(
        mirror,
        &["init", "--bare", "--quiet", "--object-format=sha1"],
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
    let packed = mirror.join("packed-refs");
    std::fs::write(&packed, carried(name, listing))
        .map_err(|e| format!("could not write {}: {e}", packed.display()))?;
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
    git(
        &mirror_path(at, remote),
        &["update-ref", "refs/heads/main", oid],
    )?;
    git(at, &["config", "branch.main.remote", remote])?;
    git(at, &["config", "branch.main.merge", "refs/heads/main"])?;
    proven(at)
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
    let mut began = std::time::Instant::now();
    let mut took = |what: &str| {
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
        return Err(
            "fetching the corpus moved its refs — the application fetches on its own, so this \
             corpus would not hold still under a measurement"
                .to_string(),
        );
    }
    println!(
        "  {} remotes, {answered} tags answered offline ({peeled} peeled), main {} ahead — and \
         fetching moved nothing [{listing} | {advertising} | {fetching}]",
        shape::REMOTES.len(),
        shape::AHEAD_OF_UPSTREAM
    );
    Ok(())
}

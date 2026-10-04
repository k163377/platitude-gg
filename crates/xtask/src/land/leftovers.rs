//! What a landing clears of its own session's making (CLAUDE.md §Git 運用):
//! the branches the session's git created — the reference-transaction hook
//! writes each one's maker down (`gate::hooks::maker`) — the branch of the
//! tree the app made for the session to start in, which the session writes
//! down as it starts, or else the landing from the session's transcript
//! (`seats::start`), and the trees that have one of them out, seats aside.
//! Each goes only when main, or the branch just landed as it stood before
//! its rebase, holds everything it holds; a tree also only with no lock, no
//! uncommitted file and nobody in it. What stays is named: a copy the work
//! made for itself is its session's to delete, and work main does not
//! hold, kept on purpose, goes under `keep/`.
//!
//! A tree the session started in that something still works in is left
//! behind: the session works in a seat, and what works there — the
//! terminal tab the app opens in it — outlasts the landing, so any later
//! landing takes it away once it can, judged as above. Nothing else
//! another session, the app or the user made is judged or named — its
//! maker answers for it.

use std::path::{Path, PathBuf};

use crate::gate::hooks::{Made, Maker};
use crate::seats::{
    Identity, WorktreeBlock, commits_in, dirty_lines, in_rig, roster_letter, same_tree,
    worktree_blocks,
};
use crate::subprocess::git_query;

/// A commit a leftover may be held by, with its tree read once.
struct Holder {
    name: String,
    tree: Option<String>,
}

/// Whose branches this landing judges: those of the session that runs it,
/// and those an earlier landing left behind.
struct Judge {
    common: PathBuf,
    session: String,
}

impl Judge {
    /// The ledger's word on `branch`, when it is this landing's to judge.
    fn judges(&self, branch: &str) -> Option<Maker> {
        if branch.is_empty() {
            return None;
        }
        let maker = crate::gate::hooks::maker(&self.common, &format!("refs/heads/{branch}"))?;
        (self.own(&maker) || maker.how == Made::LeftBehind).then_some(maker)
    }

    fn own(&self, maker: &Maker) -> bool {
        maker.session == self.session
    }
}

/// What a sweep says: its lines, and what earlier landings left behind
/// that still stands, said once at the end — with the branches of this
/// session's start trees that something still works in, to be left behind.
#[derive(Default)]
struct Said {
    lines: Vec<String>,
    standing: Vec<String>,
    busy_starts: Vec<String>,
}

/// Clears what this session made and holds nothing main does not — nor
/// the landed branch as it stood before its rebase (`unrebased`), which a
/// copy taken from it before its history was rewritten is held by — and
/// says what went and what stayed. The tree the branch landed from
/// (`landed`) is left: the landing is still writing its record there. A
/// landing with no session behind it (a person's) has nothing of its own.
pub(super) fn clear(here: &str, unrebased: Option<&str>, landed: &str) {
    let session = Identity::current(None).session;
    if session.trim().is_empty() {
        return;
    }
    let Some(common) = crate::subprocess::common_git_dir(here) else {
        println!("leftovers: git could not name the repository, so nothing was cleared");
        return;
    };
    let main = git_query(here, &["rev-parse", "main"]);
    let holders: Vec<Holder> = std::iter::once("refs/heads/main")
        .chain(unrebased.filter(|tip| main.as_deref() != Some(*tip)))
        .map(|name| Holder {
            name: name.to_string(),
            tree: git_query(here, &["rev-parse", &format!("{name}^{{tree}}")]),
        })
        .collect();
    let judge = Judge {
        common: PathBuf::from(common),
        session,
    };
    for line in sweep(here, &judge, &holders, landed) {
        println!("leftovers: {line}");
    }
}

fn sweep(here: &str, judge: &Judge, holders: &[Holder], landed: &str) -> Vec<String> {
    let Some(listing) = git_query(here, &["worktree", "list", "--porcelain"]) else {
        return vec!["git could not list the worktrees, so nothing was cleared".to_string()];
    };
    // Ahead of the judging: what the session's own start may not have
    // written down.
    crate::seats::start::adopt(&listing, &judge.session);
    let mut said = Said::default();
    trees(here, &listing, judge, holders, landed, &mut said);
    branches(here, judge, holders, &mut said);
    let left = leave_behind(judge, &said.busy_starts);
    said.lines.extend(left);
    if !said.standing.is_empty() {
        said.lines.push(format!(
            "still standing, left behind by earlier landings — their sessions' work, not this \
             one's to delete: {}",
            said.standing.join(", ")
        ));
    }
    said.lines
}

/// The trees this landing judges, taken down or said to stay.
fn trees(
    here: &str,
    listing: &str,
    judge: &Judge,
    holders: &[Holder],
    landed: &str,
    said: &mut Said,
) {
    // The first tree listed is the primary checkout, which nothing clears.
    for tree in worktree_blocks(listing).iter().skip(1) {
        let Some(maker) = judge.judges(&tree.branch) else {
            continue;
        };
        if roster_letter(&tree.path).is_some()
            || in_rig(&tree.path)
            || same_tree(&tree.path, landed)
        {
            continue;
        }
        let name = shown(&tree.path);
        let head = head_in(listing, &tree.path);
        let branch = &tree.branch;
        match (
            judge.own(&maker),
            take_down_tree(here, tree, head.as_deref(), holders),
        ) {
            (true, Ok(())) => said.lines.push(format!(
                "took away worktree {name} ({branch}) — main holds all it held"
            )),
            (true, Err(stays)) => {
                said.lines
                    .push(format!("kept worktree {name} ({branch}) — {}", stays.why()));
                if maker.how == Made::StartedIn && matches!(stays, Stays::InUse(_)) {
                    said.busy_starts.push(branch.clone());
                }
            }
            (false, Ok(())) => said.lines.push(format!(
                "took away worktree {name} ({branch}), which session {} left behind — main \
                 holds all it held",
                maker.session
            )),
            (false, Err(stays)) => said
                .standing
                .push(format!("worktree {name} ({})", stays.why())),
        }
    }
}

/// The commit a tree's HEAD names, as git's list gives it — read there so
/// a tree whose directory went is judged like any other.
fn head_in(listing: &str, path: &str) -> Option<String> {
    let block = listing.split("\n\n").find(|block| {
        block.lines().any(|line| {
            line.strip_prefix("worktree ")
                .is_some_and(|at| at.replace('\\', "/") == path)
        })
    })?;
    block
        .lines()
        .find_map(|line| line.strip_prefix("HEAD "))
        .map(str::to_string)
}

/// A tree's name: the last part of its path.
fn shown(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
}

/// Why a tree stays where it is.
enum Stays {
    /// Something works in it: once nothing does, it may go.
    InUse(String),
    /// Anything else — its lock, its own work, its files, git's refusal.
    Kept(String),
}

impl Stays {
    fn why(&self) -> &str {
        match self {
            Self::InUse(why) | Self::Kept(why) => why,
        }
    }
}

/// Takes a tree off the disk and out of git's list, or answers why not.
fn take_down_tree(
    here: &str,
    tree: &WorktreeBlock,
    head: Option<&str>,
    holders: &[Holder],
) -> Result<(), Stays> {
    if tree.locked {
        return Err(Stays::Kept(match tree.reason.as_str() {
            "" => "it is locked".to_string(),
            reason => format!("it is locked ({reason})"),
        }));
    }
    let head = head.ok_or_else(|| Stays::Kept("git's list names no HEAD for it".to_string()))?;
    if !held(here, head, holders) {
        return Err(Stays::Kept(own_work(here, head)));
    }
    if !Path::new(&tree.path).is_dir() {
        // Its directory went before it; only git's record is left. No
        // --force: a directory back by the time git looks is judged clean
        // first.
        return git_query(here, &["worktree", "remove", &tree.path])
            .map(|_| ())
            .ok_or_else(|| {
                Stays::Kept("its directory is gone, and git would not drop its record".to_string())
            });
    }
    match dirty_lines(&tree.path) {
        Some(0) => {}
        Some(dirty) => return Err(Stays::Kept(format!("{dirty} uncommitted file(s)"))),
        None => return Err(Stays::Kept("git could not read its status".to_string())),
    }
    let aside = move_aside(&tree.path).map_err(Stays::InUse)?;
    // Judged again where nobody can reach it any more: what was written or
    // committed there while it was judged stays with it.
    if let Err(why) = as_judged(&aside, head) {
        return Err(Stays::Kept(put_back(&aside, &tree.path, &why)));
    }
    // The directory is out of its name, so git drops only its record.
    if git_query(here, &["worktree", "remove", &tree.path]).is_none() {
        return Err(Stays::Kept(put_back(
            &aside,
            &tree.path,
            "git would not drop it from its list",
        )));
    }
    delete(&aside).map_err(Stays::Kept)
}

/// Whether a tree moved aside is still what was judged: clean, its HEAD
/// where it was. git reads it there: its `.git` file names its record
/// wherever it stands.
fn as_judged(aside: &Path, head: &str) -> Result<(), String> {
    let dir = aside.to_string_lossy().replace('\\', "/");
    match dirty_lines(&dir) {
        Some(0) => {}
        Some(dirty) => return Err(format!("{dirty} file(s) were written as it was judged")),
        None => return Err("git could not read it once it was moved aside".to_string()),
    }
    match git_query(&dir, &["rev-parse", "HEAD"]) {
        Some(now) if now == head => Ok(()),
        _ => Err("its HEAD moved as it was judged".to_string()),
    }
}

/// Puts a tree back under its name after a step turned it down, and says
/// why it stays.
fn put_back(aside: &Path, path: &str, why: &str) -> String {
    match std::fs::rename(aside, path) {
        Ok(()) => why.to_string(),
        Err(e) => format!("{why}, and its files stay at {} ({e})", aside.display()),
    }
}

/// Moves a directory out of its name once nobody works in it — after
/// which nobody can start to.
fn move_aside(path: &str) -> Result<PathBuf, String> {
    nobody_in(path)?;
    let aside = PathBuf::from(format!("{path}.leaving-{}", std::process::id()));
    std::fs::rename(path, &aside)
        .map_err(|e| format!("it would not move aside ({e}): a process works in it"))?;
    Ok(aside)
}

/// Asked by the rename itself: Windows will not rename a directory a
/// process stands in or holds a file of.
#[cfg(windows)]
fn nobody_in(_path: &str) -> Result<(), String> {
    Ok(())
}

/// A rename asks nobody here, so the processes' working directories are
/// read first, where Linux shows each of them.
#[cfg(target_os = "linux")]
fn nobody_in(path: &str) -> Result<(), String> {
    let tree = canonical(path)?;
    let processes = std::fs::read_dir("/proc").map_err(|e| format!("/proc would not read: {e}"))?;
    for process in processes.flatten() {
        if std::fs::read_link(process.path().join("cwd")).is_ok_and(|cwd| cwd.starts_with(&tree)) {
            return Err(format!(
                "process {} works in it",
                process.file_name().to_string_lossy()
            ));
        }
    }
    Ok(())
}

/// The same, asked of `lsof` where there is no /proc (macOS). lsof exits
/// non-zero when a process would not be read, and what it printed of the
/// rest still stands.
#[cfg(all(unix, not(target_os = "linux")))]
fn nobody_in(path: &str) -> Result<(), String> {
    let tree = canonical(path)?;
    let listed = std::process::Command::new("lsof")
        .args(["-a", "-d", "cwd", "-F", "pn"])
        .output()
        .map_err(|e| format!("lsof would not run, so nobody can say who works in it: {e}"))?;
    let text = String::from_utf8_lossy(&listed.stdout);
    let mut pid = "";
    for line in text.lines() {
        if let Some(number) = line.strip_prefix('p') {
            pid = number;
        } else if let Some(cwd) = line.strip_prefix('n')
            && Path::new(cwd).starts_with(&tree)
        {
            return Err(format!("process {pid} works in it"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn canonical(path: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|e| format!("its path would not read: {e}"))
}

fn delete(aside: &Path) -> Result<(), String> {
    std::fs::remove_dir_all(aside)
        .map_err(|e| format!("its files stay at {} ({e})", aside.display()))
}

/// The branches this landing judges that no tree has out, beside main and
/// the seats' own: each goes when a holder holds all it holds — deleted
/// only while it still names the commit that was judged — and the rest are
/// named.
fn branches(here: &str, judge: &Judge, holders: &[Holder], said: &mut Said) {
    let (Some(listing), Some(refs)) = (
        git_query(here, &["worktree", "list", "--porcelain"]),
        git_query(
            here,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname)",
                "refs/heads/",
            ],
        ),
    ) else {
        said.lines
            .push("git could not list the branches, so none was cleared".to_string());
        return;
    };
    let out: Vec<String> = worktree_blocks(&listing)
        .into_iter()
        .map(|tree| tree.branch)
        .collect();
    let mut went = Vec::new();
    let mut kept = Vec::new();
    for (full, oid) in refs.lines().filter_map(|line| line.split_once(' ')) {
        let Some(name) = full.strip_prefix("refs/heads/") else {
            continue;
        };
        if !swept_branch(name) || out.iter().any(|branch| branch == name) {
            continue;
        }
        let Some(maker) = judge.judges(name) else {
            continue;
        };
        let stays = if !held(here, oid, holders) {
            own_work(here, oid)
        } else if git_query(here, &["update-ref", "-d", full, oid]).is_none() {
            "it moved as it was judged".to_string()
        } else {
            // `branch -D` would take its section too; there may be none.
            let _ = git_query(
                here,
                &["config", "--remove-section", &format!("branch.{name}")],
            );
            let was = &oid[..oid.len().min(10)];
            went.push(if judge.own(&maker) {
                format!("{name} (was {was})")
            } else {
                format!(
                    "{name} (was {was}, left behind by session {})",
                    maker.session
                )
            });
            continue;
        };
        if judge.own(&maker) {
            kept.push(format!("{name} ({stays})"));
        } else {
            said.standing.push(format!("branch {name} ({stays})"));
        }
    }
    if !went.is_empty() {
        said.lines.push(format!(
            "took away branch(es) {} — main holds all they held",
            went.join(", ")
        ));
    }
    if !kept.is_empty() {
        said.lines.push(format!(
            "kept branch(es) this session made: {} — a copy the work made for itself is its \
             session's to delete, and work main does not hold, kept on purpose, goes under keep/ \
             (CLAUDE.md §Git 運用)",
            kept.join(", ")
        ));
    }
}

/// Hands the trees the app made for this session to start in that
/// something still works in — the terminal tab the app opens there
/// outlasts the session's work — to any later landing, with their
/// branches (`busy`), and says so. A start tree kept for anything else —
/// its own files, its own commits — stays this session's to answer for.
fn leave_behind(judge: &Judge, busy: &[String]) -> Option<String> {
    let mut left = Vec::new();
    let mut stuck = Vec::new();
    for branch in busy {
        match crate::gate::hooks::write_down(
            &judge.common,
            &format!("refs/heads/{branch}"),
            &judge.session,
            Made::LeftBehind,
        ) {
            Ok(()) => left.push(branch.as_str()),
            Err(why) => stuck.push(format!("{branch} ({why})")),
        }
    }
    let mut said = Vec::new();
    if !left.is_empty() {
        said.push(format!(
            "left {} to whichever landing comes next — the app made it, with its tree, for this \
             session to start in, and what still works there (the terminal tab the app opens in \
             it) is not this session's to stop; that landing takes it away once it can",
            left.join(", ")
        ));
    }
    if !stuck.is_empty() {
        said.push(format!(
            "could not leave {} to a later landing, so none will take it away — name it to the \
             user",
            stuck.join(", ")
        ));
    }
    (!said.is_empty()).then(|| said.join("; "))
}

/// Whether a branch is one this sweep judges at all: neither main nor a
/// seat's own, which the roster resets and the landing moves.
fn swept_branch(name: &str) -> bool {
    name != "main"
        && !name
            .strip_prefix("worktree-")
            .is_some_and(|letter| crate::seats::SEATS.contains(&letter))
}

/// Whether one of `holders` holds everything `commit` does: has it in its
/// history, or would gain nothing by merging it — which a rewritten copy
/// of landed work passes (a branch from before a rebase or a reordering).
fn held(here: &str, commit: &str, holders: &[Holder]) -> bool {
    holders.iter().any(|holder| {
        git_query(here, &["merge-base", "--is-ancestor", commit, &holder.name]).is_some()
            || absorbed(here, commit, holder)
    })
}

/// Whether merging `commit` into the holder leaves the holder's tree as
/// it is. A conflict is no: git answers it with a non-zero exit, and it is
/// a change the holder does not have.
fn absorbed(here: &str, commit: &str, holder: &Holder) -> bool {
    let Some(tree) = holder.tree.as_deref() else {
        return false;
    };
    git_query(here, &["merge-tree", "--write-tree", &holder.name, commit])
        .is_some_and(|merged| merged.lines().next() == Some(tree))
}

/// What a tree or branch holds that main does not, said as the reason it
/// stays.
fn own_work(here: &str, commit: &str) -> String {
    match commits_in(here, &format!("main..{commit}")) {
        Some(ahead) => format!("{ahead} commit(s) main does not hold"),
        None => "git could not count what main does not hold".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{head_in, shown, swept_branch};

    /// Read from the list, so a tree whose directory went has one too.
    #[test]
    fn a_trees_head_is_read_out_of_gits_list() {
        let listing = "worktree C:/x/repo\nHEAD 1111\nbranch refs/heads/main\n\n\
                       worktree C:/x/repo/.claude/worktrees/gone\nHEAD 2222\ndetached\nprunable \
                       gitdir file points to non-existent location\n\n\
                       worktree C:\\x\\repo\\.claude\\worktrees\\spent\nHEAD 3333\nbranch \
                       refs/heads/claude/spent\n";
        assert_eq!(
            head_in(listing, "C:/x/repo/.claude/worktrees/gone").as_deref(),
            Some("2222")
        );
        assert_eq!(
            head_in(listing, "C:/x/repo/.claude/worktrees/spent").as_deref(),
            Some("3333"),
            "spelled the way the listing's reader spells a path"
        );
        assert_eq!(head_in(listing, "C:/x/repo/.claude/worktrees"), None);
        assert_eq!(shown("C:/x/repo/.claude/worktrees/spent/"), "spent");
    }

    #[test]
    fn main_and_the_seats_branches_are_never_swept() {
        for name in ["main", "worktree-a", "worktree-f"] {
            assert!(!swept_branch(name), "{name}");
        }
        for name in [
            "claude/awesome-fermi-761463",
            "keep/pre-reorg",
            "worktree-tooltip",
            "worktree-g",
            "report-temp",
        ] {
            assert!(swept_branch(name), "{name}");
        }
    }
}

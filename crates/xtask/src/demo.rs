//! `cargo xtask demo-repo` — throwaway repositories in known states.
//!
//! Every invocation builds a fresh repository under the system temp dir
//! (or `--at <dir>`), isolated from the developer's git configuration, and
//! prints its path. Nothing is ever reused: a sandbox whose state has
//! drifted is worse than none, so the sandbox is always newly made.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(args: &[String]) -> Result<PathBuf, String> {
    let mut preset: Option<&str> = None;
    let mut at: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--at" => {
                let dir = it.next().ok_or("--at needs a directory")?;
                at = Some(PathBuf::from(dir));
            }
            other if preset.is_none() => preset = Some(other),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let preset = preset.ok_or("demo-repo needs a preset (see `cargo xtask`)")?;
    create(preset, at)
}

/// Builds `preset` and returns the work-tree path.
pub fn create(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    create_named(preset, at, "repo")
}

/// Builds `preset` with the work tree called `name` rather than `repo`.
///
/// A tab is titled after its work-tree folder (`models::tabs::title_of`),
/// and every demo repository being called the same thing is fine until
/// the subject is the strip itself — a row of identical names cannot show
/// which tabs gave way and which were left alone.
pub fn create_named(preset: &str, at: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    let root = match at {
        Some(dir) => dir,
        None => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            std::env::temp_dir()
                .join("pg-demo")
                .join(format!("{preset}-{nanos}"))
        }
    };
    let mut repo = DemoRepo::init(&root, name)?;
    match preset {
        "basic" => basic(&mut repo)?,
        "dirty" => dirty(&mut repo)?,
        "eol" => eol(&mut repo)?,
        "conflict" => conflict(&mut repo)?,
        "rebase-conflict" => rebase_conflict(&mut repo)?,
        "rebase-staged" => rebase_staged(&mut repo)?,
        "rebase-empty" => rebase_empty(&mut repo)?,
        "cherry-pick-conflict" => cherry_pick_conflict(&mut repo)?,
        "conflict-kinds" => conflict_kinds(&mut repo)?,
        "drop-collides" => drop_collides(&mut repo)?,
        "drop-stops" => drop_stops(&mut repo)?,
        "stashes" => stashes(&mut repo)?,
        "detached" => detached(&mut repo)?,
        "behind" => behind(&mut repo)?,
        "diverged" => diverged(&mut repo)?,
        "unpublished" => unpublished(&mut repo)?,
        "noremote" => noremote(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "errsig" => errsig(&mut repo)?,
        "co-authors" => co_authors(&mut repo)?,
        "authorship" => authorship(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
        "edges" => edges(&mut repo)?,
        "long" => long(&mut repo)?,
        "empty" => {}
        other => return Err(format!("unknown preset: {other}")),
    }
    Ok(repo.work)
}

/// A repository under construction. Commits get ascending timestamps
/// (past → now) so the graph looks like history, not like a fixture.
struct DemoRepo {
    root: PathBuf,
    work: PathBuf,
    global_config: PathBuf,
    base_epoch: u64,
    tick: u64,
}

/// Spacing between commit timestamps.
const TICK_SECS: u64 = 30 * 60;
/// How far in the past the history starts.
const HISTORY_SECS: u64 = 40 * 60 * 60;

impl DemoRepo {
    fn init(root: &Path, name: &str) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| format!("creating {}: {e}", root.display()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let mut repo = Self {
            root: root.to_path_buf(),
            work: root.join(name),
            global_config: root.join("no-global-config"),
            base_epoch: now.saturating_sub(HISTORY_SECS),
            tick: 0,
        };
        std::fs::create_dir_all(&repo.work).map_err(|e| e.to_string())?;
        repo.git(&["init", "-b", "main"])?;
        for (key, value) in [
            ("user.name", "Demo User"),
            ("user.email", "demo@example.com"),
            ("commit.gpgsign", "false"),
            ("tag.gpgSign", "false"),
            ("core.autocrlf", "false"),
        ] {
            repo.git(&["config", key, value])?;
        }
        Ok(repo)
    }

    fn command(&mut self, dir: &Path, args: &[&str]) -> Command {
        self.tick += 1;
        let stamp = format!("{} +0000", self.base_epoch + self.tick * TICK_SECS);
        let mut cmd = Command::new("git");
        cmd.args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", &self.global_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp);
        cmd
    }

    fn git(&mut self, args: &[&str]) -> Result<String, String> {
        let dir = self.work.clone();
        self.git_at(&dir, args)
    }

    fn git_at(&mut self, dir: &Path, args: &[&str]) -> Result<String, String> {
        let out = crate::run_captured(&mut self.command(dir, args))?;
        if !out.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// For commands whose non-zero exit is the state we want (a merge
    /// stopping on conflicts). Only a spawn failure is an error.
    fn git_expecting_stop(&mut self, args: &[&str]) -> Result<(), String> {
        let dir = self.work.clone();
        crate::run_captured(&mut self.command(&dir, args)).map(drop)
    }

    fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.work.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| format!("writing {rel}: {e}"))
    }

    fn commit(&mut self, rel: &str, content: &str, message: &str) -> Result<(), String> {
        self.write(rel, content)?;
        self.git(&["add", "--", rel])?;
        self.git(&["commit", "-m", message])?;
        Ok(())
    }

    /// Commits something written `earlier` seconds before it was
    /// committed, optionally by somebody other than this repository's
    /// own identity. `--date` beats the `GIT_AUTHOR_DATE` every command
    /// here carries — the committer date keeps the marching stamp, so
    /// the two moments come out apart by exactly what was asked for.
    fn commit_written_earlier(
        &mut self,
        rel: &str,
        content: &str,
        message: &str,
        author: Option<&str>,
        earlier: u64,
    ) -> Result<(), String> {
        self.write(rel, content)?;
        self.git(&["add", "--", rel])?;
        // Two commands from now is what the commit itself will carry;
        // reading the base rather than the clock keeps it reproducible.
        let written = self.base_epoch + (self.tick + 2) * TICK_SECS - earlier;
        let date = format!("--date={written} +0000");
        let mut args = vec!["commit", &date];
        let author_arg = author.map(|a| format!("--author={a}"));
        if let Some(arg) = author_arg.as_deref() {
            args.push(arg);
        }
        args.extend(["-m", message]);
        self.git(&args)?;
        Ok(())
    }

    /// Runs git with `input` on its standard input and returns what it
    /// printed. One process for a batch of refs: a repository of
    /// thousands of tags built a `git tag` at a time is minutes of
    /// process spawning on Windows.
    fn git_stdin(&mut self, args: &[&str], input: &str) -> Result<String, String> {
        let dir = self.work.clone();
        let mut child = self
            .command(&dir, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn git {args:?}: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("git took no standard input")?;
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("writing to git {args:?}: {e}"))?;
        drop(stdin);
        let out = child
            .wait_with_output()
            .map_err(|e| format!("waiting for git {args:?}: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Creates `<root>/origin.git` (bare), wires it as `origin`.
    fn add_origin(&mut self) -> Result<(), String> {
        let bare = self.root.join("origin.git");
        std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
        self.git_at(&bare.clone(), &["init", "--bare", "-b", "main"])?;
        let url = file_url(&bare);
        self.git(&["remote", "add", "origin", &url])?;
        Ok(())
    }
}

fn file_url(path: &Path) -> String {
    let mut p = path.to_string_lossy().replace('\\', "/");
    if !p.starts_with('/') {
        p.insert(0, '/');
    }
    format!("file://{p}")
}

/// Every combination of "how many people the message credits" and
/// "is it signed", newest first: one co-author signed, three signed,
/// three unsigned, one unsigned, and a commit crediting nobody.
///
/// The spellings differ on purpose — `Co-Authored-By` is what the tooling
/// writes, `Co-authored-by` is what the convention documents — because
/// git's `key=` matches either and the reader must not care.
fn co_authors(repo: &mut DemoRepo) -> Result<(), String> {
    const CROWD: &str = "feat: write this one with a crowd\n\n\
         The body sits above the trailers, the way it always does.\n\n\
         Co-authored-by: Claude Opus 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Fable 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Opus 4.8 <noreply@anthropic.com>";
    // The last address is deliberately long: an address has no length
    // worth trusting, and a card that sizes itself to one has to elide
    // rather than run off the window.
    const CROWD_SIGNED: &str = "feat: write this one with a crowd, signed\n\n\
         Co-authored-by: Claude Opus 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Fable 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Opus 4.8 \
         <an.address.long.enough.to.need.eliding@subdomain.example.co.jp>";
    const PAIR: &str = "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>";

    repo.commit(
        "README.md",
        "# demo\n\nA repository with credited company.\n",
        "docs: start the readme",
    )?;
    repo.commit(
        "src/pair.txt",
        "written by two\n",
        &format!("feat: write this one with company\n\n{PAIR}"),
    )?;
    repo.commit("src/crowd.txt", "written by four\n", CROWD)?;

    // A key this repository vouches for, so the commits signed with it
    // verify rather than merely carrying a signature.
    let trusted = keygen(repo, "trusted", "demo@example.com")?;
    let allowed = repo.root.join("allowed_signers");
    std::fs::write(&allowed, format!("demo@example.com {trusted}"))
        .map_err(|e| format!("writing allowed_signers: {e}"))?;
    let trusted_key = repo.root.join("trusted.pub");
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("gpg.ssh.allowedSignersFile", config_path(&allowed)),
        ("user.signingkey", config_path(&trusted_key)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }

    repo.commit(
        "src/crowd.txt",
        "written by four, and signed\n",
        CROWD_SIGNED,
    )?;

    // A name long enough to crowd the mark it sits next to: the name is
    // the side that gives way, so the tick stays on screen.
    repo.write("src/long.txt", "written under a long name\n")?;
    repo.git(&["add", "--", "src/long.txt"])?;
    repo.git(&[
        "commit",
        "--author=Alexander Kirillov-Petrov <alexander@example.com>",
        "-m",
        &format!("feat: write this one under a long name\n\n{PAIR}"),
    ])?;

    // Signed with a key nobody vouched for: git reads the signature and
    // cannot judge it. Every SSH signature falls into this state when no
    // allowedSigners file is configured at all, so it is not a corner.
    keygen(repo, "stranger", "stranger@example.com")?;
    let stranger_key = repo.root.join("stranger.pub");
    repo.git(&["config", "user.signingkey", &config_path(&stranger_key)])?;
    repo.commit(
        "src/unjudged.txt",
        "signed by a stranger\n",
        &format!("feat: write this one signed by an unvouched key\n\n{PAIR}"),
    )?;
    repo.git(&["config", "user.signingkey", &config_path(&trusted_key)])?;

    // A signature that no longer matches what it signed: sign properly,
    // then swap the tree underneath. Nothing an ordinary repository does
    // produces this, and it is the one state the pane spends words on,
    // so it has to be reachable from a preset.
    repo.commit(
        "src/tampered.txt",
        "before\n",
        &format!("feat: write this one and then tamper with it\n\n{PAIR}"),
    )?;
    repo.write("src/tampered.txt", "after the signature was made\n")?;
    repo.git(&["add", "--", "src/tampered.txt"])?;
    let swapped_tree = repo.git(&["write-tree"])?;
    let tampered = retree_head(repo, &swapped_tree)?;
    repo.git(&["update-ref", "refs/heads/main", &tampered])?;
    repo.git(&["reset", "--hard", "HEAD"])?;

    repo.commit(
        "src/pair.txt",
        "written by two, and signed\n",
        &format!("feat: write this one with company, signed\n\n{PAIR}"),
    )?;
    Ok(())
}

/// Every way the person who wrote a commit and the person who put it
/// here can be two, newest first: a patch applied by somebody else days
/// after it was written, one applied by somebody else the moment it
/// arrived (a squash merge on a forge), one the same hand committed
/// later than it wrote it (an amend, a rebase), and two ordinary
/// commits, where the two are one person at one moment.
///
/// Measured shares of the divergent shapes, so the card is not being
/// built for a corner (2026-08-09, `author != committer` / `author date
/// != commit date`): kotlinx.coroutines 47.5% / 34.3%, JetBrains/kotlin
/// 47.5% / 89.9%, jackson-module-kotlin 16.8% / 9.2%.
fn authorship(repo: &mut DemoRepo) -> Result<(), String> {
    const MAILED: &str = "Yuki Tanaka <yuki.tanaka@example.com>";
    const DAY: u64 = 24 * 60 * 60;

    repo.commit(
        "README.md",
        "# demo\n\nA repository where the credit is split.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit_written_earlier(
        "src/app.txt",
        "app v1, and it waits for slow disks\n",
        "fix: hold the door open for slow disks",
        None,
        2 * DAY,
    )?;
    repo.write("src/merged.txt", "came in through the web\n")?;
    repo.git(&["add", "--", "src/merged.txt"])?;
    repo.git(&[
        "commit",
        &format!("--author={MAILED}"),
        "-m",
        "feat: take the config out into a file",
    ])?;
    repo.commit_written_earlier(
        "src/mailed.txt",
        "arrived as a patch\n",
        "perf: stop reading the index twice",
        Some(MAILED),
        3 * DAY,
    )?;
    Ok(())
}

/// Rewrites HEAD's commit object with a different tree, keeping every
/// other header — including the signature, which is what makes the
/// result read as broken rather than as unsigned (measured: `%G?` goes
/// from `G` to `B`).
fn retree_head(repo: &mut DemoRepo, tree: &str) -> Result<String, String> {
    let dir = repo.work.clone();
    let out = crate::run_captured(&mut repo.command(&dir, &["cat-file", "commit", "HEAD"]))?;
    if !out.status.success() {
        return Err("git cat-file commit HEAD failed".to_string());
    }
    let patched = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| {
            if line.starts_with("tree ") {
                format!("tree {tree}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let path = repo.root.join("tampered-commit");
    std::fs::write(&path, patched).map_err(|e| format!("writing tampered commit: {e}"))?;
    repo.git(&["hash-object", "-w", "-t", "commit", &config_path(&path)])
}

/// Bytes a ref's last component may have. Measured in throwaway
/// repositories on both systems: a loose ref is the file `<name>.lock`
/// against a 255-byte filename, so Linux stops at 250 — while NTFS counts
/// UTF-16 units and took 100 kanji (301 bytes) without complaint. The
/// smaller wall is the shared one, and a repository that holds a ref only
/// Windows can spell is a repository Linux cannot check out.
const REF_WALL: usize = 250;
/// Bytes a path component may have: 255 on both, and the worktree path
/// has to fit MAX_PATH under a temp directory, so files stay well inside.
const PATH_ROOM: usize = 120;

/// `head`, then kanji until the whole is exactly `bytes` long. The point
/// is the last character: it ends *on* the wall, so anything that slices
/// by byte index cuts it in half there — which is how this project lost
/// its graph once (core.md, the co-author key).
fn to_the_byte(head: &str, bytes: usize) -> String {
    const FILL: [char; 5] = ['長', 'い', '名', '前', 'の'];
    let mut s = String::from(head);
    let mut i = 0;
    while s.len() + 3 <= bytes {
        s.push(FILL[i % FILL.len()]);
        i += 1;
    }
    while s.len() < bytes {
        s.push('x');
    }
    s
}

/// A paragraph somebody pasted, in both scripts, *at least* `bytes` long.
/// git puts no wall in front of a subject, a body, an author name or a URL
/// — it took a megabyte of each in the same measurement — so what stands in
/// for "the limit" here is the worst thing a person plausibly does.
///
/// At least, not at most: asking for less than one sentence used to return
/// the two characters of the terminator, and a co-author whose name is
/// `終端` tests nothing.
pub(crate) fn pasted(bytes: usize) -> String {
    let unit = "この行は長い日本語の文章で、折り返しと省略の両方を試すために置いてある。 \
                And an English clause rides along so the run of Latin text is measured too. ";
    let mut s = String::new();
    while s.len() < bytes {
        s.push_str(unit);
    }
    s.push_str("終端");
    s
}

/// The message a release actually gets written with: paragraphs, a list,
/// and a note in Japanese. Long enough that no pane shows it whole —
/// which is what the description box's grip is pulled for — but written
/// rather than repeated, because a wall of the same sentence tells you
/// nothing about how a real message wraps
/// (デザイン規約 §コミットメッセージの 2 つの枠).
///
/// Some paragraphs are wrapped at 72 columns and some are one long line,
/// because both turn up in real repositories — one from an editor, the
/// other pasted in — and they are the two things a box that wraps has to
/// be looked at doing.
const LONG_MESSAGE: &str = "\
refactor: move the whole store behind one interface

Every reader of the store used to reach into it its own way: some took a
lock and walked the map, some asked the index and then went back for the
row, and two of them cached what they found. That was fine while there
was one writer and nothing to invalidate, and it stopped being fine the
moment the background walk started landing rows while a pane was reading
them.

So the store has an interface now, and nothing outside it knows how the rows are held. What that costs is a hop through a trait object on every read; what it buys is that invalidation happens in one place, and that the next change to how rows are stored touches one file rather than eleven — this paragraph is one long line on purpose, the way a pasted one arrives, so the box is seen wrapping text that nobody wrapped for it.

The parts worth knowing about:

- Readers take a snapshot, not a lock. A snapshot is cheap (it clones a
  handle, not the rows) and it never blocks the writer, so a pane that
  is halfway through drawing cannot stall the walk that feeds it.
- Writes go through one method, which is also where the notification is
  raised. There is no way left to change a row without saying so.
- The two caches are gone. Both existed to skip a lookup that is now a
  vector index, and both had a way to go stale that nobody had noticed
  because the tests seeded them in order.
- The index is built once per batch instead of once per row. On the
  reference repository that is the difference between 40ms and 3ms, and
  it is the only reason this is worth doing at all rather than leaving
  the interface for later.

読み手が増えるたびに同じ罠を踏み直していたので、入口を 1 つにまとめた。
ここから先の変更は、行の持ち方を変えても呼ぶ側に出ない。逆に言えば、
この 1 ファイルの外に置き場所を作った時点で同じ話が戻ってくる。

What is deliberately not in here: the on-disk format is untouched, the
walk still produces rows in the same order, and nothing about how the
panes ask for a range has changed. Those are three separate arguments
and this commit is already the wrong size for having any of them in it.
";

/// Where the bulk of one commit lands, and how many files each place
/// takes. Spread over real-looking directories rather than one flat
/// heap: the file list is a tree first, and a tree of one folder is not
/// a tree.
const SPRAWL: [(&str, usize, &str); 6] = [
    ("src/core", 18, "rs"),
    ("src/ui", 14, "rs"),
    ("src/net", 9, "rs"),
    ("tests", 16, "rs"),
    ("docs", 12, "md"),
    ("assets/icons", 11, "svg"),
];

/// A commit nobody can read at a glance, over a tree nobody can scroll at
/// a glance, in a work tree of the same. Everything here is long on
/// purpose: the message runs past any pane, the commit touches 80 files,
/// and the working tree carries 60-odd changes of its own — which is the
/// state the description box's grip, the CHANGES list and the WIP list
/// are all hard to look at without.
fn long(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository with more in every commit than fits on screen.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/core/store.rs", "// the store\n", "feat: add the store")?;
    repo.commit(
        "src/ui/pane.rs",
        "// a pane that reads the store\n",
        "feat: add a pane that reads it",
    )?;

    // A remote, and the ordinary history sent to it before the wall goes
    // on top: a repository with nothing to fetch from reads as a broken
    // window rather than as a preset about something else, and leaving
    // the wall unpushed is what makes sending 80 files something that
    // can be tried here.
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    // The wall: one commit that touches every file in the sprawl.
    for (dir, count, ext) in SPRAWL {
        for i in 0..count {
            repo.write(
                &format!("{dir}/part_{i:02}.{ext}"),
                &format!("// {dir} part {i:02}, before the rewrite\n"),
            )?;
        }
    }
    repo.git(&["add", "--all"])?;
    repo.git(&["commit", "-m", LONG_MESSAGE])?;

    // And a work tree in the same state: most of the sprawl edited, a
    // few files nobody has added yet, one gone, one renamed. The mix is
    // what makes the list worth scrolling — every row is a different
    // icon — but the point of this preset is the length.
    for (dir, count, ext) in SPRAWL {
        for i in (0..count).step_by(4).flat_map(|s| [s, s + 1, s + 2]) {
            if i >= count {
                continue;
            }
            repo.write(
                &format!("{dir}/part_{i:02}.{ext}"),
                &format!("// {dir} part {i:02}, after the rewrite\n\n// and a second thought\n"),
            )?;
        }
    }
    for i in 0..8 {
        repo.write(
            &format!("src/core/sketch_{i:02}.rs"),
            &format!("// sketch {i:02}, not added yet\n"),
        )?;
    }
    std::fs::remove_file(repo.work.join("src/net/part_08.rs"))
        .map_err(|e| format!("removing src/net/part_08.rs: {e}"))?;
    repo.git(&["mv", "docs/part_00.md", "docs/renamed.md"])?;
    Ok(())
}

/// Every string the UI shows, at both ends of what git allows, with
/// Japanese in all of them. Two repositories in one: the short end is a
/// single character everywhere (including a commit with *no* message,
/// which git accepts and reads back empty), the long end sits on the
/// measured wall.
fn edges(repo: &mut DemoRepo) -> Result<(), String> {
    let wall_branch = to_the_byte("b", REF_WALL);
    let wall_tag = to_the_byte("t", REF_WALL);
    let wall_file = to_the_byte("f", PATH_ROOM);
    let long_author = format!("{} <{}@example.com>", pasted(200), to_the_byte("m", 60));

    // A履歴 that is ordinary enough to read, so the extremes stand out.
    repo.commit(
        "README.md",
        "# 端\n\nちょうど端の値だけを集めたリポジトリ。\n",
        "docs: 端の値を集める",
    )?;

    // The short end: one character of everything.
    repo.commit("q", "x\n", "日")?;
    // git takes a commit with no message at all and reads it back empty.
    repo.git(&["commit", "--allow-empty", "--allow-empty-message", "-m", ""])?;
    repo.git(&["commit", "--allow-empty", "--author=日 <あ>", "-m", "一"])?;

    // The long end: a subject nobody meant to write, and a body under it.
    // The body is longer than any pane is tall on purpose: the details
    // pane's description box can be pulled open by its corner, and a body
    // that runs out before the room does never reaches the bound that
    // pull stops at (デザイン規約 §コミットメッセージの 2 つの枠).
    let long_subject = pasted(2000);
    let long_body = format!(
        "{}\n\n{}\n\nCo-authored-by: {} <{}@example.com>\n",
        pasted(2000),
        pasted(3000),
        pasted(120),
        to_the_byte("c", 40)
    );
    repo.write(&wall_file, "端の名前のファイル\n")?;
    repo.git(&["add", "--", &wall_file])?;
    repo.git(&["commit", "-m", &long_subject, "-m", &long_body])?;

    // A deep path, a name with a space, and one that is only Japanese.
    repo.commit(
        "第一階層/第二階層/第三階層/第四階層/深い場所のファイル.txt",
        "deep\n",
        "feat: 深い階層にファイルを置く",
    )?;
    repo.commit("名前に 空白 が入る.txt", "space\n", "feat: 空白入りの名前")?;

    // An author whose name is a paragraph, on its own commit.
    repo.git(&[
        "commit",
        "--allow-empty",
        &format!("--author={long_author}"),
        "-m",
        "chore: 著者名が段落のコミット",
    ])?;

    // Refs at both ends. The one-character branch is non-ASCII on purpose.
    repo.git(&["branch", "あ"])?;
    repo.git(&["branch", &wall_branch])?;
    repo.git(&["tag", "x"])?;
    repo.git(&["tag", "-a", &wall_tag, "-m", &pasted(300)])?;

    // Remotes: an ordinary one, one whose *name* is absurd and which
    // really answers, and one whose *URL* is absurd and is never asked.
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "あ", "x"])?;

    // The wall-length tag drifts away from the long-named remote's copy:
    // push it, then move the local one. A drift is what puts the same
    // name on two chips, and the second chip is the only place in the UI
    // that says *which* remote a reading came from — the last string
    // anywhere that carries a repository's own name for something.
    //
    // Drift rather than remote-only, because a tag a fetch can reach is
    // one auto-follow brings down: only a tag on a commit nobody here
    // has stays away, and such a tag is on no row to hang a chip from
    // (core.md, タグのリモート状態のデータ).
    //
    // And no unreachable remote lives here: `fetch --prune --all` walks
    // every one of them, so a single absurd URL would turn every fetch
    // in this repository into a failure and every fetch verb into a
    // FAIL. A URL has no pane to be too long in anyway.
    // At the wall, like the tag it will qualify: a remote's name is a ref
    // component too, and the two together are what the chip list has to
    // divide a row between. At 90 bytes the pair merely reached the
    // window's edge and proved nothing.
    let long_remote = to_the_byte("r", REF_WALL);
    let bare = repo.root.join("long.git");
    std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
    repo.git_at(&bare.clone(), &["init", "--bare", "-b", "main"])?;
    let long_remote_url = file_url(&bare);
    repo.git(&["remote", "add", &long_remote, &long_remote_url])?;
    repo.git(&["push", &long_remote, "main", &wall_tag])?;
    repo.git(&[
        "commit",
        "--allow-empty",
        "-m",
        "chore: タグを動かす前の一手",
    ])?;
    repo.git(&["tag", "--force", &wall_tag])?;

    // Stashes at both ends.
    repo.write("q", "x\nstashed\n")?;
    repo.git(&["stash", "push", "-m", "日"])?;
    repo.write("q", "x\nstashed again\n")?;
    repo.git(&["stash", "push", "-m", &pasted(400)])?;

    // A dirty tree whose file rows carry the same extremes.
    repo.write(&wall_file, "端の名前のファイル\n編集した行\n")?;
    repo.git(&["add", "--", &wall_file])?;
    repo.write("名前に 空白 が入る.txt", "space\n編集\n")?;
    repo.write(&to_the_byte("u", PATH_ROOM), "untracked\n")?;
    repo.write("z", "")?;
    Ok(())
}

/// Branches, a remote one commit behind, tags in both places, a stash,
/// and a dirty working tree — the state most verbs can act on.
fn basic(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository for kicking tires.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.commit(
        "src/app.txt",
        "app v1\nwith settings\n",
        "feat: read settings",
    )?;
    repo.git(&["tag", "-a", "v0.1", "-m", "first cut"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;
    // Japanese subject and body on purpose: every screenshot of this
    // preset then exercises CJK rendering, and 直 / 骨 make a wrong
    // (Chinese-variant) glyph visible at a glance.
    repo.commit(
        "docs/guide.md",
        "guide v1\n\n## 使い方\n直感的な操作の案内。骨組みだけ先に日本語で書く。\n",
        "docs: 利用案内の骨子を日本語で直す",
    )?;
    repo.commit("src/lib.txt", "lib v2\n", "fix: harden the library")?;

    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.commit("src/topic.txt", "topic ready\n", "feat: finish the topic")?;
    repo.git(&["switch", "main"])?;

    repo.commit(
        "src/app.txt",
        "app v2\nwith settings\n",
        "feat: rework the app",
    )?;
    repo.git(&["tag", "v0.2"])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "feature/topic-a", "v0.1", "v0.2"])?;

    // A branch that exists only on the remote: push it, drop the local.
    repo.git(&["switch", "--create", "feature/remote-only", "main"])?;
    repo.commit("src/remote.txt", "remote work\n", "feat: work kept remote")?;
    repo.git(&["push", "origin", "feature/remote-only"])?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "-D", "feature/remote-only"])?;

    // Ahead of origin/main by one, so push has something to do.
    repo.commit("docs/guide.md", "guide v2\n", "docs: extend the guide")?;
    // A tag only this clone has.
    repo.git(&["tag", "v0.3-local"])?;

    // One stash…
    repo.write("src/lib.txt", "lib v2\nstashed experiment\n")?;
    repo.git(&["stash", "push", "-m", "experiment on the library"])?;
    // …and a dirty working tree: staged, unstaged, untracked. The
    // unstaged edit REPLACES the first line, so the diff's body line 0 is
    // a change — the stage-line / discard-line hooks pick line 0, and a
    // context line there would select nothing.
    repo.write("src/app.txt", "app v2\nwith settings\nstaged line\n")?;
    repo.git(&["add", "--", "src/app.txt"])?;
    repo.write("docs/guide.md", "guide v2, reworded\nunstaged line\n")?;
    repo.write("notes.txt", "untracked scratch\n")?;
    Ok(())
}

/// Every WIP bucket at once: staged, unstaged, staged+unstaged on one
/// file, a staged rename, and an untracked file.
fn dirty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "a v1\n", "feat: a")?;
    repo.commit("b.txt", "b v1\n", "feat: b")?;
    repo.commit("c.txt", "c v1\n", "feat: c")?;
    repo.commit(
        "renamed-from.txt",
        "moves around\n",
        "feat: file that moves",
    )?;

    repo.write("a.txt", "a v1\nstaged\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    repo.write("b.txt", "b v1\nunstaged\n")?;
    repo.write("c.txt", "c v1\nstaged\n")?;
    repo.git(&["add", "--", "c.txt"])?;
    repo.write("c.txt", "c v1\nstaged\nand unstaged on top\n")?;
    repo.git(&["mv", "renamed-from.txt", "renamed-to.txt"])?;
    // Long enough to be read as a file rather than as a line: the diff of
    // something the repository has never seen is all additions under one
    // heading, and one line of it cannot show what that looks like.
    repo.write(
        "untracked.txt",
        "Notes for the next release\n\
         \n\
         - decide what goes in the first tag\n\
         - write the install steps down\n\
         - check the licence headers\n\
         - measure the cold start once more\n\
         \n\
         Nothing here is recorded yet.\n",
    )?;
    Ok(())
}

/// All four line-ending cases at once, in a directory with a house style.
///
/// The neighbours are what makes the two estimated cases sayable: three
/// usable votes are needed, and `flipped.kt` (CRLF in the work tree),
/// `mixed.kt` (mixed) and `bare.kt` (no ending at all) can none of them
/// vote. The four plain ones sort ahead of `flipped.kt`, so the sample is
/// unanimous LF and the notice may say "here".
fn eol(repo: &mut DemoRepo) -> Result<(), String> {
    for name in ["alpha", "beta", "delta", "gamma"] {
        repo.commit(
            &format!("src/{name}.kt"),
            &format!("fun {name}() = \"{name}\"\n"),
            &format!("feat: {name}"),
        )?;
    }
    repo.commit(
        "src/flipped.kt",
        "fun one() = 1\nfun two() = 2\nfun three() = 3\n",
        "feat: three of them",
    )?;
    // Long enough that the untouched lines outnumber the changed one, so
    // the notice can name what the file uses.
    repo.commit(
        "src/mixed.kt",
        "fun a() = 1\nfun b() = 2\nfun c() = 3\nfun d() = 4\nfun e() = 5\n",
        "feat: five of them",
    )?;
    repo.commit("src/bare.kt", "fun bare() = 0", "feat: no ending at all")?;
    // An ordinary change with nothing to say, so "a file is marked" and
    // "this commit carries a marked file" can be told apart.
    repo.commit("src/plain.kt", "fun plain() = 1\n", "feat: an ordinary one")?;

    // (a) every line's ending changes, and nothing else does.
    repo.write(
        "src/flipped.kt",
        "fun one() = 1\r\nfun two() = 2\r\nfun three() = 3\r\n",
    )?;
    // (b) one line lands with the other ending.
    repo.write(
        "src/mixed.kt",
        "fun a() = 1\nfun b() = 2\nfun c() = 3\r\nfun d() = 4\nfun e() = 5\n",
    )?;
    // (d) the file that had none gains its first.
    repo.write("src/bare.kt", "fun bare() = 0\r\n")?;
    repo.write("src/plain.kt", "fun plain() = 1\nfun alsoPlain() = 2\n")?;
    // (c) a file the repository has never seen, disagreeing with its
    // neighbours.
    repo.write(
        "src/fresh.kt",
        "fun fresh() = \"new\"\r\nfun alsoFresh() = \"new\"\r\n",
    )?;
    Ok(())
}

/// A merge stopped on conflicts: MERGE_HEAD present, one unmerged path.
///
/// The file is long enough to be read as one: a conflicted path's diff is
/// the combined form, which puts context, our side, their side and the
/// markers git wrote in one hunk — a one-line file shows none of that.
/// The two sides also disagree about one line and agree about another, so
/// both the fenced part and the part that merged cleanly are on screen.
fn conflict(repo: &mut DemoRepo) -> Result<(), String> {
    const BASE: &str = "\
Release checklist
=================

- pick the version number
- write the notes
- tag the commit
- upload the archives
";
    repo.commit("shared.txt", BASE, "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit(
        "shared.txt",
        &BASE
            .replace("- pick the version number", "- agree the version number")
            .replace(
                "- upload the archives",
                "- upload the archives and checksums",
            ),
        "feat: change shared",
    )?;

    repo.git(&["switch", "main"])?;
    repo.commit(
        "shared.txt",
        &BASE.replace("- pick the version number", "- decide the version number"),
        "fix: change shared too",
    )?;
    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}

/// A rebase stopped part-way, which a stopped merge cannot stand in for:
/// it steps (so it counts `1/2` and takes `--skip` / `--quit`), and the
/// two sides swap over — the commit being replayed is "theirs".
fn rebase_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    // A second commit behind the conflicting one, so the count has
    // somewhere to go and `--skip` has a next step to move to.
    repo.commit("later.txt", "after\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git(&["switch", "feature/clash"])?;
    repo.git_expecting_stop(&["rebase", "main"])?;
    Ok(())
}

/// A drop whose replay goes through and whose *restore* is what collides:
/// the uncommitted edit sits on the line the newest commit rewrote, and on
/// nothing the replay itself has to apply.
///
/// The drop is of HEAD, so the verb needs no argument. Taking that commit
/// out puts the line back the way it was, and the work coming out of the
/// stash changed the same line — two versions of one line, which is the
/// landing a move already has (規約 §未コミット変更がある状態での移動).
fn drop_collides(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- pick the version number\n- tag it\n- upload the archives\n",
        "docs: write the release steps down",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- agree the version number\n- tag it\n- upload the archives\n",
        "docs: reword the first step",
    )?;
    repo.write(
        "docs/release.md",
        "# releasing\n\n- read the version number off the milestone\n- tag it\n\
         - upload the archives\n",
    )?;
    Ok(())
}

/// A drop the commits after it depend on, over a dirty tree: git will not
/// replay while the work is there, and once it is stashed out of the way
/// the replay walks into the hole the drop leaves and stops part-way.
///
/// The commit to take out is `row:2` — the WIP row sits above the newest
/// commit, so the third row of the graph is the second commit back. The
/// uncommitted edit is somewhere else entirely, so it is what makes git
/// refuse without taking any part in what the replay collides over.
fn drop_stops(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n",
        "docs: tag it",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n- upload the archives\n",
        "docs: upload the archives",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n- upload the archives\n- announce it\n",
        "docs: announce it",
    )?;
    repo.write("README.md", "# demo\n\nnotes, still being written\n")?;
    Ok(())
}

/// A branch that has never been sent anywhere, in a repository with two
/// remotes — so the question the first push raises has something to pick
/// between, and a name on the far side to run into.
///
/// `taken` exists on `origin` one commit behind us, which is exactly the
/// case git will not refuse: the push fast-forwards somebody else's
/// branch. The checked-out branch has no upstream at all.
fn unpublished(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    // A second place to send things, so the chooser has a choice to make.
    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;

    // Somebody else's branch, already over there and behind us.
    repo.git(&["switch", "--create", "taken"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: theirs")?;
    // Pushed without `-u`, so nothing here records that it went anywhere.
    repo.git(&["push", "origin", "taken"])?;

    repo.git(&["switch", "main"])?;
    repo.git(&["switch", "--create", "feature/new-thing"])?;
    repo.commit("src/new.txt", "new\n", "feat: draft the new thing")?;
    repo.commit("src/new.txt", "new v2\n", "feat: finish the new thing")?;
    Ok(())
}

/// Commits and nowhere to send them: no remote at all. The first push
/// cannot even pick a destination here — the remote dialog opens by
/// itself, name prefilled `origin`, with the question standing behind it.
fn noremote(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    Ok(())
}

/// The same rebase, one step further on: the conflict resolved and
/// staged, so `--continue` is live and the file list has left CONFLICTS.
fn rebase_staged(repo: &mut DemoRepo) -> Result<(), String> {
    rebase_conflict(repo)?;
    repo.write("shared.txt", "main side\nfeature side\n")?;
    repo.git(&["add", "shared.txt"])?;
    Ok(())
}

/// A rebase stopped on a commit that came out empty — git's own
/// `Otherwise, please use 'git rebase --skip'`. Nothing is conflicted or
/// staged, which is what makes leaving this one out cost nothing.
///
/// Reached with `--empty=stop` rather than through an interactive
/// rebase: the state is the same one, and driving `-i` from here would
/// need a sequence editor on the PATH of three operating systems.
fn rebase_empty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("f.txt", "a\n", "feat: root")?;
    repo.git(&["switch", "--create", "topic"])?;
    repo.commit("f.txt", "a\nX\n", "feat: adds X")?;
    repo.commit("h.txt", "keep\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    // The same net line, arriving with company: not a clean cherry-pick
    // of "adds X", so the cherry-pick filter cannot be what drops it.
    repo.write("f.txt", "a\nX\n")?;
    repo.write("g.txt", "unrelated\n")?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: X arrives with company"])?;
    repo.git(&["switch", "topic"])?;
    repo.git_expecting_stop(&["rebase", "--empty=stop", "main"])?;
    Ok(())
}

/// A cherry-pick stopped on a conflict: it steps the way a rebase does
/// (so it takes `--skip` and `--quit`) but keeps no count, which is what
/// tells the card's two tests apart.
fn cherry_pick_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git_expecting_stop(&["cherry-pick", "feature/clash"])?;
    Ok(())
}

/// A merge stopped on four different kinds of conflict at once, so the
/// rows that name what each side did can be read side by side: both
/// changed it (`UU`), both added it (`AA`), deleted here and changed
/// there (`DU`), changed here and deleted there (`UD`).
fn conflict_kinds(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("both.txt", "base\n", "feat: shared base")?;
    repo.commit("ours-del.txt", "base\n", "feat: one we will drop")?;
    repo.commit("theirs-del.txt", "base\n", "feat: one they will drop")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.write("both.txt", "feature side\n")?;
    repo.write("ours-del.txt", "feature keeps editing\n")?;
    repo.write("added.txt", "feature's new file\n")?;
    std::fs::remove_file(repo.work.join("theirs-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "feat: the feature side of all four"])?;

    repo.git(&["switch", "main"])?;
    repo.write("both.txt", "main side\n")?;
    repo.write("theirs-del.txt", "main keeps editing\n")?;
    repo.write("added.txt", "main's new file\n")?;
    std::fs::remove_file(repo.work.join("ours-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: the main side of all four"])?;

    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}

/// Three stashes; the newest carries an untracked file.
fn stashes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("work.txt", "v1\n", "feat: work")?;
    repo.commit("other.txt", "v1\n", "feat: other")?;
    repo.write("work.txt", "v1\nfirst try\n")?;
    repo.git(&["stash", "push", "-m", "first try"])?;
    repo.write("other.txt", "v1\nsecond angle\n")?;
    repo.git(&["stash", "push", "-m", "second angle"])?;
    repo.write("work.txt", "v1\nthird pass\n")?;
    repo.write("sketch.txt", "untracked sketch\n")?;
    repo.git(&[
        "stash",
        "push",
        "--include-untracked",
        "-m",
        "third, with a sketch",
    ])?;
    Ok(())
}

/// HEAD detached at a tagged commit.
fn detached(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.commit("a.txt", "v3\n", "feat: three")?;
    repo.git(&["tag", "v1.0", "HEAD~1"])?;
    repo.commit("a.txt", "v4\n", "feat: four")?;
    repo.git(&["switch", "--detach", "v1.0"])?;
    Ok(())
}

/// The remote holds commits a fetch would bring in (made by a second
/// clone). This repo has not fetched yet.
fn behind(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    let seeder = repo.root.join("seeder");
    let url = file_url(&repo.root.join("origin.git"));
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    std::fs::write(seeder.join("a.txt"), "v2\nremote work\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "a.txt"])?;
    repo.git_at(
        &seeder.clone(),
        &["commit", "-m", "feat: pushed while you slept"],
    )?;
    repo.git_at(&seeder.clone(), &["push"])?;
    Ok(())
}

/// Both sides moved on, and this repository has already seen it happen:
/// the fetch is part of the preset, so the toolbar offers the overwrite
/// (`push -f`) from the moment the window opens rather than after a verb.
///
/// Break the remote's URL afterwards (`.git/config`) and the overwrite
/// fails without the tracking refs moving — which is how the refused shape
/// of that button gets photographed.
fn diverged(repo: &mut DemoRepo) -> Result<(), String> {
    behind(repo)?;
    repo.git(&["fetch", "origin"])?;
    repo.commit("b.txt", "ours\n", "feat: work of our own")?;
    Ok(())
}

/// Every outcome the details pane can show, and a working tree set up to
/// make one more: a commit signed by a key this repository vouches for
/// (`G`), one signed by a key it has never heard of (`U` — measured, not
/// the `E` one might expect), and one not signed at all. SSH signing is
/// what a throwaway repository can do on its own: a passphrase-less key
/// needs no agent, so no pinentry can appear.
fn signed(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: land before signing was on")?;

    let trusted = keygen(repo, "trusted", "demo@example.com")?;
    // Made, never vouched for: its public line goes nowhere.
    keygen(repo, "stranger", "stranger@example.com")?;

    // Only the first key is vouched for; the other one is a signature
    // git can read but cannot judge.
    let allowed = repo.root.join("allowed_signers");
    std::fs::write(&allowed, format!("demo@example.com {trusted}"))
        .map_err(|e| format!("writing allowed_signers: {e}"))?;
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("gpg.ssh.allowedSignersFile", config_path(&allowed)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }

    let stranger_key = repo.root.join("stranger.pub");
    repo.git(&["config", "user.signingkey", &config_path(&stranger_key)])?;
    repo.commit("b.txt", "v1\n", "feat: signed by a key nobody vouched for")?;

    let trusted_key = repo.root.join("trusted.pub");
    repo.git(&["config", "user.signingkey", &config_path(&trusted_key)])?;
    repo.commit("c.txt", "v1\n", "feat: signed and verified")?;

    // Something to commit, so the editor that says "will be signed" has
    // a reason to be reachable and its button is live.
    repo.write("a.txt", "v1\nabout to be committed\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    Ok(())
}

/// Writes a passphrase-less ed25519 key pair under the demo root and
/// returns the public key's one line.
fn keygen(repo: &DemoRepo, name: &str, comment: &str) -> Result<String, String> {
    let path = repo.root.join(name);
    let out = crate::run_captured(
        Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&path),
    )?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::read_to_string(path.with_extension("pub"))
        .map(|k| k.trim().to_string())
        .map_err(|e| format!("reading {name}.pub: {e}"))
}

/// git reads config values with its own parser, which takes forward
/// slashes on every platform.
fn config_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The one verdict `signed` cannot stage: `E`, a signature git cannot
/// check. SSH signing never answers it — no allowedSignersFile is `N`,
/// a missing or short allowedSignersFile is `U`, tampered bytes and a
/// missing verifier are both `B` (all measured) — so the commit here is
/// OpenPGP-signed, and `E` is what gpg says when the public key is in
/// no keyring it can see. The key was made once, in a throwaway home
/// that no longer exists, and the signed object is carried whole below:
/// building needs no gpg and no key material, and no machine can hold
/// the key, so the verdict cannot drift to `G`. Verifying does need a
/// gpg binary — without one git calls the same commit unsigned
/// (measured: `N`) — which Git for Windows bundles and ci/linux
/// installs.
fn errsig(repo: &mut DemoRepo) -> Result<(), String> {
    // The object names its tree, so the same tree is built first; the
    // hash checks prove nothing drifted, byte for byte.
    repo.write("a.txt", "one\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    let tree = repo.git(&["write-tree"])?;
    if tree != ERRSIG_TREE {
        return Err(format!("errsig tree drifted: {tree}"));
    }
    let commit = repo.git_stdin(
        &["hash-object", "-w", "-t", "commit", "--stdin"],
        ERRSIG_OBJECT,
    )?;
    if commit != ERRSIG_COMMIT {
        return Err(format!("errsig commit drifted: {commit}"));
    }
    repo.git(&["update-ref", "refs/heads/main", &commit])?;
    repo.git(&["reset", "--hard"])?;
    Ok(())
}

const ERRSIG_TREE: &str = "20e50a07feffafe7699bf38ff4027a606f406eaa";
const ERRSIG_COMMIT: &str = "bdc88d46075d5f43d0f8b23a8f48d280769ff273";
/// `git cat-file commit` of the signed commit, escaped a line at a time
/// so the checkout's line endings cannot reach the bytes. The armour's
/// blank line really is `" "` — a space under the `gpgsig` header's
/// continuation indent.
const ERRSIG_OBJECT: &str = concat!(
    "tree 20e50a07feffafe7699bf38ff4027a606f406eaa\n",
    "author demo <demo@example.com> 1767323045 +0000\n",
    "committer demo <demo@example.com> 1767323045 +0000\n",
    "gpgsig -----BEGIN PGP SIGNATURE-----\n",
    " \n",
    " iIcEABYKAC8WIQQdamFUB//cf9AEH47UOMlB1A5vegUCankafxEcZGVtb0BleGFt\n",
    " cGxlLmNvbQAKCRDUOMlB1A5ven70AP9L7BWNVvo87cSiucHqL52AuGc6uD5BI/ad\n",
    " tIXQBS7ncgD9Gsrff1I162MIgFMh+Hr21cNHvfCKTdsPLb2BTp2r5w0=\n",
    " =OTHe\n",
    " -----END PGP SIGNATURE-----\n",
    "\n",
    "feat: sign with a key that is not shipped\n",
);

/// Every state a tag can be in with respect to the remote, so the badge
/// and the name colour can be read side by side (デザイン規約 §グラフ行の
/// ダブルクリック). Nothing shows until a fetch: `ls-remote --tags` is what carries
/// it, so run this preset with the `fetch` verb.
///
/// | tag          | here          | on origin                    |
/// |--------------|---------------|------------------------------|
/// | `v1.0`       | second commit | the same commit              |
/// | `v2.0-local` | HEAD          | nowhere                      |
/// | `v1.5`       | HEAD          | the second commit — a drift  |
/// | `v0.9-theirs`| —             | a commit no branch there has |
///
/// `v0.9-theirs` comes from the seeder on a branch deleted straight after,
/// which is what keeps a fetch from quietly bringing the tag down with it
/// (measured: auto-following only takes tags whose commits it downloads).
fn tags(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# tags\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"])?;
    repo.git(&["tag", "v1.5"])?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "v1.0", "v1.5"])?;

    repo.commit("src/app.txt", "app v2\n", "feat: rework the app")?;
    // Never pushed…
    repo.git(&["tag", "v2.0-local"])?;
    // …and one moved here after it was published, which no fetch undoes.
    repo.git(&["tag", "-f", "v1.5"])?;

    let seeder = repo.root.join("seeder");
    let url = file_url(&repo.root.join("origin.git"));
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    repo.git_at(&seeder.clone(), &["switch", "--create", "gone"])?;
    std::fs::write(seeder.join("side.txt"), "theirs\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "side.txt"])?;
    repo.git_at(&seeder.clone(), &["commit", "-m", "feat: their side"])?;
    repo.git_at(&seeder.clone(), &["tag", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "gone", "v0.9-theirs"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "--delete", "gone"])?;
    Ok(())
}

/// How many tags `manytags` puts on. Enough that TAGS cannot fit in the
/// pane at any window height anybody works at — which is the whole point
/// of the preset, and the shape a release-tagging repository really has
/// (the reference repository carries 45,000).
const MANY_TAGS: usize = 2000;

/// `basic`, buried in tags. What the folded rail does when a section has
/// more rows than the pane is tall can only be read here: with a handful
/// of tags the peek is content-sized and every placement rule looks alike
/// (デザイン規約 §左メニューを畳む).
fn manytags(repo: &mut DemoRepo) -> Result<(), String> {
    basic(repo)?;
    let head = repo.git(&["rev-parse", "HEAD"])?;
    let mut batch = String::new();
    for n in 0..MANY_TAGS {
        // Flat names on purpose: a `/` in a tag name is a folder in the
        // list, and folded folders are fewer rows than tags.
        batch.push_str(&format!(
            "create refs/tags/v1.{}.{} {head}\n",
            n / 100,
            n % 100
        ));
    }
    repo.git_stdin(&["update-ref", "--stdin"], &batch)?;
    Ok(())
}

//! The trees read: one bare repository the primary checkout keeps under
//! `target/relied-messages`, fetched from each supplier through git, and one
//! `git grep` a tree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::table::{Form, Supplier};

const UPSTREAM: &str = "https://github.com/git/git";
const GFW: &str = "https://github.com/git-for-windows/git";
const APPLE: &str = "https://github.com/apple-oss-distributions/Git";
const UBUNTU: &str = "https://git.launchpad.net/ubuntu/+source/git";
const OPENSSH: &str = "https://github.com/openssh/openssh-portable";

/// Where the product's minimum git is written.
const VERSION_RS: &str = "crates/platitude-core/src/version.rs";
const MINIMUM_LINE: &str = "pub const MINIMUM_GIT: (u32, u32) = (";

/// The minimum container's Ubuntu (`linux` reads the same line).
const DOCKERFILE: &str = "ci/linux/Dockerfile";
const UBUNTU_ARG: &str = "ARG UBUNTU=";

/// The name git-ubuntu's branches go by, for each release the container
/// may name.
const CODENAMES: [(&str, &str); 1] = [("24.04", "noble")];

/// The OpenSSH releases read: from 8.0, older than any the supported
/// systems carry (Windows 10's own is 8.1).
const OPENSSH_FROM: (u32, u32) = (8, 0);

/// Each supplier's line of trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Line {
    Upstream,
    Gfw,
    Apple,
    Ubuntu,
    OpenSsh,
}

pub(super) const LINES: [Line; 5] = [
    Line::Upstream,
    Line::Gfw,
    Line::Apple,
    Line::Ubuntu,
    Line::OpenSsh,
];

impl Line {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Upstream => "upstream",
            Self::Gfw => "gfw",
            Self::Apple => "apple",
            Self::Ubuntu => "ubuntu",
            Self::OpenSsh => "openssh",
        }
    }

    pub(super) fn from_word(word: &str) -> Option<Self> {
        LINES.into_iter().find(|line| line.word() == word)
    }

    pub(super) fn supplier(self) -> Supplier {
        match self {
            Self::OpenSsh => Supplier::OpenSsh,
            _ => Supplier::Git,
        }
    }

    /// Where the supplier's tree stands inside the tree fetched.
    fn prefix(self) -> &'static str {
        match self {
            Self::Apple => "src/git/",
            _ => "",
        }
    }

    /// The development head, read as well as the releases; Apple and
    /// Ubuntu publish none, and their newest is their head.
    fn head(self) -> Option<&'static str> {
        match self {
            Self::Upstream | Self::OpenSsh => Some("master"),
            Self::Gfw => Some("main"),
            Self::Apple | Self::Ubuntu => None,
        }
    }

    fn namespace(self) -> String {
        format!("refs/{}", self.word())
    }

    /// The repository of the cache the line is fetched into.
    fn repository(self) -> &'static str {
        match self {
            Self::Upstream | Self::Gfw => "git",
            Self::Apple => "apple",
            Self::Ubuntu => "ubuntu",
            Self::OpenSsh => "openssh",
        }
    }
}

/// What this tree says the lines are read from.
pub(super) struct Lines {
    minimum: (u32, u32),
    codename: String,
}

impl Lines {
    pub(super) fn of_this_tree(root: &Path) -> Result<Self, String> {
        let version = read(root, VERSION_RS)?;
        let minimum = version
            .lines()
            .find_map(|line| line.trim().strip_prefix(MINIMUM_LINE))
            .and_then(|rest| rest.split_once(')'))
            .and_then(|(pair, _)| pair.split_once(','))
            .and_then(|(major, minor)| {
                Some((major.trim().parse().ok()?, minor.trim().parse().ok()?))
            })
            .ok_or_else(|| format!("{VERSION_RS} names no `MINIMUM_GIT`"))?;
        let docker = read(root, DOCKERFILE)?;
        let release = docker
            .lines()
            .find_map(|line| line.trim().strip_prefix(UBUNTU_ARG))
            .map(|release| release.trim().trim_matches('"').to_string())
            .ok_or_else(|| format!("{DOCKERFILE} names no Ubuntu release ({UBUNTU_ARG}…)"))?;
        let codename = CODENAMES
            .iter()
            .find(|(number, _)| *number == release)
            .map(|(_, name)| (*name).to_string())
            .ok_or_else(|| {
                format!(
                    "{DOCKERFILE} names Ubuntu {release}: add its codename to \
                     relied_messages::sources::CODENAMES"
                )
            })?;
        Ok(Self { minimum, codename })
    }

    fn ubuntu_branch(&self) -> String {
        format!("{}-updates", self.codename)
    }

    /// Whether `line`'s release `name` is one read: a release, not a
    /// candidate, from the minimum on.
    fn reads(&self, line: Line, name: &str) -> bool {
        let Some(version) = version_of(line, name) else {
            return false;
        };
        match line {
            Line::Upstream | Line::Gfw => version_at_least(&version, self.minimum),
            Line::OpenSsh => version_at_least(&version, OPENSSH_FROM),
            // Its base is read off the tree.
            Line::Apple => true,
            Line::Ubuntu => false,
        }
    }
}

fn read(root: &Path, file: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))
}

fn version_at_least(version: &[u32], least: (u32, u32)) -> bool {
    (version.first().copied(), version.get(1).copied()) >= (Some(least.0), Some(least.1))
}

/// A release's version, for ordering it in its line; `None` for a name
/// that is no release (a candidate, a tag of something else).
pub(super) fn version_of(line: Line, name: &str) -> Option<Vec<u32>> {
    let numbers = |text: &str, separator: char| -> Option<Vec<u32>> {
        text.split(separator)
            .map(|part| part.parse().ok())
            .collect()
    };
    match line {
        Line::Upstream => numbers(name.strip_prefix('v')?, '.').filter(|v| v.len() == 3),
        Line::Gfw => {
            let (base, build) = name.strip_prefix('v')?.split_once(".windows.")?;
            let mut version = numbers(base, '.').filter(|v| v.len() == 3)?;
            version.push(build.parse().ok()?);
            Some(version)
        }
        Line::Apple => Some(vec![name.strip_prefix("Git-")?.parse().ok()?]),
        Line::OpenSsh => {
            let rest = name.strip_prefix("V_")?;
            let (release, portable) = rest.split_once("_P")?;
            let mut version = numbers(release, '_')?;
            version.push(portable.parse().ok()?);
            Some(version)
        }
        Line::Ubuntu => None,
    }
}

/// One tree read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Tree {
    pub(super) line: Line,
    /// The release's tag, or its head's branch.
    pub(super) name: String,
    /// What git names it by in the cache.
    pub(super) rev: String,
}

impl Tree {
    pub(super) fn label(&self) -> String {
        format!("{} {}", self.line.word(), self.name)
    }
}

/// The bare repositories the trees are fetched into, one a supplier: one
/// that takes a line one commit deep is shallow as a whole, and Launchpad
/// answers a fetch into a shallow repository with nothing for minutes on
/// end. git and Git for Windows share one, as GfW's history is git's.
pub(super) struct Cache {
    root: PathBuf,
}

impl Cache {
    pub(super) fn open() -> Result<Self, String> {
        let root = cache_dir();
        std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
        Ok(Self { root })
    }

    fn dir(&self, line: Line) -> PathBuf {
        self.root.join(format!("{}.git", line.repository()))
    }

    fn git(&self, line: Line, args: &[&str]) -> Result<String, String> {
        let dir = self.dir(line);
        if !dir.join("HEAD").is_file() {
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            git_in(&dir, &["init", "--bare", "--quiet"])?;
        }
        git_in(&dir, args)
    }

    /// Whether `line`'s head is only one commit deep here — or not here
    /// at all — so its line is fetched one deep too: the weekly run's
    /// cache, which never took more.
    fn one_deep(&self, line: Line) -> bool {
        let Some(head) = line.head() else {
            return true;
        };
        let Ok(oid) = self.git(
            line,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{}/{head}", line.namespace()),
            ],
        ) else {
            return true;
        };
        let shallow = std::fs::read_to_string(self.dir(line).join("shallow")).unwrap_or_default();
        shallow
            .lines()
            .any(|boundary| boundary.trim() == oid.trim())
    }

    fn fetch(
        &self,
        line: Line,
        what: &str,
        refspecs: &[String],
        depth: bool,
    ) -> Result<(), String> {
        println!("fetching {what}…");
        let mut args = vec!["fetch", "--quiet", "--no-tags", "--prune"];
        if depth {
            args.push("--depth=1");
        }
        args.push(url_of(line));
        args.extend(refspecs.iter().map(String::as_str));
        self.git(line, &args).map(drop)
    }

    /// Every line whole: each release and head, for the audit — and for
    /// the weekly run's report, in a cache that only ever took the heads
    /// one commit deep, at that depth.
    pub(super) fn fetch_all(&self, lines: &Lines) -> Result<(), String> {
        self.fetch(
            Line::Upstream,
            "git",
            &[
                "+refs/heads/master:refs/upstream/master".to_string(),
                "+refs/tags/v2.*:refs/upstream/tags/v2.*".to_string(),
            ],
            self.one_deep(Line::Upstream),
        )?;
        self.fetch(
            Line::Gfw,
            "Git for Windows",
            &[
                "+refs/heads/main:refs/gfw/main".to_string(),
                "+refs/tags/v2.*:refs/gfw/tags/v2.*".to_string(),
            ],
            self.one_deep(Line::Gfw),
        )?;
        self.fetch(
            Line::OpenSsh,
            "OpenSSH",
            &[
                "+refs/heads/master:refs/openssh/master".to_string(),
                "+refs/tags/V_*:refs/openssh/tags/V_*".to_string(),
            ],
            self.one_deep(Line::OpenSsh),
        )?;
        self.fetch_ubuntu(lines)?;
        self.fetch_apple(lines)
    }

    /// Ubuntu's branch, where the cache does not hold the commit Launchpad
    /// names — moved a few times a year. Whole: Launchpad serves the
    /// branch's history in a minute or two and a single commit of it
    /// not at all, working out where to cut it.
    fn fetch_ubuntu(&self, lines: &Lines) -> Result<(), String> {
        let branch = lines.ubuntu_branch();
        let published = git_here(&[
            "ls-remote",
            UBUNTU,
            &format!("refs/heads/applied/ubuntu/{branch}"),
        ])?;
        let oid = published.split_whitespace().next().unwrap_or_default();
        let held = self
            .git(
                Line::Ubuntu,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/ubuntu/{branch}"),
                ],
            )
            .unwrap_or_default();
        if !oid.is_empty() && held.trim() == oid {
            return Ok(());
        }
        self.fetch(
            Line::Ubuntu,
            "Ubuntu",
            &[format!(
                "+refs/heads/applied/ubuntu/{branch}:refs/ubuntu/{branch}"
            )],
            false,
        )
    }

    /// Apple's newest tags, one deep, until one is older than the minimum
    /// — its base is only known off its tree.
    fn fetch_apple(&self, lines: &Lines) -> Result<(), String> {
        let mut tags = remote_tags(APPLE, Line::Apple)?;
        tags.sort_by_key(|tag| version_of(Line::Apple, tag));
        for tag in tags.iter().rev() {
            let local = format!("refs/apple/tags/{tag}");
            if self
                .git(Line::Apple, &["rev-parse", "--verify", "--quiet", &local])
                .is_err()
            {
                self.fetch(
                    Line::Apple,
                    &format!("Apple {tag}"),
                    &[format!("+refs/tags/{tag}:{local}")],
                    true,
                )?;
            }
            if !self
                .apple_base(&local)
                .is_some_and(|base| version_at_least(&base, lines.minimum))
            {
                break;
            }
        }
        Ok(())
    }

    /// The git release an Apple tree is built from (`DEF_VER`).
    fn apple_base(&self, rev: &str) -> Option<Vec<u32>> {
        let script = self
            .git(
                Line::Apple,
                &["show", &format!("{rev}:src/git/GIT-VERSION-GEN")],
            )
            .ok()?;
        let tag = script
            .lines()
            .find_map(|line| line.trim().strip_prefix("DEF_VER="))?;
        version_of(Line::Upstream, tag.trim())
    }

    /// The development heads, one commit deep in a cache that has none
    /// yet, and Ubuntu's branch where it moved off the commit the ledger
    /// read. Apple publishes tags alone ([`Self::fetch_unread_releases`]).
    pub(super) fn fetch_heads(
        &self,
        lines: &Lines,
        ledger: &super::ledger::Ledger,
    ) -> Result<Vec<Tree>, String> {
        let mut heads = Vec::new();
        for line in [Line::Upstream, Line::Gfw, Line::OpenSsh] {
            let Some(head) = line.head() else { continue };
            let local = format!("{}/{head}", line.namespace());
            self.fetch(
                line,
                &format!("{} {head}", line.word()),
                &[format!("+refs/heads/{head}:{local}")],
                self.one_deep(line),
            )?;
            heads.push(Tree {
                line,
                name: head.to_string(),
                rev: local,
            });
        }
        let branch = lines.ubuntu_branch();
        let published = git_here(&[
            "ls-remote",
            UBUNTU,
            &format!("refs/heads/applied/ubuntu/{branch}"),
        ])?;
        let oid = published.split_whitespace().next().unwrap_or_default();
        if ledger.head_oid(Line::Ubuntu) != Some(oid) {
            self.fetch_ubuntu(lines)?;
            heads.push(Tree {
                line: Line::Ubuntu,
                rev: format!("refs/ubuntu/{branch}"),
                name: branch,
            });
        }
        Ok(heads)
    }

    /// The releases each line has published that the ledger did not read,
    /// fetched one deep.
    pub(super) fn fetch_unread_releases(
        &self,
        lines: &Lines,
        ledger: &super::ledger::Ledger,
    ) -> Result<Vec<Tree>, String> {
        let mut unread = Vec::new();
        for line in [Line::Upstream, Line::Gfw, Line::OpenSsh] {
            for tag in remote_tags(url_of(line), line)? {
                if !lines.reads(line, &tag) || ledger.has_read(line, &tag) {
                    continue;
                }
                let local = format!("{}/tags/{tag}", line.namespace());
                self.fetch(
                    line,
                    &format!("{} {tag}", line.word()),
                    &[format!("+refs/tags/{tag}:{local}")],
                    true,
                )?;
                unread.push(Tree {
                    line,
                    name: tag,
                    rev: local,
                });
            }
        }
        let mut apple = remote_tags(APPLE, Line::Apple)?;
        apple.retain(|tag| !ledger.has_read(Line::Apple, tag));
        apple.sort_by_key(|tag| version_of(Line::Apple, tag));
        // Only those newer than every tag read: an older one unread is
        // older than the minimum.
        let newest_read = ledger
            .newest(Line::Apple)
            .and_then(|tag| version_of(Line::Apple, tag));
        for tag in apple
            .into_iter()
            .filter(|tag| version_of(Line::Apple, tag) > newest_read)
        {
            let local = format!("refs/apple/tags/{tag}");
            self.fetch(
                Line::Apple,
                &format!("Apple {tag}"),
                &[format!("+refs/tags/{tag}:{local}")],
                true,
            )?;
            if self
                .apple_base(&local)
                .is_some_and(|base| version_at_least(&base, lines.minimum))
            {
                unread.push(Tree {
                    line: Line::Apple,
                    name: tag,
                    rev: local,
                });
            }
        }
        Ok(unread)
    }

    /// Every tree the cache holds that is read, each line in its order with
    /// its head last.
    pub(super) fn trees(&self, lines: &Lines) -> Result<Vec<Tree>, String> {
        let mut trees = Vec::new();
        for line in LINES {
            let listed = self.git(
                line,
                &["for-each-ref", "--format=%(refname)", &line.namespace()],
            )?;
            let tags = format!("{}/tags/", line.namespace());
            let mut releases: Vec<(Vec<u32>, Tree)> = listed
                .lines()
                .filter_map(|refname| {
                    let name = refname.strip_prefix(&tags)?;
                    let version = version_of(line, name)?;
                    lines.reads(line, name).then(|| {
                        (
                            version,
                            Tree {
                                line,
                                name: name.to_string(),
                                rev: refname.to_string(),
                            },
                        )
                    })
                })
                .collect();
            releases.sort_by(|a, b| a.0.cmp(&b.0));
            if line == Line::Apple {
                releases.retain(|(_, tree)| {
                    self.apple_base(&tree.rev)
                        .is_some_and(|base| version_at_least(&base, lines.minimum))
                });
            }
            trees.extend(releases.into_iter().map(|(_, tree)| tree));
            let head = match line {
                Line::Ubuntu => Some(lines.ubuntu_branch()),
                _ => line.head().map(str::to_string),
            };
            if let Some(head) = head {
                let rev = format!("{}/{head}", line.namespace());
                if listed.lines().any(|refname| refname == rev) {
                    trees.push(Tree {
                        line,
                        name: head,
                        rev,
                    });
                }
            }
        }
        Ok(trees)
    }

    /// The commit a tree is.
    pub(super) fn oid(&self, tree: &Tree) -> Result<String, String> {
        self.git(
            tree.line,
            &["rev-parse", &format!("{}^{{commit}}", tree.rev)],
        )
        .map(|said| said.trim().to_string())
    }

    /// The package version an Ubuntu tree is (`debian/changelog`'s first
    /// line), and the git release an Apple one is built from.
    pub(super) fn describe(&self, tree: &Tree) -> Option<String> {
        match tree.line {
            Line::Ubuntu => {
                let changelog = self
                    .git(
                        Line::Ubuntu,
                        &["show", &format!("{}:debian/changelog", tree.rev)],
                    )
                    .ok()?;
                let first = changelog.lines().next()?;
                let (_, rest) = first.split_once('(')?;
                let (version, _) = rest.split_once(')')?;
                Some(
                    version
                        .rsplit_once(':')
                        .map_or(version, |(_, v)| v)
                        .to_string(),
                )
            }
            Line::Apple => self
                .apple_base(&tree.rev)
                .map(|v| v.iter().map(u32::to_string).collect::<Vec<_>>().join(".")),
            _ => None,
        }
    }
}

fn url_of(line: Line) -> &'static str {
    match line {
        Line::Upstream => UPSTREAM,
        Line::Gfw => GFW,
        Line::Apple => APPLE,
        Line::Ubuntu => UBUNTU,
        Line::OpenSsh => OPENSSH,
    }
}

/// The release tags a remote publishes for `line`.
fn remote_tags(url: &str, line: Line) -> Result<Vec<String>, String> {
    let said = git_here(&["ls-remote", "--tags", "--refs", url])?;
    Ok(said
        .lines()
        .filter_map(|row| row.split_once('\t'))
        .filter_map(|(_, refname)| refname.strip_prefix("refs/tags/"))
        .filter(|name| version_of(line, name).is_some())
        .map(str::to_string)
        .collect())
}

/// The primary checkout's `target/relied-messages`: one cache every seat shares,
/// as `crate::temp` keeps one tree of throwaways.
fn cache_dir() -> PathBuf {
    if std::env::var_os(crate::linux::IN_CONTAINER).is_some() {
        return crate::temp::root().join("relied-messages");
    }
    let workspace = crate::tree::workspace_root();
    let primary = crate::tree::primary_root(&workspace.to_string_lossy()).unwrap_or(workspace);
    primary.join("target").join("relied-messages")
}

fn git_in(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.arg("--git-dir").arg(dir).args(args);
    answer(command, args)
}

fn git_here(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args);
    answer(command, args)
}

fn answer(mut command: Command, args: &[&str]) -> Result<String, String> {
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "git {} — {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The lines one tree holds for one form: the file, and the line as
/// written, its indent left off. In the order git reads them, repeats kept
/// — a second `die()` of the same words is a change too.
pub(super) type Held = Vec<(String, String)>;

/// What each tree held: one entry a tree, each form's lines in the
/// table's order.
pub(super) struct Read {
    pub(super) tree: Tree,
    pub(super) held: Vec<Held>,
}

/// One `git grep` a tree, for every source of its supplier.
pub(super) fn read_all(cache: &Cache, trees: &[Tree], forms: &[Form]) -> Result<Vec<Read>, String> {
    println!("reading {} trees…", trees.len());
    trees
        .iter()
        .map(|tree| {
            let held = read_tree(cache, tree, forms)?;
            Ok(Read {
                tree: tree.clone(),
                held,
            })
        })
        .collect()
}

fn read_tree(cache: &Cache, tree: &Tree, forms: &[Form]) -> Result<Vec<Held>, String> {
    let supplier = tree.line.supplier();
    let sources: Vec<_> = forms
        .iter()
        .flat_map(|form| form.sources.iter())
        .filter(|source| source.supplier == supplier)
        .collect();
    if sources.is_empty() {
        return Ok(vec![Held::new(); forms.len()]);
    }
    let words: BTreeSet<&str> = sources.iter().map(|source| source.words.as_str()).collect();
    let paths: BTreeSet<String> = sources
        .iter()
        .map(|source| format!("{}{}", tree.line.prefix(), source.path))
        .collect();
    let mut args: Vec<String> = ["grep", "-I", "-i", "-F", "-z"].map(String::from).to_vec();
    for word in &words {
        args.push("-e".to_string());
        args.push((*word).to_string());
    }
    args.push(tree.rev.clone());
    args.push("--".to_string());
    args.extend(paths);
    let mut command = Command::new("git");
    command
        .arg("--git-dir")
        .arg(cache.dir(tree.line))
        .args(&args);
    let output = crate::subprocess::run_captured(&mut command)?;
    // 1 is "nothing matched": a tree that holds none of the words.
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!(
            "git grep in {} — {}",
            tree.label(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let said = String::from_utf8_lossy(&output.stdout);
    let found = grepped(&said, &tree.rev, tree.line.prefix());
    Ok(forms
        .iter()
        .map(|form| {
            found
                .iter()
                .filter(|(path, line)| {
                    form.sources
                        .iter()
                        .any(|source| source.supplier == supplier && source.names(path, line))
                })
                .filter_map(|(path, line)| Some((path.clone(), what_it_says(line)?)))
                .collect()
        })
        .collect())
}

/// `git grep -z`'s rows over a tree: `<rev>:<path>\0<line>`.
fn grepped(said: &str, rev: &str, prefix: &str) -> Held {
    said.lines()
        .filter_map(|row| {
            let (name, line) = row.split_once('\0')?;
            let path = name.strip_prefix(rev)?.strip_prefix(':')?;
            let path = path.strip_prefix(prefix).unwrap_or(path);
            Some((path.to_string(), line.trim().to_string()))
        })
        .collect()
}

/// What a line of C says, as the ledger keeps it: the function it calls
/// (`die` against `error` is the exit code) and its string literals — the
/// words — and nothing of the rest, so a variable renamed or a comment
/// reworded is no change. `None` for a line with no literal: a comment.
pub(super) fn what_it_says(line: &str) -> Option<String> {
    let mut literals: Vec<String> = Vec::new();
    let mut call: Option<String> = None;
    let mut word = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut literal = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            literal.push(c);
                            literal.extend(chars.next());
                        }
                        '"' => break,
                        _ => literal.push(c),
                    }
                }
                literals.push(literal);
                word.clear();
            }
            '\'' => {
                // A character, `'!'` or `'\''`.
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => drop(chars.next()),
                        '\'' => break,
                        _ => {}
                    }
                }
                word.clear();
            }
            '(' => {
                if call.is_none() && literals.is_empty() && !NOT_CALLS.contains(&word.as_str()) {
                    call = (!word.is_empty()).then(|| word.clone());
                }
                word.clear();
            }
            c if c.is_ascii_alphanumeric() || c == '_' => word.push(c),
            _ => word.clear(),
        }
    }
    if literals.is_empty() {
        return None;
    }
    let quoted: Vec<String> = literals.iter().map(|l| format!("\"{l}\"")).collect();
    Some(match call {
        Some(call) => format!("{call}: {}", quoted.join(" ")),
        None => quoted.join(" "),
    })
}

/// What a word before `(` can be that calls nothing worth keeping: the
/// translation markers and C's own.
const NOT_CALLS: [&str; 9] = [
    "_", "N_", "Q_", "if", "while", "for", "switch", "return", "sizeof",
];

#[cfg(test)]
mod tests {
    use super::{Line, grepped, version_of, what_it_says};

    /// Held to the words and the call that says them: a renamed argument
    /// or a reworded comment is no change, `die` turned `error` is one.
    #[test]
    fn a_line_is_kept_as_its_call_and_its_words() {
        assert_eq!(
            what_it_says("die(_(\"'%s' is not a working tree\"), av[0]);").as_deref(),
            Some("die: \"'%s' is not a working tree\"")
        );
        assert_eq!(
            what_it_says("ret = error(_(\"The pre-rebase hook refused to rebase.\"));").as_deref(),
            Some("error: \"The pre-rebase hook refused to rebase.\"")
        );
        assert_eq!(
            what_it_says("print_ref_status('!', \"[rejected]\", ref, NULL,").as_deref(),
            Some("print_ref_status: \"[rejected]\"")
        );
        assert_eq!(
            what_it_says("strbuf_addf(buf, _(\"Unable to create '%s.lock': %s.\\n\\n\"),")
                .as_deref(),
            Some("strbuf_addf: \"Unable to create '%s.lock': %s.\\n\\n\"")
        );
        assert_eq!(
            what_it_says("#define DISPLAY_PREFIX \"remote: \"").as_deref(),
            Some("\"remote: \"")
        );
        assert_eq!(
            what_it_says("if (x) die(_(\"a \\\"b\\\"\"));").as_deref(),
            Some("die: \"a \\\"b\\\"\"")
        );
        assert_eq!(
            what_it_says(" * Returns 1 if there are unstaged changes"),
            None
        );
    }

    #[test]
    fn each_line_orders_its_releases_and_passes_over_the_rest() {
        assert_eq!(version_of(Line::Upstream, "v2.43.7"), Some(vec![2, 43, 7]));
        assert_eq!(version_of(Line::Upstream, "v2.44.0-rc1"), None);
        assert_eq!(
            version_of(Line::Gfw, "v2.55.0.windows.3"),
            Some(vec![2, 55, 0, 3])
        );
        assert_eq!(version_of(Line::Gfw, "v2.55.0"), None);
        assert_eq!(version_of(Line::Apple, "Git-155"), Some(vec![155]));
        assert_eq!(version_of(Line::OpenSsh, "V_9_6_P1"), Some(vec![9, 6, 1]));
        assert_eq!(version_of(Line::OpenSsh, "V_9_6"), None);
    }

    #[test]
    fn a_row_is_its_file_and_its_line_with_the_tree_taken_off() {
        let said = "refs/apple/tags/Git-155:src/git/lockfile.c\0\t\tstrbuf_addf(buf, x);\n";
        assert_eq!(
            grepped(said, "refs/apple/tags/Git-155", "src/git/"),
            [("lockfile.c".to_string(), "strbuf_addf(buf, x);".to_string())]
        );
    }
}

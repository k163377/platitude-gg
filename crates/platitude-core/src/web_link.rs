//! Links to the pages GitHub draws for what a remote holds — a commit, a
//! branch, a tag, a comparison, a file, lines of a file — made from the
//! remote's URL alone, and which commit and which name such a link points
//! at ([`side_at`], [`branch_name_on`]). Nothing is asked of GitHub or of git: the address is read
//! off `remote.<name>.url` as written, so a URL git rewrites on the way
//! out (`url.<base>.insteadOf`) or an ssh host alias is not seen through.
//!
//! Only github.com: an enterprise host is indistinguishable by its name
//! from any other forge.

use crate::oid::Oid;

/// The repository's page on GitHub (`https://github.com/<owner>/<repo>`)
/// for a remote URL that names one; `None` for anything else.
///
/// Takes the shapes git takes for a host: `https://`, `http://`, `ssh://`,
/// `git://`, `git+ssh://` / `ssh+git://` (user and port left out), and the
/// scp-like `[user@]host:owner/repo`. A trailing `.git` or `/` goes. The
/// owner and repository are kept as written — GitHub answers to either
/// case.
pub fn github_repo(url: &str) -> Option<String> {
    let (host, path) = host_and_path(url.trim())?;
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "github.com" | "www.github.com" | "ssh.github.com"
    ) {
        return None;
    }
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    if !is_name(owner) || !is_name(repo) {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}"))
}

/// The host and the path after it. The scp-like form is git's only where
/// no `/` comes before the first `:` — otherwise it is a local path
/// (`git help clone`, GIT URLS).
fn host_and_path(url: &str) -> Option<(&str, &str)> {
    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "https" | "http" | "ssh" | "git" | "git+ssh" | "ssh+git"
        ) {
            return None;
        }
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        let host = host.split_once(':').map_or(host, |(h, _)| h);
        return Some((host, path));
    }
    let (before, path) = url.split_once(':')?;
    if before.contains('/') {
        return None;
    }
    Some((before.rsplit_once('@').map_or(before, |(_, h)| h), path))
}

/// What GitHub lets an owner or a repository be called: letters, digits,
/// `-`, `_`, `.` — and never `.` or `..`, which a path would read as a step.
fn is_name(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// A commit's page.
pub fn commit(repo: &str, oid: &Oid) -> String {
    format!("{repo}/commit/{}", oid.to_hex())
}

/// A branch's page: its files at its tip, under the name it has on that
/// remote.
pub fn branch(repo: &str, name: &str) -> String {
    format!("{repo}/tree/{}", path_part(name))
}

/// A tag's page: the release made from it, or the tag itself where none is.
pub fn tag(repo: &str, name: &str) -> String {
    format!("{repo}/releases/tag/{}", path_part(name))
}

/// The difference between two commits as `git diff <from> <to>` takes it —
/// GitHub's two-dot comparison. Its default, three dots, measures from the
/// merge base instead.
pub fn compare(repo: &str, from: &Oid, to: &Oid) -> String {
    format!("{repo}/compare/{}..{}", from.to_hex(), to.to_hex())
}

/// `path` as it stands in commit `at`, drawn the way GitHub draws it (a
/// Markdown file rendered).
pub fn file(repo: &str, at: &Oid, path: &str) -> String {
    format!("{repo}/blob/{}/{}", at.to_hex(), path_part(path))
}

/// Lines `first..=last` of `path` as it stands in commit `at` — GitHub's
/// permalink, which a GitHub comment unfolds into the lines themselves. A
/// file GitHub draws rendered (Markdown and the rest of its markup, a
/// notebook, a table) has no line numbers unless asked for its source.
pub fn lines(repo: &str, at: &Oid, path: &str, first: u32, last: u32) -> String {
    let anchor = if last > first {
        format!("L{first}-L{last}")
    } else {
        format!("L{first}")
    };
    let plain = if drawn_rendered(path) { "?plain=1" } else { "" };
    format!(
        "{repo}/blob/{}/{}{plain}#{anchor}",
        at.to_hex(),
        path_part(path)
    )
}

/// Whether GitHub shows the file rendered rather than as lines: the
/// extensions github/markup renders (`markup/markdown.rb`, `markups.rb`,
/// reStructuredText's `.rst.txt` among them) and the files GitHub draws
/// itself — notebooks, tables, maps, 3D models, diagrams and SVG
/// (docs.github.com, "Working with non-code files").
fn drawn_rendered(path: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    if leaf.ends_with(".rst.txt") || leaf.ends_with(".rest.txt") {
        return true;
    }
    let Some((_, ext)) = leaf.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext,
        "md" | "markdown"
            | "mdown"
            | "mkdn"
            | "mkd"
            | "mdwn"
            | "mdx"
            | "litcoffee"
            | "textile"
            | "rdoc"
            | "org"
            | "creole"
            | "mediawiki"
            | "wiki"
            | "rst"
            | "rest"
            | "asciidoc"
            | "adoc"
            | "asc"
            | "pod"
            | "pod6"
            | "ipynb"
            | "csv"
            | "tsv"
            | "geojson"
            | "topojson"
            | "stl"
            | "mermaid"
            | "mmd"
            | "svg"
    )
}

/// What a diff is between, as the commit card shows it: one commit
/// against its first parent (none for a root commit), or two commits
/// compared as `git diff <from> <to>` takes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOf {
    Commit { commit: Oid, parent: Option<Oid> },
    Pair { from: Oid, to: Oid },
}

/// Where a link to one side of a diff points: `at`, the commit whose file
/// that side is, and `held`, the commit a remote has to have for the page
/// to be there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideAt {
    pub held: Oid,
    pub at: Oid,
}

/// The newer side of a commit's diff is the commit; the older is its
/// parent, which every remote holding the commit has. Two compared, each
/// side is its own commit, which a remote has to have itself. `None` for
/// the older side of a root commit — there is no file before it.
pub fn side_at(diff: DiffOf, newer: bool) -> Option<SideAt> {
    match diff {
        DiffOf::Commit { commit, .. } if newer => Some(SideAt {
            held: commit,
            at: commit,
        }),
        DiffOf::Commit { commit, parent } => parent.map(|parent| SideAt {
            held: commit,
            at: parent,
        }),
        DiffOf::Pair { from, to } => {
            let side = if newer { to } else { from };
            Some(SideAt {
                held: side,
                at: side,
            })
        }
    }
}

/// The name a branch goes by on `remote`, for its page there: the name of
/// the remote branch it is measured against where that is on `remote`
/// (`upstream`, cut into remote and name), else `own` — a local branch's
/// own name, or a remote branch's name on its own remote.
pub fn branch_name_on<'a>(
    own: &'a str,
    upstream: Option<(&str, &'a str)>,
    remote: &str,
) -> &'a str {
    match upstream {
        Some((on, name)) if on == remote => name,
        _ => own,
    }
}

/// Which of the remotes holding something a menu leads with: `preferred`
/// where it holds it, else the first in the order given (name order).
/// `None` where nobody does.
pub fn lead<'a>(preferred: &str, holders: &'a [String]) -> Option<&'a str> {
    holders
        .iter()
        .find(|h| *h == preferred)
        .or_else(|| holders.first())
        .map(String::as_str)
}

/// A ref name or a path for a URL's path: `/` stays a separator, every
/// byte outside the unreserved set is percent-encoded — a `#` or `?` in a
/// branch name would otherwise end the path.
fn path_part(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = "https://github.com/octo/hello-world";

    fn oid(c: char) -> Oid {
        Oid::from_hex_str(&c.to_string().repeat(40)).expect("test oid")
    }

    #[test]
    fn every_shape_git_takes_for_github_names_the_same_page() {
        for url in [
            "https://github.com/octo/hello-world",
            "https://github.com/octo/hello-world.git",
            "https://github.com/octo/hello-world/",
            "http://github.com/octo/hello-world.git",
            "https://someone:token@github.com/octo/hello-world.git",
            "https://www.github.com/octo/hello-world",
            "https://GitHub.com/octo/hello-world",
            "git@github.com:octo/hello-world.git",
            "git@github.com:/octo/hello-world.git",
            "github.com:octo/hello-world",
            "ssh://git@github.com/octo/hello-world.git",
            "ssh://git@github.com:22/octo/hello-world",
            "ssh://git@ssh.github.com:443/octo/hello-world.git",
            "git+ssh://git@github.com/octo/hello-world.git",
            "ssh+git://git@github.com/octo/hello-world.git",
            "git://github.com/octo/hello-world.git",
            "  https://github.com/octo/hello-world.git\n",
        ] {
            assert_eq!(github_repo(url).as_deref(), Some(REPO), "{url}");
        }
    }

    #[test]
    fn the_names_are_kept_as_written() {
        assert_eq!(
            github_repo("git@github.com:Octo/Hello.World_2.git").as_deref(),
            Some("https://github.com/Octo/Hello.World_2")
        );
    }

    #[test]
    fn anything_but_a_github_repository_names_none() {
        for url in [
            "https://gitlab.com/octo/hello-world.git",
            "https://github.example.com/octo/hello-world.git",
            "https://notgithub.com/octo/hello-world.git",
            "git@github-work:octo/hello-world.git",
            "file:///tmp/octo/hello-world.git",
            "/srv/git/github.com:octo/hello-world",
            "C:\\repos\\hello-world",
            "../hello-world",
            "https://github.com/octo",
            "https://github.com/octo/hello-world/tree/main",
            "https://github.com/octo/hello world",
            "https://github.com/octo/..",
            "ftp://github.com/octo/hello-world",
            "",
        ] {
            assert_eq!(github_repo(url), None, "{url}");
        }
    }

    #[test]
    fn each_page_has_its_address() {
        assert_eq!(
            commit(REPO, &oid('a')),
            format!("{REPO}/commit/{}", "a".repeat(40))
        );
        assert_eq!(
            branch(REPO, "feature/login"),
            format!("{REPO}/tree/feature/login")
        );
        assert_eq!(tag(REPO, "v1.2.0"), format!("{REPO}/releases/tag/v1.2.0"));
        assert_eq!(
            compare(REPO, &oid('a'), &oid('b')),
            format!("{REPO}/compare/{}..{}", "a".repeat(40), "b".repeat(40))
        );
        assert_eq!(
            lines(REPO, &oid('c'), "src/main.rs", 10, 20),
            format!("{REPO}/blob/{}/src/main.rs#L10-L20", "c".repeat(40))
        );
        assert_eq!(
            lines(REPO, &oid('c'), "src/main.rs", 7, 7),
            format!("{REPO}/blob/{}/src/main.rs#L7", "c".repeat(40))
        );
        assert_eq!(
            file(REPO, &oid('c'), "docs/README.md"),
            format!("{REPO}/blob/{}/docs/README.md", "c".repeat(40)),
            "a whole file is drawn as GitHub draws it, rendered"
        );
    }

    #[test]
    fn a_name_that_would_end_the_path_is_encoded() {
        assert_eq!(
            branch(REPO, "fix/#12?q=a b%"),
            format!("{REPO}/tree/fix/%2312%3Fq%3Da%20b%25")
        );
        assert_eq!(
            branch(REPO, "日本"),
            format!("{REPO}/tree/%E6%97%A5%E6%9C%AC")
        );
        assert_eq!(
            lines(REPO, &oid('c'), "docs/a b.txt", 1, 1),
            format!("{REPO}/blob/{}/docs/a%20b.txt#L1", "c".repeat(40))
        );
    }

    #[test]
    fn a_file_drawn_rendered_is_asked_for_its_source() {
        assert_eq!(
            lines(REPO, &oid('c'), "docs/README.MD", 3, 4),
            format!(
                "{REPO}/blob/{}/docs/README.MD?plain=1#L3-L4",
                "c".repeat(40)
            )
        );
        assert_eq!(
            lines(REPO, &oid('c'), "md/main.rs", 3, 3),
            format!("{REPO}/blob/{}/md/main.rs#L3", "c".repeat(40)),
            "a folder's name is not an extension"
        );
        assert_eq!(
            lines(REPO, &oid('c'), "Makefile", 3, 3),
            format!("{REPO}/blob/{}/Makefile#L3", "c".repeat(40))
        );
        for path in ["docs/a.mdx", "guide.rst.txt", "map.geojson", "flow.mmd"] {
            assert!(
                lines(REPO, &oid('c'), path, 1, 1).ends_with("?plain=1#L1"),
                "{path}"
            );
        }
        assert!(
            lines(REPO, &oid('c'), "notes.txt", 1, 1).ends_with(".txt#L1"),
            "a plain .txt is lines"
        );
    }

    #[test]
    fn a_side_of_a_commits_diff_is_the_commit_or_its_parent() {
        let diff = DiffOf::Commit {
            commit: oid('a'),
            parent: Some(oid('b')),
        };
        assert_eq!(
            side_at(diff, true),
            Some(SideAt {
                held: oid('a'),
                at: oid('a')
            })
        );
        assert_eq!(
            side_at(diff, false),
            Some(SideAt {
                held: oid('a'),
                at: oid('b')
            }),
            "the file before the commit, on the remotes the commit is on"
        );
        let root = DiffOf::Commit {
            commit: oid('a'),
            parent: None,
        };
        assert_eq!(side_at(root, false), None, "a root commit has no before");
    }

    #[test]
    fn a_side_of_two_compared_is_its_own_commit() {
        let pair = DiffOf::Pair {
            from: oid('a'),
            to: oid('b'),
        };
        assert_eq!(
            side_at(pair, true),
            Some(SideAt {
                held: oid('b'),
                at: oid('b')
            })
        );
        assert_eq!(
            side_at(pair, false),
            Some(SideAt {
                held: oid('a'),
                at: oid('a')
            })
        );
    }

    #[test]
    fn a_branch_goes_by_its_upstreams_name_on_its_upstreams_remote() {
        let upstream = Some(("origin", "feature/login"));
        assert_eq!(branch_name_on("login", upstream, "origin"), "feature/login");
        assert_eq!(
            branch_name_on("login", upstream, "fork"),
            "login",
            "elsewhere by its own name"
        );
        assert_eq!(branch_name_on("login", None, "origin"), "login");
    }

    #[test]
    fn the_preferred_remote_leads_where_it_holds_it() {
        let holders = vec![
            "fork".to_string(),
            "origin".to_string(),
            "upstream".to_string(),
        ];
        assert_eq!(lead("origin", &holders), Some("origin"));
        assert_eq!(
            lead("mirror", &holders),
            Some("fork"),
            "else the first by name"
        );
        assert_eq!(lead("origin", &[]), None);
    }
}

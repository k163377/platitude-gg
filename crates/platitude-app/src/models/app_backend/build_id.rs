/// What the right pane's corner names this binary by (デザイン規約 §アプリ名).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildId {
    /// The tag the crate's version is released under, built at it or not.
    pub tag: String,
    /// Built at the commit that tag names.
    pub release: bool,
    /// The commit built from, at the length the window spells hashes; empty
    /// for a release, whose tag already names it, and where git could not say
    /// (build.rs).
    pub commit: String,
}

pub fn build_id() -> BuildId {
    id_of(
        env!("CARGO_PKG_VERSION"),
        env!("PLATITUDE_BUILD_COMMIT"),
        env!("PLATITUDE_RELEASE_COMMIT"),
    )
}

fn id_of(version: &str, head: &str, tagged: &str) -> BuildId {
    let release = !head.is_empty() && head == tagged;
    BuildId {
        tag: format!("v{version}"),
        release,
        commit: if release {
            String::new()
        } else {
            head.get(..8).unwrap_or(head).to_string()
        },
    }
}

/// The worktree this binary was built in, empty when it was not built in
/// one — the right pane's corner names it so parallel sessions' windows
/// can be told apart (CLAUDE.md §ビルド・テスト). Read from the build
/// path, so a build from a plain checkout carries no mark.
pub fn build_tree() -> String {
    tree_of(&env!("CARGO_MANIFEST_DIR").replace('\\', "/")).to_string()
}

/// The worktree directory named in `path`, which separates with `/`. Empty
/// when the path names none.
fn tree_of(path: &str) -> &str {
    const WORKTREES: &str = "/.claude/worktrees/";
    let Some(at) = path.find(WORKTREES).map(|at| at + WORKTREES.len()) else {
        return "";
    };
    let rest = &path[at..];
    &rest[..rest.find('/').unwrap_or(rest.len())]
}

#[cfg(test)]
mod tests {
    use super::{BuildId, id_of, tree_of};

    const HEAD: &str = "c7b549aaecbf57ca932322cb87c5798c111ea6b8";

    #[test]
    fn a_build_at_its_versions_tag_is_the_release() {
        assert_eq!(
            id_of("0.1.0", HEAD, HEAD),
            BuildId {
                tag: "v0.1.0".into(),
                release: true,
                commit: String::new(),
            }
        );
    }

    #[test]
    fn any_other_build_is_a_snapshot_of_the_tag_to_come() {
        let snapshot = BuildId {
            tag: "v0.1.0".into(),
            release: false,
            commit: "c7b549aa".into(),
        };
        // No such tag yet, and a tag that names another commit.
        assert_eq!(id_of("0.1.0", HEAD, ""), snapshot);
        assert_eq!(
            id_of("0.1.0", HEAD, "0c86d42a7da25d5a4499b16e6f229eb4b04cbebe"),
            snapshot
        );
    }

    #[test]
    fn a_build_git_could_not_read_is_never_the_release() {
        assert_eq!(
            id_of("0.1.0", "", ""),
            BuildId {
                tag: "v0.1.0".into(),
                release: false,
                commit: String::new(),
            }
        );
    }

    #[test]
    fn names_the_worktree_a_build_came_from() {
        assert_eq!(
            tree_of(
                "C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/labels/crates/platitude-app"
            ),
            "labels"
        );
        assert_eq!(
            tree_of("/home/x/platitude-gg/.claude/worktrees/perf/crates/platitude-app"),
            "perf"
        );
    }

    #[test]
    fn leaves_every_other_build_unmarked() {
        // The primary checkout, and anyone else's plain checkout.
        assert_eq!(
            tree_of("C:/Users/x/IdeaProjects/platitude-gg/crates/platitude-app"),
            ""
        );
        assert_eq!(
            tree_of("/build/platitude-gg-0.0.0/crates/platitude-app"),
            ""
        );
    }
}

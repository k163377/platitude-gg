/// The worktree this binary was built in, empty when it was not built in
/// one. Parallel sessions each build their own exe and any of them may put
/// a window on the screen, and the windows are otherwise identical — this
/// is what the corner of the right pane says to tell them apart, beside
/// the git version (CLAUDE.md ビルド・テスト).
///
/// It comes from the build path, not from the environment: the exe lives
/// in the tree that built it, so the mark travels with the file however it
/// is started, and a build from a plain checkout — every build anyone
/// outside this repository makes — carries none at all.
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
    use super::tree_of;

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
        // The primary checkout, and the plain checkout anyone outside this
        // repository builds from — neither carries a mark into what ships.
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

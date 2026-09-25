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

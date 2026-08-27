//! What a tab is called: as much of its path as it takes to tell it
//! apart from the other tabs in the strip (デザイン規約 §タブの所作).
//!
//! A rule over the whole strip rather than over one path, so it lives
//! away from the model that applies it: a name is only ever ambiguous
//! against the names standing beside it, and the same folder is called
//! one thing on its own and another once its namesake is opened.

use std::collections::HashMap;
use std::path::{Component, MAIN_SEPARATOR_STR, Path};

/// Names every open repository, in the order they were handed over.
///
/// Each starts at its own folder name and grows a parent at a time —
/// **the whole group that shares a name grows together**, so two
/// repositories called `repo` both come out as `<parent>/repo` rather
/// than one of them keeping the bare word (デザイン規約 §タブの所作). A
/// group that is still ambiguous a parent up grows again, which is what
/// takes `1/foo/repo` and `2/foo/repo` down to the level they differ at.
///
/// A name that nobody shares never grows: the strip is read for the
/// repositories in it, and a path standing in a tab that has no namesake
/// is answering a question nobody asked.
pub(super) fn names_for(paths: &[&str]) -> Vec<String> {
    let parts: Vec<Vec<String>> = paths.iter().map(|path| segments(path)).collect();
    // One segment each to begin with — the folder the repository is in.
    // A path with no named segment at all (a drive root) has nothing to
    // grow and answers with itself throughout.
    let mut depth: Vec<usize> = parts.iter().map(|s| usize::from(!s.is_empty())).collect();
    loop {
        let mut sharing: HashMap<String, Vec<usize>> = HashMap::new();
        for (at, segments) in parts.iter().enumerate() {
            sharing
                .entry(name_at(segments, depth[at], paths[at]))
                .or_default()
                .push(at);
        }
        let mut grew = false;
        for group in sharing.into_values() {
            if group.len() < 2 {
                continue;
            }
            for at in group {
                // Two tabs on one path cannot happen — the strip moves to
                // the tab already holding a repository rather than opening
                // a second (`TabsModel::position_of`) — but a group that
                // cannot grow is what ends the loop rather than looping it.
                if depth[at] < parts[at].len() {
                    depth[at] += 1;
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    parts
        .iter()
        .enumerate()
        .map(|(at, segments)| name_at(segments, depth[at], paths[at]))
        .collect()
}

/// The last `depth` segments of a path, joined the way the platform
/// spells one.
///
/// A name grown as far as the path itself answers with **the path as it
/// was given** rather than with its segments put back together: the
/// pieces have lost the root they hung off, and `home/ada/repo` is not a
/// place. Nothing shorter can lose anything — every segment above the
/// cut is still there to be read in the hover (デザイン規約 §hover のツールチップ).
fn name_at(segments: &[String], depth: usize, whole: &str) -> String {
    if depth == 0 || depth >= segments.len() {
        return whole.to_string();
    }
    segments[segments.len() - depth..].join(MAIN_SEPARATOR_STR)
}

/// The named parts of a path, root and separators dropped.
///
/// Read through `Path` rather than split on a character, so each platform
/// says for itself what separates one segment from the next: a backslash
/// is a folder boundary on Windows and an ordinary letter of a name
/// everywhere else. The prefix comes along as a segment of its own —
/// `C:` against `D:` is the only thing telling two drives' `repo` apart.
fn segments(path: &str) -> Vec<String> {
    Path::new(path.trim())
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir | Component::CurDir | Component::ParentDir => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::names_for;

    /// Joined the way the platform spells a path, so the expectations
    /// below read as paths on the machine running them.
    fn path(parts: &[&str]) -> String {
        parts.join(std::path::MAIN_SEPARATOR_STR)
    }

    #[test]
    fn a_name_nobody_shares_is_the_folder_and_nothing_more() {
        let one = path(&["", "home", "ada", "platitude-gg"]);
        let two = path(&["", "home", "ada", "notes"]);
        assert_eq!(
            names_for(&[&one, &two]),
            vec!["platitude-gg".to_string(), "notes".to_string()]
        );
    }

    #[test]
    fn two_repositories_of_one_name_both_show_their_parent() {
        let one = path(&["", "src", "foo", "repo"]);
        let two = path(&["", "src", "bar", "repo"]);
        assert_eq!(
            names_for(&[&one, &two]),
            vec![path(&["foo", "repo"]), path(&["bar", "repo"])]
        );
    }

    /// The parent is shared too, so the pair goes on up to the level
    /// they differ at — and stops there.
    #[test]
    fn a_shared_parent_grows_the_pair_one_level_further() {
        let one = path(&["", "src", "1", "foo", "repo"]);
        let two = path(&["", "src", "2", "foo", "repo"]);
        assert_eq!(
            names_for(&[&one, &two]),
            vec![path(&["1", "foo", "repo"]), path(&["2", "foo", "repo"])]
        );
    }

    /// Only what is ambiguous grows: a tab standing on a name of its own
    /// keeps it however far its neighbours have to go.
    #[test]
    fn the_tab_beside_them_keeps_its_bare_name() {
        let one = path(&["", "src", "1", "foo", "repo"]);
        let two = path(&["", "src", "2", "foo", "repo"]);
        let three = path(&["", "src", "solo"]);
        assert_eq!(
            names_for(&[&one, &two, &three]),
            vec![
                path(&["1", "foo", "repo"]),
                path(&["2", "foo", "repo"]),
                "solo".to_string()
            ]
        );
    }

    /// A group splits as soon as its members stop reading alike: the two
    /// under `foo` need a third level, and the one that never shared a
    /// parent is done at the second.
    #[test]
    fn a_group_stops_growing_the_moment_it_comes_apart() {
        let one = path(&["", "src", "a", "foo", "repo"]);
        let two = path(&["", "src", "b", "foo", "repo"]);
        let three = path(&["", "src", "bar", "repo"]);
        assert_eq!(
            names_for(&[&one, &two, &three]),
            vec![
                path(&["a", "foo", "repo"]),
                path(&["b", "foo", "repo"]),
                path(&["bar", "repo"])
            ]
        );
    }

    /// Grown as far as the path goes, the name is the path as it was
    /// given — root, drive letter and the spelling it arrived in.
    #[test]
    fn a_name_grown_to_the_whole_path_is_that_path() {
        let one = path(&["", "one", "repo"]);
        let two = path(&["", "two", "repo"]);
        assert_eq!(names_for(&[&one, &two]), vec![one.clone(), two.clone()]);
    }

    #[test]
    fn an_empty_strip_is_named_in_no_time() {
        assert!(names_for(&[]).is_empty());
    }

    /// A trailing separator names the same folder, so it names the same
    /// tab — the path is trimmed of it the way `repo_key` is.
    #[test]
    fn a_trailing_separator_does_not_make_a_name_of_its_own() {
        let bare = path(&["", "src", "repo"]);
        let slashed = format!("{bare}{}", std::path::MAIN_SEPARATOR);
        assert_eq!(names_for(&[&slashed]), vec!["repo".to_string()]);
    }

    /// Two drives, one folder name: the prefix is a segment, so the
    /// letters are what the pair is told apart by.
    #[cfg(windows)]
    #[test]
    fn two_drives_are_told_apart_by_their_letters() {
        assert_eq!(
            names_for(&[r"C:\repo", r"D:\repo"]),
            vec![r"C:\repo".to_string(), r"D:\repo".to_string()]
        );
    }
}

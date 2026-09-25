//! What a tab is called: as much of its path as it takes to tell it
//! apart from the other tabs in the strip (デザイン規約 §タブの所作).
//! A rule over the whole strip, so it lives away from the model that
//! applies it.

use std::collections::HashMap;
use std::path::{Component, Path};

/// Names every open repository, in the order they were handed over.
///
/// Each starts at its folder name; every group sharing a name grows a
/// parent at a time together until it comes apart, and a name nobody
/// shares stays put.
pub(super) fn names_for(paths: &[&str]) -> Vec<String> {
    let parts: Vec<Vec<String>> = paths.iter().map(|path| segments(path)).collect();
    // A drive root has no named segment: it starts at 0 and answers
    // with itself throughout.
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
                // A group that cannot grow is what ends the loop.
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

/// The last `depth` segments of a path, joined with `/` on every
/// platform (デザイン規約 §パスの区切り).
///
/// Grown as far as the path itself, it answers with the path as given:
/// rejoined pieces drop the root (`home/ada/repo` is not a place).
fn name_at(segments: &[String], depth: usize, whole: &str) -> String {
    if depth == 0 || depth >= segments.len() {
        return whole.to_string();
    }
    segments[segments.len() - depth..].join("/")
}

/// The named parts of a path, root and separators dropped.
///
/// Read through `Path`: a backslash is a folder boundary on Windows and
/// a letter of a name elsewhere. Only the reading follows the platform,
/// not the joining ([`name_at`]). The prefix is a segment of its own —
/// `C:` against `D:` is all that tells two drives' `repo` apart.
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

    /// An input path in the host's separator; the expected names stay
    /// `/` whichever that is.
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
            vec!["foo/repo".to_string(), "bar/repo".to_string()]
        );
    }

    #[test]
    fn a_shared_parent_grows_the_pair_one_level_further() {
        let one = path(&["", "src", "1", "foo", "repo"]);
        let two = path(&["", "src", "2", "foo", "repo"]);
        assert_eq!(
            names_for(&[&one, &two]),
            vec!["1/foo/repo".to_string(), "2/foo/repo".to_string()]
        );
    }

    #[test]
    fn the_tab_beside_them_keeps_its_bare_name() {
        let one = path(&["", "src", "1", "foo", "repo"]);
        let two = path(&["", "src", "2", "foo", "repo"]);
        let three = path(&["", "src", "solo"]);
        assert_eq!(
            names_for(&[&one, &two, &three]),
            vec![
                "1/foo/repo".to_string(),
                "2/foo/repo".to_string(),
                "solo".to_string()
            ]
        );
    }

    #[test]
    fn a_group_stops_growing_the_moment_it_comes_apart() {
        let one = path(&["", "src", "a", "foo", "repo"]);
        let two = path(&["", "src", "b", "foo", "repo"]);
        let three = path(&["", "src", "bar", "repo"]);
        assert_eq!(
            names_for(&[&one, &two, &three]),
            vec![
                "a/foo/repo".to_string(),
                "b/foo/repo".to_string(),
                "bar/repo".to_string()
            ]
        );
    }

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

    #[test]
    fn a_trailing_separator_does_not_make_a_name_of_its_own() {
        let bare = path(&["", "src", "repo"]);
        let slashed = format!("{bare}{}", std::path::MAIN_SEPARATOR);
        assert_eq!(names_for(&[&slashed]), vec!["repo".to_string()]);
    }

    #[cfg(windows)]
    #[test]
    fn a_grown_name_is_spelled_with_forward_slashes() {
        assert_eq!(
            names_for(&[r"C:\src\foo\repo", r"C:\src\bar\repo"]),
            vec!["foo/repo".to_string(), "bar/repo".to_string()]
        );
    }

    #[cfg(windows)]
    #[test]
    fn two_drives_are_told_apart_by_their_letters() {
        assert_eq!(
            names_for(&[r"C:\repo", r"D:\repo"]),
            vec![r"C:\repo".to_string(), r"D:\repo".to_string()]
        );
    }
}

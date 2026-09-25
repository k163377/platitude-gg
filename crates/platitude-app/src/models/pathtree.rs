//! Filing `a/b/c.txt` paths under the directories they name, for both
//! file lists. The leaf is the caller's: the details pane copies its rows
//! in, the sidebar hangs an index.

use std::collections::BTreeMap;

/// One directory of a path tree, and whatever the caller left in it.
pub(super) struct DirNode<T> {
    dirs: BTreeMap<String, DirNode<T>>,
    files: Vec<T>,
}

// Not derived: `#[derive(Default)]` would require `T: Default`.
impl<T> Default for DirNode<T> {
    fn default() -> Self {
        Self {
            dirs: BTreeMap::new(),
            files: Vec::new(),
        }
    }
}

impl<T> DirNode<T> {
    /// Files `path` under the directories it names, with the leaf `make`
    /// builds from the byte offset where the row's own name begins (also
    /// how far a rename's source is cut back beside it).
    ///
    /// A path ending in `/` is itself the leaf and keeps its `/`: git's
    /// spelling of an embedded repository, one entry even under `-uall`
    /// (`status::load`).
    pub(super) fn insert(&mut self, path: &str, make: impl FnOnce(usize) -> T) {
        let mut node = self;
        let mut rest = path.strip_suffix('/').unwrap_or(path);
        let mut from = 0;
        while let Some((dir, tail)) = rest.split_once('/') {
            node = node.dirs.entry(dir.to_string()).or_default();
            from += dir.len() + 1;
            rest = tail;
        }
        node.files.push(make(from));
    }

    /// The directories under this one in display order, as (label, node
    /// the label ends at). A chain with nothing beside it compacts into one
    /// `a/b/c` row.
    pub(super) fn folders(&self) -> impl Iterator<Item = (String, &Self)> {
        self.dirs.iter().map(|(name, child)| {
            let mut label = name.clone();
            let mut target = child;
            while target.files.is_empty() && target.dirs.len() == 1 {
                let Some((next_name, next)) = target.dirs.iter().next() else {
                    break;
                };
                label.push('/');
                label.push_str(next_name);
                target = next;
            }
            (label, target)
        })
    }

    /// The leaves sitting in this directory itself.
    pub(super) fn files(&self) -> &[T] {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree as one line per row, folders as `label/` and leaves as
    /// what `insert` left on them, indented by depth.
    fn drawn(node: &DirNode<(String, usize)>, depth: usize) -> Vec<String> {
        let mut out = Vec::new();
        for (label, target) in node.folders() {
            out.push(format!("{}{label}/", "  ".repeat(depth)));
            out.extend(drawn(target, depth + 1));
        }
        for (path, from) in node.files() {
            out.push(format!("{}{}", "  ".repeat(depth), &path[*from..]));
        }
        out
    }

    fn tree(paths: &[&str]) -> DirNode<(String, usize)> {
        let mut root = DirNode::default();
        for path in paths {
            root.insert(path, |from| ((*path).to_string(), from));
        }
        root
    }

    #[test]
    fn a_leaf_is_told_where_its_own_name_begins() {
        let root = tree(&["docs/guide/intro.md"]);
        let folders: Vec<String> = root.folders().map(|(label, _)| label).collect();
        assert_eq!(folders, vec!["docs/guide"]);
        assert_eq!(drawn(&root, 0), vec!["docs/guide/", "  intro.md"]);
    }

    #[test]
    fn a_file_at_the_root_has_nothing_cut_off_it() {
        let root = tree(&["README.md"]);
        assert_eq!(root.files(), [("README.md".to_string(), 0)]);
        assert_eq!(root.folders().count(), 0);
    }

    #[test]
    fn a_chain_with_nothing_beside_it_becomes_one_row() {
        assert_eq!(
            drawn(&tree(&["a/b/c/deep.txt"]), 0),
            vec!["a/b/c/", "  deep.txt"]
        );
    }

    #[test]
    fn a_chain_stops_compacting_where_it_forks() {
        assert_eq!(
            drawn(&tree(&["a/b/one.txt", "a/c/two.txt"]), 0),
            vec!["a/", "  b/", "    one.txt", "  c/", "    two.txt"]
        );
    }

    #[test]
    fn a_directory_holding_a_file_of_its_own_keeps_its_row() {
        // `a` cannot join `a/b` into one row: the row would then have to
        // show `here.txt` under a label that is no longer its directory.
        assert_eq!(
            drawn(&tree(&["a/here.txt", "a/b/deeper.txt"]), 0),
            vec!["a/", "  b/", "    deeper.txt", "  here.txt"]
        );
    }

    #[test]
    fn a_directory_git_would_not_open_is_a_row_and_not_a_shelf() {
        // An embedded repository: git's one entry `vendor/nest/`.
        assert_eq!(
            drawn(&tree(&["vendor/nest/", "vendor/plain.txt"]), 0),
            vec!["vendor/", "  nest/", "  plain.txt"]
        );
        assert_eq!(drawn(&tree(&["nest/"]), 0), vec!["nest/"]);
    }

    #[test]
    fn directories_come_before_the_files_beside_them_and_sort_by_name() {
        assert_eq!(
            drawn(
                &tree(&["zeta.txt", "b/two.txt", "a/one.txt", "alpha.txt"]),
                0
            ),
            vec![
                "a/",
                "  one.txt",
                "b/",
                "  two.txt",
                "zeta.txt",
                "alpha.txt",
            ],
            "files keep the order they arrived in; directories are sorted"
        );
    }
}

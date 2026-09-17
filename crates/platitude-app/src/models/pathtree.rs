//! Filing `a/b/c.txt` paths under the directories they name, the way both
//! file lists show them.
//!
//! The leaf is whatever the caller hangs on it, so the two readers keep
//! what tells them apart: the details pane copies its rows in, and the
//! sidebar hangs an index and points at the row it already has.

use std::collections::BTreeMap;

/// One directory of a path tree, and whatever the caller left in it.
pub(super) struct DirNode<T> {
    dirs: BTreeMap<String, DirNode<T>>,
    files: Vec<T>,
}

// Written out: a directory is empty whatever the leaf is, and
// `#[derive(Default)]` would ask the leaf to have one too.
impl<T> Default for DirNode<T> {
    fn default() -> Self {
        Self {
            dirs: BTreeMap::new(),
            files: Vec::new(),
        }
    }
}

impl<T> DirNode<T> {
    /// Files `path` under the directories it names and leaves the leaf
    /// `make` builds at the end of it.
    ///
    /// `make` is handed how many bytes of the path the folders above the
    /// row already spell — which is where the row's own name begins, and
    /// how far a rename's source is cut back beside it.
    ///
    /// **A path that ends in `/` is itself the leaf.** git spells a
    /// directory it will not open that way — a repository of its own
    /// inside the working copy, whose files belong to that repository and
    /// so stay one entry even under `-uall` (`status::load`). The last `/`
    /// therefore stays on the row's own
    /// name.
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

    /// The directories under this one in display order, each as the label
    /// it shows and the node that label ends at.
    ///
    /// A chain of directories with nothing beside it compacts into one
    /// `a/b/c` row: the rows in between would each hold a single arrow and
    /// nothing else.
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
        // What git answers with for a repository of its own sitting in the
        // working copy: the one entry `vendor/nest/`, beside the files of
        // the directory it shares. The row is that directory.
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

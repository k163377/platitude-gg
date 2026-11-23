//! The CHANGES list as a tree, and the rebuild of the display rows.

use super::*;

/// Turns flat changed-file entries into an indented tree: directories
/// first (alphabetical), single-child directory chains compacted into one
/// row (`a/b/c`), leaves labeled by their last segment.
fn build_file_tree(raw: &[FileItem], overrides: &HashMap<String, bool>) -> Vec<FileItem> {
    let mut root = DirNode::default();
    for entry in raw {
        root.insert(&entry.path, |cut| {
            let mut leaf = entry.clone();
            leaf.name = entry.path[cut..].to_string();
            // A rename's source gives up the same folder prefix, when it
            // has it.
            leaf.orig_name =
                crate::encode::rename_source(&entry.orig_path, &entry.path, cut).to_string();
            leaf
        });
    }
    fn emit(
        node: &DirNode<FileItem>,
        prefix: &str,
        depth: i32,
        overrides: &HashMap<String, bool>,
        out: &mut Vec<FileItem>,
    ) {
        for (label, target) in node.folders() {
            let key = format!("{prefix}{label}");
            let expanded = overrides.get(&key).copied().unwrap_or(true);
            out.push(FileItem {
                name: label,
                path: key.clone(),
                depth,
                folder: true,
                collapsed: !expanded,
                ..Default::default()
            });
            if expanded {
                emit(target, &format!("{key}/"), depth + 1, overrides, out);
            }
        }
        for f in node.files() {
            let mut item = f.clone();
            item.depth = depth;
            out.push(item);
        }
    }
    let mut out = Vec::new();
    emit(&root, "", 0, overrides, &mut out);
    out
}

/// The rows the tree stands at with every folder open, counted up to two — what the details pane keeps of the list
/// however short (`DetailsPane.listFloor`). Read off the files alone, so a folder opened or shut, or the view turned
/// between tree and paths, leaves it where it is and the list does not move under the hand that pressed: two files
/// are two rows in either view, one file one row at the top or two under its folder.
pub(super) fn floor_rows(raw: &[FileItem]) -> i32 {
    match raw {
        [] => 0,
        [one] if !one.path.contains('/') => 1,
        _ => 2,
    }
}

impl DetailsModel {
    pub(super) fn rebuild_rows(&mut self) {
        self.files = if self.tree_view {
            build_file_tree(&self.raw_files, &self.folder_overrides)
        } else {
            self.raw_files.clone()
        };
    }
}

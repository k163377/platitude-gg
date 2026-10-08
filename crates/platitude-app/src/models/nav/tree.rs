use super::*;

impl NavSectionModel {
    /// Trees one group run of file rows. Fold keys are group-prefixed so
    /// equal paths in different groups fold apart.
    pub(super) fn file_tree_into(
        &self,
        run: std::ops::Range<usize>,
        group: &str,
        out: &mut Vec<Arranged>,
    ) {
        // Leaves are (source index, name start): nothing is copied.
        let mut root: DirNode<(u32, u32)> = DirNode::default();
        for at in run {
            let Some(of) = self.all.entry(at) else {
                continue;
            };
            root.insert(of.full(), |from| (at as u32, from as u32));
        }
        fn emit(
            node: &DirNode<(u32, u32)>,
            group: &str,
            prefix: &str,
            depth: i32,
            overrides: &HashMap<String, bool>,
            out: &mut Vec<Arranged>,
        ) {
            for (label, target) in node.folders() {
                let path = format!("{prefix}{label}");
                let key = format!("{group}:{path}");
                let expanded = overrides.get(&key).copied().unwrap_or(true);
                out.push(Arranged::Made(Box::new(NavItem {
                    name: label,
                    full: key.clone(),
                    // `full` is the fold key; the path itself rides in
                    // the rename slot (see `NavItem::orig_path`).
                    orig_path: path.clone(),
                    group: group.to_string(),
                    depth,
                    folder: true,
                    change: fold_state(expanded),
                    ..Default::default()
                })));
                if expanded {
                    emit(
                        target,
                        group,
                        &format!("{path}/"),
                        depth + 1,
                        overrides,
                        out,
                    );
                }
            }
            for (at, from) in node.files() {
                out.push(Arranged::At {
                    at: *at,
                    depth,
                    from: *from,
                });
            }
        }
        emit(&root, group, "", 0, &self.folder_overrides, out);
    }

    /// Remote roots start collapsed except under a filter
    /// (`build_remote_groups`); everything else starts open.
    /// `toggle_folder` reads this same default, so the first click on an
    /// open row folds it.
    pub(super) fn folder_expanded(&self, key: &str, depth: i32) -> bool {
        self.folder_overrides
            .get(key)
            .copied()
            .unwrap_or(!(self.section == "remotes" && depth == 0 && self.filter.is_empty()))
    }

    /// The REMOTES under a filter: a row per remote with a match, open by
    /// default under the tree's own fold key (`folder_expanded`), its
    /// branches flat under it (デザイン規約 §左メニューの所作
    /// 「絞り込み中も REMOTES は remote ごとの行を残す」). A remote is a place,
    /// not a folder of names: no name cut takes its row out. A ref with no
    /// slash stands flat, ahead of the groups.
    pub(super) fn build_remote_groups(&self, needle: &str) -> Vec<Arranged> {
        let Source::Remotes(snapshot) = &self.all else {
            return Vec::new();
        };
        let remotes: Vec<&str> = snapshot
            .remote_names
            .iter()
            .map(|name| name.as_str())
            .collect();
        // Groups in first-met order, looked up rather than closed when the
        // name moves on: `my`'s branches sort on both sides of `my/fork`'s.
        let mut groups: Vec<(&str, Vec<Arranged>)> = Vec::new();
        let mut out = Vec::new();
        for at in 0..self.all.len() {
            let Some(leaf) = self.all.entry(at).filter(|_| !self.hidden_at(at)) else {
                continue;
            };
            let name = leaf.name();
            if !name.to_lowercase().contains(needle) {
                continue;
            }
            let Some((remote, branch)) = platitude_core::refs::split_remote_ref_or_first_slash(
                name,
                remotes.iter().copied(),
            ) else {
                out.push(Arranged::At {
                    at: at as u32,
                    depth: 0,
                    from: 0,
                });
                continue;
            };
            let row = Arranged::At {
                at: at as u32,
                depth: 1,
                from: (name.len() - branch.len()) as u32,
            };
            match groups.iter_mut().find(|(of, _)| *of == remote) {
                Some((_, rows)) => rows.push(row),
                None => groups.push((remote, vec![row])),
            }
        }
        for (remote, rows) in groups {
            let expanded = self.folder_expanded(remote, 0);
            out.push(Arranged::Made(Box::new(NavItem {
                name: remote.to_string(),
                full: remote.to_string(),
                depth: 0,
                folder: true,
                change: fold_state(expanded),
                ..Default::default()
            })));
            if expanded {
                out.extend(rows);
            }
        }
        out
    }

    /// Turns the flat sorted name list into an indented tree with a
    /// collapsible folder row per `/` level. Leaves point at source rows;
    /// the segment shown falls out of the depth (`shown_name`).
    pub(super) fn build_tree(&self) -> Vec<Arranged> {
        self.build_tree_with(|key, depth| self.folder_expanded(key, depth))
    }

    /// The same tree, with `open` deciding the folds — the section's own
    /// (`build_tree`), or all open (`card`).
    pub(super) fn build_tree_with(&self, open: impl Fn(&str, i32) -> bool) -> Vec<Arranged> {
        let mut out = Vec::new();
        let mut open_path: Vec<String> = Vec::new();
        // Depth at which a collapsed folder swallows its descendants.
        let mut collapsed_at: Option<usize> = None;

        for at in 0..self.all.len() {
            // Hidden leaves are skipped before their folders open, so a
            // folder whose every leaf is hidden goes too.
            let Some(leaf) = self.all.entry(at).filter(|_| !self.hidden_at(at)) else {
                continue;
            };
            let name = leaf.name();
            let segments: Vec<&str> = name.split('/').collect();
            let folder_count = segments.len() - 1;

            // Longest common folder prefix with the previous entry.
            let mut common = 0;
            while common < open_path.len()
                && common < folder_count
                && open_path[common] == segments[common]
            {
                common += 1;
            }
            open_path.truncate(common);
            if let Some(depth) = collapsed_at
                && depth >= open_path.len()
            {
                collapsed_at = None;
            }

            for (depth, segment) in segments.iter().enumerate().take(folder_count).skip(common) {
                open_path.push((*segment).to_string());
                if collapsed_at.is_some() {
                    continue;
                }
                let key = open_path.join("/");
                let expanded = open(&key, depth as i32);
                out.push(Arranged::Made(Box::new(NavItem {
                    name: (*segment).to_string(),
                    full: key,
                    depth: depth as i32,
                    folder: true,
                    change: fold_state(expanded),
                    ..Default::default()
                })));
                if !expanded {
                    collapsed_at = Some(depth);
                }
            }
            if collapsed_at.is_none() {
                out.push(Arranged::At {
                    at: at as u32,
                    depth: folder_count as i32,
                    from: (name.len() - segments[folder_count].len()) as u32,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    #[test]
    fn the_remote_tree_points_at_rows_and_folds_over_them() {
        let mut model = section(
            "remotes",
            Source::Remotes(snapshot(
                vec![remote("origin/feature/one"), remote("origin/main")],
                Vec::new(),
            )),
        );
        // Remote roots start collapsed, so the folder row is all there is.
        model.arrange();
        assert_eq!(model.shown_rows(), 1);
        assert_eq!(says(&model, 0, Role::Name), "origin");
        assert!(flags(&model, 0, Role::Folder));
        assert_eq!(says(&model, 0, Role::Change), FOLDED);

        model.folder_overrides.insert("origin".to_string(), true);
        model.arrange();
        // origin / feature / one / main
        assert_eq!(model.shown_rows(), 4);
        assert_eq!(says(&model, 1, Role::Name), "feature");
        assert_eq!(says(&model, 2, Role::Name), "one");
        assert_eq!(says(&model, 2, Role::Full), "origin/feature/one");
        assert_eq!(depth_of(&model, 2), 2);
        assert_eq!(says(&model, 3, Role::Name), "main");
        assert_eq!(says(&model, 3, Role::Full), "origin/main");
        assert_eq!(depth_of(&model, 3), 1);
        assert_eq!(says(&model, 3, Role::OidHex), oid("b").to_hex());

        assert_eq!(
            model.oid_of_name("origin/main".to_string()),
            oid("b").to_hex()
        );
        assert_eq!(model.name_at(2), "one");
    }
    #[test]
    fn the_file_tree_folds_each_run_of_its_own() {
        let mut model = section("files", Source::files(pending()));
        model.tree_view = true;
        model.arrange();

        // conflicts: a.txt / unstaged: src, b.txt, c.txt / staged: src,
        // b.txt, d.txt
        assert_eq!(model.shown_rows(), 7);
        assert_eq!(says(&model, 0, Role::Name), "a.txt");
        assert_eq!(says(&model, 1, Role::Name), "src");
        assert!(flags(&model, 1, Role::Folder));
        // The fold key is run-prefixed; the plain path rides beside it.
        assert_eq!(says(&model, 1, Role::Full), "unstaged:src");
        assert_eq!(says(&model, 1, Role::OrigPath), "src");
        assert_eq!(says(&model, 2, Role::Name), "b.txt");
        assert_eq!(says(&model, 2, Role::Full), "src/b.txt");
        assert_eq!(depth_of(&model, 2), 1);
        assert_eq!(says(&model, 3, Role::Name), "c.txt");
        assert_eq!(says(&model, 4, Role::Full), "staged:src");

        // Folding one run leaves the other alone.
        model
            .folder_overrides
            .insert("unstaged:src".to_string(), false);
        model.arrange();
        assert_eq!(model.shown_rows(), 6);
        assert_eq!(says(&model, 1, Role::Change), FOLDED);
        assert_eq!(says(&model, 2, Role::Name), "c.txt");
    }

    /// An embedded repository arrives as the one entry `vendor/nest/`
    /// (`status_integration`): a leaf row, not a folder over a nameless
    /// row.
    #[test]
    fn a_directory_git_would_not_open_is_one_row() {
        use platitude_core::status::{StatusItem, WorktreeStatus};
        let untracked = |path: &str| StatusItem::Untracked {
            path: path.to_string(),
        };
        let mut model = section(
            "files",
            Source::files(WorktreeStatus {
                items: vec![untracked("vendor/nest/"), untracked("vendor/plain.txt")],
                ..Default::default()
            }),
        );
        model.tree_view = true;
        model.arrange();

        assert_eq!(model.shown_rows(), 3);
        assert_eq!(says(&model, 0, Role::Name), "vendor");
        assert!(flags(&model, 0, Role::Folder));
        assert_eq!(says(&model, 1, Role::Name), "nest/");
        assert!(!flags(&model, 1, Role::Folder));
        assert_eq!(says(&model, 1, Role::Full), "vendor/nest/");
        assert_eq!(depth_of(&model, 1), 1);
        assert_eq!(says(&model, 2, Role::Name), "plain.txt");
    }
}

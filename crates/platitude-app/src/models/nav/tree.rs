use super::*;

impl NavSectionModel {
    /// Trees one group run of file rows: single-child directory chains
    /// compact into one `a/b/c` row; fold-toggle keys are group-prefixed
    /// so equal paths in different groups fold apart.
    pub(super) fn wt_tree_into(
        &self,
        run: std::ops::Range<usize>,
        group: &str,
        out: &mut Vec<Arranged>,
    ) {
        // The leaves are the source rows themselves: an index and where in
        // its path the file's own name begins, so nothing is copied.
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

    /// Section default: remote roots (one per remote) start collapsed —
    /// that is the per-repository fold — everything else starts open.
    /// **Under a filter a remote's row starts open**: what the filter
    /// found is what the reader asked to see (`build_remote_groups`).
    /// The toggle reads the same default (`toggle_folder`), so the first
    /// click on that row folds it rather than "opening" an open row.
    pub(super) fn folder_expanded(&self, key: &str, depth: i32) -> bool {
        self.folder_overrides
            .get(key)
            .copied()
            .unwrap_or(!(self.section == "remotes" && depth == 0 && self.filter.is_empty()))
    }

    /// The REMOTES under a filter: every remote with a branch that
    /// answers keeps a row of its own, open, and those branches stand
    /// flat under it by their branch names — `feature/topic-a` under
    /// `origin` (デザイン規約 §左メニューの所作「絞り込み中も REMOTES は remote
    /// ごとの行を残す」). A remote is where a branch is rather than a folder
    /// of its name: its row is what the right-click and the push mark
    /// stand on, and no name drawn in this panel begins with it, so no
    /// cut ever takes it out.
    ///
    /// The match is still read off the whole name (`origin/feat` finds
    /// the row). The cut between the two halves is by configured name,
    /// and by the first slash while no configured name owns the ref
    /// (`refs::split_remote_ref_or_first_slash` — the names may not have
    /// arrived, or the remote may be gone from configuration). A ref
    /// with no slash at all stands flat, ahead of the groups.
    ///
    /// **Open by default, and foldable.** A filter that hid what it found
    /// would answer "nothing" for "something", so the row starts open
    /// here whatever the tree's default is; the arrow folds it the way
    /// the tree's rows fold, and the override carries over into the tree
    /// (`folder_expanded`).
    pub(super) fn build_remote_groups(&self, needle: &str) -> Vec<Arranged> {
        let Source::Remotes(snapshot) = &self.all else {
            return Vec::new();
        };
        let remotes: Vec<&str> = snapshot
            .remote_names
            .iter()
            .map(|name| name.as_str())
            .collect();
        // The groups in the order their first branch is met. The rows are
        // name-ordered, and a remote called `my` beside one called
        // `my/fork` has its branches on both sides of the other's, so a
        // group is looked up rather than closed when the name moves on.
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

    /// Turns the flat sorted name list into an indented tree with
    /// collapsible folder rows for every `/` level.
    ///
    /// The leaves are pointed at: what a row of the tree adds to the name
    /// the source holds is its depth, and the segment it shows falls out
    /// of that (`shown_name`).
    pub(super) fn build_tree(&self) -> Vec<Arranged> {
        let mut out = Vec::new();
        let mut open_path: Vec<String> = Vec::new();
        // Depth at which a collapsed folder swallows its descendants.
        let mut collapsed_at: Option<usize> = None;

        for at in 0..self.all.len() {
            // Skipped before its folders are opened: the rows that carry
            // a `/` are the only thing that puts a folder row on screen,
            // so a leaf shown as gone takes with it any folder it was the
            // last of — and leaves the ones it shared standing, because
            // the next leaf opens those itself.
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
                let expanded = self.folder_expanded(&key, depth as i32);
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
                    // One folder per `/`, so the segment on show starts
                    // after the last of them.
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
        let mut model = section("worktree", Source::files(pending()));
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

    /// git will not open a repository sitting in the working copy: what it
    /// hands over is the one entry `vendor/nest/`, whatever it was asked
    /// about untracked files (`status_integration`). That entry is a row —
    /// a folder row with a nameless row under it would offer to open what
    /// there is nothing to put in.
    #[test]
    fn a_directory_git_would_not_open_is_one_row() {
        use platitude_core::status::{StatusItem, WorkTreeStatus};
        let untracked = |path: &str| StatusItem::Untracked {
            path: path.to_string(),
        };
        let mut model = section(
            "worktree",
            Source::files(WorkTreeStatus {
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

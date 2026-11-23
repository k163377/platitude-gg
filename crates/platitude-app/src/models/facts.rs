//! Pure rules QML asks by value: every slot reads only its arguments (the
//! exception app-ui.md grants the `GitFacts` singleton), so a slot that
//! reads state has no place here.
//!
//! Adding an argument to a slot means counting every QML caller
//! (`grep -rn "<slotName>" --include=*.qml`): nothing type-checks the
//! call, and a call one argument short leaves the single line
//! `Error: Insufficient arguments` and kills the calling function there.

use std::ops::RangeInclusive;

use platitude_core::offers::{FarSide, RowStanding, TreeStanding, UpstreamPlace};
use qtbridge::qobject;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::qml_register;
use crate::encode::{Chips, Fields, Mates, One, Optional, Record, RefMenu, WorktreeMaking, field};
use crate::models::nav::{CardRecord, RowLead};

/// The singleton every QML file can ask a rule of without wiring a model
/// through.
#[derive(Default)]
pub struct GitFacts;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl GitFacts {
    /// How many hex characters the find bar needs before it reads a line as
    /// an object name (`find::MIN_OID_PREFIX`) — the number its hint says.
    #[qslot]
    fn oid_floor(&self) -> i32 {
        platitude_core::find::MIN_OID_PREFIX as i32
    }

    /// Whether a hex id is the WIP row's all-zero sentinel
    /// (`Oid::zero_like`). Empty is "no id at all".
    #[qslot]
    fn wip_oid(&self, hex: String) -> bool {
        platitude_core::oid::Oid::hex_is_zero(&hex)
    }

    /// Whether the discard log's filter line is in any of the words an
    /// entry shows (`encode::shown_matches`).
    #[qslot]
    fn shown_matches(&self, shown: Vec<String>, line: String) -> bool {
        crate::encode::shown_matches(&shown, &line)
    }

    /// Whether a discard of `paths` leaves something the discard record does
    /// not keep — no record before the first commit, and a path the status
    /// names `not_copied` is left out of the copy
    /// (`discards::recorded_whole`). Such a discard is held in `danger`, the
    /// rest in `warning` (デザイン規約 §回数押し).
    #[qslot]
    fn discard_unrecorded(
        &self,
        unborn: bool,
        not_copied: Vec<String>,
        paths: Vec<String>,
    ) -> bool {
        !platitude_core::discards::recorded_whole(unborn, &not_copied, &paths)
    }

    /// `chips` less the ones `gone` names by key — asked where the chips are
    /// drawn (`GraphRowDelegate`, `GraphHeadPin`), not by a role: the rows
    /// are lent out (`QListModel::get`), and `GraphRowItem` is at its
    /// 15-field cap.
    #[qslot]
    fn chips_shown(&self, chips: Chips, gone: Vec<String>) -> Chips {
        crate::encode::chips_shown(&chips, &gone)
    }

    /// The ref kind a chip's word names: `branch` / `remote` / `tag`, or
    /// `""` for the two markers, which name nothing to act on.
    #[qslot]
    fn ref_kind(&self, kind: String) -> String {
        crate::encode::ref_kind_word(&kind).to_string()
    }

    /// What a right-click on a chip of that word aims the menu at, and what a
    /// spaced second click renames: the ref kinds, and `worktree` for the
    /// chip of a worktree with no branch (named by its folder) — a worktree
    /// has a menu (its WORKTREE card) and a folder to rename
    /// (`worktree_rename`), where it is no ref to switch to (`ref_kind`). `""`
    /// for the detached HEAD.
    #[qslot]
    fn named_kind(&self, kind: String) -> String {
        crate::encode::named_kind_word(&kind).to_string()
    }

    /// What the WORKTREE card offers for one worktree (`RefWorktreeMenu.standOn`):
    /// `state` is the worktree's row word (`models::nav::item` — `MAIN` /
    /// `LOCKED` / `PRUNABLE` / empty), `here` whether the tab stands in it.
    #[qslot]
    fn worktree_card_offers(&self, state: String, here: bool) -> Vec<String> {
        platitude_core::offers::worktree_card(
            state == crate::models::nav::MAIN,
            state == crate::models::nav::LOCKED,
            here,
        )
        .words()
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    /// What the spaced second click does on a worktree on no branch
    /// (`offers::worktree_rename`): `box`, `locked`, `gone`, or empty for
    /// nothing. `state` is the worktree's row word, as above.
    #[qslot]
    fn worktree_rename(&self, state: String) -> String {
        platitude_core::offers::worktree_rename(
            state == crate::models::nav::MAIN,
            state == crate::models::nav::LOCKED,
            state == crate::models::nav::PRUNABLE,
        )
        .word()
        .to_string()
    }

    /// The remote half of a remote-tracking name (`origin/main`), as the
    /// writes read it (`TrackedBranch::listed`): the longest of
    /// `remote_names` wins (a remote's name may contain `/`); where none
    /// owns the ref the first slash answers, so the gesture still acts and
    /// git refuses loudly — the list may still be loading, or the remote
    /// gone while its refs remain. `""` only where there is no slash at
    /// all.
    #[qslot]
    fn remote_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        platitude_core::remote::TrackedBranch::listed(&full, names(&remote_names))
            .map(|row| row.far().remote.clone())
            .unwrap_or_default()
    }

    /// The branch half of the same cut; a name no slash divides is all
    /// branch (the shape a rename box is typed in).
    #[qslot]
    fn branch_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        match platitude_core::remote::TrackedBranch::listed(&full, names(&remote_names)) {
            Some(row) => row.far().branch.clone(),
            None => full,
        }
    }

    /// The name a branch called `name` is listed under on the remote the
    /// row `full` is on (`origin/main`, `renamed` → `origin/renamed`) — the
    /// new side a replace, or a rename carried over, shows and checks
    /// before the write that sends it there reads the row the same way
    /// (`TrackedBranch::beside`). `""` where `full` names no remote.
    #[qslot]
    fn remote_ref_beside(&self, full: String, name: String, remote_names: Vec<String>) -> String {
        platitude_core::remote::TrackedBranch::listed(&full, names(&remote_names))
            .map(|row| row.beside(&name).to_string())
            .unwrap_or_default()
    }

    /// What the toolbar's push stands on, read whole
    /// (`remote::HeadPush::facts`): the standing in the word
    /// `PublishFlow.pushState` branches on and what a press does in it,
    /// where the label says it goes, the counts the tip says and the remote
    /// branch an overwrite leases to — all off one destination, the one the
    /// send (`remote::plan_current_push`) resolves. A tab that is not
    /// `open` stands closed whatever its branch says; the label is still
    /// named.
    ///
    /// `head` is the model's own record handed on as it was read
    /// (`WorktreeModel.headPush`); QML builds none. Both marks are the
    /// status's, read with the counts under them — never the tab's
    /// `pushDefault`, which the refs snapshot sends out before or after
    /// (`remote::StatusPush::push_default`). The configured remotes and the
    /// remote a branch that tracks nothing, marked nowhere, goes to are the
    /// tab's.
    #[qslot]
    fn toolbar_push(
        &self,
        head: crate::encode::Push,
        open: bool,
        remote_names: Vec<String>,
        default_remote: String,
    ) -> One<crate::encode::ToolbarPush> {
        let mut facts = head.facts(names(&remote_names), &default_remote);
        if !open {
            facts.standing = platitude_core::remote::PushStanding::Closed;
        }
        One::new(facts.into())
    }

    /// What the right-click menu on a ref offers for one stand, read whole
    /// (`encode::RefMenu`): the row's own rows and the BRANCH and TAG cards'
    /// (`offers::ref_menu`), where the branch's upstream stands
    /// (`offers::upstream_place`), the `pull` row (`offers::pull_row`) and
    /// the WORKTREE card's two making rows (`offers::worktree_rows`). Asked
    /// as the menu opens and as it stands again, never by a binding
    /// (app-ui.md「メニューは `AppMenu` 系で書く」), by both entrances through
    /// one part (`RefMenuFacts`).
    ///
    /// `tree` is the model's own record handed on as it was read:
    /// `WorktreeModel.reading` as the menu stood on it — the model's, or the
    /// one kept while a command is out (`RepoPage.treeAtAsk`) —, or
    /// `WorktreeModel.standing` where a binding or a press reads the tree as
    /// it is; this slot reads the standing's part of either. QML builds
    /// none: a map short of a field reads as the default tree (no HEAD,
    /// nothing standing) with only a warning in the log (`encode::One`).
    /// Every slot here that takes a `tree` takes it so.
    ///
    /// `lead` is the row and where it leads, as the worktrees section
    /// answered it (`NavSectionModel.leadOf`). `card` is the record the
    /// row's card stood on, handed on as it was read —
    /// `NavSectionModel.branchUpstream` on a branch's or a remote branch's
    /// row, `tagMenu` on a tag's — and `undefined` on any other row, which
    /// reads nothing over there (`offers::FarSide`). The `pull` row reads
    /// the same record: on a local branch's row, its far side is the row's
    /// own. `local_listed`: the local branch the row lands on is there —
    /// a remote row whose local one is takes its way through that row.
    #[qslot]
    fn ref_menu(
        &self,
        oid_hex: String,
        open: bool,
        tree: One<TreeStanding>,
        lead: One<RowLead>,
        card: Optional<CardRecord>,
        local_listed: bool,
    ) -> One<RefMenu> {
        use platitude_core::offers;
        let row = row_standing(&lead, &oid_hex, open, &tree);
        let far = card.as_ref().map_or(FarSide::Unread, CardRecord::far);
        let upstream = match &*card {
            Some(CardRecord::Branch(upstream)) => {
                offers::upstream_place(&upstream.upstream, upstream.upstream_here, &tree)
            }
            _ => UpstreamPlace::None,
        };
        let making = offers::worktree_rows(&row, !lead.stopped_by.is_empty(), local_listed);
        One::new(RefMenu::new(
            offers::ref_menu(&row, far),
            offers::pull_row(&row, far),
            upstream,
            WorktreeMaking::new(making, lead.placed),
        ))
    }

    /// What the right-click menu on a commit row may offer, the same way
    /// (`CommitMenuState.openRowMenu`). `stash`: the row is a stash's;
    /// `provisional`: the row is one the discard log's picked entry draws
    /// (`GraphModel.provisionalAt`).
    #[qslot]
    fn commit_menu_offers(
        &self,
        open: bool,
        tree: One<TreeStanding>,
        oid_hex: String,
        stash: bool,
        provisional: bool,
    ) -> Vec<String> {
        let offers = platitude_core::offers::commit_menu(open, &tree, &oid_hex, stash);
        let offers = if provisional {
            offers.on_provisional()
        } else {
            offers
        };
        offers.words().into_iter().map(str::to_string).collect()
    }

    /// What the details pane's message boxes do with the commit on screen,
    /// in the word `RepoPage.messageEdit` branches on: `amend` where typing
    /// lands, `stash` / `not-head` / `standing` where it does not, `""` with
    /// no message on screen; `stash`: the commit on screen is a stash's.
    /// Asked as a live binding, not at open: the boxes stay open while HEAD
    /// moves, and must stop taking typing then.
    // The binding follows `tree` through the model's property
    // (`WorktreeModel.standing`, one notification for every field), so the
    // answer moves with HEAD and the operation; `open` is the page's and
    // stays apart.
    #[qslot]
    fn message_edit(
        &self,
        open: bool,
        tree: One<TreeStanding>,
        oid_hex: String,
        stash: bool,
    ) -> String {
        platitude_core::offers::message_edit(open, &tree, &oid_hex, stash)
            .as_str()
            .to_string()
    }

    /// The move a press on a ref adds up to, in the word
    /// `RepoPage.switchToRef` branches on: `switch` / `materialize` /
    /// `move` / `holder`, or `""` — also for any row but a branch's or a
    /// remote branch's (the chip's own `kind`, which the sidebar's
    /// `refSwitchRequested` hands on too), and for no local branch to land
    /// on (`offers::SwitchTarget`). `lead` is the row as `refMenu` takes
    /// it, `local_listed` whether its local branch is there.
    #[qslot]
    fn switch_action(
        &self,
        lead: One<RowLead>,
        tree: One<TreeStanding>,
        local_listed: bool,
    ) -> String {
        platitude_core::offers::switch_action(
            platitude_core::offers::SwitchTarget::from_words(&lead.kind, &lead.local),
            &tree,
            lead.leads == platitude_core::offers::Leads::Holder,
            local_listed,
        )
        .as_str()
        .to_string()
    }

    /// Whether the move a press already sent has reached the screen —
    /// `RepoPage` holds the next press until it has, so a double press
    /// sends one command. `landing_listed`: the branch it lands on is
    /// listed.
    #[qslot]
    fn move_landed(&self, landing: String, tree: One<TreeStanding>, landing_listed: bool) -> bool {
        platitude_core::offers::move_landed(&landing, &tree, landing_listed)
    }

    /// Whether `name` is the branch HEAD is on — the name a question draws
    /// in the current branch's colour (デザイン規約 §ref の種別). The two
    /// sides of a conflict ask the model instead
    /// (`WorktreeModel.sideOursCurrent`), a chip its own record (`isHead`).
    #[qslot]
    fn is_current(&self, tree: One<TreeStanding>, name: String) -> bool {
        tree.is_current(&name)
    }

    /// The folder name a clone of `url` would land in, offered before
    /// anybody types one; `""` where the URL carries none, which leaves the
    /// accept button refusing.
    #[qslot]
    fn clone_folder_name(&self, url: String) -> String {
        platitude_core::remote::folder_name_for(&url)
    }

    /// The repository's page on GitHub for a remote URL that names one,
    /// `""` for any other (`web_link::github_repo`) — what decides whether a
    /// remote's links are offered at all.
    #[qslot]
    fn github_repo(&self, url: String) -> String {
        platitude_core::web_link::github_repo(&url).unwrap_or_default()
    }

    /// A commit's page in that repository; `""` for an id that is none.
    #[qslot]
    fn github_commit_link(&self, repo: String, oid: String) -> String {
        platitude_core::Oid::from_hex_str(oid.trim())
            .map(|oid| platitude_core::web_link::commit(&repo, &oid))
            .unwrap_or_default()
    }

    /// A branch's page, by the name it has on that remote.
    #[qslot]
    fn github_branch_link(&self, repo: String, name: String) -> String {
        platitude_core::web_link::branch(&repo, &name)
    }

    #[qslot]
    fn github_tag_link(&self, repo: String, name: String) -> String {
        platitude_core::web_link::tag(&repo, &name)
    }

    /// The comparison `git diff <from> <to>` makes; `""` unless both are
    /// ids.
    #[qslot]
    fn github_compare_link(&self, repo: String, from: String, to: String) -> String {
        match (
            platitude_core::Oid::from_hex_str(from.trim()),
            platitude_core::Oid::from_hex_str(to.trim()),
        ) {
            (Ok(from), Ok(to)) => platitude_core::web_link::compare(&repo, &from, &to),
            _ => String::new(),
        }
    }

    /// `path` as it stands in commit `oid`; `""` for an id that is none.
    #[qslot]
    fn github_file_link(&self, repo: String, oid: String, path: String) -> String {
        platitude_core::Oid::from_hex_str(oid.trim())
            .map(|oid| platitude_core::web_link::file(&repo, &oid, &path))
            .unwrap_or_default()
    }

    /// Lines `first..=last` of `path` in commit `oid` (`file_lines`); `""`
    /// for an id that is none or a line that is not one.
    #[qslot]
    fn github_lines_link(
        &self,
        repo: String,
        oid: String,
        path: String,
        first: i32,
        last: i32,
    ) -> String {
        let (Ok(oid), Some(lines)) = (
            platitude_core::Oid::from_hex_str(oid.trim()),
            file_lines(first, last),
        ) else {
            return String::new();
        };
        platitude_core::web_link::lines(&repo, &oid, &path, lines)
    }

    /// Which of the remotes holding something a menu leads with — the
    /// preferred one where it holds it, else the first by name
    /// (`web_link::lead`); `""` where none does.
    #[qslot]
    fn link_lead(&self, preferred: String, holders: Vec<String>) -> String {
        platitude_core::web_link::lead(&preferred, &holders)
            .unwrap_or_default()
            .to_string()
    }

    /// Where a link to one side of the diff the commit card shows points
    /// (`web_link::side_at`): commit `sha` against `parent`, or —
    /// `comparing` — `from` against `to`. `undefined` where that side has
    /// no commit (a root commit's before) or an id is none.
    #[qslot]
    fn github_side_at(
        &self,
        newer: bool,
        comparing: bool,
        sha: String,
        parent: String,
        from: String,
        to: String,
    ) -> Optional<LinkAt> {
        use platitude_core::web_link::{DiffOf, side_at};
        let id = |hex: &str| platitude_core::Oid::from_hex_str(hex.trim()).ok();
        let diff = if comparing {
            id(&from)
                .zip(id(&to))
                .map(|(from, to)| DiffOf::Pair { from, to })
        } else {
            id(&sha).map(|commit| DiffOf::Commit {
                commit,
                parent: id(&parent),
            })
        };
        Optional::new(
            diff.and_then(|diff| side_at(diff, newer))
                .map(|side| LinkAt {
                    held: side.held.to_hex(),
                    at: side.at.to_hex(),
                }),
        )
    }

    /// The name branch `name` goes by on `remote`, for its page there
    /// (`web_link::branch_name_on`): a local branch (`kind` `branch`) by the
    /// name of the remote branch it is measured against (`upstream`) on
    /// that one's remote, else by its own; a remote branch (`kind` `remote`,
    /// `name` whole, as `upstream/main`) by its name on its own remote.
    #[qslot]
    fn github_branch_name(
        &self,
        kind: String,
        name: String,
        upstream: String,
        remote: String,
        remote_names: Vec<String>,
    ) -> String {
        let measured = if kind == "remote" { &name } else { &upstream };
        let cut = if measured.is_empty() {
            None
        } else {
            platitude_core::refs::split_remote_ref_or_first_slash(measured, names(&remote_names))
        };
        let own = if kind == "remote" {
            cut.map_or("", |(_, branch)| branch)
        } else {
            name.as_str()
        };
        platitude_core::web_link::branch_name_on(own, cut, &remote).to_string()
    }

    /// The folder a path sits in, as a `file:` URL for a chooser to start
    /// in; empty where there is no folder above it. The reverse of
    /// [`Self::picked_path`]: separators, drive letters and percent-encoding
    /// are not QML's to spell.
    #[qslot]
    fn folder_url_of(&self, path: String) -> String {
        let path = path.trim();
        if path.is_empty() {
            return String::new();
        }
        std::path::Path::new(path)
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .map(crate::urlpath::file_url)
            .unwrap_or_default()
    }

    /// Whether two spellings name one folder: git's worktree paths and a
    /// session's differ in separator on Windows and in case on Windows and
    /// macOS. Two empty paths are not the same folder.
    #[qslot]
    fn same_path(&self, one: String, other: String) -> bool {
        !one.is_empty()
            && platitude_core::session::same_path_key(&one)
                == platitude_core::session::same_path_key(&other)
    }

    /// The local path a `file://` URL names — what a box shows once the
    /// platform's chooser answers, for a folder or a file alike.
    #[qslot]
    fn picked_path(&self, url: String) -> String {
        crate::urlpath::file_url_to_path(&url)
            .to_string_lossy()
            .into_owned()
    }

    /// The address half of a typed identity; `""` where nothing in the
    /// text reads as one.
    #[qslot]
    fn identity_email_of(&self, text: String) -> String {
        platitude_core::trailers::split_identity(&text).1
    }

    /// The name half of a typed identity; `""` where nothing stands
    /// before an address.
    #[qslot]
    fn identity_name_of(&self, text: String) -> String {
        platitude_core::trailers::split_identity(&text).0
    }

    /// The last segment of a path, cut where the platform ends a folder.
    #[qslot]
    fn path_leaf(&self, path: String) -> String {
        crate::urlpath::path_leaf(&path).to_string()
    }

    /// Whoever a message being typed credits, in the records a commit's
    /// own trailers reach the details pane as. Which lines count is
    /// `platitude_core::trailers`' alone.
    #[qslot]
    fn co_authors_of(&self, body: String) -> Mates {
        crate::encode::mates_of(&platitude_core::trailers::co_authors_in(&body))
    }

    /// Whether a name is one git would take for a tag — asked per
    /// keystroke by the tag boxes (a new tag, a tag's rename), so it never
    /// runs git (`platitude_core::tag`).
    #[qslot]
    fn valid_tag_name(&self, name: String) -> bool {
        platitude_core::tag::is_valid_name(&name)
    }

    /// The same for a branch, which git also refuses to open with `-` or to
    /// call `HEAD` — the branch boxes (a new branch, a branch's rename, a
    /// remote branch's, a new worktree's) (`platitude_core::branch`).
    #[qslot]
    fn valid_branch_name(&self, name: String) -> bool {
        platitude_core::branch::is_valid_name(&name)
    }

    /// The same for a stash's label: free text on one line.
    #[qslot]
    fn valid_stash_message(&self, message: String) -> bool {
        platitude_core::stash::is_valid_message(&message)
    }

    /// What somebody called a stash, out of the line the list shows; empty
    /// where the whole line is git's own (`WIP on …`). Asked before a pop,
    /// the last moment the entry is there.
    #[qslot]
    fn stash_label(&self, message: String) -> String {
        platitude_core::stash::label_in(&message).to_string()
    }
}

/// The row `refMenu` is asked about, standing on `tree`.
fn row_standing<'a>(
    lead: &'a RowLead,
    oid_hex: &str,
    open: bool,
    tree: &'a TreeStanding,
) -> RowStanding<'a> {
    RowStanding {
        target: platitude_core::offers::RefTarget::from_words(&lead.kind, &lead.full),
        has_commit: !oid_hex.is_empty(),
        open,
        tree,
        leads: lead.leads,
    }
}

fn names(remote_names: &[String]) -> impl Iterator<Item = &str> {
    remote_names.iter().map(String::as_str)
}

/// The lines of a file QML names by its first and last line numbers,
/// numbered from 1; `None` where `first` is no line or `last` is negative.
/// A `last` before `first` — `0` among them — is `first`.
fn file_lines(first: i32, last: i32) -> Option<RangeInclusive<u32>> {
    let first = u32::try_from(first).ok().filter(|&first| first > 0)?;
    let last = u32::try_from(last).ok()?;
    Some(first..=last.max(first))
}

/// Where a link to one side of a diff points (`GitFacts.githubSideAt`):
/// the commit whose file it is (`at`) and the commit a remote has to have
/// for the page to be there (`held`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkAt {
    pub held: String,
    pub at: String,
}

impl Record for LinkAt {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("held", &self.held)
            .put("at", &self.at)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            held: field(map, "held")?,
            at: field(map, "at")?,
        })
    }
}

qml_register!(GitFacts, "GitFacts", singleton = true);

#[cfg(test)]
mod tests {
    use super::{GitFacts, file_lines};
    use platitude_core::remote::{HeadPush, PushTrack, StatusPush};

    /// `main` one commit ahead of `origin/main`, marked nowhere of its own,
    /// the repository's mark as the status read it.
    fn head(track: Option<PushTrack>, push_default: &str) -> crate::encode::Push {
        crate::encode::Push::new(HeadPush {
            branch: "main".into(),
            settled: true,
            upstream: "origin/main".into(),
            upstream_counts: Some(platitude_core::AheadBehind {
                ahead: 1,
                behind: 0,
            }),
            status: StatusPush {
                track,
                push_default: Some(push_default.to_string()).filter(|mark| !mark.is_empty()),
                ..StatusPush::default()
            },
            ..HeadPush::default()
        })
    }

    /// No mark leaves the push to the upstream; one on another remote takes
    /// it there, under the branch's own name, read by the counts there.
    #[test]
    fn the_toolbars_push_is_read_off_the_models_record_under_its_marks() {
        let push = |track: Option<PushTrack>, push_default: &str| {
            let answer = GitFacts.toolbar_push(
                head(track, push_default),
                true,
                vec!["fork".to_string(), "origin".to_string()],
                "origin".to_string(),
            );
            (*answer).clone()
        };
        assert_eq!(
            push(None, ""),
            crate::encode::ToolbarPush {
                state: "ready".into(),
                sends_plain: true,
                overwrites: false,
                target: "origin/main".into(),
                measured: "origin/main".into(),
                ahead: 1,
                behind: 0,
                upstream: "origin/main".into(),
            }
        );
        let elsewhere = push(None, "fork");
        assert_eq!(
            (elsewhere.state.as_str(), elsewhere.target.as_str()),
            ("elsewhere", "fork/main")
        );
        assert_eq!(
            (elsewhere.measured.as_str(), elsewhere.ahead),
            ("", 0),
            "nothing counted over there"
        );
        let there = PushTrack {
            remote: "fork".into(),
            tracking: "fork/main".into(),
            counts: platitude_core::AheadBehind {
                ahead: 1,
                behind: 2,
            },
        };
        let diverged = push(Some(there), "fork");
        assert_eq!(
            (
                diverged.state.as_str(),
                diverged.measured.as_str(),
                diverged.ahead,
                diverged.behind
            ),
            ("diverged", "fork/main", 1, 2),
            "the destination's own counts, and its ref to lease to"
        );
        assert_eq!(
            (diverged.sends_plain, diverged.overwrites),
            (false, true),
            "only the held overwrite reaches"
        );
    }

    /// A tab that is not open sends nothing, and still names where its
    /// branch would go.
    #[test]
    fn a_tab_that_is_not_open_stands_closed() {
        let answer = GitFacts.toolbar_push(
            head(None, ""),
            false,
            vec!["origin".to_string()],
            "origin".to_string(),
        );
        assert_eq!(
            (
                answer.state.as_str(),
                answer.sends_plain,
                answer.overwrites,
                answer.target.as_str()
            ),
            ("closed", false, false, "origin/main")
        );
    }

    /// One stand answers every row of a ref's menu off the row's lead and
    /// its card's record: here a branch another worktree has out, tracking
    /// `origin/topic` with a commit of its own.
    #[test]
    fn a_refs_menu_is_answered_whole_for_one_stand() {
        use crate::encode::{One, Optional};
        use crate::models::nav::{BranchUpstream, CardRecord, RowLead};
        use platitude_core::offers::{BranchFar, Leads, Reading, TreeStanding};
        let tree = || {
            One::new(TreeStanding {
                branch: "main".into(),
                upstream: "origin/main".into(),
                ..TreeStanding::default()
            })
        };
        let lead = RowLead {
            kind: "branch".into(),
            full: "topic".into(),
            local: "topic".into(),
            leads: Leads::Holder,
            path: "C:/work/other".into(),
            called: "other".into(),
            placed: true,
            ..RowLead::default()
        };
        let card = BranchUpstream {
            far: Some(BranchFar {
                reading: Reading::Together,
                tracked: true,
                has_remote: true,
                ahead: 1,
            }),
            counterpart: "origin/topic".into(),
            upstream: "origin/topic".into(),
            ..BranchUpstream::default()
        };
        let oid = "ab".repeat(20);
        let menu = (*GitFacts.ref_menu(
            oid.clone(),
            true,
            tree(),
            One::new(lead.clone()),
            Optional::some(CardRecord::Branch(card)),
            true,
        ))
        .clone();
        assert!(
            menu.switch_to && !menu.switch_asks,
            "the press opens the holder"
        );
        assert!(!menu.delete, "git keeps a branch out elsewhere");
        assert!(menu.delete_remote && menu.set_upstream && menu.unset_upstream);
        assert_eq!(menu.upstream_where, "remote");
        assert_eq!(
            (menu.pull_runs.as_str(), menu.pull_out.as_str()),
            ("aside", "held"),
            "the pull row reads the card's record as the row's own far side"
        );
        assert_eq!(
            (
                menu.making.here,
                menu.making.checkout.as_str(),
                menu.making.out.as_str()
            ),
            (true, "branch", "held")
        );
        assert_eq!(
            GitFacts.switch_action(One::new(lead.clone()), tree(), true),
            "holder"
        );
        let nowhere = RowLead {
            placed: false,
            ..lead
        };
        let unplaced =
            GitFacts.ref_menu(oid, true, tree(), One::new(nowhere), Optional::none(), true);
        assert!(
            !unplaced.making.here,
            "a box whose Enter could make nothing is not offered"
        );
        assert_eq!(
            (
                unplaced.upstream_where.as_str(),
                unplaced.pull_runs.as_str()
            ),
            ("", ""),
            "no card record, nothing read over there"
        );
    }

    #[test]
    fn the_lines_named_start_at_line_one_and_run_forward() {
        assert_eq!(file_lines(7, 7), Some(7..=7));
        assert_eq!(file_lines(10, 20), Some(10..=20));
        assert_eq!(
            file_lines(10, 3),
            Some(10..=10),
            "a last before the first is the first"
        );
        assert_eq!(file_lines(3, 0), Some(3..=3), "as is a last of 0");
        assert_eq!(file_lines(0, 3), None, "there is no line 0");
        assert_eq!(file_lines(-1, 3), None);
        assert_eq!(file_lines(3, -1), None);
    }
}

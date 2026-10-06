//! Pure rules QML asks by value: every slot reads only its arguments (the
//! exception app-ui.md grants the `GitFacts` singleton), so a slot that
//! reads state has no place here.

use qtbridge::qobject;

use super::qml_register;
use crate::encode::{Chips, Mates};

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
    /// rest in `warning` (デザイン規約 §長押し).
    #[qslot]
    fn discard_unrecorded(
        &self,
        unborn: bool,
        not_copied: Vec<String>,
        paths: Vec<String>,
    ) -> bool {
        !platitude_core::discards::recorded_whole(unborn, &not_copied, &paths)
    }

    /// `chips` less the ones `gone` names by key.
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

    /// What a right-click on a chip of that word aims the menu at: the ref
    /// kinds, and `worktree` for a worktree's folder chip — a worktree has a
    /// menu (its WORKTREE card), where it has no name to rename or switch
    /// to (`ref_kind`). `""` for the detached HEAD.
    #[qslot]
    fn menu_kind(&self, kind: String) -> String {
        crate::encode::menu_kind_word(&kind).to_string()
    }

    /// What the WORKTREE card offers for one worktree (`RefWorktreeMenu.standOn`):
    /// `state` is the worktree's row word (`models::nav::item` — `MAIN` /
    /// `LOCKED` / `PRUNABLE` / empty), `here` whether the tab stands in it.
    #[qslot]
    fn worktree_card_offers(&self, state: String, here: bool, busy_count: i32) -> Vec<String> {
        platitude_core::offers::worktree_card(
            state == crate::models::nav::MAIN,
            state == crate::models::nav::LOCKED,
            here,
            busy_count,
        )
        .words()
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    /// The two rows that make a worktree (`offers::worktree_rows`), as
    /// words: `here`, then `checkout-branch` / `checkout-track`. `kind` is
    /// the row's word, empty on a commit row that draws no name;
    /// `local_exists` whether a remote row's local branch is there.
    #[qslot]
    #[expect(clippy::too_many_arguments)]
    fn worktree_offers(
        &self,
        kind: String,
        full: String,
        oid_hex: String,
        open: bool,
        busy_count: i32,
        current_branch: String,
        held_by_worktree: String,
        local_exists: bool,
    ) -> Vec<String> {
        platitude_core::offers::worktree_rows(
            platitude_core::offers::RefKind::from_word(&kind),
            &full,
            &oid_hex,
            open,
            busy_count,
            &current_branch,
            &held_by_worktree,
            local_exists,
        )
        .words()
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    /// The remote half of a remote-tracking name (`origin/main`): the
    /// longest of `remote_names` wins (a remote's name may contain `/`);
    /// where none owns the ref the first slash answers, so the gesture
    /// still acts and git refuses loudly — the list may still be loading,
    /// or the remote gone while its refs remain. `""` only where there is
    /// no slash at all.
    #[qslot]
    fn remote_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        platitude_core::refs::split_remote_ref_or_first_slash(&full, names(&remote_names))
            .map(|(remote, _)| remote.to_string())
            .unwrap_or_default()
    }

    /// The branch half of the same cut; a name no slash divides is all
    /// branch (the shape a rename box is typed in).
    #[qslot]
    fn branch_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        match platitude_core::refs::split_remote_ref_or_first_slash(&full, names(&remote_names)) {
            Some((_, branch)) => branch.to_string(),
            None => full,
        }
    }

    /// What a push of the current branch can do, in the word
    /// `PublishFlow.pushState` branches on. A closed tab is the caller's to
    /// add: this answers for the repository alone. The `push_*` three are
    /// the destination's own counts (`WorkingTreeModel.pushTracking`), read
    /// where the push goes elsewhere.
    #[qslot]
    #[expect(clippy::too_many_arguments)]
    fn push_standing(
        &self,
        unborn: bool,
        detached: bool,
        branch: String,
        upstream: String,
        upstream_tracked: bool,
        ahead: i32,
        behind: i32,
        push_remote: String,
        push_default: String,
        remote_names: Vec<String>,
        push_tracking: String,
        push_ahead: i32,
        push_behind: i32,
    ) -> String {
        platitude_core::remote::push_standing(
            unborn,
            detached,
            &branch,
            &upstream,
            upstream_tracked,
            ahead,
            behind,
            &push_remote,
            &push_default,
            names(&remote_names),
            &platitude_core::remote::PushTrack {
                tracking: push_tracking,
                ahead: push_ahead,
                behind: push_behind,
            },
        )
        .as_str()
        .to_string()
    }

    /// Whether a mark sends the push to another remote than the
    /// upstream's — where the counts that speak for it are the
    /// destination's own, and so the ones a tooltip says and a lease pins
    /// to (`PublishFlow.pushElsewhere`). The same rule `push_standing`
    /// weighs, so the words and the standing read one set of counts.
    #[qslot]
    fn push_goes_elsewhere(
        &self,
        upstream: String,
        push_remote: String,
        push_default: String,
        remote_names: Vec<String>,
    ) -> bool {
        platitude_core::remote::pushes_elsewhere(
            &upstream,
            &push_remote,
            &push_default,
            names(&remote_names),
        )
    }

    /// Where the toolbar's push button would send this branch, spelled as
    /// the label shows it; empty where there is no branch or nowhere to
    /// send it. Asked of core because the send (`remote::plan_current_push`)
    /// reads the same table: a second spelling in a binding lets the label
    /// name another remote than the push goes to.
    #[qslot]
    fn push_target(
        &self,
        branch: String,
        upstream: String,
        push_remote: String,
        push_default: String,
        default_remote: String,
        remote_names: Vec<String>,
    ) -> String {
        platitude_core::remote::push_target(
            &branch,
            &upstream,
            &push_remote,
            &push_default,
            &default_remote,
            names(&remote_names),
        )
    }

    /// What the right-click menu on a ref may offer, as the words
    /// `RefRowMenu.offerOn` reads — asked once as the menu opens
    /// (app-ui.md「メニューは `AppMenu` 系で書く」).
    #[qslot]
    #[expect(clippy::too_many_arguments)]
    fn ref_menu_offers(
        &self,
        kind: String,
        full: String,
        oid_hex: String,
        open: bool,
        busy_count: i32,
        current_branch: String,
        detached: bool,
        op_text: String,
        conflict_count: i32,
        held_by_worktree: String,
        remote_counterpart: String,
        remote_drifted: bool,
        default_remote: String,
        tag_sides: String,
        current_upstream: String,
    ) -> Vec<String> {
        let Some(kind) = platitude_core::offers::RefKind::from_word(&kind) else {
            return Vec::new();
        };
        platitude_core::offers::ref_menu(
            kind,
            &full,
            &oid_hex,
            open,
            busy_count,
            &current_branch,
            detached,
            &op_text,
            conflict_count,
            &held_by_worktree,
            &remote_counterpart,
            remote_drifted,
            &default_remote,
            &tag_sides,
            &current_upstream,
        )
        .words()
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    /// What the right-click menu on a commit row may offer, the same way
    /// (`CommitMenuState.openRowMenu`).
    #[qslot]
    #[expect(clippy::too_many_arguments)]
    fn commit_menu_offers(
        &self,
        open: bool,
        busy_count: i32,
        current_branch: String,
        detached: bool,
        op_text: String,
        oid_hex: String,
        head_oid: String,
        stash_ref: String,
    ) -> Vec<String> {
        platitude_core::offers::commit_menu(
            open,
            busy_count,
            &current_branch,
            detached,
            &op_text,
            &oid_hex,
            &head_oid,
            &stash_ref,
        )
        .words()
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    /// What the details pane's message boxes do with the commit on screen,
    /// in the word `RepoPage.messageEdit` branches on: `amend` where typing
    /// lands, `stash` / `not-head` / `standing` where it does not, `""` with
    /// no message on screen. Asked as a live binding, not at open: the
    /// boxes stay open while HEAD moves, and must stop taking typing then.
    // The parameters mirror the core rule one-for-one; a struct between the
    // two would be a shape QML cannot see.
    #[qslot]
    fn message_edit(
        &self,
        open: bool,
        oid_hex: String,
        head_oid: String,
        stash_ref: String,
        op_text: String,
        editing: bool,
    ) -> String {
        platitude_core::offers::message_edit(
            open, &oid_hex, &head_oid, &stash_ref, &op_text, editing,
        )
        .as_str()
        .to_string()
    }

    /// The move a press on a ref adds up to, in the word
    /// `RepoPage.switchToRef` branches on: `switch` / `materialize` /
    /// `move` / `holder`, or `""` — also for any `kind` but the chip words
    /// `branch` / `remote`.
    #[qslot]
    fn switch_action(
        &self,
        kind: String,
        local: String,
        current_branch: String,
        held_by_worktree: String,
        local_oid: String,
    ) -> String {
        platitude_core::offers::switch_action(
            &kind,
            &local,
            &current_branch,
            &held_by_worktree,
            &local_oid,
        )
        .as_str()
        .to_string()
    }

    /// Whether the move a press already sent has reached the screen —
    /// `RepoPage` holds the next press until it has, so a double press
    /// sends one command.
    #[qslot]
    fn move_landed(&self, landing: String, current_branch: String, landing_oid: String) -> bool {
        platitude_core::offers::move_landed(&landing, &current_branch, &landing_oid)
    }

    /// The folder name a clone of `url` would land in, offered before
    /// anybody types one; `""` where the URL carries none, which leaves the
    /// accept button refusing.
    #[qslot]
    fn clone_folder_name(&self, url: String) -> String {
        platitude_core::remote::folder_name_for(&url)
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

    /// Whether a name is one git would take for a branch or a tag —
    /// asked per keystroke by the rename box, so it never runs git.
    #[qslot]
    fn valid_ref_name(&self, name: String) -> bool {
        platitude_core::tag::is_valid_name(&name)
    }

    /// The same for a branch alone, which git also refuses to open with `-`
    /// or to call `HEAD` — what the new worktree's box asks, whose name git
    /// hands on as an option otherwise (`platitude_core::branch`).
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

fn names(remote_names: &[String]) -> impl Iterator<Item = &str> {
    remote_names.iter().map(String::as_str)
}

qml_register!(GitFacts, "GitFacts", singleton = true);

//! Pure rules QML asks by value: stateless slots over platitude-core and
//! this crate's wire formats. Every slot reads only its arguments, so a
//! binding that passes its own properties re-evaluates exactly when they
//! change — the qproperty rule (.claude/rules/app-ui.md) is about slots
//! that read state QML cannot see, and there is none here to read.

use qtbridge::qobject;

use super::qml_register;
use crate::encode::{Chips, Mates};

/// The singleton every QML file can ask a rule of without wiring a model
/// through: which id is the synthetic WIP row's, which chips a gone set
/// leaves standing, what a remote ref or a typed identity splits into,
/// what a push can do, which names git would take, what a stash line
/// was called.
#[derive(Default)]
pub struct GitFacts;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl GitFacts {
    /// Whether a hex id is the synthetic uncommitted-changes row's
    /// all-zero sentinel (`Oid::zero_like`). Empty is "no id at all".
    #[qslot]
    fn wip_oid(&self, hex: String) -> bool {
        platitude_core::oid::Oid::hex_is_zero(&hex)
    }

    /// The chips still standing once the page's gone set has spoken
    /// (`encode::chips_shown`): `chips` as the row carries them, less the
    /// ones `gone` names by key.
    #[qslot]
    fn chips_shown(&self, chips: Chips, gone: Vec<String>) -> Chips {
        crate::encode::chips_shown(&chips, &gone)
    }

    /// The kind a chip's word names as a ref the menus can act on
    /// (`encode::ref_kind_word`): `branch` / `remote` / `tag`, and `""`
    /// for the two markers, which name nothing to act on.
    #[qslot]
    fn ref_kind(&self, kind: String) -> String {
        crate::encode::ref_kind_word(&kind).to_string()
    }

    /// The remote half of a remote-tracking name (`origin/main`), read
    /// against the configured names (`RepoTab.remoteNames`): the longest
    /// configured name wins — a remote's own name may contain `/` — and
    /// where none owns the ref the first slash answers, so the gesture
    /// still acts and git gets to refuse loudly
    /// (`refs::split_remote_ref_or_first_slash` — the list may still be
    /// loading, or the remote may be gone from configuration while its
    /// refs remain). `""` only where there is no slash at all: that name
    /// is not a remote branch.
    #[qslot]
    fn remote_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        platitude_core::refs::split_remote_ref_or_first_slash(&full, names(&remote_names))
            .map(|(remote, _)| remote.to_string())
            .unwrap_or_default()
    }

    /// The branch half of the same cut. A name no slash divides is all
    /// branch — the shape a rename box is typed in.
    #[qslot]
    fn branch_of_ref(&self, full: String, remote_names: Vec<String>) -> String {
        match platitude_core::refs::split_remote_ref_or_first_slash(&full, names(&remote_names)) {
            Some((_, branch)) => branch.to_string(),
            None => full,
        }
    }

    /// What a push of the current branch can do
    /// (`platitude_core::remote::push_standing`), in the word
    /// `PublishFlow.pushState` branches on. The tab-lifecycle half of
    /// "closed" stays the caller's: this answers for the repository
    /// alone.
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
        )
        .as_str()
        .to_string()
    }

    /// Where the toolbar's push button would send this branch
    /// (`platitude_core::remote::push_target`), spelled as the label shows
    /// it. Empty where there is no branch to send or nowhere to send it.
    ///
    /// Asked of core: the order the two marks and the upstream are
    /// weighed in is git's, and the send (`remote::plan_current_push`)
    /// reads it off the same table. A second spelling of it in a binding
    /// is how the label came to name the remote a branch tracks while
    /// the push went to the one it marks.
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

    /// What the right-click menu on a ref may offer
    /// (`platitude_core::offers::ref_menu`), as the words the opening
    /// function reads (`RefRowMenu.offerOn`) — asked once as the menu
    /// opens, so the answers freeze while it stands (app-ui.md §メニュー).
    /// The lookups behind `held_by_worktree` / `remote_counterpart` /
    /// `remote_drifted` stay the models'; this only weighs what they
    /// answered.
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

    /// What the right-click menu on a commit row may offer
    /// (`platitude_core::offers::commit_menu`), the same words and the
    /// same freeze-at-open contract (`CommitMenuState.openRowMenu`).
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

    /// What the details pane's message boxes do with the commit on
    /// screen (`platitude_core::offers::message_edit`), in the word
    /// `RepoPage.messageEdit` branches on: `amend` where typing lands,
    /// `stash` / `not-head` / `standing` where it does not and the box
    /// has a reason to give, `""` where there is no message on screen.
    /// **Asked as a binding, live** — the boxes stand open while the
    /// repository moves under them, so a commit that stops being HEAD's
    /// has to stop taking typing.
    // The parameter row mirrors the core rule one-for-one; folding it
    // into a struct would put a QML-invisible shape between the two.
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

    /// The move a press on a ref adds up to
    /// (`platitude_core::offers::switch_action`), in the word
    /// `RepoPage.switchToRef` branches on: `switch` / `materialize` /
    /// `move` / `holder`, `""` for a ref nothing moves onto. `kind` is
    /// the chip's word (`branch` / `remote`; any other moves nothing).
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

    /// Whether the move a press already sent has reached the screen
    /// (`platitude_core::offers::move_landed`) — what `RepoPage` holds
    /// the next press behind, so a chip pressed twice does not send the
    /// same command twice.
    #[qslot]
    fn move_landed(&self, landing: String, current_branch: String, landing_oid: String) -> bool {
        platitude_core::offers::move_landed(&landing, &current_branch, &landing_oid)
    }

    /// What to call the folder a clone of `url` would land in
    /// (`platitude_core::remote::folder_name_for`) — the name the box is
    /// offered before anybody types one. `""` where the URL carries none,
    /// which is what leaves the accept button refusing.
    #[qslot]
    fn clone_folder_name(&self, url: String) -> String {
        platitude_core::remote::folder_name_for(&url)
    }

    /// The folder a path sits in, as a `file:` URL — where a chooser
    /// opened on that path should start (`urlpath::file_url`). Empty
    /// where the path has no folder above it, and for the empty path.
    ///
    /// The other direction of [`Self::picked_path`], and here for the
    /// same reason: separators, drive letters and percent-encoding are
    /// the one part of a path QML has no business spelling out.
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

    /// The local path a `file://` URL names
    /// (`urlpath::file_url_to_path`) — what a box shows once the
    /// platform's chooser has answered with one. Folder or file: the
    /// chooser answers in URLs either way, and a path is what every box
    /// this reaches is holding.
    #[qslot]
    fn picked_path(&self, url: String) -> String {
        crate::urlpath::file_url_to_path(&url)
            .to_string_lossy()
            .into_owned()
    }

    /// The address half of a typed identity
    /// (`platitude_core::trailers::split_identity`); `""` where nothing
    /// in the text reads as one.
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

    /// The last segment of a path, whichever separator wrote it — the
    /// name a working copy's folder is shown by (`urlpath::path_leaf`).
    #[qslot]
    fn path_leaf(&self, path: String) -> String {
        crate::urlpath::path_leaf(&path).to_string()
    }

    /// Whoever a message being typed credits, in the same records a
    /// commit's own trailers reach the details pane as
    /// (`encode::mates_of`). Whether a line counts is
    /// `platitude_core::trailers` — the one place that rule is written.
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

    /// The same question for a stash's label, which is free text on one
    /// line of its own.
    #[qslot]
    fn valid_stash_message(&self, message: String) -> bool {
        platitude_core::stash::is_valid_message(&message)
    }

    /// What somebody called a stash, out of the line the list shows —
    /// empty where the whole line is git's own (`WIP on …`). Asked
    /// before a pop, which is the last moment the entry is there to ask.
    #[qslot]
    fn stash_label(&self, message: String) -> String {
        platitude_core::stash::label_in(&message).to_string()
    }
}

/// The configured names as core reads them.
fn names(remote_names: &[String]) -> impl Iterator<Item = &str> {
    remote_names.iter().map(String::as_str)
}

qml_register!(GitFacts, "GitFacts", singleton = true);

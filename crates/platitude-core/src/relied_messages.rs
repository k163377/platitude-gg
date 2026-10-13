//! git's messages the product relies on: the words a classifier tells
//! git's answers apart by, and each piece the screen cuts out of git's
//! words to set in a sentence of its own. They are written here and nowhere
//! else — a classifier names one of these, never a literal of its own.
//!
//! Not here: machine formats asked for (`--porcelain`, `-z`, `--format`,
//! trace2), and git's words shown as they stand (the command log, the
//! header's first line, an opening's refusal) — a rewording there changes
//! the words on the screen and nothing else.
//!
//! git promises no wording for its porcelain, a maintenance release
//! rewords too, and Git for Windows carries words of its own
//! (gitの扱い.md §git の文言に依存する判定). So each message names where it
//! is written, and `cargo xtask relied-messages` holds those lines of every
//! release from the minimum on, of `master`, of Git for Windows, of Apple
//! Git and of the minimum container's Ubuntu to `crates/xtask/relied-messages.txt`
//! — which releases were read, and where the lines changed:
//!
//! - `git:` — a file of git's tree and words on one of its lines (any case);
//!   `openssh:` — the same in OpenSSH, which git passes the words of on.
//!   Every line of that file holding them is held, so a message git writes
//!   in many places names the files of the ones read here
//! - `witness:` — a test in `tests/it` that has real git say it, which the
//!   weekly run holds to `master`'s git; `none` where no test reaches it
//!
//! A form added or changed here is checked against those releases before
//! the gate takes it (`cargo xtask relied-messages audit`).
//!
//! Two kinds: a form ([`Words`]) is one line's words, what the sources are
//! held to; a message ([`Relied`]) is the forms that each say it, any one of
//! them found being it. No reading branches on the version of the git that
//! runs: every form a release writes stands in its message, and the version
//! is no key to them (the same number says other words in Git for Windows,
//! Apple Git and a distribution's backports). Only forms that mean another
//! thing in another release would be told apart, by what git said.

/// One form of a message: words git writes on a line of its sources. Made
/// here alone, so every form is one the sources are held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Words(&'static str);

impl Words {
    /// The words, for a reader that cuts git's text at them.
    pub(crate) const fn text(self) -> &'static str {
        self.0
    }

    /// Whether git said them.
    pub(crate) fn found_in(self, said: &str) -> bool {
        said.contains(self.0)
    }

    /// [`Self::found_in`], case left out on both sides.
    pub(crate) fn found_in_any_case(self, said: &str) -> bool {
        said.to_ascii_lowercase()
            .contains(&self.0.to_ascii_lowercase())
    }

    /// Whether git's words are these, whole (a reason in a push's brackets).
    pub(crate) fn is(self, said: &str) -> bool {
        said == self.0
    }
}

/// A message the product relies on: the forms that say it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Relied {
    forms: &'static [Words],
    any_case: bool,
}

impl Relied {
    const fn any(forms: &'static [Words]) -> Self {
        Self {
            forms,
            any_case: false,
        }
    }

    const fn any_case(forms: &'static [Words]) -> Self {
        Self {
            forms,
            any_case: true,
        }
    }

    /// Whether git said it: any of its forms.
    pub(crate) fn found_in(self, said: &str) -> bool {
        self.forms.iter().any(|form| match self.any_case {
            true => form.found_in_any_case(said),
            false => form.found_in(said),
        })
    }
}

// ---- Another git's lock (`report::lock_taken`)

/// A lock file another process holds. The tail is the C library's
/// `strerror(EEXIST)`, which git sets after the file's name.
///
/// - git: `lockfile.c` `Unable to create '%s`
/// - witness: `commit_integration::a_commit_another_git_holds_the_index_for_is_reported_as_locked`
/// - witness: `stage_integration::refusals::another_gits_index_lock_is_reported_for_each_whole_file_verb`
/// - witness: `push_refusals::a_far_side_that_cannot_take_it_now_says_why`
pub(crate) const LOCK_FILE_TAKEN: Words = Words(".lock': File exists");

/// The reftable ref store's lock, one for all its refs: no file named, and
/// never translated.
///
/// - git: `refs/reftable-backend.c` `cannot lock references`
/// - witness: `rename_integration::a_reftable_stores_lock_is_reported_as_a_lock_where_git_names_it`
pub(crate) const REFS_LOCK_TAKEN: Words = Words("cannot lock references");

/// Another git holds a lock this one needed, whichever store it is in
/// (`report::lock_taken` weighs it against [`LOCK_LEFT_BEHIND`]).
pub(crate) const LOCK_TAKEN: Relied = Relied::any(&[LOCK_FILE_TAKEN, REFS_LOCK_TAKEN]);

/// Where `core.lockfilePid` left the holder's pid beside the lock and that
/// process is gone: git itself says the lock is stale, no git's.
///
/// - git: `lockfile.c` `which is no longer running`
/// - witness: none — it takes `core.lockfilePid` and a pid no process holds
pub(crate) const LOCK_LEFT_BEHIND: Words = Words("which is no longer running");

// ---- Worktrees (`report::worktree_not_removed` / `worktree_not_added`, `worktrees::remove`)

/// Changes stand in the way of `worktree remove`.
///
/// - git: `builtin/worktree.c` `contains modified or untracked files`
/// - witness: `worktree_remove::uncommitted_work_keeps_the_worktree_and_the_reason_is_the_screens`
pub(crate) const WORKTREE_UNCLEAN: Words = Words("contains modified or untracked files");

/// `worktree add` onto a place still listed for a worktree whose folder is
/// gone, locked or not.
///
/// - git: `builtin/worktree.c` `is a missing but`
/// - witness: `worktree_add::a_place_still_listed_is_a_report_and_says_what_it_left`
pub(crate) const WORKTREE_PLACE_LISTED: Words = Words("is a missing but ");

/// A path git lists no worktree at: removed elsewhere already.
///
/// - git: `builtin/worktree.c` `is not a working tree`
/// - witness: `worktree_remove::a_worktree_already_gone_is_removed_as_asked`
pub(crate) const NOT_A_WORKTREE: Words = Words("' is not a working tree");

/// `worktree remove` turned down over a lock.
///
/// - git: `builtin/worktree.c` `cannot remove a locked working tree`
/// - witness: `worktree_remove::a_lock_keeps_the_worktree_and_says_its_reason`
pub(crate) const WORKTREE_LOCKED: Words = Words("cannot remove a locked working tree");

/// What sets the lock's reason off after [`WORKTREE_LOCKED`] — the reason
/// is the owner's own words, which the screen sets in a sentence.
///
/// - git: `builtin/worktree.c` `lock reason:`
/// - witness: `worktree_remove::a_lock_keeps_the_worktree_and_says_its_reason`
pub(crate) const LOCK_REASON: Words = Words(", lock reason: ");

/// git's advice after the reason, which the screen's sentence leaves out.
///
/// - git: `builtin/worktree.c` `use 'remove -f -f'`
/// - witness: `worktree_remove::a_lock_keeps_the_worktree_and_says_its_reason`
pub(crate) const REMOVE_ADVICE: Words = Words("\nuse 'remove -f -f'");

// ---- Another worktree's work applied here (`session::carried_apply`, `report::carried_refused`)

/// `stash apply` could not refresh and write the index: unmerged entries
/// here, or another git's lock where git does not name it.
///
/// - git: `builtin/stash.c` `could not write index`
/// - witness: `carried_apply::a_refusal_over_nothing_it_would_overwrite_is_reported_as_gits`
pub(crate) const INDEX_UNWRITTEN: Words = Words("could not write index");

/// `stash apply` over a merge standing here.
///
/// - git: `builtin/stash.c` `cannot apply a stash in the middle of a merge`
/// - witness: none — no scene in `tests/it` reaches it
pub(crate) const STASH_OVER_A_MERGE: Words = Words("cannot apply a stash in the middle of a merge");

/// Untracked files here an apply would remove.
///
/// - git: `unpack-trees.c` `would be removed by merge`
/// - witness: none — no scene in `tests/it` reaches it
pub(crate) const REMOVED_BY_MERGE: Words = Words("would be removed by merge");

/// What `git stash apply --no-index` turns another worktree's work down
/// with: the index it could not refresh and write, a merge standing here,
/// work here it would overwrite or untracked files it would remove. The
/// words that make its exit 1 a refusal rather than a conflicted landing,
/// which says nothing on stderr, so the row in the log is a failure with
/// them under it (`GitCommand::except_when_it_says`).
pub(crate) const CARRIED_TURNED_DOWN: Relied = Relied::any(&[
    INDEX_UNWRITTEN,
    STASH_OVER_A_MERGE,
    OVERWRITES_WORK,
    REMOVED_BY_MERGE,
]);

// ---- Work in the way of a merge, a pull, a checkout, an apply (`integrate`)

/// Tracked changes or untracked files git would write over — under a
/// merge, a pull's merge, a checkout or a stash's apply.
///
/// - git: `unpack-trees.c` `would be overwritten`
/// - git: `builtin/merge.c` `would be overwritten`
/// - git: `merge-ort-wrappers.c` `would be overwritten`
/// - witness: `integrate_integration::merge::a_merge_refused_over_work_in_the_way_keeps_gits_words_in_the_log`
/// - witness: `remote_integration::a_pull_refused_over_work_in_the_way_keeps_gits_words_in_the_log`
/// - witness: `carried_apply::a_change_here_in_the_way_is_reported_and_nothing_is_written`
pub(crate) const OVERWRITES_WORK: Words = Words("would be overwritten");

/// Work in the way, as the words that make a merge's exit 1 a
/// refusal rather than an answer (`GitCommand::except_when_it_says`).
pub(crate) const REFUSED_OVER_WORK: Relied = Relied::any(&[OVERWRITES_WORK]);

// ---- Rebase (`integrate::rebase`, `sequencer::run`)

/// The `pre-rebase` hook here turned the rebase down, before anything
/// moved.
///
/// - git: `builtin/rebase.c` `The pre-rebase hook refused to rebase`
/// - witness: `integrate_integration::refused::a_pre_rebase_hook_that_says_no_is_a_report`
pub(crate) const PRE_REBASE_REFUSED: Words = Words("The pre-rebase hook refused to rebase.");

/// The checkout a rebase starts with would write over untracked files
/// here. Not the shorter tail: local changes to tracked files end in the
/// same words.
///
/// - git: `unpack-trees.c` `untracked working tree files would be overwritten by checkout`
/// - witness: `integrate_integration::refused::a_rewrite_whose_first_checkout_would_overwrite_untracked_files_is_turned_down`
/// - witness: `integrate_integration::refused::a_rebase_untracked_files_turn_down_leaves_gits_words_on_its_row`
pub(crate) const UNTRACKED_IN_THE_WAY: Words =
    Words("untracked working tree files would be overwritten by checkout");

/// The head of git's clean-tree check for a rebase (read lower-cased);
/// [`UNSTAGED_CHANGES`] or [`UNCOMMITTED_CHANGES`] follows it.
///
/// - git: `wt-status.c` `cannot %s:`
/// - witness: `integrate_integration::rebase::periodic::a_dirty_tree_stops_a_plain_rebase_before_it_touches_anything`
/// - witness: `integrate_integration::standing_op::periodic::a_rebase_under_a_standing_operation_is_refused_as_a_dirty_tree`
pub(crate) const REBASE_OVER_CHANGES: Words = Words("cannot rebase:");

/// The working tree's half of the clean-tree check (read lower-cased).
///
/// - git: `wt-status.c` `You have unstaged changes`
/// - witness: `integrate_integration::rebase::periodic::a_dirty_tree_stops_a_plain_rebase_before_it_touches_anything`
pub(crate) const UNSTAGED_CHANGES: Words = Words("unstaged changes");

/// The index's half of the clean-tree check (read lower-cased).
///
/// - git: `wt-status.c` `contains uncommitted changes`
/// - witness: `integrate_integration::standing_op::periodic::a_rebase_under_a_standing_operation_is_refused_as_a_dirty_tree`
pub(crate) const UNCOMMITTED_CHANGES: Words = Words("uncommitted changes");

/// Either half of the clean-tree check, after [`REBASE_OVER_CHANGES`].
pub(crate) const REBASE_DIRT: Relied = Relied::any_case(&[UNSTAGED_CHANGES, UNCOMMITTED_CHANGES]);

/// A pick's `prepare-commit-msg` hook said no, inside the rebase's own
/// process.
///
/// - git: `sequencer.c` `'prepare-commit-msg' hook failed`
/// - witness: `integrate_integration::refused::a_squash_its_prepare_commit_msg_hook_turns_down_is_put_back_and_reported`
pub(crate) const PREPARE_HOOK_FAILED: Words = Words("'prepare-commit-msg' hook failed");

// ---- A pick or a revert that left nothing to record (`integrate::pick`, read lower-cased)

/// cherry-pick weighs `--allow-empty` before writing, and stops in words
/// of its own.
///
/// - git: `builtin/commit.c` `The previous cherry-pick is now empty`
/// - witness: `integrate_integration::pick_revert::the_commits_around_an_empty_pick_still_land`
pub(crate) const PICK_NOW_EMPTY: Words = Words("the previous cherry-pick is now empty");

/// A revert's `git commit` refusing an empty commit — on stdout, with
/// stderr empty.
///
/// - git: `wt-status.c` `nothing to commit`
/// - witness: `integrate_integration::pick_revert::the_commits_around_an_empty_revert_still_land`
pub(crate) const NOTHING_TO_COMMIT: Words = Words("nothing to commit");

// ---- Signing (`commit::signing_failed`)

/// gpg or gpgsm gave no signature.
///
/// - git: `gpg-interface.c` `gpg failed to sign the data`
/// - witness: none — it takes a gpg that fails
pub(crate) const GPG_UNSIGNED: Words = Words("gpg failed to sign the data");

/// ssh signing with no key set where the program asks for one.
///
/// - git: `gpg-interface.c` `user.signingKey needs to be set for ssh signing`
/// - witness: none — the scenes in `tests/it` name no key and reach [`SSH_KEY_UNNAMED`]
pub(crate) const SSH_KEY_UNSET: Words = Words("user.signingKey needs to be set for ssh signing");

/// ssh signing with no key named and no command to name one.
///
/// - git: `gpg-interface.c` `either user.signingkey or gpg.ssh.defaultKeyCommand needs to be configured`
/// - witness: `commit_integration::a_commit_git_could_not_sign_is_reported_as_unsigned`
/// - witness: `integrate_integration::refused::a_replay_with_no_signing_key_named_is_put_back_and_reported`
pub(crate) const SSH_KEY_UNNAMED: Words =
    Words("either user.signingkey or gpg.ssh.defaultKeyCommand needs to be configured");

/// The `ssh-keygen` there cannot sign.
///
/// - git: `gpg-interface.c` `ssh-keygen -Y sign is needed for ssh signing`
/// - witness: none — it takes an `ssh-keygen` older than 8.2
pub(crate) const SSH_KEYGEN_CANNOT_SIGN: Words =
    Words("ssh-keygen -Y sign is needed for ssh signing");

/// The key could not pass through its temporary file.
///
/// - git: `gpg-interface.c` `failed writing ssh signing key`
/// - witness: none — it takes a temporary folder that refuses the write
pub(crate) const SSH_KEY_UNWRITTEN: Words = Words("failed writing ssh signing key");

/// The signature could not be read back from its temporary file.
///
/// - git: `gpg-interface.c` `failed reading ssh signing data buffer`
/// - witness: none — it takes a temporary file gone under git
pub(crate) const SSH_SIGNATURE_UNREAD: Words = Words("failed reading ssh signing data buffer");

/// git could not sign, in words of its own that name the signing. Not
/// `failed to write commit object`, which a commit ends with after any of
/// them and after an object store git cannot write to alike.
pub(crate) const SIGNING_FAILED: Relied = Relied::any(&[
    GPG_UNSIGNED,
    SSH_KEY_UNSET,
    SSH_KEY_UNNAMED,
    SSH_KEYGEN_CANNOT_SIGN,
    SSH_KEY_UNWRITTEN,
    SSH_SIGNATURE_UNREAD,
]);

// ---- A switch refused over work in the way (`branch`, read lower-cased)

/// Untracked files the switch would write over or remove.
///
/// - git: `unpack-trees.c` `untracked working tree file`
/// - witness: `branch_integration::periodic::untracked_files_in_the_way_are_a_refusal_too`
pub(crate) const SWITCH_OVER_UNTRACKED: Words = Words("untracked working tree file");

/// A folder the switch would replace holds untracked files.
///
/// - git: `unpack-trees.c` `would lose untracked files`
/// - witness: none — no scene in `tests/it` reaches it
pub(crate) const SWITCH_LOSES_UNTRACKED: Words = Words("would lose untracked files");

/// Local changes the switch would write over.
///
/// - git: `unpack-trees.c` `would be overwritten by checkout`
/// - witness: `branch_integration::periodic::a_move_is_refused_when_the_changes_are_in_the_way`
pub(crate) const SWITCH_OVER_CHANGES: Words = Words("would be overwritten by checkout");

/// The switch's "your work is in the way" refusal, which a stash gets past.
pub(crate) const SWITCH_IN_THE_WAY: Relied = Relied::any_case(&[
    SWITCH_OVER_UNTRACKED,
    SWITCH_LOSES_UNTRACKED,
    SWITCH_OVER_CHANGES,
]);

// ---- Opening a folder (`repo::open`)

/// A folder in no repository, whichever way git looked — and a `.git` git
/// will not take, which it calls the same (a `.git` folder in every
/// release, a `gitdir:` file up to 2.55), so `repo::open` reads the folder
/// for that.
///
/// - git: `setup.c` `not a git repository`
/// - witness: `exec::open_rejects_a_non_repository`
pub(crate) const NOT_A_REPOSITORY: Words = Words("not a git repository");

/// A repository another account owns that `safe.directory` does not name.
///
/// - git: `setup.c` `detected dubious ownership`
/// - witness: `exec::open_names_a_folder_git_will_not_trust_as_untrusted`
pub(crate) const DUBIOUS_OWNERSHIP: Words = Words("detected dubious ownership");

// ---- A configuration file git could not open (`process::config_unread`)

/// What a repository command dies with when a configuration file it found
/// would not open.
///
/// - git: `config.c` `unknown error occurred while reading the configuration files`
/// - witness: `config_unread::an_opening_over_a_configuration_git_cannot_open_names_it`
/// - witness: `config_unread::a_read_runs_once_more_and_a_write_does_not`
pub(crate) const CONFIG_UNREAD: Words =
    Words("unknown error occurred while reading the configuration files");

/// A file git could not open — as a warning before going on, or as the
/// fatal line where it asked for the file first. A URL in the quotes is a
/// remote out of reach, not a file.
///
/// - git: `wrapper.c` `unable to access '%s'`
/// - witness: `config_unread::an_opening_over_a_configuration_git_cannot_open_names_it`
/// - witness: `config_unread::a_read_runs_once_more_and_a_write_does_not`
pub(crate) const FILE_UNOPENED: Words = Words("unable to access '");

/// What opens a line git died with.
///
/// - git: `usage.c` `_("fatal: ")`
/// - witness: `config_unread::an_opening_over_a_configuration_git_cannot_open_names_it`
pub(crate) const FATAL_LEAD: Words = Words("fatal: ");

/// What opens a line git warned with and went on after.
///
/// - git: `usage.c` `_("warning: ")`
/// - witness: `config_unread::a_read_runs_once_more_and_a_write_does_not`
pub(crate) const WARNING_LEAD: Words = Words("warning: ");

// ---- A push turned down (`remote::refusal`)

/// This end turned the ref down itself (`--porcelain`'s ref line).
///
/// - git: `transport.c` `"[rejected]"`
/// - witness: `remote_integration::a_push_that_is_only_out_of_date_reports_gits_own_advice`
pub(crate) const REJECTED: Words = Words("[rejected]");

/// The far side turned the ref down.
///
/// - git: `transport.c` `"[remote rejected]"`
/// - witness: `push_refusals::a_far_side_that_keeps_a_branch_says_which_rule`
pub(crate) const REMOTE_REJECTED: Words = Words("[remote rejected]");

/// Commits on the remote not here yet.
///
/// - git: `transport.c` `"fetch first"`
/// - witness: `remote_integration::a_push_that_is_only_out_of_date_reports_gits_own_advice`
pub(crate) const FETCH_FIRST: Words = Words("(fetch first)");

/// Commits on the remote, here but not in what was sent.
///
/// - git: `transport.c` `"non-fast-forward"`
/// - witness: `remote_integration::a_non_fast_forward_push_is_refused_until_forced`
pub(crate) const NOT_A_FAST_FORWARD: Words = Words("(non-fast-forward)");

/// A lease with no oid under `push.useForceIfIncludes`.
///
/// - git: `transport.c` `"remote ref updated since checkout"`
/// - witness: none — no scene in `tests/it` sets `push.useForceIfIncludes`
pub(crate) const UPDATED_SINCE_CHECKOUT: Words = Words("(remote ref updated since checkout)");

/// This end turned the ref down because the remote holds commits the send
/// would drop: what a fetch answers.
pub(crate) const WOULD_DROP: Relied =
    Relied::any(&[FETCH_FIRST, NOT_A_FAST_FORWARD, UPDATED_SINCE_CHECKOUT]);

/// A lease the remote has left.
///
/// - git: `transport.c` `"stale info"`
/// - witness: `remote_integration::a_remote_branch_that_moved_since_the_last_fetch_is_not_deleted`
/// - witness: `remote_tags_integration::a_lease_pinned_to_a_commit_the_remote_has_left_is_refused`
pub(crate) const STALE_INFO: Words = Words("(stale info)");

/// A plain tag push onto a name the remote holds on another commit.
///
/// - git: `transport.c` `"already exists"`
/// - witness: `remote_tags_integration::the_same_commit_is_weighed_where_the_push_goes`
pub(crate) const ALREADY_EXISTS: Words = Words("(already exists)");

/// The end of a far side's hook refusal — git's own, and the name a forge
/// gives its rules (`protected branch hook declined`).
///
/// - git: `builtin/receive-pack.c` `hook declined`
/// - witness: `remote_integration::a_branch_the_far_side_keeps_comes_back_as_a_refusal_with_its_words`
pub(crate) const HOOK_DECLINED: Words = Words("hook declined");

/// `receive.denyNonFastForwards` over there — the far side's reason, read
/// whole.
///
/// - git: `builtin/receive-pack.c` `"non-fast-forward"`
/// - witness: `push_refusals::a_far_side_that_takes_no_rewrite_is_not_out_of_date`
pub(crate) const FAR_NOT_A_FAST_FORWARD: Words = Words("non-fast-forward");

/// `receive.denyDeletes` over there.
///
/// - git: `builtin/receive-pack.c` `"deletion prohibited"`
/// - witness: `push_refusals::a_far_side_that_keeps_a_branch_says_which_rule`
pub(crate) const DELETION_PROHIBITED: Words = Words("deletion prohibited");

/// `receive.denyCurrentBranch` over there.
///
/// - git: `builtin/receive-pack.c` `"branch is currently checked out"`
/// - witness: `push_refusals::a_far_side_with_the_branch_checked_out_says_so`
pub(crate) const CHECKED_OUT_THERE: Words = Words("branch is currently checked out");

/// `receive.denyDeleteCurrent` over there.
///
/// - git: `builtin/receive-pack.c` `"deletion of the current branch prohibited"`
/// - witness: `push_refusals::a_far_side_that_keeps_a_branch_says_which_rule`
pub(crate) const CURRENT_DELETION_PROHIBITED: Words =
    Words("deletion of the current branch prohibited");

/// A push from a shallow clone the far side takes no shallow update from.
///
/// - git: `builtin/receive-pack.c` `"shallow update not allowed"`
/// - witness: none — no scene in `tests/it` pushes from a shallow clone
pub(crate) const SHALLOW_REFUSED: Words = Words("shallow update not allowed");

/// The far side could not take the pack in; its `remote:` lines say why.
///
/// - git: `builtin/receive-pack.c` `"unpacker error"`
/// - witness: `push_refusals::a_far_side_that_cannot_take_it_now_says_why`
pub(crate) const UNPACKER_ERROR: Words = Words("unpacker error");

/// index-pack's own words on the far side, in the locale it runs in there.
///
/// - git: `builtin/index-pack.c` `pack exceeds maximum allowed size`
/// - witness: `push_refusals::a_far_side_that_cannot_take_it_now_says_why`
pub(crate) const PACK_TOO_LARGE: Words = Words("pack exceeds maximum allowed size");

/// What opens a line the far side wrote, as against this end's git.
///
/// - git: `sideband.c` `DISPLAY_PREFIX "remote: "`
/// - witness: `push_refusals::a_far_side_that_cannot_take_it_now_says_why`
pub(crate) const REMOTE_LEAD: Words = Words("remote:");

// ---- A remote out of reach for want of a credential (`remote::unreached`)

/// HTTPS with no credential to send: no helper answered and the prompt is
/// off (`GIT_TERMINAL_PROMPT=0`).
///
/// - git: `prompt.c` `terminal prompts disabled`
/// - witness: none — it takes an HTTPS remote that asks
pub(crate) const PROMPTS_DISABLED: Words = Words("terminal prompts disabled");

/// A helper's answer turned down.
///
/// - git: `remote-curl.c` `Authentication failed for`
/// - witness: none — it takes an HTTPS remote that asks
pub(crate) const AUTHENTICATION_FAILED: Words = Words("Authentication failed for");

/// HTTPS with no credential to send, or one a helper gave turned down.
pub(crate) const CREDENTIALS_UNSENT: Relied =
    Relied::any(&[PROMPTS_DISABLED, AUTHENTICATION_FAILED]);

/// The server took none of the keys offered — OpenSSH's words, which git
/// repeats as the remote's.
///
/// - openssh: `sshconnect2.c` `Permission denied (%s)`
/// - witness: none — it takes an SSH server
pub(crate) const SSH_KEY_REFUSED: Words = Words("Permission denied (publickey");

/// The server's host key is not in `known_hosts`, and there is no terminal
/// to ask on.
///
/// - openssh: `sshconnect2.c` `Host key verification failed`
/// - witness: none — it takes an SSH server
pub(crate) const HOST_KEY_UNKNOWN: Words = Words("Host key verification failed");

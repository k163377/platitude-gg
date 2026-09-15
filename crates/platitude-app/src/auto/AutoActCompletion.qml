pragma ComponentBehavior: Bound

import QtQuick
// For AppBackend (the argument-conditioned exceptions below): the name resolves per file, and without this
// import the first verb to reach one of them dies on a ReferenceError inside the completion lookup.
import platitude
import platitude.ui

/// Which verbs write, which of them hand their completion to somebody else, and which are owed the status behind
/// their write — the three questions `AutoActDriver.prepareCompletion` asks before a verb runs, and the three
/// lists nothing else reads.
///
/// Kept apart from the verbs because they are read the other way round: a verb is one branch and one sampler,
/// while each of these is one line per verb in a list every verb has to be found in.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
QtObject {
    id: policy


    function isWriteAct(act) {
        return ["publish", "publish-taken", "publish-add", "publish-go",
                "publish-new-go", "commit", "commit-refused", "notice-over-diff",
                "stale-part", "amend", "amend-reset-author",
                "stash", "stash-lands", "stash-file", "stage-many-go",
                "discard-many-go", "take-side-ours", "take-side-theirs",
                "open-mergetool", "discard-file-go", "delete-file-go",
                "discard-staged-go", "switch", "switch-remote", "nav-dbl",
                "rename-branch", "rename-tag", "rename-stash", "rename-taken",
                "rename-remote-go", "rename-local-upstream", "delete-branch",
                "delete-branch-go", "delete-tag-go", "delete-stash-go",
                "delete-remote-go", "remote-refused", "delete-force", "delete-branch-refused",
                "delete-branch-chip", "delete-stood-down",
                "set-upstream-go",
                "delete-stash-row", "stash-apply-row", "stash-pop-row",
                "branch-at-tag", "dbl-local", "dbl-remote", "ref-list-pick", "graph-rename", "move-branch",
                "name-branch", "squash", "reword", "cherry-pick", "reset-soft",
                "fold-across-merge", "fold-off-branch", "fold-first-commit", "fold-unfetched-base", "drop-last-commit",
                "reset-mixed", "reset-hard", "drop-commit-go", "merge-branch",
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "wip-landing-stopped",
                "rebase-plan-run", "rebase-edit-stop", "rebase-edit-stop-out",
                "rebase-onto", "revert-commit", "op-exit-go", "op-exit-lands", "stage-hunk",
                "stage-line", "keep-place", "code-shrink", "discard-hunk-go", "line-back", "diff-follow",
                "line-run",
                "stage-all", "unstage-all", "resolve-all",
                "push", "push-outdated", "tag-refused",
                "force-push", "push-retry", "fetch", "fetch-ref-list",
                "commands", "commands-select", "commands-copy", "commands-sweep",
                "commands-fail", "commands-clear",
                "fetch-recover", "fetch-recover-held",
                "fetch-fail", "fetch-resume"].indexOf(act) >= 0
            // A double-click on a tag writes nothing: it opens the box for a name instead (デザイン規約 §左メニューの所作),
            // and a run held at the write barrier for one waits out the watchdog in silence.
            && !(act === "nav-dbl" && Harness.autoActArg.startsWith("tag:"))
            // The bare form opens the menu's hold row and stops there; only the `go` argument carries the
            // hold through to a write, so a bare run held at the write barrier would wait out the watchdog.
            && !(act === "delete-stash-row" && Harness.autoActArg !== "go")
    }

    /// Whether this verb's picture is of the page the status behind its write leaves, rather than of the page the
    /// write's answer arrives on.
    ///
    /// **The two are different pages, and which one a run reaches is the machine's to decide.** core answers a
    /// write before it publishes the status that write invalidated (`session::write::run_write`), so at the answer
    /// the working-tree row this write is about to take away is still standing and the page has not yet decided
    /// where to put the reader. Both moments settle (`PageSettled`), so the census walked from one names
    /// `WipTallyRow` and from the other `FileRowDelegate` — a generated file moving under a tree nobody touched,
    /// and a gate that refuses to stamp for it.
    ///
    /// **Only where the status is owed.** A fetch that brought nothing down reads the refs and stops without one
    /// (`AfterWrite::Refs`), so a verb named here whose write is one of those waits out the watchdog. What is
    /// waited on, and how a run arms it, is `AutoActDriver.statusOwedFrom`.
    function owesStatus(act) {
        return ["commit", "amend", "op-exit-go"].indexOf(act) >= 0
    }

    function defersCompletion(act) {
        return ["publish", "publish-taken", "publish-remotes", "publish-add",
                "publish-go", "publish-new-go", "publish-dismiss",
                "amend-reset-author", "amend-author",
                "eol-commit", "eol-hover", "commit-face",
                "stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go",
                "diff-file", "conflict-sides", "diff-tick", "line-tools", "hunk-tools",
                "diff-select", "diff-copy", "diff-menu", "diff-copy-removed", "diff-sweep",
                "diff-band-sweep", "diff-bar", "diff-blank", "diff-escape",
                "preview", "preview-unstaged", "preview-staged", "preview-close",
                // The write barrier is behind these, not in front of them: five land on the working tree's own
                // row, which the graph pass after the write is what puts there, and the last has to read the
                // commit it just made.
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "wip-landing-stopped",
                "merge-commit",
                "code-send", "code-grow", "code-shrink", "code-swap",
                "line-back", "diff-follow", "line-run",
                "stage-all", "unstage-all", "resolve-all",
                // The write answers before the listing that lets the stood-in rows go is even asked for, so the
                // write barrier would call the middle of the operation its end.
                "delete-stood-down",
                "keep-place", "colour-place", "delete-branch-go", "nav-fold",
                "nav-peek", "nav-unfold", "nav-peek-rename", "nav-peek-away",
                "nav-peek-into", "nav-peek-out", "nav-peek-shut", "nav-close",
                "nav-filter", "nav-tip", "nav-jump", "nav-reclick", "nav-reclick-away", "nav-rename-far",
                // The scroll it is about is taken in its own sampler, a layout pass after the dispatch: without
                // this the render barrier owns the completion and photographs the list where it opened.
                "nav-pin-edge",
                // The replay has to be under way before the doors can be tried, so the sampler that waits for it is
                // the one owner; the render barrier behind it would otherwise photograph the panes before any of it.
                "doors-held",
                "nav-branch-box", "nav-rename-box", "nav-tag-box",
                // All five end in the rail sampler; without this the render barrier is a second
                // completion owner and photographs the pane before the fold has landed.
                "diff-fold", "diff-unfold", "diff-fold-by-hand", "diff-fold-by-rename",
                "diff-keep-folded",
                // The write barrier is behind this one: the row it makes is put in the sidebar by the read that
                // follows the write, and the write answers first.
                "create-tag",
                // The three that take the branch back. Their landing is the branch arriving on the commit that was
                // asked for **and** the working tree the mode left behind, and those two are published together
                // after the answer (`session::write::run_write` joins them) — so the write barrier stands in front
                // of the claim, not behind it, and the two owners race. `--hard` is the one that loses: it is
                // judged on a tree the barrier photographs before the reset wrote over it.
                "reset-soft", "reset-mixed", "reset-hard",
                // Both wait for the readings that decide the push row's shape, and the second runs its press from
                // there — so the barrier is behind the wait rather than in front of it.
                "tag-menu", "push-tag", "delete-remote-tag", "delete-tag-both", "tag-refused",
                "nav-add-remote", "push-default", "push-target", "remote-menu", "remote-url",
                "publish-remotes-marked", "tags-eye",
                "delete-branch-refused", "delete-branch-chip", "remote-refused", "chip-menu", "chip-menu-current",
                // The press is two writes — the staging in front and the commit behind it — so the write barrier is
                // answered by whichever of them finished first and stands in front of the claim. What these wait for
                // is the editor's own answer reaching the editor (`RepoPage.commitAnswered`).
                "commit", "amend",
                // The bar is what these wait for, and it comes down after the write's own answer.
                "commit-refused", "notice-over-diff", "push-outdated", "stale-part",
                "fold-across-merge", "fold-off-branch", "fold-first-commit", "fold-unfetched-base", "drop-last-commit",
                // The same bar with nothing written behind it: the preview turns the plan down before it opens,
                // so the notice is the whole answer and no write barrier stands in front of it.
                "plan-across-merge", "plan-off-branch", "plan-unfetched-base",
                // Dress only: nothing is written, and the bar is raised through the page's own door.
                "report-tone", "rename-tag-box",
                // The write barrier is behind this one: what it photographs is the box still standing after git's
                // answer, and the answer is what puts the words in it.
                "rename-taken",
                "delete-blocked-tip", "switch-stopped", "switch-lands",
                // The write barrier is in front of this one's claim: the second press is turned away before any
                // write is made, so the answer to the first is not what the run is waiting for.
                "switch-remote-twice",
                // Stops at its question, so the ask bar settling is the completion — a write
                // never comes (the write half is "-go", which stays a write act above).
                "rename-remote", "set-upstream",
                // A write does come, but the subject is the question its answer raises,
                // so the bar settling is the completion (the bar takes 200ms to come down).
                "rename-local-upstream",
                // The write barrier is behind this one: the question is answered from the timer, not before it.
                "set-upstream-go",
                "move-ask", "ask-sweep", "switch-conflicted", "switch-held", "switch-mark",
                // Nothing is written for this one either: a report raised through the page's door and a question
                // raised over it, and the completion is both bars having stopped moving after the press.
                "ask-over-notice",
                // The write barrier is behind these, not in front of them: the commands that clear the way and the
                // move they carry only start once the question standing in the graph has been answered.
                "switch-stopped-go", "switch-conflicted-go", "delete-branch-early", "delete-branch-early-far",
                // Deliberately not a write act: what it photographs is the moment before the answer.
                "delete-gone",
                // The list has to be up before one of its rows can be pressed, and the menu up before the report can
                // say the list stayed — one sampler owns the whole of it.
                "list-menu",
                "ref-list", "ref-list-card", "ref-list-lit", "ref-list-choose", "row-part", "graph-reclick",
                "graph-reclick-list",
                "graph-reclick-scrolled", "graph-reclick-across", "graph-reclick-mark",
                "graph-reclick-still", "graph-reclick-lanes", "rename-box-out",
                // The presses go in one per tick and the rows draw the choice a tick behind the last of them, so
                // the sampler that watches the highlight settle owns the whole of it.
                "graph-choose", "graph-choose-range",
                // And the two that go on into the list the choice puts up.
                "graph-choose-said", "graph-choose-sweep", "graph-choose-diff", "graph-choose-dbl",
                "graph-choose-drop",
                "signature", "signature-tip", "stash-tip", "path-tip", "tip-copy", "tip-sweep",
                "row-card", "card-sweep", "menu-hover",
                // The rest the row opens its card after is a beat away, so the return cannot be made until the
                // sampler has seen the card come up.
                "row-card-return",
                // The press on the card's note and the pane it lands in are one sampler's walk: the card has to be
                // holding a cut message before the note is there to press, and what the press asks for arrives after
                // it. A render barrier in front of that photographs the commit that was open before.
                "card-message", "card-message-esc",
                // The rest on that same note waits on the same first beat, and then on the paint: the colours are
                // read back off the note itself, which has nothing to say until the card has laid it out.
                "card-note-lit",
                "author-card", "author-card-open", "co-authors", "co-authors-open",
                "details-grow", "details-grow-squeeze", "wip-grow", "wip-grow-squeeze",
                "details-fit", "details-select", "details-select-away", "details-sweep", "details-hand", "hash-tip", "hash-tip-counting",
                "corner", "graph-step", "graph-step-edge", "graph-step-far",
                "graph-step-named", "graph-step-dirty", "graph-step-diff", "graph-step-hold",
                "diff-step",
                "diff-step-edge", "changes-step", "changes-step-edge", "wip-step", "wip-step-shut",
                "wip-embedded",
                "changes-fold", "changes-unfold",
                "graph-bar", "graph-bar-away", "pane-bar", "pane-bar-away",
                "text-bar", "text-bar-away", "middle-scroll",
                "graph-tail", "graph-tail-more",
                "graph-head", "graph-head-below", "graph-head-back",
                "graph-head-go", "graph-head-lit", "wip-lanes",
                // Three arrivals behind the press, each from a different tree: the copy's row, the copy's file list,
                // and a file of that copy read on its own. Its own sampler owns the end of all three. The pane at
                // rest waits out the first two and the rows built from them.
                "carried-read", "carried-stand",
                "divider-refuse", "commands-fail-shut", "commands-escape",
                "commands-select", "commands-copy", "commands-sweep",
                "cherry-pick", "merge-branch", "revert-commit", "reword", "edit-message",
                "edit-message-leave", "edit-message-focus",
                // All four end in their own samplers: the plan arrives through the feed, and the three that run wait
                // out the write's own landing (the gone row / the edit marker) behind the write barrier. The last
                // waits out one more status after that — where putting the stop down leaves the reader, which the
                // continuation's own answer comes too early to say (`AutoActDriver.awaitOpExitLanding`).
                "rebase-plan", "rebase-plan-run", "rebase-edit-stop", "rebase-edit-stop-out",
                // The reorder, which finishes inside the same sampler the overview does: the whole carry is one
                // turn once the plan stands, and it writes nothing.
                "plan-fold-carry",
                // And the key pressed on that same face, which arrives through the same feed: what Escape does to
                // the plan is read once the rows are there, not while the read is still out.
                "plan-escape", "plan-escape-held",
                // The opening face and the running replay, each held from an edge of its own and each finished by
                // the sampler that took that edge. Deliberately not write acts: the first writes nothing at all, and
                // what the second photographs is the middle of the write rather than its answer — held at the write
                // barrier it would wait for the very thing that takes its picture away.
                "plan-loading", "replay-running",
                // The three about the right pane's boxes end in the same sampler: the plan arrives through the feed,
                // and each step of the walk is entered off the state the one before it asked for — the last of them
                // waits out a rev-list nobody asked for until the boxes were typed into again.
                "plan-reword-verb", "plan-reword-out", "plan-reword-ask",
                // The same sampler shape, entered one step earlier: this one types before there is a plan at all, so
                // the plan it opens arrives through the feed in the middle of its own walk.
                "plan-amend-kept",
                // The same shape once more: the plan arrives through the feed, and the walk over the right pane's
                // remaining doors is entered a step at a time off what the step before it asked for.
                "plan-details-held",
                "op-exit-lands",
                "push-retry", "fetch-ref-list", "avatar-assign", "avatar-badge",
                "avatar-rest", "avatar-hover", "avatar-tip",
                "find", "find-next", "find-prev", "find-drop",
                // These flows are completed by Main/WindowAutoActDriver. Some still begin here (picker, command
                // failure, recovery), but the page must never photograph their intermediate state before the
                // window-level predicate has answered.
                "open-fetches",
                "open-picker", "commands-clear", "fetch-recover", "fetch-recover-held",
                "clone-dialog", "clone-go", "clone-refused",
                "open-not-a-repo", "open-bare", "open-not-a-repo-retry",
                "open-not-a-repo-cancel", "open-dialog-sweep", "open-fail-tab",
                // Not write acts, though both commit: `quit-waits` photographs the moment before the held write's
                // answer, and `quit-locked` waits the landing out in its own sampler — a write barrier in front of
                // either would hold the shot for an answer the verb is about the absence of.
                "quit-waits", "quit-locked",
                // The same wait over a save the hub holds rather than a session's write: photographed with the
                // save still held, and the exit that joins it is the run's own, after the picture.
                "quit-save-held",
                "open-fail-tab-bare", "open-fail-tab-log", "open-fail-sweep", "identity",
                "identity-half", "identity-tip", "band", "app-menu", "app-menu-reclick", "tab-widths",
                "tab-mark", "tab-name", "tab-drag", "tab-hold", "tab-edge", "tab-carry",
                "tab-pin", "tab-pin-go", "tab-open-go",
                "window-fill", "solo", "gate-sweep", "window-floor",
                "badges", "badges-hover", "badges-hover-early", "graph-stale", "graph-stopped",
                "wip-landing",
                "band-actions", "band-actions-none",
                "band-actions-fold", "band-actions-alert", "band-actions-stopped", "fetch-tip-link",
                "old-git", "old-git-card",
                "old-git-fold", "state", "middle-close", "open-again",
                "force-push-hold", "fetch-busy", "fetch-fail", "fetch-hover", "fetch-resume", "fetch-tip",
                "stash-state",
                "settings-tools", "settings-tools-loading", "settings-switch",
                "settings-repo", "settings-repo-pick", "settings-eol", "settings-git-path",
                "settings-git-leave", "settings-processes",
                "avatar-settings", "avatar-combo", "avatar-row-lit", "avatar-remove"].indexOf(act) >= 0
    }
}

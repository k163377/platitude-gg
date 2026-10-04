pragma ComponentBehavior: Bound

import QtQuick
// For `Harness` (the argument-conditioned exceptions below): without this import the first verb to reach one of
// them dies on a ReferenceError inside the completion lookup.
import platitude
import platitude.ui

/// Which verbs write, which hand their completion to somebody else, and which are owed the status behind their
/// write — the three questions `AutoActDriver.prepareCompletion` asks before a verb runs.
///
/// Kept apart from the verbs because it is read the other way round: one list every verb has to be found in.
QtObject {
    id: policy

    function isWriteAct(act) {
        return ["publish", "publish-taken", "publish-tip", "publish-add", "publish-go", "publish-enter",
                "publish-new-go", "commit", "commit-refused", "notice-over-diff",
                "stale-part", "amend", "amend-reset-author",
                "stash", "stash-lands", "stash-file", "stage-many-go",
                "discard-many-go", "take-side-ours", "take-side-theirs",
                "open-mergetool", "discard-file-go", "delete-file-go",
                "discard-staged-go", "switch", "switch-remote", "nav-dbl",
                "rename-branch", "rename-tag", "rename-stash", "rename-taken",
                "replace-remote-go", "rename-local-upstream", "rename-local-upstream-go",
                "rename-local-upstream-tip", "rename-tag-remote", "rename-tag-remote-go", "rename-tag-remote-tip",
                "delete-branch",
                "delete-branch-go", "delete-tag-go", "delete-stash-go",
                "delete-remote-go", "remote-refused", "delete-force", "delete-branch-refused",
                "delete-upstream-go", "delete-both-go",
                "delete-branch-chip", "delete-stood-down",
                "set-upstream-go", "set-upstream-enter", "publish-upstream",
                "delete-stash-row", "stash-apply-row", "stash-pop-row",
                "branch-at-tag", "dbl-local", "dbl-remote", "ref-list-pick", "graph-rename", "move-branch",
                "name-branch", "squash", "reword", "cherry-pick", "reset-soft",
                "fold-first-commit", "drop-last-commit",
                "reset-mixed", "reset-hard", "drop-commit-go", "merge-branch",
                "pull-go", "pull-ahead",
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
            // A double-click on a tag only opens the box for a name (デザイン規約 §左メニューの所作); held at the
            // write barrier, the run waits out the watchdog.
            && !(act === "nav-dbl" && Harness.autoActArg.startsWith("tag:"))
            // Bare, this only opens the menu's hold row; only `go` writes (same wait as above).
            && !(act === "delete-stash-row" && Harness.autoActArg !== "go")
    }

    /// Whether this verb's picture is of the page the status behind its write leaves — the later of the two
    /// pages a write reaches.
    ///
    /// **Which of the two a run reaches is the machine's to decide.** core answers a write before it publishes the
    /// status that write invalidated (`session::write::run_write`), so at the answer the working-tree row the write
    /// takes away still stands. Both moments settle (`PageSettled`), so the census walked from one names
    /// `WipTallyRow` and from the other `FileRowDelegate` — a generated file moving under an untouched tree, and a
    /// gate that refuses to stamp for it.
    ///
    /// **Only where the status is owed**: a fetch that brought nothing down stops without one (`AfterWrite::Refs`),
    /// so a verb named here whose write ends that way waits out the watchdog. The wait is
    /// `AutoActDriver.statusOwedFrom`.
    function owesStatus(act) {
        return ["commit", "amend", "op-exit-go"].indexOf(act) >= 0
    }

    function defersCompletion(act) {
        return ["publish", "publish-taken", "publish-tip", "publish-remotes", "publish-add",
                "publish-go", "publish-enter", "publish-new-go", "publish-dismiss",
                "amend-reset-author", "amend-author",
                "eol-commit", "eol-hover", "commit-face", "op-exit",
                "stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go",
                "diff-file", "conflict-sides", "diff-tick", "line-tools", "hunk-tools",
                "diff-select", "diff-copy", "diff-menu", "diff-copy-removed", "diff-sweep",
                "diff-band-sweep", "diff-bar", "diff-blank", "diff-escape",
                "diff-split", "split-tools", "split-copy",
                "preview", "preview-unstaged", "preview-staged", "preview-close",
                // The write barrier is behind these: the stops land on the working tree's own row, which the graph
                // pass after the write puts there, and `merge-commit` has to read the commit it just made.
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "wip-landing-stopped",
                "merge-commit",
                "code-send", "code-grow", "code-shrink", "code-swap",
                "line-back", "diff-follow", "line-run",
                "stage-all", "unstage-all", "resolve-all",
                // The write answers before the listing that lets the stood-in rows go is asked for; the write
                // barrier would call the middle of the operation its end.
                "delete-stood-down",
                "keep-place", "colour-place", "delete-branch-go", "nav-fold",
                "nav-peek", "nav-unfold", "nav-peek-rename", "nav-peek-away",
                "nav-peek-into", "nav-peek-out", "nav-peek-shut", "nav-close", "menu-peek",
                "nav-filter", "nav-tip", "nav-jump", "nav-open", "nav-open-foot", "nav-open-then", "nav-open-held",
                "nav-drag-open", "nav-peek-open", "nav-open-tip", "nav-open-tag", "nav-follow", "nav-follow-lit",
                "nav-reclick", "nav-reclick-away", "nav-rename-far",
                // Framed in their own sampler once the fold, the pick's walk, the card or the restore they may ask for
                // has landed (`AutoActRecoverVerbs`).
                "recover", "recover-open",
                // Scrolled in its own sampler a layout pass after the dispatch; the render barrier would photograph
                // the list where it opened.
                "nav-pin-edge",
                // The fold is clicked in its own sampler and the seat comes out a layout pass later; the render
                // barrier would photograph the list with every row still in it.
                "nav-pin-seat",
                // The replay has to be under way before the doors can be tried; the render barrier would photograph
                // the panes before any of it.
                "doors-held",
                "nav-branch-box", "nav-rename-box", "nav-tag-box",
                // All five end in the rail sampler; the render barrier would photograph the pane before the fold.
                "diff-fold", "diff-unfold", "diff-fold-by-hand", "diff-fold-by-rename",
                "diff-keep-folded",
                // The write barrier is behind this one: the row it makes is put in the sidebar by the read that
                // follows the write, and the write answers first.
                "create-tag",
                // Their landing is the branch on the asked-for commit **and** the tree the mode left, published
                // together after the answer (`session::write::run_write`) — at the write barrier `--hard` is
                // photographed on the tree before the reset wrote over it.
                "reset-soft", "reset-mixed", "reset-hard",
                // The tag verbs wait for the readings that decide the menu's rows, and the pressing ones press from
                // there — so the barrier is behind the wait.
                "tag-menu", "push-tag", "delete-remote-tag", "delete-tag-both", "tag-refused",
                "tag-chip-menu", "tag-line-menu",
                // The menu and its WORKTREE card open from a sampler once the row is laid out; the pressing ones click
                // from there and wait for the list the copy leaves, or for the bar git's refusal comes down in.
                "worktree-menu", "worktree-remove", "worktree-remove-refused", "worktree-graph",
                // The same for the rows that make a copy (`AutoActCopyVerbs`); the two that land stand the tab in the
                // new copy, which the window reads (`WindowTabActs`).
                "worktree-add-menu", "worktree-box", "worktree-new-refused", "worktree-add", "worktree-new",
                "nav-add-remote", "push-default", "push-target", "push-hover", "remote-menu", "remote-url",
                "publish-remotes-marked", "tags-eye",
                "delete-branch-refused", "delete-branch-chip", "remote-refused", "chip-menu", "chip-menu-current",
                // The card has to be up before the row it came for can be read; the second waits for the line under
                // the pointer as well.
                "pull-menu", "pull-blocked",
                // The press is two writes (the staging, then the commit), so the write barrier answers on the first;
                // these wait for the editor's own answer (`RepoPage.commitAnswered`).
                "commit", "amend",
                // The bar these wait for comes down after the write's own answer.
                "commit-refused", "notice-over-diff", "push-outdated", "stale-part",
                "fold-first-commit", "drop-last-commit",
                // The same bar with nothing written: the preview turns the plan down before it opens.
                "plan-off-branch",
                // Dress only: nothing is written, and the bar is raised through the page's own door.
                "report-tone", "rename-tag-box",
                // Photographs the box still standing after git's answer, which is what puts the words in it.
                "rename-taken",
                "delete-blocked-tip", "switch-stopped", "switch-lands",
                // The second press is turned away before any write, so the subject is past the first one's answer.
                "switch-remote-twice",
                // Stops at its question, so the ask bar settling is the completion.
                "replace-remote", "replace-remote-tip", "set-upstream", "set-upstream-list",
                // A write does come, but the subject is the question its answer raises,
                // so the bar settling is the completion.
                "rename-local-upstream", "rename-local-upstream-go", "rename-local-upstream-tip",
                // The tag's own pair, one wait longer: a remote's `refs/tags/` has no local record, so that reading
                // has to be in before the rename that raises the question is made.
                "rename-tag-remote", "rename-tag-remote-go", "rename-tag-remote-tip",
                // The write barrier is behind these two: the question is answered from the timer, after it.
                "set-upstream-go", "set-upstream-enter",
                // And behind this one twice over: the run is about the press made once the status behind the
                // upstream's write has been read back.
                "publish-upstream",
                "move-ask", "ask-sweep", "switch-conflicted", "switch-held", "switch-mark",
                // Nothing written: a report and a question over it, complete once both bars stop moving.
                "ask-over-notice",
                // The write barrier is behind these: the commands that clear the way and the
                // move they carry only start once the question standing in the graph has been answered.
                "switch-stopped-go", "switch-conflicted-go", "delete-branch-early", "delete-branch-early-far",
                // Deferred by design: what it photographs is the moment before the answer.
                "delete-gone",
                // The list must be up before a row is pressed, and the menu before the report; one sampler owns it all.
                "list-menu",
                "ref-list", "ref-list-card", "ref-list-lit", "ref-list-choose",
                "ref-list-follow", "ref-list-follow-lit",
                "row-part", "ref-list-leave", "graph-reclick",
                "graph-head-list",
                "graph-reclick-list",
                "graph-reclick-scrolled", "graph-reclick-across", "graph-reclick-mark",
                "graph-reclick-still", "graph-reclick-lanes", "rename-box-out",
                // The presses go in one per tick and the rows draw the choice a tick behind the last, so the
                // sampler that watches the highlight settle owns the whole of it.
                "graph-choose", "graph-choose-range",
                // And the ones that go on into the list the choice puts up.
                "graph-choose-said", "graph-choose-sweep", "graph-choose-diff", "graph-choose-dbl",
                "graph-choose-drop",
                "signature", "signature-tip", "stash-tip", "path-tip", "tip-copy", "tip-sweep",
                "row-card", "card-sweep", "menu-hover", "menu-list",
                // The card opens after a rest, so the return waits until the sampler has seen it come up.
                "row-card-return",
                // One sampler's walk: the card must hold a cut message before the note can be pressed, and what the
                // press asks for arrives after it — a render barrier would photograph the commit open before.
                "card-message", "card-message-esc",
                // The same first beat, then the paint: the colours are read off the note, which has nothing to say
                // until the card has laid it out.
                "card-note-lit",
                "author-card", "author-card-open", "co-authors", "co-authors-open",
                "details-grow", "details-grow-squeeze", "wip-grow", "wip-grow-squeeze",
                "details-fit", "details-align", "details-parents", "details-parents-card", "details-parents-go",
                "details-select", "details-select-away", "details-sweep", "details-hand", "field-menu", "key-menu",
                "hash-tip", "hash-tip-counting",
                "corner", "graph-step", "graph-step-edge", "graph-step-far",
                "graph-step-named", "graph-step-dirty", "graph-step-diff", "graph-step-hold",
                "diff-step",
                "diff-step-edge", "changes-step", "changes-step-edge", "wip-step",
                "changes-shut", "wip-shut",
                "wip-embedded",
                "changes-fold", "changes-unfold",
                "graph-bar", "graph-bar-away", "pane-bar", "pane-bar-away", "bar-arrow", "bar-track",
                "text-bar", "text-bar-away", "middle-scroll", "middle-scroll-exit",
                // The surface is put up and filled on the sampler's own ticks; a render barrier would photograph it
                // before the press.
                "middle-hand",
                "graph-tail", "graph-tail-more",
                "graph-head", "graph-head-below", "graph-head-back",
                "graph-head-go", "graph-head-lit", "wip-lanes",
                // Three arrivals behind the press, each from a different tree: the copy's row, its file list, and one
                // of its files. `carried-stand` waits out the first two and the rows built from them.
                "carried-read", "carried-stand",
                "divider-refuse", "commands-fail-shut", "commands-escape",
                "commands-select", "commands-copy", "commands-sweep",
                "cherry-pick", "merge-branch", "revert-commit", "reword", "edit-message",
                // Pressed off the card once nothing else runs; the tip it lands on is waited out behind the write
                // barrier. `pull-ahead` is answered without moving anything — it waits for the panel it opens to
                // show the row git wrote.
                "pull-go", "pull-ahead",
                "edit-message-leave", "edit-message-focus", "edit-message-away",
                // All four end in their own samplers: the plan arrives through the feed, and the three that run wait
                // out the write's own landing (the gone row / the edit marker) behind the write barrier. The last
                // also waits out where putting the stop down leaves the reader, which the continuation's answer
                // comes too early to say (`AutoActDriver.awaitOpExitLanding`).
                "rebase-plan", "rebase-plan-run", "rebase-edit-stop", "rebase-edit-stop-out",
                // The reorder finishes inside the overview's sampler: one turn once the plan stands, nothing written.
                "plan-fold-carry",
                // Escape on that same face: what it does to the plan is read once the rows are there.
                "plan-escape", "plan-escape-held",
                // Each finished by the sampler that latched its edge. `replay-running` photographs the middle of the
                // write — held at the write barrier it would wait for the thing that takes its picture away.
                "plan-loading", "replay-running",
                // One sampler: the plan arrives through the feed and each step is entered off the state the one
                // before asked for — the last waits out a rev-list only the retyping asks for.
                "plan-reword-verb", "plan-reword-out", "plan-reword-ask",
                // The same shape, entered a step earlier: it types before there is a plan, so the plan arrives
                // through the feed mid-walk.
                "plan-amend-kept",
                // The same shape, walking the right pane's remaining doors.
                "plan-details-held",
                "op-exit-lands",
                "push-retry", "fetch-ref-list", "avatar-assign", "avatar-badge",
                "avatar-rest", "avatar-hover", "avatar-tip",
                "find", "find-next", "find-prev", "find-drop", "find-scroll", "find-hint", "find-hint-key", "band-find",
                // Completed by `WindowAutoActDriver`; some begin here (picker, command failure, recovery), but the
                // shot waits for the window-level predicate.
                "open-fetches",
                "open-picker", "commands-clear", "fetch-recover", "fetch-recover-held",
                "clone-dialog", "clone-go", "clone-refused",
                "open-not-a-repo", "open-bare", "open-not-a-repo-retry",
                "open-not-a-repo-cancel", "open-dialog-sweep", "open-fail-tab",
                // Deferred though both commit: `quit-waits` photographs before the held write's answer, and
                // `quit-locked` waits the landing out in its own sampler.
                "quit-waits", "quit-locked",
                // The same over a save the hub holds: photographed while held; the exit that joins it comes after.
                "quit-save-held",
                "open-fail-tab-bare", "open-fail-tab-log", "open-fail-sweep", "identity",
                "identity-half", "identity-tip", "band", "app-menu", "app-menu-reclick", "tab-widths",
                "ops-panel", "ops-stand", "ops-stand-repos", "ops-stand-copies", "ops-branch",
                "ops-branch-folder", "ops-branch-pick", "ops-copy-pick", "ops-repo-pick",
                "tab-mark", "tab-name", "tab-drag", "tab-hold", "tab-edge", "tab-carry",
                "tab-pin", "tab-pin-go", "tab-open-go", "tab-open-moved-on",
                "window-fill", "solo", "gate-sweep", "window-floor",
                "badges", "badges-hover", "badges-hover-early", "badges-all", "badges-all-hover",
                "graph-stale", "graph-stopped", "no-lfs", "no-lfs-card",
                "wip-landing", "wip-commit-half",
                "band-actions", "band-actions-none",
                "band-actions-fold", "band-actions-alert", "band-actions-stopped", "fetch-tip-link",
                "old-git", "old-git-card",
                "old-git-fold", "state", "middle-close", "open-again",
                "force-push-hold", "fetch-busy", "fetch-fail", "fetch-hover", "fetch-resume", "fetch-tip",
                "stash-state",
                "settings-tools", "settings-tools-loading", "settings-tools-enter", "settings-hand",
                "settings-switch", "settings-sweep",
                "settings-repo", "settings-repo-pick", "settings-eol", "settings-git-path",
                "settings-git-leave", "settings-processes",
                "avatar-settings", "avatar-combo", "avatar-row-lit", "avatar-remove",
                "avatar-enter"].indexOf(act) >= 0
    }
}

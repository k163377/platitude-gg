pragma ComponentBehavior: Bound

import QtQuick

/// Which verbs write, and which of them hand their completion to somebody else — the two questions
/// `AutoActDriver.prepareCompletion` asks before a verb runs, and the two lists nothing else reads.
///
/// Kept apart from the verbs because they are read the other way round: a verb is one branch and one sampler,
/// while each of these is one line per verb in a list every verb has to be found in.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below, so the code under them reads as it
/// did when it was all one file.
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
                "rename-branch", "rename-tag", "rename-stash",
                "rename-remote-go", "rename-local-upstream", "delete-branch",
                "delete-branch-go", "delete-tag-go", "delete-stash-go",
                "delete-remote-go", "remote-refused", "delete-force", "delete-branch-refused",
                "set-upstream-go",
                "delete-stash-row", "stash-apply-row", "stash-pop-row",
                "branch-at-tag", "dbl-local", "dbl-remote", "ref-list-pick", "graph-rename", "move-branch",
                "name-branch", "squash", "reword", "cherry-pick", "reset-soft",
                "reset-mixed", "reset-hard", "drop-commit-go", "merge-branch",
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "rebase-onto", "revert-commit", "op-exit-go", "stage-hunk",
                "stage-line", "keep-place", "discard-hunk-go", "line-back", "diff-follow",
                "line-run",
                "stage-all", "unstage-all", "resolve-all",
                "push", "push-outdated", "tag-refused",
                "force-push", "push-retry", "fetch", "fetch-ref-list",
                "commands", "commands-select", "commands-copy", "commands-sweep",
                "commands-fail", "commands-clear",
                "fetch-recover",
                "fetch-fail", "fetch-resume"].indexOf(act) >= 0
            // A double-click on a tag writes nothing: it opens the box for a name instead (デザイン規約 §左メニューの所作),
            // and a run held at the write barrier for one waits out the watchdog in silence.
            && !(act === "nav-dbl" && AppBackend.autoActArg.startsWith("tag:"))
    }

    function defersCompletion(act) {
        return ["publish", "publish-taken", "publish-remotes", "publish-add",
                "publish-go", "publish-new-go", "publish-dismiss",
                "amend-reset-author", "amend-author",
                "eol-commit", "eol-hover", "commit-face",
                "stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go",
                "diff-file", "conflict-sides", "diff-tick", "line-tools", "hunk-tools",
                "diff-select", "diff-copy", "diff-menu", "diff-copy-removed", "diff-sweep",
                "diff-band-sweep", "diff-bar",
                "preview", "preview-unstaged", "preview-staged",
                // The write barrier is behind these, not in front of them: five land on the working tree's own
                // row, which the graph pass after the write is what puts there, and the last has to read the
                // commit it just made.
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "merge-commit",
                "code-send", "line-back", "diff-follow", "line-run",
                "stage-all", "unstage-all", "resolve-all",
                "keep-place", "colour-place", "delete-branch-go", "nav-fold",
                "nav-peek", "nav-unfold", "nav-peek-rename", "nav-peek-away",
                "nav-peek-into", "nav-peek-out", "nav-peek-shut", "nav-close",
                "nav-filter", "nav-tip", "nav-reclick", "nav-reclick-away", "nav-rename-far",
                "nav-branch-box", "nav-rename-box", "nav-tag-box",
                // The write barrier is behind this one: the row it makes is put in the sidebar by the read that
                // follows the write, and the write answers first.
                "create-tag",
                // Both wait for the readings that decide the push row's shape, and the second runs its press from
                // there — so the barrier is behind the wait rather than in front of it.
                "tag-menu", "push-tag", "delete-remote-tag", "delete-tag-both", "tag-refused",
                "nav-add-remote", "push-default", "remote-menu", "remote-url",
                "publish-remotes-marked", "tags-eye",
                "delete-branch-refused", "remote-refused", "chip-menu", "chip-menu-current",
                // The bar is what these wait for, and it comes down after the write's own answer.
                "commit-refused", "notice-over-diff", "push-outdated", "stale-part",
                // Dress only: nothing is written, and the bar is raised through the page's own door.
                "report-tone", "rename-tag-box",
                "delete-blocked-tip", "switch-stopped", "switch-lands",
                // The write barrier is in front of this one's claim: the second press is turned away before any
                // write is made, so the answer to the first is not what the run is waiting for.
                "switch-remote-twice",
                // Stops at its question, so the ask bar settling is the completion — a write
                // never comes (the write half is "-go", which stays a write act above).
                "rename-remote", "set-upstream",
                // The write barrier is behind this one: the question is answered from the timer, not before it.
                "set-upstream-go",
                "move-ask", "ask-sweep", "switch-conflicted", "switch-held", "switch-mark",
                // The write barrier is behind these, not in front of them: the commands that clear the way and the
                // move they carry only start once the question standing in the graph has been answered.
                "switch-stopped-go", "switch-conflicted-go", "delete-branch-early",
                // Deliberately not a write act: what it photographs is the moment before the answer.
                "delete-gone",
                "ref-list-card", "row-part", "graph-reclick", "graph-reclick-list",
                "graph-reclick-scrolled", "graph-reclick-across", "graph-reclick-mark",
                "graph-reclick-still", "graph-reclick-lanes", "rename-box-out",
                "signature", "signature-tip", "stash-tip", "path-tip", "tip-copy", "tip-sweep",
                "row-card", "card-sweep", "menu-hover",
                "author-card", "author-card-open", "co-authors", "co-authors-open",
                "details-grow", "details-grow-squeeze", "wip-grow", "wip-grow-squeeze",
                "details-fit", "details-select", "details-select-away", "details-sweep",
                "corner", "graph-step", "graph-step-edge", "graph-step-far",
                "graph-step-named", "graph-step-dirty", "graph-step-diff", "graph-step-hold",
                "diff-step",
                "diff-step-edge", "changes-step", "changes-step-edge", "wip-step",
                "changes-fold", "changes-unfold",
                "graph-bar", "graph-bar-away", "pane-bar", "pane-bar-away",
                "text-bar", "text-bar-away", "middle-scroll",
                "graph-tail", "graph-head", "graph-head-below", "graph-head-back",
                "graph-head-go", "graph-head-lit", "wip-lanes",
                "divider-refuse", "commands-fail-shut", "commands-select", "commands-copy", "commands-sweep",
                "cherry-pick", "merge-branch", "revert-commit", "reword", "edit-message",
                "edit-message-leave", "edit-message-focus",
                "push-retry", "fetch-ref-list", "avatar-assign", "avatar-badge",
                "find", "find-next", "find-prev", "find-drop",
                // These flows are completed by Main/WindowAutoActDriver. Some still begin here (picker, command
                // failure, recovery), but the page must never photograph their intermediate state before the
                // window-level predicate has answered.
                "open-fetches",
                "open-picker", "commands-clear", "fetch-recover",
                "open-not-a-repo", "open-bare", "open-not-a-repo-retry",
                "open-not-a-repo-cancel", "open-dialog-sweep", "open-fail-tab",
                "open-fail-tab-bare", "open-fail-tab-log", "open-fail-sweep", "identity",
                "identity-half", "identity-tip", "band", "app-menu", "app-menu-reclick", "tab-widths",
                "tab-mark", "tab-name", "tab-drag", "tab-hold", "tab-edge", "tab-carry",
                "window-fill", "solo", "gate-sweep", "window-floor",
                "badges", "badges-hover", "band-actions", "band-actions-none",
                "band-actions-fold", "band-actions-alert", "old-git", "old-git-card",
                "old-git-fold", "state", "middle-close", "open-again",
                "force-push-hold", "fetch-busy", "fetch-fail", "fetch-resume", "fetch-tip",
                "stash-state",
                "settings-tools", "settings-tools-loading",
                "avatar-settings", "avatar-combo", "avatar-row-lit", "avatar-remove"].indexOf(act) >= 0
    }
}

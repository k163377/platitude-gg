pragma Singleton

import QtQuick

// Stands in for the app's `GitFacts` under `qmltestrunner`, which registers no Rust type (`cargo xtask qmltest` stages
// it into the `platitude` module): only the argument-only facts the row delegates, the discard log's filter, the leave
// question and a remote's menu read, so a real `GraphRowDelegate`, `RefListPopup`, `RecoverPane`, `LeaveFlow` or
// `RemoteRowMenu` can stand in a test at all. The rules are Rust's
// (`models::facts`, `encode`, tested there); a test that leans on an answer here is testing this stand-in, not the app.
QtObject {
    /// `encode::chips_shown`: the chips less the ones `gone` names by key, a held mark gone with its worktree
    /// taken off. `gone` is left undefined by a stand-in view with no model behind it.
    function chipsShown(chips, gone) {
        const out = []
        const taken = gone === undefined || gone === null ? [] : gone
        for (const chip of chips || []) {
            if (taken.indexOf(chip.key) >= 0)
                continue
            if (chip.held && taken.indexOf("held:" + chip.name) >= 0)
                out.push(Object.assign({}, chip, { "held": false, "locked": false }))
            else
                out.push(chip)
        }
        return out
    }
    /// `TreeStanding::is_current`: whether `name` is the branch HEAD is on — none while HEAD is on no branch.
    function isCurrent(tree, name) {
        return tree.branch !== "" && name === tree.branch
    }
    /// `encode::ref_kind_word`: the ref a chip's word names, `""` for the two markers.
    function refKind(kind) {
        return kind === "branch" || kind === "remote" || kind === "tag" ? kind : ""
    }
    /// `encode::named_kind_word`: `refKind`, and `worktree` for the chip of a worktree with no branch (named by its
    /// folder).
    function namedKind(kind) {
        return kind === "worktree" ? "worktree" : refKind(kind)
    }
    /// `encode::shown_matches`: whether a filter's line is in any of the words an entry shows, case folded; a line of
    /// nothing but whitespace leaves every entry in.
    function shownMatches(shown, line) {
        if (line.trim() === "")
            return true
        const needle = line.toLowerCase()
        return shown.some(words => String(words).toLowerCase().indexOf(needle) >= 0)
    }
    /// `encode::file_key`: `<bucket>:<path>`, the key a file row goes by in the pane.
    function fileKey(bucket, path) {
        return bucket + ":" + path
    }
    /// `encode::file_of_key`: that key taken apart at its first colon, `undefined` without one.
    function fileOfKey(key) {
        const cut = key.indexOf(":")
        return cut < 0 ? undefined
                       : { "bucket": key.substring(0, cut), "path": key.substring(cut + 1) }
    }
    /// `menu::lines` through `remoteMenu`: where the remote is, and the origin mark, greyed on the remote the keys
    /// already name; every row greyed while the tab opens.
    function remoteMenu(open, remote, markedOrigin) {
        if (!open)
            return seated({ "rows": ["remote-url", "mark-origin"], "whys": ["opening", "opening"] })
        const marked = remote !== "" && remote === markedOrigin
        return seated({ "rows": ["remote-url", "mark-origin"], "whys": ["", marked ? "already-origin" : ""] })
    }
    /// Each row's seat, as `encode::MenuAnswer` carries it (`menu::Row::place`): its first-level group, its card and
    /// the card's group, and whether it acts on the worktree itself (`menu::Row::on_worktree`) — in core's drawing
    /// order. The app's encode test holds this table to core's, row by row and in order
    /// (`the_test_double_seats_every_row`).
    readonly property var seats: ({
        "branch-here": ["start", "", "", false],
        "reach": ["start", "", "", false],
        "cherry-pick": ["copy", "", "", false],
        "revert": ["copy", "", "", false],
        "apply-changes": ["copy", "", "", false],
        "apply-stash": ["copy", "", "", false],
        "pop-stash": ["copy", "", "", false],
        "squash": ["rewrite", "", "", false],
        "drop": ["rewrite", "", "", false],
        "rebase-interactive": ["rewrite", "", "", false],
        "merge": ["into-here", "", "", false],
        "rebase": ["into-here", "", "", false],
        "reset-soft": ["into-here", "reset", "move", false],
        "reset-mixed": ["into-here", "reset", "move", false],
        "reset-hard": ["into-here", "reset", "move", false],
        "pull": ["sync", "", "", false],
        "push": ["sync", "", "", false],
        "delete-branch": ["delete", "", "", false],
        "delete-remote-branch": ["delete", "", "", false],
        "delete-both": ["delete", "", "", false],
        "drop-stash": ["delete", "", "", false],
        "set-upstream": ["branch-card", "branch", "upstream", false],
        "unset-upstream": ["branch-card", "branch", "upstream", false],
        "worktree-here": ["worktree-card", "worktree", "make", false],
        "worktree-add": ["worktree-card", "worktree", "make", false],
        "remove-worktree": ["worktree-card", "worktree", "delete", true],
        "tag-here": ["tag-card", "tag", "make", false],
        "push-tag": ["tag-card", "tag", "send", false],
        "delete-tag": ["tag-card", "tag", "delete", false],
        "delete-remote-tag": ["tag-card", "tag", "delete", false],
        "delete-tag-both": ["tag-card", "tag", "delete", false],
        "keep-ours": ["resolve", "", "", false],
        "take-theirs": ["resolve", "", "", false],
        "merge-tool": ["resolve", "", "", false],
        "stash-files": ["clear", "", "", false],
        "discard": ["clear", "", "", false],
        "step-pick": ["keep", "", "", false],
        "step-reword": ["keep", "", "", false],
        "step-edit": ["keep", "", "", false],
        "step-squash": ["fold", "", "", false],
        "step-fixup": ["fold", "", "", false],
        "step-drop": ["leave", "", "", false],
        "remote-url": ["configure", "", "", false],
        "mark-origin": ["configure", "", "", false]
    })
    /// A hand-built answer (`rows` / `whys`, in any order, and any other field) as core's comes: its `lines` in
    /// drawing order, each with its seat (`encode::MenuLine`).
    function seated(answer) {
        const order = Object.keys(seats)
        const at = answer.rows.map((row, i) => i)
                              .sort((a, b) => order.indexOf(answer.rows[a]) - order.indexOf(answer.rows[b]))
        const lines = at.map(i => {
            const seat = seats[answer.rows[i]]
            return { "word": answer.rows[i], "why": answer.whys[i], "group": seat[0], "card": seat[1],
                     "cardGroup": seat[2], "onWorktree": seat[3] }
        })
        const out = Object.assign({}, answer, { "lines": lines })
        delete out.rows
        delete out.whys
        return out
    }
}

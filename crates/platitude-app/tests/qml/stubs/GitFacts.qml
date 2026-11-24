pragma Singleton

import QtQuick

// Stands in for the app's `GitFacts` under `qmltestrunner`, which registers no Rust type (`cargo xtask qmltest` stages
// it into the `platitude` module): only the argument-only facts the row delegates, the discard log's filter and the
// leave question read, so a real `GraphRowDelegate`, `RefListPopup`, `RecoverPane` or `LeaveFlow` can stand in a test
// at all. The rules are Rust's
// (`models::facts`, `encode`, tested there); a test that leans on an answer here is testing this stand-in, not the app.
QtObject {
    /// `Oid::hex_is_zero`: the uncommitted row's all-zero id; empty is no id at all.
    function wipOid(hex) {
        return typeof hex === "string" && hex.length > 0 && /^0+$/.test(hex)
    }
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
    /// `encode::file_of_key`: that key taken apart at its first colon, both halves empty without one.
    function fileOfKey(key) {
        const cut = key.indexOf(":")
        return cut < 0 ? { "bucket": "", "path": "" }
                       : { "bucket": key.substring(0, cut), "path": key.substring(cut + 1) }
    }
}

pragma Singleton

import QtQuick

// Stands in for the app's `GitFacts` under `qmltestrunner`, which registers no Rust type (`cargo xtask qmltest` stages
// it into the `platitude` module): only the argument-only facts the row delegates read as they are built, so a real
// `GraphRowDelegate` or `RefListPopup` can stand in a test at all. The rules are Rust's (`models::facts`,
// `encode::labels`, tested there); a test that leans on an answer here is testing this stand-in, not the app.
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
    /// `encode::ref_kind_word`: the ref a chip's word names, `""` for the two markers.
    function refKind(kind) {
        return kind === "branch" || kind === "remote" || kind === "tag" ? kind : ""
    }
    /// `encode::named_kind_word`: `refKind`, and `worktree` for a worktree's folder chip.
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
}

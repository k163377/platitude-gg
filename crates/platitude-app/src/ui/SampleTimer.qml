import QtQuick

// The automation's sampling beat: state is observed on this cadence, and
// nothing is ever declared done by counting its ticks — the verb's own
// causal edges decide (app-ui.md §UI 自動化の因果性). One type instead of
// two hundred magic 25s, so the cadence has a name.
//
// **The name is what holds the rule up.** `cargo xtask waits` reads the
// harness for every span of time a file spells for itself, and an object
// of this type spells none — it is the one place the number stands, the
// way the suites' `bounded` is theirs. So a beat taken from here needs
// no marker and a beat of any file's own needs one; write a bare
// `Timer` with 25 in it and the tool names it, which is the point.
Timer {
    interval: 25
    repeat: true
}

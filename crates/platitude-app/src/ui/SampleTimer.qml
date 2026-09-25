import QtQuick

// The automation's sampling beat (app-ui.md §UI 自動化). A type of its own so the number stands in one place:
// `cargo xtask waits` names every span a harness file spells itself, and an object of this type spells none.
Timer {
    interval: 25
    repeat: true
}

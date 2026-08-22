import QtQuick

// The automation's sampling beat: state is observed on this cadence, and
// nothing is ever declared done by counting its ticks — the verb's own
// causal edges decide (app-ui.md §UI 自動化の因果性). One type instead of
// a hundred and twenty-nine magic 25s, so the cadence has a name.
Timer {
    interval: 25
    repeat: true
}

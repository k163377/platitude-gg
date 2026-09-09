pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The two things a run can be told to do to a page as it opens, neither of which is a verb: show the working tree
/// (`PGG_AUTO_WIP=1`), and pick the newest commit and open its first changed file (`PGG_AUTO_SELECT=1`). What they are
/// for is a picture with something in every pane, and a measurement with a selection to time.
///
/// Each one drives the page through the same door a hand goes through — `showWip` / `activateRow` / `toggleDiff` — so
/// what they exercise is the page's own wiring rather than a second copy of it.
Item {
    id: start

    required property var page
    required property var graphModel
    required property var detailsModel
    required property var diffModel
    required property var worktreeModel
    required property var graphPane

    /// The page's own default selection stays out of the way while one of the two below is picking: it would land
    /// first and be photographed instead (`RepoPage.rowPickedElsewhere`). A `Binding` rather than an assignment,
    /// because the page is handed over before it has read the property once.
    Binding {
        target: start.page
        property: "rowPickedElsewhere"
        value: Harness.autoSelect || Harness.autoPerf || Harness.autoWip
    }

    // PGG_AUTO_WIP=1: open the WIP view once uncommitted changes are known.
    Connections {
        target: start.worktreeModel
        enabled: Harness.autoWip
        function onChanged() {
            if (start.worktreeModel.total > 0 && !start.page.wipShown) {
                start.graphPane.setCurrentRow(0)
                start.page.showWip()
            }
        }
    }

    // PGG_AUTO_SELECT=1: select the newest commit, then open the first changed file's diff — the full pipeline, for the
    // screenshot runs.
    property bool selected: false
    Connections {
        target: start.graphModel
        enabled: Harness.autoSelect && !Harness.autoPerf
        function onStatsChanged() {
            if (start.selected || start.graphModel.rowTotal === 0)
                return
            // **The newest commit, not the newest row.** A dirty working tree puts the WIP row on top, and selecting
            // that one shows the pending changes instead of a commit — no details are asked for, so a measurement that
            // reads the interaction budget off this hook measures nothing and says so (`xtask perf`'s `missing`). Every
            // demo repository is dirty, and a stash written after the tip stands over it in the same way without being
            // a commit of the history to photograph. The rule is the model's, so the page's own default and the perf
            // driver answer it the same way (`GraphModel.newestCommitRow`).
            const row = start.graphModel.newestCommitRow()
            if (row >= 0) {
                start.selected = true
                start.graphPane.setCurrentRow(row)
                start.page.activateRow(start.graphModel.oidAt(row), row)
            }
        }
    }
    Connections {
        target: start.detailsModel
        enabled: Harness.autoSelect && !Harness.autoPerf
        function onChanged() {
            if (start.detailsModel.shaHex !== "" && start.detailsModel.fileTotal > 0
                    && start.diffModel.title === "")
                start.page.toggleDiff("commit", start.detailsModel.filePathAt(0),
                                      start.detailsModel.fileOrigPathAt(0))
        }
    }
}

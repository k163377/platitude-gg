pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What a run can be told to do to a page as it opens, without a verb: show the worktree (`PGG_AUTO_WIP=1`), or
/// pick the newest commit and open its first changed file (`PGG_AUTO_SELECT=1`). Both go through the page's own
/// `showWip` / `activateRow` / `toggleDiff`.
Item {
    id: start

    required property var page
    required property var graphModel
    required property var detailsModel
    required property var diffModel
    required property var unstagedModel
    required property var graphPane

    /// Keeps the page's own default selection out of the way while one of these picks — it would land first and be
    /// photographed (`RepoPage.rowPickedElsewhere`). A `Binding`, because the page is handed over before it has read
    /// the property once.
    Binding {
        target: start.page
        property: "rowPickedElsewhere"
        value: Harness.autoSelect || Harness.autoPerf || Harness.autoWip
    }

    // PGG_AUTO_WIP=1: open the WIP view once uncommitted changes are known.
    Connections {
        target: start.unstagedModel
        enabled: Harness.autoWip
        function onChanged() {
            if (start.unstagedModel.total > 0 && !start.page.wipShown) {
                start.graphPane.setCurrentRow(0)
                start.page.showWip()
            }
        }
    }

    // PGG_AUTO_SELECT=1: select the newest commit, then open its first changed file's diff.
    property bool selected: false
    Connections {
        target: start.graphModel
        enabled: Harness.autoSelect && !Harness.autoPerf
        function onStatsChanged() {
            if (start.selected || start.graphModel.rowTotal === 0)
                return
            // Not row 0: the WIP row (every demo repository is dirty) and a newer stash stand over the tip, and the
            // WIP row asks for no details (`GraphModel.newestCommitRow`).
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

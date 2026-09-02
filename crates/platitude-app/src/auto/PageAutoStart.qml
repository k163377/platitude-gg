pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The three things a run can be told to do to a page as it opens, none of which is a verb: show the working tree
/// (`PG_AUTO_WIP=1`), park the view at one end (`PG_SCROLL_TO`), and pick the newest commit and open its first changed
/// file (`PG_AUTO_SELECT=1`). What they are for is a picture with something in every pane, and a measurement with a
/// selection to time.
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
    required property var sidebarPane

    /// The page's own default selection stays out of the way while one of the three below is picking: it would land
    /// first and be photographed instead (`RepoPage.rowPickedElsewhere`). A `Binding` rather than an assignment,
    /// because the page is handed over before it has read the property once.
    Binding {
        target: start.page
        property: "rowPickedElsewhere"
        value: AppBackend.autoSelect || AppBackend.autoPerf || AppBackend.autoWip
    }

    // PG_AUTO_WIP=1: open the WIP view once uncommitted changes are known.
    Connections {
        target: start.worktreeModel
        enabled: AppBackend.autoWip
        function onChanged() {
            if (start.worktreeModel.total > 0 && !start.page.wipShown) {
                start.graphPane.setCurrentRow(0)
                start.page.showWip()
            }
        }
    }

    // PG_SCROLL_TO=top|bottom|nav-bottom: jump the graph — or the sidebar's branch list — after the final pass settles,
    // using the same clamped math as the wheel. Parks the view once and then stays out of the way: re-running on every
    // pass would drag a background refresh back to the edge, which is the one thing a scrolled view must not do on its
    // own.
    property bool scrolled: false
    Timer {
        id: scrollToTimer
        interval: 600
        onTriggered: {
            start.scrolled = true
            if (AppBackend.scrollTo === "nav-bottom") {
                start.sidebarPane.autoSections.scrollBranchesToEnd()
                return
            }
            start.graphPane.view.contentY =
                start.graphPane.view.clampY(AppBackend.scrollTo === "bottom" ? 1e12 : -1e12)
        }
    }
    Connections {
        target: start.graphModel
        enabled: AppBackend.scrollTo !== "" && !start.scrolled
        function onStatsChanged() {
            if (start.graphModel.finishCount > 0)
                scrollToTimer.restart()
        }
    }

    // PG_AUTO_SELECT=1: select the newest commit, then open the first changed file's diff — the full pipeline, for the
    // screenshot runs.
    property bool selected: false
    Connections {
        target: start.graphModel
        enabled: AppBackend.autoSelect && !AppBackend.autoPerf
        function onStatsChanged() {
            if (start.selected || start.graphModel.rowTotal === 0)
                return
            // **The newest commit, not the newest row.** A dirty working tree puts the WIP row on top, and selecting
            // that one shows the pending changes instead of a commit — no details are asked for, so a measurement that
            // reads the interaction budget off this hook measures nothing and says so (`xtask perf`'s `missing`). Every
            // demo repository is dirty.
            for (let row = 0; row < start.graphModel.rowTotal; row++) {
                const oid = start.graphModel.oidAt(row)
                if (oid !== "" && !GitFacts.wipOid(oid)) {
                    start.selected = true
                    start.graphPane.setCurrentRow(row)
                    start.page.activateRow(oid)
                    return
                }
            }
        }
    }
    Connections {
        target: start.detailsModel
        enabled: AppBackend.autoSelect && !AppBackend.autoPerf
        function onChanged() {
            if (start.detailsModel.shaHex !== "" && start.detailsModel.fileTotal > 0
                    && start.diffModel.title === "")
                start.page.toggleDiff("commit", start.detailsModel.filePathAt(0),
                                      start.detailsModel.fileOrigPathAt(0))
        }
    }
}

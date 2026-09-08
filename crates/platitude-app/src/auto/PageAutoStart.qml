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
        value: Harness.autoSelect || Harness.autoPerf || Harness.autoWip
    }

    // PG_AUTO_WIP=1: open the WIP view once uncommitted changes are known.
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

    // PG_SCROLL_TO=top|bottom|nav-bottom: jump the graph — or the sidebar's branch list — to one end once the page has
    // stopped arriving, using the same clamped math as the wheel. Parks the view once and then stays out of the way:
    // re-running on every pass would drag a background refresh back to the edge, which is the one thing a scrolled
    // view must not do on its own.
    //
    // **The end of the arriving is asked for, not waited out** (規約 §UI 自動化の因果性). What has to be true is that
    // no further pass will rewrite the rows the view was parked against, and a stretch of quiet cannot tell "the
    // passes are over" from "the machine has not got to the next one yet" — it says yes early on a loaded machine and
    // parks the view at an edge the pass after it moves. `PageSettled` is the one rule for that, and it is the same
    // one the census walks from and the page verbs start from. `finishCount` stands beside it because settled is also
    // what a page with nothing coming answers (a tab that was never given a path): a pass has to have landed before
    // there is an end to park at.
    property bool scrolled: false
    /// A binding rather than a call, so the parking has an edge to hear — every term is a property read, and the
    /// dependencies are captured through `settled()` as they would be inline (`WindowCensus.settled` is this shape
    /// for the same reason).
    ///
    /// **The rule stands before the knob, and the order is the whole point**: `&&` stops at the first false, so a
    /// `PG_SCROLL_TO` read first would leave `settled()` unevaluated in every run that did not ask to scroll — which
    /// is every run the gate makes, since no verb sets that variable. A name that went wrong there would then be
    /// found by nobody until someone scrolled by hand. Read first, it is exercised wherever a page opens, and the
    /// answer is still false for the runs that asked for nothing.
    readonly property bool parkReady: PageSettled.settled(start.page) && start.graphModel.finishCount > 0
                                      && Harness.scrollTo !== ""
    onParkReadyChanged: {
        if (!start.parkReady || start.scrolled)
            return
        start.scrolled = true
        if (Harness.scrollTo === "nav-bottom") {
            start.sidebarPane.autoSections.scrollBranchesToEnd()
            return
        }
        start.graphPane.view.contentY =
            start.graphPane.view.clampY(Harness.scrollTo === "bottom" ? 1e12 : -1e12)
    }

    // PG_AUTO_SELECT=1: select the newest commit, then open the first changed file's diff — the full pipeline, for the
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

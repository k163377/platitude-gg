import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// Fetch, and everything the network has to say about fetching (デザイン規約 §リモートから取り込む). The band hands in the
// active page and its own automation latch; the shared width box (`widestText` / `widestCode`) stays the band's to
// assign, since it is measured across all three buttons.
ActionButton {
    id: fetchButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The waiting visual, held up by the band past the fetch that raised it (`TopBar.holdFetchBusy`).
    property bool busyLatched: false
    /// Fetches that failed in a row, whoever asked for them.
    readonly property int fails: fetchButton.curPage !== null ? fetchButton.curPage.pageTab.fetchFailures : 0
    /// Enough of them that the timer was stopped. Only a hold on this button starts it again.
    readonly property bool stopped: fetchButton.curPage !== null && fetchButton.curPage.pageTab.autoFetchSuspended

    kind: "fetch"
    text: fetchButton.stopped ? qsTr("Resume") : "fetch"
    code: !fetchButton.stopped
    // Only the stopped step takes a colour for its word: a run of failures is said by the frame and the mark
    // while the word stays plain (デザイン規約 §長押し — 警告の色は語ではなく 枠と印が持つ).
    tone: fetchButton.stopped ? Theme.danger : Theme.textPrimary
    // Read off the state rather than off `tone`: through the wait the word takes the disabled step like any
    // word that cannot be pressed, and the frame, the ring and the mark are what still say this is the fetch
    // that has been failing (デザイン規約 §暗く落とした段).
    toneDim: fetchButton.stopped ? Theme.dangerDim
             : fetchButton.fails > 0 ? Theme.warningDim
             : Theme.textMuted
    frameColor: fetchButton.stopped ? Theme.danger
                : fetchButton.fails > 0 ? Theme.warning
                : "transparent"
    // Only while the word is still `fetch`: once it reads `Resume`, the word is the news.
    alert: fetchButton.fails > 0 && !fetchButton.stopped
    alertTone: Theme.warning
    holdMs: fetchButton.stopped ? Metrics.holdMs : 0
    // Whoever asked for it, the network shows here: a fetch on the timer turns the button the way a clicked one
    // does.
    busy: fetchButton.busyLatched
          || (fetchButton.curPage !== null
              && (fetchButton.curPage.pageTab.busyOp === "fetch" || fetchButton.curPage.pageTab.autoFetchRunning))
    enabled: fetchButton.curPage !== null
             && fetchButton.curPage.pageTab.remoteCount > 0
             && (fetchButton.stopped || fetchButton.curPage.pageTab.busyCount === 0)
    tip: {
        if (fetchButton.curPage === null)
            return ""
        // Nothing under a pointer that cannot press this. What the button would do is not news while it cannot
        // be done, and both states that grey it out are already said elsewhere on screen: REMOTES counts 0 in
        // the left menu, and another git command running is on this band (デザイン規約 §無効 — ボタンの無効は
        // ツールチップを持たない). Said out loud because a disabled control still takes hover and still opens its
        // attached ToolTip (measured, rules-refs/app-ui.md §hover).
        if (!fetchButton.enabled)
            return ""
        const what = fetchButton.stopped
                     ? qsTr("Automatic fetching stopped after %n failure(s). Hold to start it again.", "",
                            fetchButton.fails)
                     : qsTr("Fetch all remotes and prune deleted branches")
        const why = fetchButton.curPage.pageTab.autoFetchError
        if (fetchButton.fails > 0 && why !== "")
            return what + "\n\n" + why
        if (fetchButton.fails > 0)
            return what
        return what + "\n" + (AppBackend.autoFetchMinutes > 0
                              ? qsTr("Automatically every %n minute(s)", "", AppBackend.autoFetchMinutes)
                              : qsTr("Automatic fetching is off"))
    }
    onActivated: fetchButton.curPage.pageTab.fetch("")
    onHeld: fetchButton.curPage.pageTab.resumeAutoFetch()
}

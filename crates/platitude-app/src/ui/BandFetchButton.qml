import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// Fetch, and everything the network has to say about fetching (デザイン規約 §リモートから取り込む). The shared width
// box (`widestText` / `widestCode`) stays the band's to assign — it is measured across all three buttons.
ActionButton {
    id: fetchButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The waiting visual, held up by the band past the fetch that raised it (`TopBar.holdFetchBusy`).
    property bool busyLatched: false
    /// Fetches that failed in a row, whoever asked for them.
    readonly property int fails: fetchButton.curPage !== null ? fetchButton.curPage.pageTab.fetchFailures : 0
    /// Enough of them that the timer was stopped. Only a press on this button starts it again.
    readonly property bool stopped: fetchButton.curPage !== null && fetchButton.curPage.pageTab.autoFetchSuspended

    /// What the button does, with the timer's state in brackets. The count's two sentences, `1 minute`, and `off`
    /// rather than `disabled` are デザイン規約 §リモートから取り込む.
    function fetchLine() {
        if (AppBackend.autoFetchMinutes <= 0)
            return qsTr("Fetch all remotes (automatic fetching off)")
        if (AppBackend.autoFetchMinutes === 1)
            return qsTr("Fetch all remotes (automatically every %n minute)", "", AppBackend.autoFetchMinutes)
        return qsTr("Fetch all remotes (automatically every %n minutes)", "", AppBackend.autoFetchMinutes)
    }

    /// What the press does — the tip's first line, and the only one while nothing has failed.
    function tipHead() {
        return fetchButton.stopped ? qsTr("Resume automatic fetching") : fetchButton.fetchLine()
    }
    /// What the failures add, on a second line; empty while there are none. `%1` is the log's name, kept a
    /// placeholder so the sentence around it can be worded per language (`Words.placeInSentence`). One plural form:
    /// the stop comes at three (`FETCH_FAILURES_BEFORE_STOP`).
    function tipNote() {
        if (fetchButton.stopped)
            return qsTr("Stopped after %n failures — see %1", "", fetchButton.fails)
        if (fetchButton.fails > 0)
            return qsTr("The last fetch failed — see %1")
        return ""
    }

    kind: "fetch"
    text: fetchButton.stopped ? qsTr("Resume") : "fetch"
    code: !fetchButton.stopped
    // Stopped: `textSecondary`, full under a hand (デザイン規約 §リモートから取り込む). Otherwise the plain word — a
    // failure is the mark's (§長押し「警告の色は枠と印が持つ」).
    tone: fetchButton.stopped && !fetchButton.lit ? Theme.textSecondary : Theme.textPrimary
    // Through the wait the word takes the disabled step; ring, mark and frame keep the warning
    // (デザイン規約 §暗く落とした段).
    toneDim: fetchButton.fails > 0 ? Theme.warningDim : Theme.textMuted
    // Framed in every state: a bare word in the panel reads as a label (デザイン規約 §進行中・長押しの定数
    // 「枠を持てる場所にだけ枠を出す」). At rest `borderStrong`: the ordinary line is lost against `Theme.bgRaised`.
    frameColor: fetchButton.stopped ? Theme.warning : Theme.borderStrong
    // The canvas inside the frame: a box on the operated row is a hole onto the work, so the row stays one surface.
    faceColor: Theme.bgSurface
    alert: fetchButton.fails > 0
    alertTone: Theme.warning
    // `Resume` wins the shared box, so its `!` has no air of its own (デザイン規約 §リモートから取り込む
    // 「箱を勝ち取った語の `!` は手前へ詰める」).
    alertTight: fetchButton.stopped
    busy: fetchButton.busyLatched
          || (fetchButton.curPage !== null
              && (fetchButton.curPage.pageTab.busyOp === "fetch" || fetchButton.curPage.pageTab.autoFetchRunning))
    enabled: fetchButton.curPage !== null
             && fetchButton.curPage.pageTab.remoteCount > 0
             && (fetchButton.stopped || fetchButton.curPage.pageTab.busyCount === 0)
    tip: {
        if (fetchButton.curPage === null)
            return ""
        // No tip while disabled (デザイン規約 §無効「ボタンや入力欄の無効はツールチップを持たない」), emptied by hand: a
        // disabled control still opens its ToolTip (rules-refs/app-ui.md「押せないボタンを黙らせる」).
        if (!fetchButton.enabled)
            return ""
        // The press on the first line, the failures on the second (デザイン規約 §リモートから取り込む).
        const note = fetchButton.tipNote()
        return note === "" ? fetchButton.tipHead()
                           : fetchButton.tipHead() + "\n" + note.arg(Words.commandsTitle)
    }
    // The linked place in the tip; empty while nothing has failed (`SharedToolTip` → `Main` → `RepoPage.tipLinkAsked`).
    tipPlace: fetchButton.fails > 0 ? Words.commandsTitle : ""
    tipHref: Words.commandsHref
    // Resuming fetches by itself (`RepoTab::restart_auto_fetch`), so nothing here asks for a second one.
    onActivated: {
        if (fetchButton.stopped)
            fetchButton.curPage.pageTab.resumeAutoFetch()
        else
            fetchButton.curPage.pageTab.fetch("")
    }
}

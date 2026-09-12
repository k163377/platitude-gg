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
    /// Enough of them that the timer was stopped. Only a press on this button starts it again.
    readonly property bool stopped: fetchButton.curPage !== null && fetchButton.curPage.pageTab.autoFetchSuspended

    /// What the button does when it is pressed, and the state of the timer in the brackets after it.
    ///
    /// **The count is branched on rather than marked.** `minute(s)` is a mark for the reader to work out, so the two
    /// readings are two whole sentences. The number stays in both — a sentence with no number in it reads like a
    /// thing that was decided rather than a thing that can be set. **`off` is the setting's own word**, the one its
    /// empty field shows as a placeholder (`SettingsAppPane`); `disabled` is what this window calls a control that
    /// cannot be pressed (§無効), which this is not.
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
    /// What the failures add, on a line of their own. Empty while there are none.
    ///
    /// **git's own words are not in it.** They are a paragraph, not a clause, and the panel that holds them puts
    /// itself up on the first failure by itself (§git が言ったことを読む場所) — so the line names that panel instead,
    /// in the word the panel's own heading wears (`Words.commandsTitle`), and that word is the place a press goes.
    ///
    /// **`%1` is the place**, so what is said about going there stays outside it and can be worded per language
    /// (`Words.placeInSentence`, the shape a ref name in a sentence already had).
    ///
    /// No pair for `failures`: the step that counts them does not exist under three (`FETCH_FAILURES_BEFORE_STOP`).
    /// `%n` still carries the number, so a language that cuts its counts somewhere other than English does has its
    /// forms.
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
    // The stopped step steps back rather than up: nothing is lost by starting the timer again, so the word and the
    // icon go to `textSecondary` together and come up to full under a hand (デザイン規約 §リモートから取り込む). Every
    // other step keeps the plain word — what failed is said by the mark (§長押し — 警告の色は語ではなく枠と印が持つ).
    tone: fetchButton.stopped && !fetchButton.lit ? Theme.textSecondary : Theme.textPrimary
    // Read off the state rather than off `tone`: through the wait the word takes the disabled step like any word
    // that cannot be pressed, and the ring, the mark and the frame where there is one are what still say this is
    // the fetch that has been failing (デザイン規約 §暗く落とした段).
    toneDim: fetchButton.fails > 0 ? Theme.warningDim : Theme.textMuted
    // Only the step that wants a hand wears a frame: a run the timer is still working through will come right on
    // its own, so it says so with the mark alone (デザイン規約 §リモートから取り込む — 枠は手が要る段のもの).
    frameColor: fetchButton.stopped ? Theme.warning : "transparent"
    alert: fetchButton.fails > 0
    alertTone: Theme.warning
    // `Resume` is the wording the shared box is measured for, so it is the one with no air of its own to stand the
    // mark in: one gap back puts the mark inside the frame rather than against it (デザイン規約 §リモートから取り込む
    // — 箱を勝ち取った語の `!`).
    alertTight: fetchButton.stopped
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
        // **One line for what the button does, and a second only where something else has to be told.** The one
        // sentence rule is about the function (デザイン規約 §hover のツールチップ) — the brackets carry whatever that
        // sentence could not — and a run of failures is not part of the function: it is why the button is wearing a
        // mark, and the brackets are spoken for.
        //
        // Pruning is not said on the first line: `fetch` is the command's own word for what it does to branches that
        // are gone, and a second clause for something the first one already means is the clause to drop.
        //
        // **The first line does not move while the timer is still going.** A fetch that failed did not stop it — it
        // will go again on the interval that line names — so what a run of failures adds is the second line and
        // nothing else. Without it the mark on the button has nothing behind it to read.
        const note = fetchButton.tipNote()
        return note === "" ? fetchButton.tipHead()
                           : fetchButton.tipHead() + "\n" + note.arg(Words.commandsTitle)
    }
    // Which word in that sentence is a place, and what the page calls it. Empty while nothing has failed — there is
    // nowhere to send anyone from a button that is working (`SharedToolTip` → `Main` → `RepoPage.tipLinkAsked`).
    tipPlace: fetchButton.fails > 0 ? Words.commandsTitle : ""
    tipHref: Words.commandsHref
    // One press, and the state decides which of the two it means: the stopped button clears the run and fetches on
    // its own (`RepoTab::restart_auto_fetch`), so nothing here asks for a fetch as well.
    onActivated: {
        if (fetchButton.stopped)
            fetchButton.curPage.pageTab.resumeAutoFetch()
        else
            fetchButton.curPage.pageTab.fetch("")
    }
}

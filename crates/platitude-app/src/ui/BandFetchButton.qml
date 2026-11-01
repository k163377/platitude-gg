import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// Fetch, and everything the network has to say about fetching (デザイン規約 §リモートから取り込む): running, failed and
// stopped are all said here and nowhere else — two places telling the same three states move the reader's eye twice
// for nothing. The shared width box (`widestText` / `widestCode`) stays the band's to assign — it is measured across
// all three buttons.
//
// The states: idle — the `fetch` chip. Fetching, whoever asked — ring and frame a step under that state's colour
// (規約 §進行中・回数押しの定数), deaf to presses with `enabled` kept. One or two failures in a row — `fetch`, word and
// mark unchanged, a `warning` `!` past the chip. Three in a row — stopped: `Resume`, no chip, a `warning` frame, word
// and mark down to `textSecondary`, the `!` kept. Every state but fetching takes a press.
ActionButton {
    id: fetchButton

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The waiting visual, held up by the band past the fetch that raised it (`TopBar.holdFetchBusy`).
    property bool busyLatched: false
    /// Fetches that failed in a row, whoever asked for them (counted by `RepoTab::fetch_settled`).
    readonly property int fails: fetchButton.curPage !== null ? fetchButton.curPage.pageTab.fetchFailures : 0
    /// Enough of them that the timer was stopped (`FETCH_FAILURES_BEFORE_STOP`). Only a press on this button starts
    /// it again — one click, no hold, and the warning stays yellow (user decision — ask before changing): `Resume`
    /// only restarts what stopped, losing and spreading nothing (規約 §回数押し「失うものをその場に出す」).
    /// With no timer to stop (automatic fetching off) this step never comes: the button stays `fetch` with the `!`,
    /// having no new move to offer.
    /// Headless: `fetch-resume`, through the button's own press (`TopBar.fetchNow`).
    readonly property bool stopped: fetchButton.curPage !== null && fetchButton.curPage.pageTab.autoFetchSuspended

    /// What the button does, with the timer's state in brackets — the press is the sentence's subject. No `prune`:
    /// what a fetch does to deleted branches is the command's own word, and a clause restating it is the one to drop.
    /// The number stays (`every 1 minute`; without it the interval does not read as a setting), and the unit is
    /// `minute`, as the settings field that takes the value writes `minutes` (規約 §設定の画面). `off` is that field's
    /// word too (its empty placeholder); not `disabled`, which means "cannot be pressed" (規約 §無効). The line stays
    /// as is while the timer runs: a failure does not stop it, so the failure adds the second line only.
    function fetchLine() {
        if (AppBackend.autoFetchMinutes <= 0)
            return qsTr("Fetch all remotes (automatic fetching off)")
        if (AppBackend.autoFetchMinutes === 1)
            return qsTr("Fetch all remotes (automatically every %n minute)", "", AppBackend.autoFetchMinutes)
        return qsTr("Fetch all remotes (automatically every %n minutes)", "", AppBackend.autoFetchMinutes)
    }

    /// What the press does — the tip's first line, and the only one while nothing has failed (first line the
    /// function, second the failure: user decision — ask before changing, 規約 §hover のツールチップ). Stopped, no
    /// brackets — what stopped has no interval — and only the resume is said (user decision — ask before changing):
    /// a resume fetching at once is what makes it one.
    function tipHead() {
        return fetchButton.stopped ? qsTr("Resume automatic fetching") : fetchButton.fetchLine()
    }
    /// What the failures add, on a second line; empty while there are none. Always there once one failed: a mark or a
    /// frame whose reason is not on the target cannot be read. One clause in this application's words (user decision
    /// — ask before changing): git's are a paragraph and would widen the box to the window. Where git's words are,
    /// the line names: the log raises itself on the first pressed failure, and `see GIT COMMANDS` spells the panel's
    /// heading — a name the tip makes pressable (`tipPlace`; 規約 §hover のツールチップ の行き先). `%1` is the log's
    /// name, kept a
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
    // `Resume`: what the press does is resume what stopped. Not `sync` (規約 §リモートから取り込む). The chip comes off in
    // this state only, since what it resumes is the automatic fetching, not a git command (規約 §git 用語のコード表記).
    text: fetchButton.stopped ? qsTr("Resume") : "fetch"
    code: !fetchButton.stopped
    // Stopped: `textSecondary`, full under a hand. Word and mark go down because this stage can wait — what stopped is
    // the timer, and everything the reader does goes on as usual; down to `textSecondary` and no further, the button
    // being pressable (規約 §無効). Otherwise the plain word — a
    // failure is the mark's (§回数押し「警告の色は枠と印が持つ」).
    tone: fetchButton.stopped && !fetchButton.lit ? Theme.textSecondary : Theme.textPrimary
    // Through the wait the word takes the disabled step; ring, mark and frame keep the warning
    // (デザイン規約 §暗く落とした段).
    toneDim: fetchButton.fails > 0 ? Theme.warningDim : Theme.textMuted
    // Framed in every state: a bare word in the panel reads as a label (デザイン規約 §進行中・回数押しの定数
    // 「枠を持てる場所にだけ枠を出す」). At rest `borderStrong`: the ordinary line is lost against `Theme.bgRaised`.
    // The `warning` frame only once stopped, the stage that needs a hand (user decision — ask before changing): after
    // one or two failures the timer goes to fix it by itself, the reader has nothing to do, and the `!` suffices.
    frameColor: fetchButton.stopped ? Theme.warning : Theme.borderStrong
    // The canvas inside the frame: a box on the operated row is a hole onto the work, so the row stays one surface.
    faceColor: Theme.bgSurface
    alert: fetchButton.fails > 0
    alertTone: Theme.warning
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
        // The press on the first line, the failures on the second (`tipHead` / `tipNote`).
        const note = fetchButton.tipNote()
        return note === "" ? fetchButton.tipHead()
                           : fetchButton.tipHead() + "\n" + note.arg(Words.commandsTitle)
    }
    // The linked place in the tip; empty while nothing has failed (`TipSeat` → `Main` → `RepoPage.tipLinkAsked`).
    // Headless: `fetch-tip-link`.
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

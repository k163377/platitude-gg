pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// HEAD's row card and one event under it, read stage by stage: a run stops at the moment its argument names and
/// is photographed there, so the moments laid side by side on the board read as the behaviour (verify-ui スキル
/// §board). The menu's blocker that latches the card (`card-latch`), and the commit that rewrites the row under it
/// (`card-rewrite`). The hand is the pane's, as it hears it (`HandWatch.stir`).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var commitMenu: driver.commitMenu
    readonly property var rowCard: driver.rowCard
    readonly property var rowHost: driver.rowHost

    function run(act, arg) {
        if (act === "card-latch") {
            // HEAD's row card, taken down by a menu raised on its row (a blocker: a dismissal), and the hand that
            // never left: nothing comes back while it stays, a tremor is its own move and the card comes back after
            // the rest. The argument is the stage to stop at — `card` / `menu` / `dismissed` / `back` (`back` when
            // none). `back:row-late` runs `back` with the worktree row held back (`PGG_FAULT_HOLD_WIP_ROW`, raised
            // by `xtask::verify::child` for this argument) and let through after the tremor, so the rebuild that
            // brings the row lands under the rest — the order a slow read gives by chance, made the run's own.
            const colon = arg.indexOf(":")
            const stage = colon < 0 ? arg : arg.substring(0, colon)
            cardLatchTimer.rowLate = colon >= 0 && arg.substring(colon + 1) === "row-late"
            cardLatchTimer.stage = stage === "" ? "back" : stage
            cardLatchTimer.step = 0
            cardLatchTimer.start()
        } else if (act === "card-rewrite") {
            // The card with the hand inside it, and the row rewritten in place under it by the commit this run
            // makes: the card stays. The argument is the stage to stop at — `inside` / `rewritten`.
            page.showWip()
            cardRewriteTimer.stage = arg === "" ? "rewritten" : arg
            cardRewriteTimer.step = 0
            cardRewriteTimer.start()
        } else {
            return false
        }
        return true
    }

    // PGG_AUTO_ACT=card-latch. The hand on HEAD's row as the pane hears it (`HandWatch.stir`), its card out, then the
    // row's menu raised: the blocker takes the card down and latches it (`HoverSeat.onBlockedByChanged`). The menu
    // dismissed, the hand still where it was: nothing comes back; a tremor is the hand's own move, and the card comes
    // back after the rest (デザイン規約 §左メニューの所作「手が動いたかの見切り」). `tremor_card=` is read in the tremor's own
    // turn: the card put out then (`visible`, up from the open call on, where `opened` waits out the enter
    // transition). No stage stops at the tremor: the rest it starts runs on under the picture, so what it shows would
    // be the machine's speed.
    //
    // `back` completes once the page has stopped arriving too: a rebuild landing after the card came back slides the
    // rows under the hand and takes the card down for another rest (`RowHoverHost`, `HoverSeat.displace`), so a
    // picture taken then is of the card gone — the state the run never claimed. The claim is read again at the
    // picture (`Awaited.claim`).
    SampleTimer {
        id: cardLatchTimer
        property string stage: "back"
        property int step: 0
        property point at: Qt.point(-1, -1)
        property string oidHex: ""
        property bool tremorCard: false
        /// The worktree row is held back and let through after the tremor (`run`).
        property bool rowLate: false
        /// Whether the hold was up when it was let through (`GraphModel.letTheWorktreeRowThrough`): a `row-late` run
        /// that found none read the ordinary order and does not pass as the controlled one.
        property bool rowHeld: false
        function done(stage) {
            cardLatchTimer.stop()
            Harness.report("card_latch stage=" + stage + " card=" + rowCard.opened + " menu=" + commitMenu.opened
                              + " latched=" + (rowHost.cardLatched !== "")
                              + " tremor_card=" + cardLatchTimer.tremorCard
                              + (cardLatchTimer.rowLate ? " row_held=" + cardLatchTimer.rowHeld : ""))
            if (stage === "back")
                Awaited.claim("card_latch", () => rowCard.opened)
            driver.complete()
        }
        /// The rows moved under a hand that did not — a rebuild landing under the rest (`HandWatch.stillHeld`): the
        /// card went with them and nothing opens until the hand's own move, so it goes onto HEAD's row where that
        /// stands now, as a reader's would (one place apart at least — a tremor where the row is where it was).
        function handBackOnHead() {
            if (!graphPane.hand.stillHeld || rowCard.opened || graphModel.loading || graphModel.headRow < 0)
                return
            const row = graphPane.view.itemAtIndex(graphModel.headRow)
            if (row === null)
                return
            let next = row.mapToItem(null, row.width / 2, row.height / 2)
            if (next.x === graphPane.hand.at.x && next.y === graphPane.hand.at.y)
                next = Qt.point(next.x + 2, next.y)
            cardLatchTimer.at = next
            graphPane.hand.stir(next)
        }
        onTriggered: {
            if (cardLatchTimer.step === 0) {
                const row = graphModel.loading || graphModel.headRow < 0 ? null
                          : graphPane.view.itemAtIndex(graphModel.headRow)
                if (!Awaited.all("card_latch_row", { "row": row !== null }))
                    return
                cardLatchTimer.oidHex = row.oid_hex
                cardLatchTimer.at = row.mapToItem(null, row.width / 2, row.height / 2)
                graphPane.hand.stir(cardLatchTimer.at)
                cardLatchTimer.step = 1
                return
            }
            if (cardLatchTimer.step === 1) {
                cardLatchTimer.handBackOnHead()
                if (!Awaited.all("card_latch_card", { "card": rowCard.opened }))
                    return
                if (cardLatchTimer.stage === "card") {
                    cardLatchTimer.done("card")
                    return
                }
                page.openRowMenu(cardLatchTimer.oidHex)
                cardLatchTimer.step = 2
                return
            }
            if (cardLatchTimer.step === 2) {
                if (!Awaited.all("card_latch_menu", { "menu": commitMenu.opened, "down": !rowCard.opened }))
                    return
                if (cardLatchTimer.stage === "menu") {
                    cardLatchTimer.done("menu")
                    return
                }
                commitMenu.dismiss()
                cardLatchTimer.step = 3
                return
            }
            if (cardLatchTimer.step === 3) {
                if (!Awaited.all("card_latch_dismissed", { "gone": !commitMenu.opened }))
                    return
                if (cardLatchTimer.stage === "dismissed") {
                    cardLatchTimer.done("dismissed")
                    return
                }
                graphPane.hand.stir(Qt.point(cardLatchTimer.at.x + 2, cardLatchTimer.at.y))
                cardLatchTimer.tremorCard = rowCard.visible
                // The row let through now lands its rebuild under the rest the tremor started.
                if (cardLatchTimer.rowLate)
                    cardLatchTimer.rowHeld = graphModel.letTheWorktreeRowThrough()
                cardLatchTimer.step = 4
                return
            }
            // The tremor was the hand's own move: the ask kept rests again, and the card comes back — and stays,
            // once nothing is still on its way to the page. Rows that landed under the still hand meanwhile took
            // it down until the hand's own next move: the hand goes back onto HEAD's row.
            cardLatchTimer.handBackOnHead()
            if (!Awaited.all("card_latch_back", { "card": rowCard.opened, "settled": PageSettled.settled(page) }))
                return
            cardLatchTimer.done("back")
        }
    }
    // PGG_AUTO_ACT=card-rewrite. The card with the hand inside it (`AppCard.contentPointed`), and the row rewritten
    // in place under it — this run commits what the preset staged, so HEAD's row is of the new commit
    // (`GraphList.rowRewritten` → `RowHoverHost.rowGone`). The card's commit is still in the graph, a row down, and the
    // reread is no reader's doing: the card the hand is in stays, the view putting itself back over that row as far as
    // it can (デザイン規約 §hover のツールチップ「外乱では閉じない」). Its state is final in the turn the reread lands.
    // `inside` stops with the card out, before the commit.
    SampleTimer {
        id: cardRewriteTimer
        property string stage: "rewritten"
        property int step: 0
        property string headBefore: ""
        function done(stage) {
            cardRewriteTimer.stop()
            Harness.report("card_rewrite stage=" + stage + " card=" + rowCard.opened + " moved="
                              + (graphModel.oidAt(graphModel.headRow) !== cardRewriteTimer.headBefore))
            driver.complete()
        }
        onTriggered: {
            if (cardRewriteTimer.step === 0) {
                const row = graphModel.loading || graphModel.headRow < 0 ? null
                          : graphPane.view.itemAtIndex(graphModel.headRow)
                if (!Awaited.all("card_rewrite_row", { "row": row !== null }))
                    return
                cardRewriteTimer.headBefore = graphModel.oidAt(graphModel.headRow)
                graphPane.hand.stir(row.mapToItem(null, row.width / 2, row.height / 2))
                cardRewriteTimer.step = 1
                return
            }
            if (cardRewriteTimer.step === 1) {
                if (!Awaited.all("card_rewrite_card", { "card": rowCard.opened }))
                    return
                rowCard.contentPointed = true
                if (cardRewriteTimer.stage === "inside") {
                    cardRewriteTimer.done("inside")
                    return
                }
                // Only what the preset staged goes in: the tree stays dirty, so its row stays at the top and the new
                // commit takes HEAD's row — the one under the hand — in place (`GraphModel::splice_notified`).
                wipPane.setMessage("feat: something", "")
                cardRewriteTimer.step = 2
                return
            }
            if (cardRewriteTimer.step === 2) {
                // Pressed once the button is live: asked each beat until it takes.
                if (!Awaited.all("card_rewrite_commit",
                                 { "pressed": driver.pressWrite("commit", () => wipPane.pressCommit()) }))
                    return
                cardRewriteTimer.step = 3
                return
            }
            if (!Awaited.all("card_rewrite_landed", {
                                 "moved": graphModel.oidAt(graphModel.headRow) !== cardRewriteTimer.headBefore }))
                return
            cardRewriteTimer.done("rewritten")
        }
    }
}

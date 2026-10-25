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
            // the rest. The argument is the stage to stop at — `card` / `menu` / `dismissed` / `tremor` / `back`
            // (`back` when none).
            cardLatchTimer.stage = arg === "" ? "back" : arg
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
    // back after the rest (デザイン規約 §左メニューの所作「手が動いたかの見切り」).
    SampleTimer {
        id: cardLatchTimer
        property string stage: "back"
        property int step: 0
        property point at: Qt.point(-1, -1)
        property string oidHex: ""
        function done(stage) {
            cardLatchTimer.stop()
            Harness.report("card_latch stage=" + stage + " card=" + rowCard.opened + " menu=" + commitMenu.opened
                              + " latched=" + (rowHost.cardLatched !== ""))
            driver.complete()
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
                if (cardLatchTimer.stage === "tremor") {
                    cardLatchTimer.done("tremor")
                    return
                }
                cardLatchTimer.step = 4
                return
            }
            // The tremor was the hand's own move: the ask kept rests again, and the card comes back.
            if (!Awaited.all("card_latch_back", { "card": rowCard.opened }))
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

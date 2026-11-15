pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui
import platitude.ui.hover

/// The line-ending card (デザイン規約 §改行コードの警告): the `!` a file row or the commit button wears over a warned
/// file and the card it opens under a hand — what the button carries over a warned index (`eol-commit`), a row's card
/// (`eol-hover`), the card going with the warning while the hand is in it (`eol-gone`), and the rows moved under a
/// still hand with the card out (`eol-walk` / `eol-stay`). Built by `AutoActDriver` only when a verb was given.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`, because naming the driver's type would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var wipPane: driver.wipPane

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "eol-commit") {
            // The shot is the state before anyone decides: the frame, the `!` and the card, nothing committed
            // (`--preset eol`, a warning on the index).
            page.showWip()
            repoTab.stageAll()
            // `amend` photographs the button's other wording.
            if (arg === "amend")
                driver.tickAmendQuietly()
            wipPane.setMessage(arg === "" || arg === "amend" ? "feat: something" : arg, "")
            // The pointer goes on before the tree settles, as a real hand's does; pointing after the warning would
            // leave that ordering untested.
            wipPane.pointAtCommit = true
            eolCommitTimer.start()
        } else if (act === "eol-hover") {
            // Writes what a real pointer writes (hover cannot be injected); the tip lands in overlay.png. The argument
            // is the path, or `<bucket>:<path>` for the same path shown in two buckets.
            page.showWip()
            wipPane.pointEol(arg)
            eolHoverTimer.start()
        } else if (act === "eol-gone") {
            // The card out with the hand inside it, and the warning gone with the status read: the row's file staged
            // away from under it, or (`button`) the staged count fallen to nothing. `inside` stops with the card out
            // and the hand in it, before the write (`button:inside` for the button's).
            page.showWip()
            const goneParts = ("" + arg).split(":")
            eolGoneTimer.button = goneParts.indexOf("button") >= 0
            eolGoneTimer.inside = goneParts.indexOf("inside") >= 0
            eolGoneTimer.order = [eolGoneTimer.button ? "staged" : "pointed"]
                .concat(eolGoneTimer.inside ? ["held"] : ["away", "lifted"])
            eolGoneTimer.start()
        } else if (act === "eol-walk" || act === "eol-stay") {
            page.showWip()
            eolWalkTimer.stay = act === "eol-stay"
            eolWalkTimer.stopAt = arg === "" ? "back" : arg
            eolWalkTimer.order = eolWalkTimer.stopAt === "gone" ? ["pointed", "scrolled"]
                                                                : ["pointed", "scrolled", "tremor"]
            eolWalkTimer.start()
        } else {
            return false
        }
        return true
    }
    // Staging has to land before the button can know what it carries.
    PageStage {
        id: eolCommitTimer
        name: "eol_commit"
        // Only the card is waited for — asking after the count first would be reading the input side.
        onTriggered: eolCommitTimer.complete({
            "name": "eol_commit_ready",
            "when": () => ({ "ready": wipPane.eolCardOpen }),
            "report": () => "eol_commit staged=" + worktree.stagedCount
                            + " warned=" + worktree.eolStagedCount
                            + " card=" + wipPane.eolCardOpen })
    }
    // The marks arrive with the status read, so the row named for its sentence is named again, each beat, until its
    // card is up.
    PageStage {
        id: eolHoverTimer
        name: "eol_hover"
        onTriggered: {
            if (eolHoverTimer.at("start")
                    && !eolHoverTimer.input("pointed", () => driver.pointEolCard(Harness.autoActArg)))
                return
            eolHoverTimer.complete({
                "name": "eol_hover_card",
                "when": () => ({ "card": wipPane.eolCardOpen }),
                "report": () => "eol_hover bucket=" + wipPane.pointedEolBucket
                                + " path=" + wipPane.pointedEolPath
                                + " card=" + wipPane.eolCardOpen
                                + " text=" + wipPane.pointedEolText })
        }
    }
    // _PGG_AUTO_ACT=eol-gone. The card is asked for as the hover asks (the row's, or the button's once something is
    // staged), the hand is put in it as the card reads a hand (`AppCard.contentPointed`), and the warning is taken
    // away with one write: the card goes, held hand or not (`WipPane.weighEol`).
    PageStage {
        id: eolGoneTimer
        name: "eol_gone"
        order: ["pointed", "away", "lifted"]
        property bool button: false
        property bool inside: false
        function pressAll(what) {
            return driver.pressWrite(what, () => {
                if (what === "stage-all")
                    repoTab.stageAll()
                else
                    repoTab.unstageAll()
                return true
            })
        }
        /// The card out on a settled page — and, for the button's, the staging before it waited out before the next
        /// write is armed.
        function cardOut() {
            return {
                "card": wipPane.eolCardOpen,
                "wrote": !eolGoneTimer.button || driver.wroteAndSettled(),
                "page": PageSettled.settled(page)
            }
        }
        onTriggered: {
            const op = eolGoneTimer
            if (op.at("start")) {
                if (op.button) {
                    // The button's form stages everything first, pressed until it takes. The button warns only while
                    // it can be pressed (`WipCommitBlock.eolWarned`): a message first, then the pointer on it.
                    op.input("staged", () => {
                        if (!op.pressAll("stage-all"))
                            return false
                        wipPane.setMessage("feat: something", "")
                        wipPane.pointAtCommit = true
                        return true
                    })
                    return
                }
                // The row's form starts on the rows as they are: its card asked for each beat until it is up.
                if (!op.input("pointed", () => driver.pointEolCard("src/flipped.kt")))
                    return
            }
            if (op.at("staged") || op.at("pointed")) {
                if (!Awaited.all("eol_gone_card", op.cardOut()))
                    return
                if (!op.inside) {
                    const away = op.button ? "unstage-all" : "stage-all"
                    // The hand into the card, then the write: pressed until it takes.
                    op.input("away", () => {
                        wipPane.eolHoverCard.contentPointed = true
                        return op.pressAll(away)
                    })
                    return
                }
                op.input("held", () => {
                    wipPane.eolHoverCard.contentPointed = true
                })
            }
            if (op.at("held")) {
                op.complete({
                    "name": "eol_gone_card",
                    "when": () => op.cardOut(),
                    "report": () => "eol_gone button=" + op.button + " stage=inside card="
                                    + wipPane.eolCardOpen + " warned=" + worktree.eolStagedCount })
                return
            }
            if (op.at("away")) {
                if (!Awaited.all("eol_gone", { "gone": !wipPane.eolCardOpen, "page": PageSettled.settled(page) }))
                    return
                // The hand off the button and the card's place before the picture: a hand resting on the commit
                // button asks for its tip after the usual rest, and a tip arriving between the completion and a
                // picture is a window that moved. Lifted, the ask is withdrawn (`TipAsk.asked`), and `Tips.current`
                // is null once no tip is shown or waiting.
                op.input("lifted", () => {
                    wipPane.pointAtCommit = false
                    wipPane.eolHoverCard.contentPointed = false
                })
            }
            op.complete({
                "name": "eol_gone_hand",
                "when": () => ({ "gone": !wipPane.eolCardOpen, "page": PageSettled.settled(page),
                                 "tip": Tips.current === null }),
                "report": () => "eol_gone button=" + op.button + " card=" + wipPane.eolCardOpen
                                + " gone=" + !wipPane.eolCardOpen + " warned=" + worktree.eolStagedCount })
        }
    }
    // _PGG_AUTO_ACT=eol-walk / eol-stay. The hand on the row's words as the pane hears it (`HandWatch.stir`), the card
    // out, then the unstaged list sent down under the still hand: the card goes and the hand is held. A whole row
    // (`eol-walk`): the light goes to the row that arrived under the hand at once, and its own card comes after a
    // tremor and the rest (デザイン規約 §hover のツールチップ「手が動いたかの見切り」). A pixel (`eol-stay`): the row
    // the hand stayed on keeps its light, and the tremor brings its card back after the rest (`HoverSeat.displace`).
    // The report reads the state at the stage it stops at — `gone=` the card down, `slid_lit=` the first row's light,
    // `under_lit=` the light of the row under the hand, `back=` the card up — but for the two read in the tremor's own
    // turn: `tremor=` that row's light, `tremor_card=` the card put out (`visible`, up from the open call on, where
    // `opened` waits out the enter transition). No stage stops at the tremor: the rest it starts runs on under the
    // picture and the census, so what they show would be the machine's speed.
    PageStage {
        id: eolWalkTimer
        name: "eol_walk"
        order: ["pointed", "scrolled", "tremor"]
        property bool stay: false
        /// The stage the run stops at: `gone` (the card down) or `back` (the card up again).
        property string stopAt: "back"
        function unstagedList() {
            const pane = wipPane.bucketPanes.find(pane => pane.section === "unstaged")
            return pane === undefined ? null : pane.list
        }
        /// The row of `src/flipped.kt` in the unstaged list — the one the hand comes onto and the list slides away.
        function slidRow() {
            return wipPane.rowIn("unstaged", "src/flipped.kt")
        }
        /// The row under the hand after the scroll — the one that arrived, or the one the hand stayed on — where the
        /// hand rests (`handAt`, witnessed in the window's frame as the hand came onto the row).
        function rowUnderHand() {
            const list = eolWalkTimer.unstagedList()
            if (list === null)
                return null
            const hand = eolWalkTimer.witnessed("handAt")
            const p = list.mapFromItem(null, hand)
            return list.itemAt(p.x, p.y + list.contentY)
        }
        /// The run's line at the stage it stopped at, read off the rows as they stand; `tremor=` / `tremor_card=` are
        /// the tremor's witnesses, false for a run stopped before it.
        function line(stopped) {
            const op = eolWalkTimer
            const row = op.slidRow()
            const under = op.rowUnderHand()
            return (op.stay ? "eol_stay" : "eol_walk") + " stage=" + stopped
                   + " gone=" + !wipPane.eolCardOpen + " slid_lit=" + (row !== null && row.washLit)
                   + " under_lit=" + (under !== null && under.washLit)
                   + " tremor=" + (stopped !== "gone" && op.witnessed("tremorLit"))
                   + " tremor_card=" + (stopped !== "gone" && op.witnessed("tremorCard"))
                   + " back=" + wipPane.eolCardOpen
                   + " path=" + wipPane.pointedEolPath
        }
        onTriggered: {
            const op = eolWalkTimer
            // Asked again until the card is up: the marks arrive with the status read (`WipPane.pointEol`).
            if (op.at("start") && !op.input("pointed", () => driver.pointEolCard("src/flipped.kt")))
                return
            if (op.at("pointed")) {
                const row = op.slidRow()
                if (!Awaited.all("eol_walk_card", { "card": wipPane.eolCardOpen, "row": row !== null }))
                    return
                op.input("scrolled", () => {
                    wipPane.hand.stir(op.witness("handAt",
                                                 row.mapToItem(null, Qt.point(row.width / 2, row.height / 2))))
                    op.unstagedList().contentY += op.stay ? 1 : Theme.rowHeight
                })
                return
            }
            if (op.at("scrolled")) {
                if (op.stopAt === "gone") {
                    op.complete({ "name": "eol_walk_gone", "when": () => ({ "gone": !wipPane.eolCardOpen }),
                                  "report": () => op.line("gone") })
                    return
                }
                if (!Awaited.all("eol_walk_gone", { "gone": !wipPane.eolCardOpen }))
                    return
                op.input("tremor", () => {
                    const under = op.rowUnderHand()
                    const hand = op.witnessed("handAt")
                    wipPane.hand.stir(Qt.point(hand.x + 2, hand.y))
                    op.witness("tremorLit", under !== null && under.washLit)
                    op.witness("tremorCard", wipPane.eolHoverCard.visible)
                })
                return
            }
            // The row under the hand asks for its card — its own, or the one kept through the hold: it comes after
            // the rest the tremor started.
            op.complete({ "name": "eol_walk_back", "when": () => ({ "card": wipPane.eolCardOpen }),
                          "report": () => op.line("back") })
        }
    }
}

pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The discard log's seat and its list (破棄記録仕様.md), put on screen for a picture — an entry picked from it, on the
/// graph under the band that tells it, the card a rest on an entry opens, and what a press on the band's `Restore`
/// leaves.
// `Item` because `QtObject` has no default property to hold the samplers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var sidebarPane: driver.sidebarPane

    /// The fold this run asked for, so the sampler knows which width it waits for.
    property bool foldWanted: false
    /// The entry this run picks, -1 for none.
    property int pickWanted: -1
    /// Whether this run rests a hand on the first entry the list shows, so the picture holds its card.
    property bool cardWanted: false
    /// The part this run brings back from the picked entry — -1 for all of it — and whether it presses at all.
    property bool restoreWanted: false
    property int restorePart: -1
    /// The tab's counts of restores and of answered writes when the press went out: a restore that brought
    /// something back moves the first, one git refused moves the second alone.
    property int restoresBefore: 0
    property int writesBefore: 0
    /// Whether this run blinks the seat as a discard does, and how many times the seat lit since.
    property bool blinkWanted: false
    property int blinks: 0

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act !== "recover" && act !== "recover-open")
            return false
        // A comma list of looks: `fold` folds the left menu, `lit` lights the seat and holds it lit for the picture,
        // `blink` blinks it as a discard does and frames it once the blink is over,
        // `filter:<words>` types into the list's filter; for `recover-open` a number picks that entry (0 when none is
        // named), `none` picks none, `card` rests a hand on the first entry the list shows, none picked, and
        // `restore` / `part:<n>` press the band's `Restore` for the whole entry / its part n once it is on the graph.
        const looks = arg === "" ? [] : arg.split(",")
        acts.foldWanted = looks.indexOf("fold") >= 0
        acts.cardWanted = act === "recover-open" && looks.indexOf("card") >= 0
        const parts = looks.filter(look => look.startsWith("part:"))
        acts.restoreWanted = act === "recover-open" && (looks.indexOf("restore") >= 0 || parts.length > 0)
        acts.restorePart = parts.length > 0 ? Number(parts[0].substring("part:".length)) : -1
        page.recoverLit = looks.indexOf("lit") >= 0
        acts.blinkWanted = looks.indexOf("blink") >= 0
        acts.blinks = 0
        if (acts.blinkWanted)
            page.blinkRecover()
        const filters = looks.filter(look => look.startsWith("filter:"))
        if (filters.length > 0)
            sidebarPane.autoRecover.filterText = filters[0].substring("filter:".length)
        if (acts.foldWanted)
            page.foldByHand(true)
        acts.pickWanted = -1
        if (act === "recover-open") {
            const named = looks.filter(look => /^\d+$/.test(look))
            const none = looks.indexOf("none") >= 0 || acts.cardWanted
            acts.pickWanted = none ? -1 : named.length > 0 ? Number(named[0]) : 0
            if (!page.recoverOpen)
                page.toggleRecover()
        }
        recoverTimer.start()
        return true
    }

    function frame() {
        const graph = page.pageGraph
        const view = acts.driver.graphPane.view
        Harness.report("recover open=" + page.recoverOpen
                       + " entries=" + page.recoverEntries.length
                       // What the filter left, and whether an entry's card is out.
                       + " listed=" + sidebarPane.autoRecover.shownEntries.length
                       + " card=" + sidebarPane.autoRecover.cardOpen
                       + " pick=" + page.recoverPick
                       + " shown=" + graph.provisionalOn
                       // Whether a part of the picked entry comes back alone too: its band's per-part presses.
                       + " byPart=" + (page.recoverPicked !== null && page.recoverPicked.byPart)
                       + " tip=" + graph.provisionalTipRow
                       + " span=" + graph.provisionalFirst + "-" + graph.provisionalLast
                       // Whether the view went to the picked entry: its tip's row on screen, and where the view stands.
                       + " tipShown=" + (graph.provisionalTipRow >= 0 && view.rowOnScreen(graph.provisionalTipRow))
                       + " y=" + Math.round(view.contentY)
                       // Per part, whether what it would bring back is on the graph.
                       + " reach=" + page.recoverReach.join("/")
                       // The restores this tab answered, and how the last one's work came back.
                       + " restores=" + driver.repoTab.restoreSeq
                       + " how=" + driver.repoTab.restoreHow
                       + " refused=" + (driver.repoTab.lastWriteError !== "")
                       + " lit=" + page.recoverLit
                       // How many times the seat lit since a `blink` began it.
                       + " blinks=" + acts.blinks
                       + " width=" + Math.round(sidebarPane.width))
        driver.complete()
    }

    // Waits for the splitter to hand the pane the width the fold asked for and for the list's read, picks the entry
    // as a press does, and waits for the graph to draw it — then frames it, or first rests the hand on an entry or
    // presses the band's `Restore`.
    SampleTimer {
        id: recoverTimer
        onTriggered: {
            if (sidebarPane.width <= 0 || sidebarPane.height <= 0)
                return
            if (acts.foldWanted !== (Math.round(sidebarPane.width) === Theme.railWidth))
                return
            // A blink is told by its count once it is over: the seat out, the animation stopped.
            if (acts.blinkWanted && !Awaited.all("recover_blink", { "over": !page.recoverBlinking }))
                return
            // The list's read is the reflogs' answer, not the press: an open list frames once it has one.
            if (page.recoverOpen && !page.recoverAnswered)
                return
            if (acts.pickWanted >= 0 && page.recoverPick !== acts.pickWanted) {
                if (acts.pickWanted >= page.recoverEntries.length)
                    return
                page.pickRecover(acts.pickWanted)
            }
            // A picked entry is on the graph once its tip's row is drawn and taken, and the right pane reads that
            // commit (`AutoActDriver.cardSettled`) — or once a walk that took its tips has answered without drawing
            // the row: the tip lies past the window, and the band says so.
            const graph = page.pageGraph
            const drawn = graph.provisionalTipRow >= 0 && page.recoverTipPending === "" && acts.driver.cardSettled
            const past = graph.provisionalWalked && graph.provisionalTipRow < 0
            if (acts.pickWanted >= 0 && !Awaited.all("recover_pick", { "shown": graph.provisionalOn,
                                                                       "drawn_or_past": drawn || past,
                                                                       "walked": graph.provisionalWalked }))
                return
            if (acts.cardWanted && !sidebarPane.autoRecover.restOnShown(0))
                return
            recoverTimer.stop()
            if (acts.cardWanted) {
                cardTimer.start()
                return
            }
            if (acts.restoreWanted) {
                acts.restoresBefore = driver.repoTab.restoreSeq
                acts.writesBefore = driver.repoTab.writeSeq
                if (!driver.pressWrite("restore", () => page.restoreRecover(acts.restorePart))) {
                    recoverTimer.start()
                    return
                }
                restoreTimer.start()
                return
            }
            acts.frame()
        }
    }
    // Counts the seat's lightings while a blink is wanted: each one is the seat coming on.
    Connections {
        target: acts.page
        function onRecoverLitChanged() {
            if (acts.blinkWanted && acts.page.recoverLit)
                acts.blinks++
        }
    }
    // The card is out once it says it opened.
    SampleTimer {
        id: cardTimer
        onTriggered: {
            if (!sidebarPane.autoRecover.cardOpen)
                return
            cardTimer.stop()
            acts.frame()
        }
    }
    // The restore answered — the write's own count moved — and the list read again after it, the entry put down; the
    // write's reads behind it are the driver's to wait out before the picture (`AutoActDriver.writeBarrier`).
    SampleTimer {
        id: restoreTimer
        onTriggered: {
            if (driver.repoTab.writeSeq === acts.writesBefore || driver.repoTab.busyCount > 0)
                return
            // Brought back: the list reads again with the entry put down. Refused: the band stands as it was, with
            // the write's error on the page.
            const brought = driver.repoTab.restoreSeq !== acts.restoresBefore
            if (brought && (page.recoverPick >= 0 || !page.recoverAnswered))
                return
            restoreTimer.stop()
            acts.frame()
        }
    }
}

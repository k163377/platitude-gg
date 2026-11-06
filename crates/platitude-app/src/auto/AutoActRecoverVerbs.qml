pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The discard log's seat and its list, put on screen for a picture — an entry picked from it, on the
/// graph under the band that tells it, the card a rest on an entry opens, what a press on the band's `Restore` leaves,
/// and Esc on an entry on the graph.
///
/// The list reads the reflogs and the record's ref (`refs/pgg/discards`) as it opens, so the fixture is a preset:
/// `--preset discards` holds every kind of entry under forty commits (numbered in `xtask::demo::discards`),
/// `--preset discards-deep` one entry past the graph's window. The loading ring and how the card opens are
/// `tests/qml/tst_recoverpane.qml`'s, the words `tst_recoverwords.qml`'s.
// `Item` because `QtObject` has no default property to hold the samplers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var sidebarPane: driver.sidebarPane

    /// Whether the run is this family's, and whether it opens the list.
    readonly property bool ours: Harness.autoAct === "recover" || Harness.autoAct === "recover-open"
    readonly property bool opens: Harness.autoAct === "recover-open"
    /// The run's argument, a comma list of looks: `fold` folds the left menu, `lit` lights the seat and holds it lit
    /// for the picture, `blink` blinks it as a discard does and frames it once the blink is over, `filter:<words>`
    /// types into the list's filter (no comma: it splits the looks); for `recover-open` a number picks that entry, by
    /// its place in the whole list (0 when none is named), `none` picks none, `card` rests a hand on the first entry
    /// the list shows, none picked (the card is in overlay.png), and `restore` / `part:<n>` press the band's `Restore`
    /// for the whole entry / its part n once it is on the graph — a refused press frames the band still standing.
    readonly property var looks: Harness.autoActArg === "" ? [] : Harness.autoActArg.split(",")
    readonly property var parts: acts.looks.filter(look => look.startsWith("part:"))
    /// The fold this run asked for, so the sampler knows which width it waits for.
    readonly property bool foldWanted: acts.looks.indexOf("fold") >= 0
    /// The entry this run picks, -1 for none.
    readonly property int pickWanted: {
        if (!acts.opens || acts.looks.indexOf("none") >= 0 || acts.cardWanted)
            return -1
        const named = acts.looks.filter(look => /^\d+$/.test(look))
        return named.length > 0 ? Number(named[0]) : 0
    }
    /// Whether this run rests a hand on the first entry the list shows, so the picture holds its card.
    readonly property bool cardWanted: acts.opens && acts.looks.indexOf("card") >= 0
    /// The part this run brings back from the picked entry — -1 for all of it — and whether it presses at all.
    readonly property bool restoreWanted: acts.opens && (acts.looks.indexOf("restore") >= 0 || acts.parts.length > 0)
    readonly property int restorePart: acts.parts.length > 0 ? Number(acts.parts[0].substring("part:".length)) : -1
    /// Whether this run blinks the seat as a discard does, and how many times the seat lit since — nothing lights it
    /// before the dispatch blinks it.
    readonly property bool blinkWanted: acts.ours && acts.looks.indexOf("blink") >= 0
    property int blinks: 0
    /// How many times the band was told an entry's commits are not on the graph (`RepoPage.recoverReach`, the yellow
    /// `Not loaded on the graph yet`) — never, on a graph that holds them, whatever walks under it.
    property int notLoaded: 0

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "recover-escape") {
            page.toggleRecover()
            escapeTimer.start()
            return true
        }
        if (act !== "recover" && act !== "recover-open")
            return false
        page.recoverLit = acts.looks.indexOf("lit") >= 0
        if (acts.blinkWanted)
            page.blinkRecover()
        const filters = acts.looks.filter(look => look.startsWith("filter:"))
        if (filters.length > 0)
            sidebarPane.autoRecover.filterText = filters[0].substring("filter:".length)
        if (acts.foldWanted)
            page.foldByHand(true)
        if (act === "recover-open" && !page.recoverOpen)
            page.toggleRecover()
        // The pick, the hand's rest and the band's press are each the stage the frame waits behind.
        recoverTimer.order = (acts.pickWanted >= 0 ? ["picked"] : [])
            .concat(acts.cardWanted ? ["rested"] : acts.restoreWanted ? ["pressed"] : [])
        recoverTimer.start()
        return true
    }

    /// The run's line, read off the seat, the list, the graph and the tab as they stand.
    function frameLine() {
        const graph = page.pageGraph
        const view = acts.driver.graphPane.view
        return "recover open=" + page.recoverOpen
               + " entries=" + page.recoverEntries.length
               // What the filter left, and whether an entry's card is out.
               + " listed=" + sidebarPane.autoRecover.shownEntries.count
               + " card=" + sidebarPane.autoRecover.cardOpen
               + " pick=" + page.recoverPick
               // The picked entry drawn on the graph for now.
               + " shown=" + graph.provisionalOn
               // Whether a part of the picked entry comes back alone too: its band's per-part presses (false for
               // what one rebase moved).
               + " byPart=" + (page.recoverPicked !== null && page.recoverPicked.byPart)
               // The entry's tip row, and the rows the scroll is held to.
               + " tip=" + graph.provisionalTipRow
               + " span=" + graph.provisionalFirst + "-" + graph.provisionalLast
               // Whether the view went to the picked entry: its tip's row on screen, and where the view stands.
               + " tipShown=" + (graph.provisionalTipRow >= 0 && view.rowOnScreen(graph.provisionalTipRow))
               + " y=" + Math.round(view.contentY)
               // Per part, whether what it would bring back is on the graph.
               + " reach=" + page.recoverReach.join("/")
               // The restores this tab answered, and how the last one's work came back (`whole` / `unstaged` /
               // `stash`).
               + " restores=" + driver.repoTab.restoreSeq
               + " how=" + driver.repoTab.restoreHow
               + " refused=" + (driver.repoTab.lastWriteError !== "")
               + " lit=" + page.recoverLit
               // How many times the seat lit since a `blink` began it.
               + " blinks=" + acts.blinks
               + " width=" + Math.round(sidebarPane.width)
    }

    // Waits for the splitter to hand the pane the width the fold asked for and for the list's read, picks the entry
    // as a press does, and waits for the graph to draw it — then frames it, or first rests the hand on an entry or
    // presses the band's `Restore`.
    PageStage {
        id: recoverTimer
        name: "recover"
        /// A picked entry is on the graph once its tip's row is drawn and taken, and the right pane reads that commit
        /// (`AutoActDriver.cardSettled`) — or once a walk that took its tips has answered without drawing the row: the
        /// tip lies past the window, and the band says so. Nothing to wait for where none is picked.
        function pickShown() {
            if (acts.pickWanted < 0)
                return ({})
            const graph = page.pageGraph
            const drawn = graph.provisionalTipRow >= 0 && page.recoverTipPending === "" && acts.driver.cardSettled
            const past = graph.provisionalWalked && graph.provisionalTipRow < 0
            return { "shown": graph.provisionalOn, "drawn_or_past": drawn || past, "walked": graph.provisionalWalked }
        }
        onTriggered: {
            const op = recoverTimer
            if (!Awaited.all("recover_ready", {
                    "sized": !(sidebarPane.width <= 0 || sidebarPane.height <= 0),
                    "fold": acts.foldWanted === (Math.round(sidebarPane.width) === Theme.railWidth) }))
                return
            // A blink is told by its count once it is over: the seat out, the animation stopped.
            if (acts.blinkWanted && !Awaited.all("recover_blink", { "over": !page.recoverBlinking }))
                return
            // The list's read is the reflogs' answer, not the press: an open list frames once it has one.
            if (!Awaited.all("recover_answered", { "answered": !(page.recoverOpen && !page.recoverAnswered) }))
                return
            // Asked again each beat until the list holds the pick: the press answers whether it does.
            if (acts.pickWanted >= 0 && op.at("start")) {
                if (!Awaited.all("recover_entry", { "listed": acts.pickWanted < page.recoverEntries.length }))
                    return
                if (!op.input("picked", () => {
                    if (page.recoverPick !== acts.pickWanted)
                        page.pickRecover(acts.pickWanted)
                    return page.recoverPick === acts.pickWanted
                }))
                    return
            }
            if (acts.cardWanted) {
                // The rest answers whether it took: the beat after asks again.
                if (!op.input("rested", () => sidebarPane.autoRecover.restOnShown(0)))
                    return
                op.stop()
                cardTimer.start()
                return
            }
            if (acts.restoreWanted) {
                if (acts.pickWanted >= 0 && !Awaited.all("recover_pick", op.pickShown()))
                    return
                // The tab's counts of restores and of answered writes as the press goes out: a restore that brought
                // something back moves the first, one git refused moves the second alone (`restoreTimer`).
                const pressed = op.input("pressed", () => {
                    op.witness("restoresBefore", driver.repoTab.restoreSeq)
                    op.witness("writesBefore", driver.repoTab.writeSeq)
                    return driver.pressWrite("restore", () => page.restoreRecover(acts.restorePart))
                })
                if (!pressed)
                    return
                op.stop()
                restoreTimer.start()
                return
            }
            op.complete({ "name": "recover_pick", "when": () => op.pickShown(), "report": () => acts.frameLine() })
        }
    }
    // Counts the seat's lightings while a blink is wanted: each one is the seat coming on.
    Connections {
        target: acts.page
        function onRecoverLitChanged() {
            if (acts.blinkWanted && acts.page.recoverLit)
                acts.blinks++
        }
        function onRecoverReachChanged() {
            if (acts.page.recoverReach.some(inReach => !inReach))
                acts.notLoaded++
        }
    }
    // PGG_AUTO_ACT=recover-escape (`--preset discards`): Esc on an entry on the graph, entered at `escapePressed` where
    // `Keys.onEscapePressed` enters, twice over. First on an entry the graph walked again from nothing under it (F5:
    // `RepoTab.refreshAll`, whose rows go and come back), with the keyboard on the graph as a press on a row leaves
    // it — it goes back to the list. Then on another entry with the log reading again, where a press may pick nothing
    // but the entry still goes down. **The line is the judgment**: the picture is the list up with nothing picked, which
    // a key that fell through to nobody after a `✕` would also draw. How the key climbs from the list and the filter
    // to the page is `tst_recoverpane.qml`'s.
    PageStage {
        id: escapeTimer
        name: "recover_escape"
        order: ["picked", "reloaded", "escaped", "pickedAgain", "escapedReading"]
        /// The entry is on the graph: its walk answered, its tip's row drawn and taken, the band told.
        function shown() {
            return { "walked": page.pageGraph.provisionalWalked, "drawn": page.pageGraph.provisionalTipRow >= 0,
                     "taken": page.recoverTipPending === "" && acts.driver.cardSettled,
                     "told": page.recoverReach.length > 0 }
        }
        onTriggered: {
            const op = escapeTimer
            const graph = page.pageGraph
            if (op.at("start")) {
                if (Awaited.all("recover_escape_listed", { "answered": page.recoverAnswered,
                                                           "listed": page.recoverEntries.length > 12 }))
                    op.input("picked", () => {
                        page.pickRecover(12)
                        return page.recoverPick === 12
                    })
            } else if (op.at("picked")) {
                if (Awaited.all("recover_escape_shown", op.shown()))
                    op.input("reloaded", () => {
                        op.witness("resets", graph.resetCount)
                        driver.repoTab.refreshAll()
                    })
            } else if (op.at("reloaded")) {
                // Its own stream, which took the rows: an older pass landing moves the finish count as well.
                const reloaded = op.shown()
                reloaded["streamed"] = graph.resetCount !== op.witnessed("resets") && !graph.loading
                if (Awaited.all("recover_escape_reloaded", reloaded))
                    op.input("escaped", () => {
                        acts.driver.graphPane.takeKeyboard()
                        op.witness("hand", acts.driver.graphPane.view.activeFocus)
                        op.witness("took", page.escapePressed())
                        op.witness("list", sidebarPane.autoRecover.entryList.activeFocus)
                    })
            } else if (op.at("escaped")) {
                if (Awaited.all("recover_escape_down", { "down": !graph.provisionalOn,
                                                         "settled": PageSettled.settled(page) }))
                    op.input("pickedAgain", () => {
                        page.pickRecover(3)
                        return page.recoverPick === 3
                    })
            } else if (op.at("pickedAgain")) {
                if (Awaited.all("recover_escape_shown_again", op.shown()))
                    op.input("escapedReading", () => {
                        page.readDiscards()
                        op.witness("reading", page.recoverReading)
                        op.witness("tookReading", page.escapePressed())
                        // Nothing is left standing: the key falls through, and the list stays up.
                        op.witness("again", page.escapePressed())
                    })
            } else {
                op.complete({
                    "name": "recover_escape_done",
                    "when": () => ({ "down": !page.pageGraph.provisionalOn, "answered": page.recoverAnswered }),
                    "report": () => "recover_escape hand=" + op.witnessed("hand")
                                    + " took=" + op.witnessed("took")
                                    + " list=" + op.witnessed("list")
                                    + " reading=" + op.witnessed("reading")
                                    + " tookReading=" + op.witnessed("tookReading")
                                    + " again=" + op.witnessed("again")
                                    + " pick=" + page.recoverPick
                                    + " shown=" + page.pageGraph.provisionalOn
                                    + " open=" + page.recoverOpen
                                    + " listKeys=" + sidebarPane.autoRecover.entryList.activeFocus
                                    + " graphKeys=" + acts.driver.graphPane.view.activeFocus
                                    + " notLoaded=" + acts.notLoaded })
            }
        }
    }
    // The card is out once it says it opened.
    PageStage {
        id: cardTimer
        name: "recover_card"
        onTriggered: cardTimer.complete({
            "name": "recover_card_ready",
            "when": () => ({ "ready": sidebarPane.autoRecover.cardOpen }),
            "report": () => acts.frameLine() })
    }
    // The restore answered — the write's own count moved — and the list read again after it, the entry put down; the
    // write's reads behind it are the driver's to wait out before the picture (`AutoActDriver.writeBarrier`). The
    // counts as the press went out are the press's own witnesses (`recoverTimer`).
    PageStage {
        id: restoreTimer
        name: "recover_restore"
        /// Brought back: the list reads again with the entry put down. Refused: the band stands as it was, with the
        /// write's error on the page.
        function restored() {
            const tab = driver.repoTab
            const brought = tab.restoreSeq !== recoverTimer.witnessed("restoresBefore")
            return {
                "ready": !(tab.writeSeq === recoverTimer.witnessed("writesBefore") || tab.busyCount > 0),
                "put_down": !(brought && (page.recoverPick >= 0 || !page.recoverAnswered)) }
        }
        onTriggered: restoreTimer.complete({
            "name": "recover_restore_ready",
            "when": () => restoreTimer.restored(),
            "report": () => acts.frameLine() })
    }
}

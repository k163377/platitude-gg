pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The commit-details pane: picking text out of it, the grip that grows its description, how it fits the room it
/// is given, and the corner that steps aside for it.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a circle: the driver is the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var gitCorner: driver.gitCorner

    /// Runs `act` if it is one of this family's, and says whether it was. Each verb is named by one family
    /// (`AutoActDriver` asks them in turn).
    function run(act, arg) {
        if (act === "details-failure") {
            driver.deferCompletion()
            driver.detailsModel.request("ffffffffffffffffffffffffffffffffffffffff")
            detailsFailure.start()
        } else if (act === "details-select" || act === "details-select-away") {
            // `<value>[:<row>]` (row 0 by default); `-away` takes `<first>+<then>`.
            const away = act === "details-select-away"
            const plus = arg.indexOf("+")
            detailsSelectTimer.firstWhich = away && plus > 0 ? arg.substring(0, plus) : ""
            const rest = away && plus > 0 ? arg.substring(plus + 1) : arg
            const cut = rest.indexOf(":")
            detailsSelectTimer.which = cut > 0 ? rest.substring(0, cut) : rest
            page.activateRow(graphModel.oidAt(cut > 0 ? Number(rest.substring(cut + 1)) : 0))
            detailsSelectTimer.start()
        } else if (act === "field-menu") {
            // `empty` (and none) / `typed` / `selected` in the commit box, `readonly` on the summary of the commit
            // under HEAD.
            fieldMenuTimer.boxState = arg === "" ? "empty" : arg
            if (arg === "readonly")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(driver.worktree.headOid) + 1))
            else
                page.showWip()
            fieldMenuTimer.start()
        } else if (act === "hash-tip" || act === "hash-tip-counting") {
            page.activateRow(graphModel.oidAt(arg === "" ? 0 : Number(arg)))
            hashTipTimer.counting = act === "hash-tip-counting"
            // Counting, the press goes in with the ask for the tip; otherwise it waits for the tip to be up.
            hashTipTimer.order = hashTipTimer.counting ? ["asked"] : ["asked", "tapped"]
            hashTipTimer.start()
        } else if (act === "details-hand") {
            page.activateRow(graphModel.oidAt(arg === "" ? 0 : Number(arg)))
            detailsHandTimer.start()
        } else if (act === "details-sweep") {
            const sweepCut = arg.indexOf(":")
            detailsSweepTimer.which = sweepCut > 0 ? arg.substring(0, sweepCut) : arg
            page.activateRow(graphModel.oidAt(sweepCut > 0 ? Number(arg.substring(sweepCut + 1)) : 0))
            detailsSweepTimer.start()
        } else if (act === "details-grow" || act === "details-grow-squeeze") {
            page.activateRow(graphModel.oidAt(Number(arg)))
            descGrowTimer.pane = detailsPane
            descGrowTimer.paneName = "details"
            descGrowTimer.squeeze = act === "details-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "wip-grow" || act === "wip-grow-squeeze") {
            // The argument is the description: the box starts empty, and an empty box has nothing to grow.
            page.showWip()
            wipPane.setMessage("feat: write the summary", arg)
            descGrowTimer.pane = wipPane
            descGrowTimer.paneName = "wip"
            descGrowTimer.squeeze = act === "wip-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "details-parents") {
            // `<row>[:<width>]`; `min` asks for nothing, which the split view holds at the pane's floor.
            const widthAt = arg.indexOf(":")
            page.activateRow(graphModel.oidAt(Number(widthAt > 0 ? arg.substring(0, widthAt) : arg)))
            if (widthAt > 0) {
                const width = arg.substring(widthAt + 1)
                page.setDetailsWidth(width === "min" ? 0 : Number(width))
            }
            detailsParentsTimer.start()
        } else if (act === "details-align") {
            // `<row>[:<width>]`, as `details-parents`.
            const alignAt = arg.indexOf(":")
            page.activateRow(graphModel.oidAt(Number(alignAt > 0 ? arg.substring(0, alignAt) : arg)))
            if (alignAt > 0) {
                const alignWidth = arg.substring(alignAt + 1)
                page.setDetailsWidth(alignWidth === "min" ? 0 : Number(alignWidth))
            }
            detailsAlignTimer.start()
        } else if (act === "details-parents-card" || act === "details-parents-go") {
            // `<row>[:<width>]`, as `details-parents`; `-go` then presses the card's second row.
            const cardWidthAt = arg.indexOf(":")
            page.activateRow(graphModel.oidAt(Number(cardWidthAt > 0 ? arg.substring(0, cardWidthAt) : arg)))
            if (cardWidthAt > 0) {
                const cardWidth = arg.substring(cardWidthAt + 1)
                page.setDetailsWidth(cardWidth === "min" ? 0 : Number(cardWidth))
            }
            parentsCardTimer.going = act === "details-parents-go"
            parentsCardTimer.order = parentsCardTimer.going ? ["card_asked", "row_pressed"] : ["card_asked"]
            parentsCardTimer.start()
        } else if (act === "details-fit") {
            // Overflow is glyphs cut at the window's edge, which headless cannot see, so the pane reports the number.
            //
            // No row named keeps the one the page opened on: `Number("")` is 0, and row 0 of a dirty tree (both
            // presets this verb runs on) is the worktree's — the card never settles and the run waits out the
            // watchdog.
            if (arg !== "")
                page.activateRow(graphModel.oidAt(Number(arg)))
            detailsFitTimer.start()
        } else if (act === "corner") {
            // Presets `basic` (corner bare) and `long` (rows run into it) are read as a pair — one half alone frames
            // like a label always on, or always off. The pane is always named (xtask refuses the empty argument), so
            // `wip` has one spelling.
            if (arg === "wip")
                page.showWip()
            else
                page.activateRow(graphModel.oidAt(Number(arg)))
            cornerTimer.start()
        } else if (act === "divider-refuse") {
            // The log's bar has no geometry while the log is shut — open it and let it lay out before measuring.
            if (arg === "log-min" && !page.commandsOpen) {
                page.commandsOpen = true
                splitRefuseTimer.start()
            } else if (arg === "desc-max" || arg === "desc-min") {
                // Row 1: row 0 of every preset is the uncommitted row, which puts the worktree in the right-hand
                // pane and the box this pulls on off screen.
                page.activateRow(graphModel.oidAt(1))
                descGrowTimer.pane = detailsPane
                descGrowTimer.paneName = "details"
                descGrowTimer.squeeze = false
                descGrowTimer.refuse = arg
                descGrowTimer.start()
            } else {
                // Carried past in the dispatch; the bar stays in hand, so the refusal still stands when the
                // completion reads it.
                page.dragDividerPast(arg)
                dividerHeldTimer.start()
            }
        } else {
            return false
        }
        return true
    }
    /// What the window did with the drag past whichever boundary `which` names (`divider-refuse`), read as it stands.
    /// A picture cannot answer it: every refusal frames the same way, and the column widths are not on screen as
    /// numbers.
    function refusalLine(which) {
        return "divider_refuse refuses=" + page.refusalShown
               + " line=" + page.refusalLineShown(which)
               + " case=" + which + " labelW=" + graphPane.labelW
               + " graphW=" + graphPane.graphColW
    }

    /// The `divider-refuse` drags the dispatch carries past itself — every bound but a shut log's and the description
    /// box's: the verb's completion is deferred (`AutoActCompletion.defersCompletion`), so it completes from here, on
    /// the refusal as it stands.
    PageStage {
        id: dividerHeldTimer
        name: "divider_held"
        onTriggered: dividerHeldTimer.complete({ "when": () => ({}),
                                                 "report": () => acts.refusalLine(Harness.autoActArg) })
    }

    PageStage {
        id: detailsFailure
        name: "details_failure"
        onTriggered: detailsFailure.complete({
            "name": "details_failure_ready",
            "when": () => ({ "ready": !(driver.detailsModel.loading
                                        || !driver.repoTab.lastError.startsWith("details: ")) }),
            "report": () => "details_failure loading=false error=true" })
    }
    /// The pane's values are fields the reader drags over (規約 §右のペインの字は掴める). A drag cannot be injected, so
    /// this picks the field out the way `Ctrl+A` does and reads back what it holds — a picture cannot answer this, as
    /// a `Text` in place of the field would draw the identical row. The selection is left standing for the shot.
    PageStage {
        id: detailsSelectTimer
        name: "details_select"
        order: ["asked"]
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        /// The value picked out first (`-away`), or empty.
        property string firstWhich: ""
        /// The selection as it stands, against what the pane shows at that value.
        function line() {
            const op = detailsSelectTimer
            const got = detailsPane.valueRow.selectedValue(op.which)
            const want = detailsPane.valueRow.shownValue(op.which)
            // A second selection must not leave the first lit. Both halves are said: a run that cleared everything
            // would answer `dropped=true` on its own.
            if (op.firstWhich !== "")
                return "details_select_away dropped="
                       + (detailsPane.valueRow.selectedValue(op.firstWhich) === "")
                       + " held=" + (want !== "" && got === want)
                       + " first=" + op.firstWhich
                       + " then=" + op.which
            // `match=` is the whole claim; `text=` rides along for the eye.
            return "details_select match=" + (want !== "" && got === want) + " which=" + op.which + " text=" + got
        }
        onTriggered: {
            const op = detailsSelectTimer
            if (op.at("start")) {
                if (!Awaited.all("details_select_ready", { "ready": driver.cardSettled }))
                    return
                op.input("asked", () => {
                    if (op.firstWhich !== "")
                        detailsPane.valueRow.selectValue(op.firstWhich)
                    detailsPane.valueRow.selectValue(op.which)
                })
                return
            }
            op.complete({ "name": "details_select_ready", "when": () => ({ "ready": driver.cardSettled }),
                          "report": () => op.line() })
        }
    }

    /// A summary box's right-click menu (`FieldMenu`) on the box in the state the argument names, asked for through the
    /// box's own request — the entry Qt's right-click takes. `rows=` (on the table) and `live=` (can be pressed) are
    /// read off the card: a greyed row and a lit one differ by a shade a picture leaves to the eye.
    PageStage {
        id: fieldMenuTimer
        name: "field_menu"
        order: ["requested"]
        /// `empty` / `typed` / `selected` / `readonly`.
        property string boxState: ""
        /// The box asked: the commit box, or the summary of the commit under HEAD.
        function box() {
            return fieldMenuTimer.boxState === "readonly" ? detailsPane.messageBlock.summaryBox
                                                           : wipPane.commitBlock.messageSeat.summaryBox
        }
        /// The box put in the state the argument names, and its menu asked for: the press.
        function ask() {
            const box = fieldMenuTimer.box()
            // Something for Paste to offer: a run's clipboard starts empty.
            driver.clipboard.copy("pasted words")
            if (fieldMenuTimer.boxState === "typed") {
                wipPane.setMessage("", "")
                // Undoable, as typing is: `text` set outright leaves nothing to undo.
                box.insert(0, "fix: typed into the box")
            } else if (fieldMenuTimer.boxState === "selected") {
                wipPane.setMessage("fix: a summary to cut a word from", "")
                box.select(7, 14)
            } else if (fieldMenuTimer.boxState === "readonly") {
                box.select(0, box.text.indexOf(" ") > 0 ? box.text.indexOf(" ") : box.length)
            } else {
                wipPane.setMessage("", "")
            }
            // The keyboard's request (no right button behind it), which only the box holding the caret answers: at
            // the caret, as Qt names a text box's every request, and the card stands under it (`FieldMenu.offerNow`).
            box.forceActiveFocus()
            const caret = box.cursorRectangle
            box.ContextMenu.requested(Qt.point(caret.x + caret.width / 2, caret.y + caret.height / 2))
        }
        /// The menu as it stands over the box.
        function line() {
            const seat = acts.menuSeatOf(fieldMenuTimer.box())
            const menu = seat !== null ? seat.item : null
            return "field_menu box=" + fieldMenuTimer.boxState
                   + " opened=" + (menu !== null && menu.opened)
                   + " rows=" + (menu !== null ? menu.rowWords(false).join(",") : "")
                   + " live=" + (menu !== null ? menu.rowWords(true).join(",") : "")
        }
        onTriggered: {
            const op = fieldMenuTimer
            const older = op.boxState === "readonly"
            if (op.at("start")) {
                if (!Awaited.all("field_menu_ready", { "ready": !(older ? !driver.cardSettled : !page.wipShown) }))
                    return
                op.input("requested", () => {
                    op.ask()
                })
            }
            op.complete({ "when": () => ({}), "report": () => op.line() })
        }
    }
    function menuSeatOf(box) {
        for (let i = 0; i < box.children.length; i++)
            if (box.children[i].editor === box)
                return box.children[i]
        return null
    }

    /// What the plate's one word does once the press it offers has been made — read off the tip already up, since a
    /// tip does not go down to change its mind. Whether the words change at all is `tst_hashplate`'s.
    ///
    /// Both claims compare the screen against what the control says it offers, so a translation stays green and a
    /// tip that kept the old sentence goes red.
    ///
    /// Witnessed as the press lands: `was`, what the tip was carrying — empty in `-counting`; `offered`, whether the
    /// tip up before the press carried the control's word.
    PageStage {
        id: hashTipTimer
        name: "hash_tip"
        /// Whether the press goes in ahead of the tip.
        property bool counting: false
        /// The plate's hash ready, and its tip up: the tip waits out its own rest, and an empty answer is "not up yet".
        function up() {
            const row = detailsPane.valueRow
            return { "ready": !(!driver.cardSettled || !row.valueReady("hash")), "up": row.hashTipSaid() !== "" }
        }
        onTriggered: {
            const op = hashTipTimer
            const row = detailsPane.valueRow
            if (op.at("start")) {
                if (!Awaited.all("hash_tip_ready", { "ready": !(!driver.cardSettled || !row.valueReady("hash")) }))
                    return
                op.input("asked", () => {
                    row.forceHashTip(true)
                    // Pressed on the beat the tip is asked for — a reader clicking before it arrives; this half
                    // claims only that nothing is up yet.
                    if (op.counting) {
                        op.witness("was", row.hashTipSaid())
                        row.tapHash()
                    }
                })
                return
            }
            if (op.counting) {
                // The tip that follows must carry the control's current word — the attached property hands over the
                // one it had when the pointer arrived unless the answer re-arms it.
                op.complete({
                    "name": "hash_tip_up",
                    "when": () => op.up(),
                    "report": () => {
                        const up = detailsPane.valueRow.hashTipSaid()
                        return "hash_tip_counting counting=" + (op.witnessed("was") === "")
                               + " said=" + (up === detailsPane.valueRow.hashTipWords())
                               + " now=" + up
                    } })
                return
            }
            if (op.at("asked")) {
                if (!Awaited.all("hash_tip_up", op.up()))
                    return
                op.input("tapped", () => {
                    const was = op.witness("was", row.hashTipSaid())
                    op.witness("offered", was === row.hashTipWords())
                    row.tapHash()
                })
            }
            op.complete({
                "when": () => ({}),
                "report": () => {
                    const plate = detailsPane.valueRow
                    const was = op.witnessed("was")
                    const after = plate.hashTipSaid()
                    const said = after !== "" && after !== was && after === plate.hashTipWords()
                    return "hash_tip offered=" + op.witnessed("offered") + " said=" + said + " was=" + was
                           + " now=" + after
                } })
        }
    }

    /// The plate's own gesture: a press released where it landed copies the whole hash, and a press that travels leaves
    /// the shown one picked out (規約 §右のペインの字は掴める). No picture answers it — the clipboard is not on screen.
    ///
    /// The drag goes first: after the tap has put the hash on the clipboard, a drag that also copied could not be
    /// told apart. A run enters the hand's own functions, so whether a press reaches the plate is `tst_hashplate`'s.
    ///
    /// Witnessed as the drag let go: `held`, whether it left the shown hash picked out; `quiet`, whether it left the
    /// pad as it found it.
    PageStage {
        id: detailsHandTimer
        name: "details_hand"
        order: ["dragged", "tapped"]
        onTriggered: {
            const op = detailsHandTimer
            // Both sides have to exist and the row has to have stopped moving — the same wait as `details-sweep`.
            const row = detailsPane.valueRow
            if (!Awaited.all("details_hand_ready", {
                "card": driver.cardSettled,
                "value": row.valueReady("hash"),
                "shown": row.shownValue("hash") !== "" }))
                return
            if (!op.still("details_hand_geom", row.valueGeom("hash")))
                return
            op.input("dragged", () => {
                const before = driver.clipboard.lastCopied
                row.dragHash()
                const shown = row.shownValue("hash")
                // `quiet=` is the half `held=` does not give: a plate that copied on every press would leave the same
                // selection standing.
                op.witness("held", shown !== "" && row.selectedValue("hash") === shown)
                op.witness("quiet", driver.clipboard.lastCopied === before)
            })
            op.input("tapped", () => {
                row.tapHash()
            })
            op.complete({
                "when": () => ({}),
                "report": () => {
                    const plate = detailsPane.valueRow
                    const want = plate.fullShown
                    // Read off the pad the copy goes through (`ClipboardHelper.lastCopied`); the clipboard will not
                    // say.
                    const acted = want !== "" && driver.clipboard.lastCopied === want
                    // And the drag's selection has let go: a kept wash reads as still dragged.
                    const letGo = plate.selectedValue("hash") === ""
                    // `hand=`: the run enters functions directly, so a hand taken out, disabled or shrunk would
                    // answer every one of them and never see a press.
                    return "details_hand acted=" + acted + " held=" + op.witnessed("held")
                           + " quiet=" + op.witnessed("quiet") + " let=" + letGo
                           + " hand=" + plate.hashHandStands() + " text=" + driver.clipboard.lastCopied
                } })
        }
    }

    /// The range selection, taken from the gaps around the values (規約 §右のペインの字は掴める). Four claims: the
    /// sweep reaches the value from every corner of the gap (`reach`), the keyboard went with it so `Ctrl+C` will
    /// land (`caret`), the words answer their own press wherever nothing stands over them (`grabs`), and a press on
    /// the value's own box is still somebody else's (`ours` — false on the plate's two, whose controls keep every
    /// press across their whole face).
    PageStage {
        id: detailsSweepTimer
        name: "details_sweep"
        order: ["swept"]
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        /// Sweeps from every corner of the gap, then presses the value's own box, reading each as it lands: both clear
        /// the board on their way in. What they found is witnessed (`found`).
        function sweep(which) {
            const valueRow = detailsPane.valueRow
            const want = valueRow.shownValue(which)
            let reach = 0
            let tries = 0
            let took = true
            let got = ""
            let on = ""
            let caret = false
            let miss = ""
            // Every corner of the gap: a reach that only works level with the words passes in the middle.
            const across = [0.15, 0.5, 0.9]
            const down = [0.05, 0.5, 0.95]
            for (let i = 0; i < across.length; i++) {
                for (let j = 0; j < down.length; j++) {
                    tries++
                    if (!valueRow.sweepAt(which, across[i], down[j])) {
                        took = false
                        continue
                    }
                    got = valueRow.selectedValue(which)
                    on = valueRow.sweptField()
                    caret = valueRow.sweptCaret()
                    if (want !== "" && got === want)
                        reach++
                    else if (miss === "")
                        // The first start that missed, and what the sweep saw — else the line has only the last try.
                        miss = "x" + across[i] + ",y" + down[j] + "," + valueRow.sweptTrace()
                }
            }
            return { "reach": reach, "tries": tries, "took": took, "got": got, "on": on, "caret": caret,
                     "miss": miss, "ours": valueRow.pressOnControl(which), "grabs": valueRow.valueGrabs(which) }
        }
        onTriggered: {
            const op = detailsSweepTimer
            // Both sides of the comparison have to exist first: the details arrive a frame ahead of the row that
            // draws them, so after `cardSettled` the value can still be empty, or its field width-less — which the
            // sweep passes over as if absent.
            const which = op.which
            if (!Awaited.all("details_sweep_ready", {
                "ready": !(!driver.cardSettled || !detailsPane.valueRow.valueReady(which)
                    || detailsPane.valueRow.shownValue(which) === "") }))
                return
            // And the row has to have stopped moving: the pane lays out more than once on its way to a commit, and a
            // sweep over a half-laid-out row lands on the line above. The geometry standing across a polish pass is
            // the settle — no number, so it holds at any pane width.
            if (!op.still("details_sweep_geom", detailsPane.valueRow.valueGeom(which)))
                return
            op.input("swept", () => {
                op.witness("found", op.sweep(which))
            })
            // `took=` and `on=` diagnose a short `reach=`: the gesture was refused at its start, or landed on another
            // line.
            op.complete({
                "when": () => ({}),
                "report": () => {
                    const found = op.witnessed("found")
                    return "details_sweep reach=" + found.reach + "/" + found.tries
                           + " caret=" + found.caret
                           + " grabs=" + found.grabs
                           + " ours=" + found.ours
                           + " which=" + op.which + " took=" + found.took
                           + " on=" + found.on + " miss=[" + found.miss + "] text=" + found.got
                } })
        }
    }

    /// The parent line against the room the row leaves it (デザイン規約 §右のペインの親). Headless sees neither a cut
    /// digit nor a crowded date, so the row says them: two parents always stand (`floor`), a count stands exactly when
    /// parents were left out and says how many (`said`), the links are git's parents in git's order (`order`), every
    /// link is cut alike (`even`), the name and the date lines are whole (`crowd`) and the pane holds its row (`fits`).
    PageStage {
        id: detailsParentsTimer
        name: "details_parents"
        /// The line as the row draws it now.
        function line() {
            const row = detailsPane.valueRow
            const plate = row.plate
            // What the plate was handed: git's parents, or a stash's base alone. `git=` says how many git has.
            const want = plate.parents
            let drawn = 0
            let ordered = true
            let even = true
            for (let i = 0; i < want.length; i++) {
                const link = plate.linkAt(i)
                if (!link || !link.visible || link.width <= 0)
                    continue
                drawn++
                ordered = ordered && i === drawn - 1 && link.field.text === want[i].substring(0, 8)
                even = even && link.field.clipped === (plate.parentDigits < plate.wholeDigits)
            }
            const more = want.length - drawn
            const said = plate.moreStands === (more > 0) && (more === 0 || plate.moreSaid === "+" + more)
            const crowd = row.nameClipped || row.dateClipped || row.matesClipped
            return "details_parents floor=" + (want.length < 2 || drawn >= 2)
                   + " said=" + said + " order=" + ordered + " even=" + even
                   + " crowd=" + crowd + " fits=" + (detailsPane.contentOverflow < 1)
                   + " of=" + want.length + " drawn=" + drawn + " more=" + more
                   + " git=" + driver.detailsModel.parentHexes.length
                   + " digits=" + plate.parentDigits
                   + " pane=" + Math.round(detailsPane.width) + " room=" + Math.round(row.parentRoom)
                   + " line=" + Math.round(plate.parentLineWidth)
                   + " gap=" + Math.round(plate.parentLineLeft(row) - row.dateLineEnd())
        }
        /// The row on the commit read: the card settled, the pane sized, the plate handed its parents.
        function ready() {
            const want = detailsPane.valueRow.plate.parents
            return {
                "card": driver.cardSettled,
                "sized": detailsPane.width > 0,
                "first": (want.length > 0 ? want[0] : "") === driver.detailsModel.parentHex }
        }
        onTriggered: {
            const row = detailsPane.valueRow
            const plate = row.plate
            // Ready before the geometry is watched: a row still on the commit before stands as still.
            if (!Awaited.all("details_parents_ready", detailsParentsTimer.ready()))
                return
            // Settled when it stands across a frame: the pane lays out more than once on its way to a commit, and a
            // width asked for arrives a frame after the row it resizes.
            const geom = Math.round(detailsPane.width) + "/" + Math.round(row.parentRoom) + "/"
                + Math.round(plate.parentLineWidth) + "/" + plate.parentsShown
            if (!detailsParentsTimer.still("details_parents_geom", geom))
                return
            detailsParentsTimer.complete({ "name": "details_parents_ready", "when": () => detailsParentsTimer.ready(),
                                           "report": () => detailsParentsTimer.line() })
        }
    }
    /// The author row's two lines, each read as one (デザイン規約 §右のペインの親): the name and the commit's hash on one
    /// baseline (`line1`), the date, the credit and the parents on the next (`line2`), and the two counts of one size
    /// (`sizes`), and the credit's face on the middle of the date line's capitals (`face`, デザイン規約 §アイコンの席 — read
    /// off the window's face, `WordFace`, not off the row). A pixel off is what the eye catches and a picture cannot be
    /// asked, so the row says it. Each value after the claims is that word's baseline less its line's first word's (the
    /// name's, the date's), and the face's middle less the capitals'; `-` is a word not drawn.
    PageStage {
        id: detailsAlignTimer
        name: "details_align"
        /// A word's baseline `v` less its line's first word's `ref`, to a tenth; `-` for a word not drawn.
        function off(v, ref) {
            return v < 0 ? "-" : (Math.round((v - ref) * 10) / 10).toString()
        }
        /// Whether a word stands on its line's baseline — a word not drawn stands anywhere.
        function level(v, ref) {
            return v < 0 || Math.abs(v - ref) < 0.5
        }
        /// The line as the row draws it now.
        function line() {
            const op = detailsAlignTimer
            const b = detailsPane.valueRow.baselines()
            const capitalsMiddle = b.date - WordFace.capitalMiddle(b.dateSize)
            const faceOff = b.face < 0 ? 0 : b.face - capitalsMiddle
            const line2 = op.level(b.mate, b.date) && op.level(b.mateCount, b.date) && op.level(b.parent, b.date)
                && op.level(b.comma, b.date) && op.level(b.parentCount, b.date)
            // `apart=`: the date line's words end a step short of the parent line's arrow, however tight the row.
            return "details_align line1=" + op.level(b.hash, b.name) + " line2=" + line2
                   + " sizes=" + (b.mateCountSize === b.parentCountSize)
                   + " apart=" + (b.apart >= Theme.spaceXs)
                   // Whole pixels both: a line of an odd height has its middle on a half.
                   + " face=" + (Math.abs(faceOff) <= 0.5)
                   + " hash=" + op.off(b.hash, b.name)
                   + " mate=" + op.off(b.mate, b.date) + " mateCount=" + op.off(b.mateCount, b.date)
                   + " parent=" + op.off(b.parent, b.date) + " comma=" + op.off(b.comma, b.date)
                   + " parentCount=" + op.off(b.parentCount, b.date)
                   + " faceOff=" + (b.face < 0 ? "-" : (Math.round(faceOff * 100) / 100).toString())
                   + " gap=" + (isFinite(b.apart) ? Math.round(b.apart) : "-")
                   + " pane=" + Math.round(detailsPane.width)
        }
        function ready() {
            return { "ready": !(!driver.cardSettled || detailsPane.width <= 0) }
        }
        onTriggered: {
            const row = detailsPane.valueRow
            if (!Awaited.all("details_align_ready", detailsAlignTimer.ready()))
                return
            const b = row.baselines()
            const geom = Math.round(detailsPane.width) + "/" + b.name + "/" + b.hash + "/" + b.date + "/" + b.parent
            if (!detailsAlignTimer.still("details_align_geom", geom))
                return
            detailsAlignTimer.complete({ "name": "details_align_ready", "when": () => detailsAlignTimer.ready(),
                                         "report": () => detailsAlignTimer.line() })
        }
    }
    /// The count's card (デザイン規約 §右のペインの親): hover cannot be injected, so the run writes the count's stand-in
    /// and reads the card the page put up. Where it stands is the claim no picture answers to the pixel: its first
    /// hash on the seat a single parent's hash has, at the line's right end and middle (`seat`), the arrow left outside
    /// the card (`clear`), nothing else of the line standing beside it (`bare`), every parent on a row of its own and
    /// whole (`whole`, `rows`). `-go` presses a row and reads where the page went (`landed`) and that the card went
    /// with the press (`shut`).
    ///
    /// Witnessed as the card was asked for: `seatY`, the first hash's row of pixels, and `plateWidth`, the plate's
    /// width, before the card came out; as the row was pressed: `want`, the parent the press names.
    PageStage {
        id: parentsCardTimer
        name: "parents_card"
        property bool going: false
        /// The card's row for parent `i` as its list has built it, or null: read off the list itself, without the
        /// scroll `ParentListCard.rowAt` asks for to bring a row in — that is an input.
        function rowBuilt(i) {
            const kids = detailsPane.authorCards.parentCard.contentItem.children
            for (let k = 0; k < kids.length; k++) {
                if (typeof kids[k].itemAtIndex === "function")
                    return kids[k].itemAtIndex(i)
            }
            return null
        }
        /// The card out, sized, its first row built and the line unrolled under it.
        function open() {
            const card = detailsPane.authorCards.parentCard
            return { "opened": card.opened,
                     "sized": card.width > 0,
                     "row": parentsCardTimer.rowBuilt(0) !== null,
                     "unrolled": detailsPane.valueRow.plate.unrolled }
        }
        /// The card as it stands now.
        function openLine() {
            const plate = detailsPane.valueRow.plate
            const card = detailsPane.authorCards.parentCard
            const arrow = plate.arrowBox(null)
            const field = parentsCardTimer.rowBuilt(0).field
            const hash = field.mapToItem(null, 0, 0)
            const line = plate.parentLineRect(null)
            const dx = hash.x + field.width - (line.x + line.width)
            const dy = hash.y - parentsCardTimer.witnessed("seatY")
            const gap = card.contentItem.mapToItem(null, 0, 0).x - card.padding - (arrow.x + arrow.width)
            let bare = !plate.moreStands
            for (let i = 0; i < plate.parents.length; i++) {
                const link = plate.linkAt(i)
                bare = bare && !!link && !link.visible
            }
            let rows = 0
            let whole = true
            for (let i = 0; i < plate.parents.length; i++) {
                const r = parentsCardTimer.rowBuilt(i)
                if (!r)
                    continue
                rows++
                whole = whole && !r.field.clipped && r.field.text === plate.parents[i].substring(0, 8)
            }
            return "parents_card open=" + card.opened
                   + " seat=" + (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5)
                   + " clear=" + (gap > -0.5) + " bare=" + bare + " whole=" + whole
                   + " still=" + (Math.abs(plate.width - parentsCardTimer.witnessed("plateWidth")) < 0.5)
                   + " rows=" + rows + " of=" + plate.parents.length
                   + " at=" + Math.round(dx) + "," + Math.round(dy) + " gap=" + Math.round(gap)
                   + " width=" + Math.round(card.width)
        }
        onTriggered: {
            const op = parentsCardTimer
            const row = detailsPane.valueRow
            const plate = row.plate
            const card = detailsPane.authorCards.parentCard
            if (op.at("start")) {
                if (!Awaited.all("parents_card_ready", { "card": driver.cardSettled, "more": plate.moreStands }))
                    return
                const geom = Math.round(detailsPane.width) + "/" + Math.round(plate.parentLineWidth)
                if (!op.still("parents_card_geom", geom))
                    return
                op.input("card_asked", () => {
                    // The first hash's row of pixels before the card comes out: the card's first hash has to stand
                    // on it.
                    op.witness("seatY", plate.linkAt(0).field.mapToItem(null, 0, 0).y)
                    // And the plate's width: the name beside it must not move as the card comes out.
                    op.witness("plateWidth", plate.width)
                    row.showParents(true)
                })
                return
            }
            if (op.at("card_asked")) {
                if (!Awaited.all("parents_card_open", op.open()))
                    return
                // The line lays itself out again as the card comes out; read it once it stands across a polish pass.
                const arrow = plate.arrowBox(null)
                const opened = Math.round(plate.parentLineWidth) + "/" + Math.round(arrow.x)
                if (!op.still("parents_card_geom", opened))
                    return
                if (!op.going) {
                    op.complete({ "name": "parents_card_open", "when": () => op.open(),
                                  "report": () => op.openLine() })
                    return
                }
                op.input("row_pressed", () => {
                    op.witness("want", plate.parents[1])
                    card.tapRow(1)
                })
                return
            }
            // The page answers the press with the parent's row, and the card went down with it.
            op.complete({
                "name": "parents_go_landed",
                "when": () => ({ "landed": page.selectedOid === op.witnessed("want"), "card": driver.cardSettled }),
                "report": () => "parents_go landed=" + (page.selectedOid === op.witnessed("want"))
                                + " shut=" + !detailsPane.authorCards.parentCard.opened })
        }
    }
    PageStage {
        id: detailsFitTimer
        name: "details_fit"
        function ready() {
            return { "card": driver.cardSettled, "sized": detailsPane.width > 0 && detailsPane.height > 0 }
        }
        // `fits=` is `contentOverflow < 1`: a pane width the splitter left on a fraction can put a fraction in the
        // answer, and what this verb is about is tens of pixels.
        function line() {
            return "details_fit fills=" + detailsPane.valueRow.fillsBlock
                   + " fits=" + (detailsPane.contentOverflow < 1)
                   + " row=" + Math.round(detailsPane.valueRow.width)
                   + "/" + Math.round(detailsPane.valueRow.parent.width)
                   + " over=" + Math.round(detailsPane.contentOverflow)
                   + " pane=" + Math.round(detailsPane.width)
                   // `overH=` rides along unjudged, like `edge=` in `window_fill`: whether the pane needs a scroll of
                   // its own depends on the window.
                   + " overH=" + Math.round(detailsPane.contentOverHeight)
                   + " paneH=" + Math.round(detailsPane.height)
        }
        onTriggered: {
            if (!Awaited.all("details_fit_ready", detailsFitTimer.ready()))
                return
            // The row has to have stopped moving: mid-layout the row and its block are both some other width, and
            // `fills` answers true about a frame nobody sees.
            const geom = detailsPane.valueRow.width + "x" + detailsPane.valueRow.parent.width
            if (!detailsFitTimer.still("details_fit_geom", geom))
                return
            detailsFitTimer.complete({ "name": "details_fit_ready", "when": () => detailsFitTimer.ready(),
                                       "report": () => detailsFitTimer.line() })
        }
    }
    PageStage {
        id: cornerTimer
        name: "corner"
        // `shown=` is the label's own visibility: reporting the room that decided it would go
        // green with the binding cut.
        //
        // The pane has to be done arriving: a details pane still reading its file list leaves its whole bottom bare,
        // so the corner reads as standing whatever the commit holds (`DetailsPane.bottomRoomSettled`). The worktree's
        // foot is the commit button's, whatever is above it.
        function ready() {
            return {
                "ready": !(gitCorner.parent === null || gitCorner.width <= 0),
                "pane": !(!page.wipShown && (!driver.cardSettled || !detailsPane.bottomRoomSettled)) }
        }
        onTriggered: {
            if (!Awaited.all("corner_ready", cornerTimer.ready()))
                return
            // And it has to have stopped moving, as in `details-fit`. Both halves are needed: the settle alone can be
            // a list that has not arrived standing still.
            const said = gitCorner.visible + "/" + Math.round(gitCorner.roomLeft)
            if (!cornerTimer.still("corner_said", said))
                return
            cornerTimer.complete({ "name": "corner_ready", "when": () => cornerTimer.ready(),
                                   "report": () => "git_corner pane=" + (page.wipShown ? "wip" : "details")
                                                   + " shown=" + gitCorner.visible
                                                   + " room=" + Math.round(gitCorner.roomLeft)
                                                   + " needs=" + Math.round(gitCorner.roomNeeded) })
        }
    }
    // The message has to be in the box, and the box laid out with it, before there is a ceiling to pull on. The
    // layout runs after the pull, so it is read once a polish pass begun after the pull has run: in the same breath
    // the list still reports its old height.
    //
    // `keeps=` says what the box borrowed room from (the author card / the commit button) is still on screen — the
    // picture cannot, because the overflow draws over the window's own footer.
    PageStage {
        id: descGrowTimer
        name: "desc_grow"
        order: ["pulled"]
        // Pulled past everything, so where it stops is the bound itself.
        readonly property int pull: 1000
        /// Which pane's box to pull — both carry the same box and hooks under the same names.
        property var pane: detailsPane
        property string paneName: "details"
        /// Whether to take the pane's room away again afterwards, by raising the command log under it — the one way a
        /// headless run can make the pane shorter than the box it is already holding.
        property bool squeeze: false
        /// Which end the grip is carried past (`divider-refuse`), or empty for the ordinary pull. With the grip in
        /// hand the box answers (規約 §掴める境界は答える).
        property string refuse: ""
        /// What the box did with the pull, read as it stands.
        function line() {
            const op = descGrowTimer
            if (op.refuse !== "")
                return "divider_refuse refuses=" + page.refusalShown
                       + " line=" + op.pane.descGrips
                       + " case=" + op.refuse
                       + " box=" + Math.round(op.pane.descHeight)
                       + " wants=" + Math.round(op.pane.descWants)
            return "description_grow keeps=" + op.pane.descKeeps
                   + " pane=" + op.paneName
                   + " grip=" + op.pane.descGrips
                   + " box=" + Math.round(op.pane.descHeight)
                   + " wants=" + Math.round(op.pane.descWants)
                   + " cap=" + Math.round(op.pane.descCap)
                   + " rows=" + op.pane.descListRows
        }
        onTriggered: {
            const op = descGrowTimer
            if (op.at("start")) {
                if (!Awaited.all("desc_grow_ready", {
                    "sized": !(op.pane.width <= 0 || op.pane.height <= 0),
                    "cap": !(op.pane.descCap <= 0) }))
                    return
                op.input("pulled", () => {
                    if (op.refuse !== "") {
                        op.pane.pullDescriptionPast(op.refuse === "desc-max")
                        return
                    }
                    op.pane.growDescription(op.pull)
                    if (op.squeeze)
                        page.toggleCommands()
                })
                return
            }
            if (!Awaited.all("desc_grow_drawn", { "drawn": op.laidOutSince("desc_grow") }))
                return
            op.complete({
                "name": "desc_grow_settle_ready",
                "when": () => ({ "page": PageSettled.settled(page), "refused": op.refuse === "" || page.refusalShown }),
                "report": () => op.line() })
        }
    }
    PageStage {
        id: splitRefuseTimer
        name: "split_refuse"
        order: ["dragged"]
        onTriggered: {
            if (splitRefuseTimer.at("start")) {
                if (!Awaited.all("split_refuse_ready", { "open": page.commandsOpen, "shown": page.commandsShown }))
                    return
                splitRefuseTimer.input("dragged", () => {
                    page.dragDividerPast(Harness.autoActArg)
                })
            }
            splitRefuseTimer.complete({ "when": () => ({}),
                                        "report": () => acts.refusalLine(Harness.autoActArg) })
        }
    }
}

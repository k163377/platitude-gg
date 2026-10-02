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
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. Each verb is named by one family
    /// (`AutoActDriver` asks them in turn).
    function run(act, arg) {
        if (act === "details-failure") {
            driver.completionDeferred = true
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
            // `empty` (and none) / `typed` / `selected` in the commit box, `readonly` on the summary of the commit under
            // HEAD.
            fieldMenuTimer.boxState = arg === "" ? "empty" : arg
            if (arg === "readonly")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(driver.workTree.headOid) + 1))
            else
                page.showWip()
            fieldMenuTimer.start()
        } else if (act === "hash-tip" || act === "hash-tip-counting") {
            page.activateRow(graphModel.oidAt(arg === "" ? 0 : Number(arg)))
            hashTipTimer.counting = act === "hash-tip-counting"
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
            parentsCardTimer.start()
        } else if (act === "details-fit") {
            // Overflow is glyphs cut at the window's edge, which headless cannot see, so the pane reports the number.
            //
            // No row named keeps the one the page opened on: `Number("")` is 0, and row 0 of a dirty tree (both
            // presets this verb runs on) is the working tree's — the card never settles and the run waits out the
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
                // Row 1: row 0 of every preset is the uncommitted row, which puts the working tree in the right-hand
                // pane and the box this pulls on off screen.
                page.activateRow(graphModel.oidAt(1))
                descGrowTimer.pane = detailsPane
                descGrowTimer.paneName = "details"
                descGrowTimer.squeeze = false
                descGrowTimer.refuse = arg
                descGrowTimer.start()
            } else {
                acts.dragDividerPast(arg)
                renderedBarrier.begin()
            }
        } else {
            return false
        }
        return true
    }
    /// The drag past whichever boundary `which` names, and what the window did with it (`divider-refuse`). A picture
    /// cannot answer it: every refusal frames the same way, and the column widths are not on screen as numbers.
    function dragDividerPast(which) {
        page.dragDividerPast(which)
        Harness.report("divider_refuse refuses=" + page.refusalShown
                          + " line=" + page.refusalLineShown(which)
                          + " case=" + which + " labelW=" + graphPane.labelW
                          + " graphW=" + graphPane.graphColW)
    }

    SampleTimer {
        id: detailsFailure
        onTriggered: {
            if (driver.detailsModel.loading || !driver.repoTab.lastError.startsWith("details: "))
                return
            detailsFailure.stop()
            Harness.report("details_failure loading=false error=true")
            renderedBarrier.begin()
        }
    }
    /// The pane's values are fields the reader drags over (規約 §右のペインの字は掴める). A drag cannot be injected, so
    /// this picks the field out the way `Ctrl+A` does and reads back what it holds — a picture cannot answer this, as
    /// a `Text` in place of the field would draw the identical row. The selection is left standing for the shot.
    SampleTimer {
        id: detailsSelectTimer
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        property bool asked: false
        /// The value picked out first (`-away`), or empty.
        property string firstWhich: ""
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (!detailsSelectTimer.asked) {
                if (detailsSelectTimer.firstWhich !== "")
                    detailsPane.valueRow.selectValue(detailsSelectTimer.firstWhich)
                detailsPane.valueRow.selectValue(detailsSelectTimer.which)
                detailsSelectTimer.asked = true
                return
            }
            detailsSelectTimer.stop()
            const got = detailsPane.valueRow.selectedValue(detailsSelectTimer.which)
            const want = detailsPane.valueRow.shownValue(detailsSelectTimer.which)
            if (detailsSelectTimer.firstWhich !== "") {
                // A second selection must not leave the first lit. Both halves are said: a run that cleared
                // everything would answer `dropped=true` on its own.
                Harness.report("details_select_away dropped="
                                  + (detailsPane.valueRow.selectedValue(detailsSelectTimer.firstWhich) === "")
                                  + " held=" + (want !== "" && got === want)
                                  + " first=" + detailsSelectTimer.firstWhich
                                  + " then=" + detailsSelectTimer.which)
                driver.complete()
                return
            }
            // `match=` is the whole claim; `text=` rides along for the eye.
            Harness.report("details_select match=" + (want !== "" && got === want)
                              + " which=" + detailsSelectTimer.which + " text=" + got)
            driver.complete()
        }
    }

    /// A summary box's right-click menu (`FieldMenu`) on the box in the state the argument names, asked for through the
    /// box's own request — the entry Qt's right-click takes. `rows=` (on the table) and `live=` (can be pressed) are read
    /// off the card: a greyed row and a lit one differ by a shade a picture leaves to the eye.
    SampleTimer {
        id: fieldMenuTimer
        /// `empty` / `typed` / `selected` / `readonly`.
        property string boxState: ""
        onTriggered: {
            const older = fieldMenuTimer.boxState === "readonly"
            if (older ? !driver.cardSettled : !page.wipShown)
                return
            fieldMenuTimer.stop()
            const box = older ? detailsPane.messageBlock.summaryBox : wipPane.commitBlock.messageSeat.summaryBox
            // Something for Paste to offer: a run's clipboard starts empty.
            driver.clipboard.copy("pasted words")
            if (fieldMenuTimer.boxState === "typed") {
                wipPane.setMessage("", "")
                // Undoable, as typing is: `text` set outright leaves nothing to undo.
                box.insert(0, "fix: typed into the box")
            } else if (fieldMenuTimer.boxState === "selected") {
                wipPane.setMessage("fix: a summary to cut a word from", "")
                box.select(7, 14)
            } else if (older) {
                box.select(0, box.text.indexOf(" ") > 0 ? box.text.indexOf(" ") : box.length)
            } else {
                wipPane.setMessage("", "")
            }
            // The keyboard's request (no right button behind it), which only the box holding the caret answers: at the
            // caret, as Qt names a text box's every request, and the card stands under it (`FieldMenu.offerNow`).
            box.forceActiveFocus()
            const caret = box.cursorRectangle
            box.ContextMenu.requested(Qt.point(caret.x + caret.width / 2, caret.y + caret.height / 2))
            const seat = acts.menuSeatOf(box)
            const menu = seat !== null ? seat.item : null
            Harness.report("field_menu box=" + fieldMenuTimer.boxState
                              + " opened=" + (menu !== null && menu.opened)
                              + " rows=" + (menu !== null ? menu.rowWords(false).join(",") : "")
                              + " live=" + (menu !== null ? menu.rowWords(true).join(",") : ""))
            driver.complete()
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
    SampleTimer {
        id: hashTipTimer
        /// Whether the press goes in ahead of the tip.
        property bool counting: false
        property bool asked: false
        /// What the tip was carrying when the press landed — empty in `-counting`.
        property string was: ""
        onTriggered: {
            const row = detailsPane.valueRow
            if (!driver.cardSettled || !row.valueReady("hash"))
                return
            if (!hashTipTimer.asked) {
                row.forceHashTip(true)
                hashTipTimer.asked = true
                // Pressed on the beat the tip is asked for — a reader clicking before it arrives; this half claims
                // only that nothing is up yet.
                if (hashTipTimer.counting) {
                    hashTipTimer.was = row.hashTipSaid()
                    row.tapHash()
                }
                return
            }
            // The tip waits out its own rest; an empty answer is "not up yet".
            const up = row.hashTipSaid()
            if (up === "")
                return
            hashTipTimer.stop()
            if (hashTipTimer.counting) {
                // The tip that follows must carry the control's current word — the attached property hands over the
                // one it had when the pointer arrived unless the answer re-arms it.
                Harness.report("hash_tip_counting counting=" + (hashTipTimer.was === "")
                                  + " said=" + (up === row.hashTipWords())
                                  + " now=" + up)
                driver.complete()
                return
            }
            hashTipTimer.was = up
            const offered = up === row.hashTipWords()
            row.tapHash()
            const after = row.hashTipSaid()
            const said = after !== "" && after !== hashTipTimer.was && after === row.hashTipWords()
            Harness.report("hash_tip offered=" + offered + " said=" + said
                              + " was=" + hashTipTimer.was + " now=" + after)
            driver.complete()
        }
    }

    /// The plate's own gesture: a press released where it landed copies the whole hash, and a press that travels leaves
    /// the shown one picked out (規約 §右のペインの字は掴める). No picture answers it — the clipboard is not on screen.
    ///
    /// The drag goes first: after the tap has put the hash on the clipboard, a drag that also copied could not be
    /// told apart. A run enters the hand's own functions, so whether a press reaches the plate is `tst_hashplate`'s.
    SampleTimer {
        id: detailsHandTimer
        property string lastGeom: ""
        onTriggered: {
            // Both sides have to exist and the row has to have stopped moving — the same wait as `details-sweep`.
            const row = detailsPane.valueRow
            if (!driver.cardSettled || !row.valueReady("hash") || row.shownValue("hash") === "")
                return
            const geom = row.valueGeom("hash")
            if (geom !== detailsHandTimer.lastGeom) {
                detailsHandTimer.lastGeom = geom
                return
            }
            detailsHandTimer.stop()
            const before = driver.clipboard.lastCopied
            row.dragHash()
            const shown = row.shownValue("hash")
            // `quiet=` is the half `held=` does not give: a plate that copied on every press would leave the same
            // selection standing.
            const held = shown !== "" && row.selectedValue("hash") === shown
            const quiet = driver.clipboard.lastCopied === before
            row.tapHash()
            const want = row.fullShown
            // Read off the pad the copy goes through (`ClipboardHelper.lastCopied`); the clipboard will not say.
            const acted = want !== "" && driver.clipboard.lastCopied === want
            // And the drag's selection has let go: a kept wash reads as still dragged.
            const letGo = row.selectedValue("hash") === ""
            // `hand=`: the run enters functions directly, so a hand taken out, disabled or shrunk would answer every
            // one of them and never see a press.
            Harness.report("details_hand acted=" + acted + " held=" + held + " quiet=" + quiet
                              + " let=" + letGo
                              + " hand=" + row.hashHandStands() + " text=" + driver.clipboard.lastCopied)
            driver.complete()
        }
    }

    /// The range selection, taken from the gaps around the values (規約 §右のペインの字は掴める). Four claims: the
    /// sweep reaches the value from every corner of the gap (`reach`), the keyboard went with it so `Ctrl+C` will
    /// land (`caret`), the words answer their own press wherever nothing stands over them (`grabs`), and a press on
    /// the value's own box is still somebody else's (`ours` — false on the plate's two, whose controls keep every
    /// press across their whole face).
    SampleTimer {
        id: detailsSweepTimer
        /// `author` / `date` / `mate` / `hash` / `parent`.
        property string which: ""
        property string lastGeom: ""
        onTriggered: {
            // Both sides of the comparison have to exist first: the details arrive a frame ahead of the row that
            // draws them, so after `cardSettled` the value can still be empty, or its field width-less — which the
            // sweep passes over as if absent.
            const which = detailsSweepTimer.which
            if (!driver.cardSettled || !detailsPane.valueRow.valueReady(which) || detailsPane.valueRow.shownValue(which) === "")
                return
            // And the row has to have stopped moving: the pane lays out more than once on its way to a commit, and a
            // sweep over a half-laid-out row lands on the line above. Two samples with the same geometry is the
            // settle — no number, so it holds at any pane width.
            const geom = detailsPane.valueRow.valueGeom(which)
            if (geom !== detailsSweepTimer.lastGeom) {
                detailsSweepTimer.lastGeom = geom
                return
            }
            detailsSweepTimer.stop()
            const want = detailsPane.valueRow.shownValue(which)
            // Every corner of the gap: a reach that only works level with the words passes in the middle.
            let reach = 0
            let tries = 0
            let took = true
            let got = ""
            let on = ""
            let caret = false
            let miss = ""
            const across = [0.15, 0.5, 0.9]
            const down = [0.05, 0.5, 0.95]
            for (let i = 0; i < across.length; i++) {
                for (let j = 0; j < down.length; j++) {
                    tries++
                    if (!detailsPane.valueRow.sweepAt(which, across[i], down[j])) {
                        took = false
                        continue
                    }
                    got = detailsPane.valueRow.selectedValue(which)
                    // Read before the next sweep, and before the press below — both clear the board on their way in.
                    on = detailsPane.valueRow.sweptField()
                    caret = detailsPane.valueRow.sweptCaret()
                    if (want !== "" && got === want)
                        reach++
                    else if (miss === "")
                        // The first start that missed, and what the sweep saw — else the line has only the last try.
                        miss = "x" + across[i] + ",y" + down[j] + "," + detailsPane.valueRow.sweptTrace()
                }
            }
            const ours = detailsPane.valueRow.pressOnControl(which)
            const grabs = detailsPane.valueRow.valueGrabs(which)
            // `took=` and `on=` diagnose a short `reach=`: the gesture was refused at its start, or landed on another
            // line.
            Harness.report("details_sweep reach=" + reach + "/" + tries
                              + " caret=" + caret
                              + " grabs=" + grabs
                              + " ours=" + ours
                              + " which=" + which + " took=" + took
                              + " on=" + on + " miss=[" + miss + "] text=" + got)
            driver.complete()
        }
    }

    /// The parent line against the room the row leaves it (デザイン規約 §右のペインの親). Headless sees neither a cut
    /// digit nor a crowded date, so the row says them: two parents always stand (`floor`), a count stands exactly when
    /// parents were left out and says how many (`said`), the links are git's parents in git's order (`order`), every
    /// link is cut alike (`even`), the name and the date lines are whole (`crowd`) and the pane holds its row (`fits`).
    SampleTimer {
        id: detailsParentsTimer
        property string lastGeom: ""
        onTriggered: {
            const row = detailsPane.valueRow
            const plate = row.plate
            // What the plate was handed: git's parents, or a stash's base alone. `git=` says how many git has.
            const want = plate.parents
            const first = driver.detailsModel.parentHex
            if (!driver.cardSettled || detailsPane.width <= 0 || (want.length > 0 ? want[0] : "") !== first)
                return
            // Settled when two samples agree: the pane lays out more than once on its way to a commit, and a width
            // asked for arrives a frame after the row it resizes.
            const geom = Math.round(detailsPane.width) + "/" + Math.round(row.parentRoom) + "/"
                + Math.round(plate.parentLineWidth) + "/" + plate.parentsShown
            if (geom !== detailsParentsTimer.lastGeom) {
                detailsParentsTimer.lastGeom = geom
                return
            }
            detailsParentsTimer.stop()
            let drawn = 0
            let order = true
            let even = true
            for (let i = 0; i < want.length; i++) {
                const link = plate.linkAt(i)
                if (!link || !link.visible || link.width <= 0)
                    continue
                drawn++
                order = order && i === drawn - 1 && link.field.text === want[i].substring(0, 8)
                even = even && link.field.clipped === (plate.parentDigits < plate.wholeDigits)
            }
            const more = want.length - drawn
            const said = plate.moreStands === (more > 0) && (more === 0 || plate.moreSaid === "+" + more)
            const crowd = row.nameClipped || row.dateClipped || row.matesClipped
            Harness.report("details_parents floor=" + (want.length < 2 || drawn >= 2)
                              + " said=" + said + " order=" + order + " even=" + even
                              + " crowd=" + crowd + " fits=" + (detailsPane.contentOverflow < 1)
                              + " of=" + want.length + " drawn=" + drawn + " more=" + more
                              + " git=" + driver.detailsModel.parentHexes.length
                              + " digits=" + plate.parentDigits
                              + " pane=" + Math.round(detailsPane.width) + " room=" + Math.round(row.parentRoom)
                              + " line=" + Math.round(plate.parentLineWidth)
                              + " gap=" + Math.round(plate.parentLineLeft(row) - row.dateLineEnd()))
            driver.complete()
        }
    }
    /// The author row's two lines, each read as one (デザイン規約 §右のペインの親): the name and the commit's hash on one
    /// baseline (`line1`), the date, the credit and the parents on the next (`line2`), and the two counts of one size
    /// (`sizes`), and the credit's face as far over the date's letters as under them (`face`, 規約 §co-author の表示 —
    /// the letters' ink is the date's own, measured here, not the figure the line centres on). A pixel off is what the
    /// eye catches and a picture cannot be asked, so the row says it. Each value after the claims is that word's
    /// baseline less its line's first word's (the name's, the date's), and the face's middle less the ink's; `-` is a
    /// word not drawn.
    SampleTimer {
        id: detailsAlignTimer
        property string lastGeom: ""
        /// The date's letters as drawn: their ink's middle is what the face is held against.
        readonly property TextMetrics dateInk: TextMetrics {
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
        }
        onTriggered: {
            const row = detailsPane.valueRow
            if (!driver.cardSettled || detailsPane.width <= 0)
                return
            const b = row.baselines()
            const geom = Math.round(detailsPane.width) + "/" + b.name + "/" + b.hash + "/" + b.date + "/" + b.parent
            if (geom !== detailsAlignTimer.lastGeom) {
                detailsAlignTimer.lastGeom = geom
                return
            }
            detailsAlignTimer.stop()
            const ink = detailsAlignTimer.dateInk
            ink.font.pixelSize = b.dateSize
            ink.text = b.dateText
            const inkMiddle = b.date + ink.tightBoundingRect.y + ink.tightBoundingRect.height / 2
            const faceOff = b.face < 0 ? 0 : b.face - inkMiddle
            const off = (v, ref) => v < 0 ? "-" : (Math.round((v - ref) * 10) / 10).toString()
            const level = (v, ref) => v < 0 || Math.abs(v - ref) < 0.5
            const line2 = level(b.mate, b.date) && level(b.mateCount, b.date) && level(b.parent, b.date)
                && level(b.comma, b.date) && level(b.parentCount, b.date)
            // `apart=`: the date line's words end a step short of the parent line's arrow, however tight the row.
            Harness.report("details_align line1=" + level(b.hash, b.name) + " line2=" + line2
                              + " sizes=" + (b.mateCountSize === b.parentCountSize)
                              + " apart=" + (b.apart >= Theme.spaceXs)
                              + " face=" + (Math.abs(faceOff) < 0.25)
                              + " hash=" + off(b.hash, b.name)
                              + " mate=" + off(b.mate, b.date) + " mateCount=" + off(b.mateCount, b.date)
                              + " parent=" + off(b.parent, b.date) + " comma=" + off(b.comma, b.date)
                              + " parentCount=" + off(b.parentCount, b.date)
                              + " faceOff=" + (b.face < 0 ? "-" : (Math.round(faceOff * 100) / 100).toString())
                              + " gap=" + (isFinite(b.apart) ? Math.round(b.apart) : "-")
                              + " pane=" + Math.round(detailsPane.width))
            driver.complete()
        }
    }
    /// The count's card (デザイン規約 §右のペインの親): hover cannot be injected, so the run writes the count's stand-in
    /// and reads the card the page put up. Where it stands is the claim no picture answers to the pixel: its first
    /// hash on the seat a single parent's hash has, at the line's right end and middle (`seat`), the arrow left outside
    /// the card (`clear`), nothing else of the line standing beside it (`bare`), every parent on a row of its own and
    /// whole (`whole`, `rows`). `-go` presses a row and reads where the page went (`landed`) and that the card went
    /// with the press (`shut`).
    SampleTimer {
        id: parentsCardTimer
        property bool going: false
        property string stage: "settle"
        property string lastGeom: ""
        property string want: ""
        property real seatY: 0
        property real plateWidth: 0
        onTriggered: {
            const row = detailsPane.valueRow
            const plate = row.plate
            const cards = detailsPane.authorCards
            const card = cards.parentCard
            if (parentsCardTimer.stage === "settle") {
                if (!driver.cardSettled || !plate.moreStands)
                    return
                const geom = Math.round(detailsPane.width) + "/" + Math.round(plate.parentLineWidth)
                if (geom !== parentsCardTimer.lastGeom) {
                    parentsCardTimer.lastGeom = geom
                    return
                }
                parentsCardTimer.stage = "open"
                // The first hash's row of pixels before the card comes out: the card's first hash has to stand on it.
                parentsCardTimer.seatY = plate.linkAt(0).field.mapToItem(null, 0, 0).y
                // And the plate's width: the name beside it must not move as the card comes out.
                parentsCardTimer.plateWidth = plate.width
                Harness.report("parents_card step=open")
                row.showParents(true)
                return
            }
            if (parentsCardTimer.stage === "open") {
                if (!card.opened || card.width <= 0 || !card.rowAt(0) || !plate.unrolled)
                    return
                // The line lays itself out again as the card comes out; read it once two samples agree.
                const arrow = plate.arrowBox(null)
                const opened = Math.round(plate.parentLineWidth) + "/" + Math.round(arrow.x)
                if (opened !== parentsCardTimer.lastGeom) {
                    parentsCardTimer.lastGeom = opened
                    return
                }
                if (parentsCardTimer.going) {
                    parentsCardTimer.stage = "go"
                    parentsCardTimer.want = plate.parents[1]
                    Harness.report("parents_card step=go")
                    card.tapRow(1)
                    return
                }
                parentsCardTimer.stop()
                const field = card.rowAt(0).field
                const hash = field.mapToItem(null, 0, 0)
                const line = plate.parentLineRect(null)
                const dx = hash.x + field.width - (line.x + line.width)
                const dy = hash.y - parentsCardTimer.seatY
                const gap = card.contentItem.mapToItem(null, 0, 0).x - card.padding - (arrow.x + arrow.width)
                let bare = !plate.moreStands
                for (let i = 0; i < plate.parents.length; i++) {
                    const link = plate.linkAt(i)
                    bare = bare && !!link && !link.visible
                }
                let rows = 0
                let whole = true
                for (let i = 0; i < plate.parents.length; i++) {
                    const r = card.rowAt(i)
                    if (!r)
                        continue
                    rows++
                    whole = whole && !r.field.clipped && r.field.text === plate.parents[i].substring(0, 8)
                }
                Harness.report("parents_card open=" + card.opened
                                  + " seat=" + (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5)
                                  + " clear=" + (gap > -0.5) + " bare=" + bare + " whole=" + whole
                                  + " still=" + (Math.abs(plate.width - parentsCardTimer.plateWidth) < 0.5)
                                  + " rows=" + rows + " of=" + plate.parents.length
                                  + " at=" + Math.round(dx) + "," + Math.round(dy) + " gap=" + Math.round(gap)
                                  + " width=" + Math.round(card.width))
                driver.complete()
                return
            }
            // go: the page answers the press with the parent's row, and the card went down with it.
            if (page.selectedOid !== parentsCardTimer.want || !driver.cardSettled)
                return
            parentsCardTimer.stop()
            Harness.report("parents_go landed=" + (page.selectedOid === parentsCardTimer.want)
                              + " shut=" + !card.opened)
            driver.complete()
        }
    }
    SampleTimer {
        id: detailsFitTimer
        // `fits=` is `contentOverflow < 1`: a pane width the splitter left on a fraction can put a fraction in the
        // answer, and what this verb is about is tens of pixels.
        property string lastGeom: ""
        onTriggered: {
            if (!driver.cardSettled || detailsPane.width <= 0 || detailsPane.height <= 0)
                return
            // The row has to have stopped moving: mid-layout the row and its block are both some other width, and
            // `fills` answers true about a frame nobody sees.
            const geom = detailsPane.valueRow.width + "x" + detailsPane.valueRow.parent.width
            if (geom !== detailsFitTimer.lastGeom) {
                detailsFitTimer.lastGeom = geom
                return
            }
            detailsFitTimer.stop()
            Harness.report(
            "details_fit fills=" + detailsPane.valueRow.fillsBlock + " fits=" + (detailsPane.contentOverflow < 1)
            + " row=" + Math.round(detailsPane.valueRow.width) + "/" + Math.round(detailsPane.valueRow.parent.width) + " over=" + Math.round(detailsPane.contentOverflow)
            + " pane=" + Math.round(detailsPane.width)
            // `overH=` rides along unjudged, like `edge=` in `window_fill`: whether the pane needs a scroll of its own
            // depends on the window.
            + " overH=" + Math.round(detailsPane.contentOverHeight)
            + " paneH=" + Math.round(detailsPane.height))
            driver.complete()
        }
    }
    SampleTimer {
        id: cornerTimer
        /// What the corner answered at the previous sample, for the settle below.
        property string lastSaid: ""
        // `shown=` is the label's own visibility: reporting the room that decided it would go
        // green with the binding cut.
        onTriggered: {
            if (gitCorner.parent === null || gitCorner.width <= 0)
                return
            // The pane has to be done arriving: a details pane still reading its file list leaves its whole bottom
            // bare, so the corner reads as standing whatever the commit holds (`DetailsPane.bottomRoomSettled`). The
            // working tree's foot is the commit button's, whatever is above it.
            if (!page.wipShown && (!driver.cardSettled || !detailsPane.bottomRoomSettled))
                return
            // And it has to have stopped moving, as in `details-fit`. Both halves are needed: the settle alone can be
            // two samples agreeing about a list that has not arrived.
            const said = gitCorner.visible + "/" + Math.round(gitCorner.roomLeft)
            if (said !== cornerTimer.lastSaid) {
                cornerTimer.lastSaid = said
                return
            }
            cornerTimer.stop()
            Harness.report(
            "git_corner pane=" + (page.wipShown ? "wip" : "details")
            + " shown=" + gitCorner.visible
            + " room=" + Math.round(gitCorner.roomLeft)
            + " needs=" + Math.round(gitCorner.roomNeeded))
            driver.complete()
        }
    }
    // The message has to be in the box, and the box laid out with it, before there is a ceiling to pull on.
    SampleTimer {
        id: descGrowTimer
        // Pulled past everything, so where it stops is the bound itself.
        readonly property int pull: 1000
        /// Which pane's box to pull — both carry the same box and hooks under the same names.
        property var pane: detailsPane
        property string paneName: "details"
        /// Whether to take the pane's room away again afterwards, by raising the command log under it — the one way a
        /// headless run can make the pane shorter than the box it is already holding.
        property bool squeeze: false
        property int frameBefore: 0
        /// Which end the grip is carried past (`divider-refuse`), or empty for the ordinary pull. With the grip in
        /// hand the box answers (規約 §掴める境界は答える).
        property string refuse: ""
        onTriggered: {
            if (descGrowTimer.pane.width <= 0 || descGrowTimer.pane.height <= 0 || descGrowTimer.pane.descCap <= 0)
                return
            descGrowTimer.stop()
            descGrowTimer.frameBefore = page.Window.window.frameCounter
            if (descGrowTimer.refuse !== "") {
                descGrowTimer.pane.pullDescriptionPast(
                    descGrowTimer.refuse === "desc-max")
                descGrowSettle.start()
                return
            }
            descGrowTimer.pane.growDescription(descGrowTimer.pull)
            if (descGrowTimer.squeeze)
                page.toggleCommands()
            descGrowSettle.start()
        }
    }
    // The layout runs after that handler, so the pull is read a frame later: in the same breath the list still
    // reports its old height.
    //
    // `keeps=` says what the box borrowed room from (the author card / the commit button) is still on screen — the
    // picture cannot, because the overflow draws over the window's own footer.
    SampleTimer {
        id: descGrowSettle
        onTriggered: {
            if (page.Window.window.frameCounter <= descGrowTimer.frameBefore)
                return
            if (descGrowTimer.refuse !== "" && !page.refusalShown)
                return
            descGrowSettle.stop()
            if (descGrowTimer.refuse !== "") {
                Harness.report("divider_refuse refuses=" + page.refusalShown
                                  + " line=" + descGrowTimer.pane.descGrips
                                  + " case=" + descGrowTimer.refuse
                                  + " box=" + Math.round(descGrowTimer.pane.descHeight)
                                  + " wants=" + Math.round(descGrowTimer.pane.descWants))
                driver.complete()
                return
            }
            Harness.report(
            "description_grow keeps=" + descGrowTimer.pane.descKeeps
            + " pane=" + descGrowTimer.paneName
            + " grip=" + descGrowTimer.pane.descGrips
            + " box=" + Math.round(descGrowTimer.pane.descHeight)
            + " wants=" + Math.round(descGrowTimer.pane.descWants)
            + " cap=" + Math.round(descGrowTimer.pane.descCap)
            + " rows=" + descGrowTimer.pane.descListRows)
            driver.complete()
        }
    }
    SampleTimer {
        id: splitRefuseTimer
        onTriggered: {
            if (!page.commandsOpen || !page.commandsShown)
                return
            splitRefuseTimer.stop()
            acts.dragDividerPast(Harness.autoActArg)
            renderedBarrier.begin()
        }
    }
}

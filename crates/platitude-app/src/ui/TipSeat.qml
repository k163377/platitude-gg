import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The window's one tooltip and who it answers (規約 §hover のツールチップ). The targets ask through `TipAsk`; this
/// decides what the box does — one `HoverSeat`, as every hover card has — and where it stands:
///
/// * **The words can be taken away** — the content is a `CardText`.
/// * **The hand can get to them** — it opens beside the hand and flush against the target: a gap is a band the
///   pointer crosses while touching neither.
/// * **It waits** — a target asks and the rest counts; the hand walking into the box is a hold, not a leave; a
///   neighbour's ask takes the box down and counts its own rest (「隣の的へは箱を下ろしてから移る」).
///
/// The box is a `ToolTip` of this seat's own, not Qt's attached one, so nothing here is inferred from the popup going
/// down: the seat knows why. A `ToolTip` all the same, since Qt lets a tooltip alone stand without blocking the
/// window's shortcuts (rules-refs/app-ui.md「hover のカードは窓の `Shortcut` を塞がない」).
///
/// An Item: the box is a child, and a QtObject has nowhere to put one.
Item {
    id: seat

    /// The item every placement is measured in — the window's content.
    required property Item host
    /// Where the hand is, window-wide (`HandWatch`); silent while a popup covers the pointer.
    required property HandWatch hand

    /// The box itself, for the harness (`Tips.box`).
    readonly property alias box: box
    /// The ask the box is out on or waiting for, null for none.
    readonly property var current: machine.target
    readonly property bool shownNow: machine.shown
    /// The beat after a leave is still counting (automation reads it), and the machine's word for where it stands.
    readonly property bool settling: machine.leaving
    readonly property string machineState: machine.state
    /// A place word in the box was pressed; `Main` hands the href to the page that owns it.
    signal linkAsked(string href)

    /// Hover stand-in for runs with no pointer: where the hand is on the target, as a share of its width (0 = left
    /// edge, 1 = right). Negative — the ordinary case — means no stand-in.
    property real handAcross: -1

    /// The pointer is on the box's ground or its words — two handlers, since background and content are siblings
    /// (rules-refs/app-ui.md「hover で開くものの 5 つの罠」(2)).
    property bool groundPointed: false
    property bool wordPointed: false
    readonly property bool pointed: seat.groundPointed || seat.wordPointed

    /// Where the box was opened beside, in `host` coordinates. Frozen when it comes out: a seat that followed the
    /// pointer would slide from under the hand walking into it.
    property real anchorX: 0
    property bool anchorKnown: false
    /// The same for y, read only by a box beside its target (`current.beside`). Separate because a run's stand-in
    /// has no height; those runs seat the box at the target's top.
    property real anchorY: 0
    property bool anchorDown: false

    // ---- what the targets ask ----------------------------------------------------------------------------------
    /// A target's ask rose. A refusal of what is typed standing — out or waiting — turns a hover's ask away: one box,
    /// left with the refusal until the typing changes it.
    function ask(a) {
        if (machine.target !== null && machine.target !== a && machine.target.typed)
            return
        machine.point(a, false)
    }
    /// A target's ask fell: a hover's starts the beat (the hand may be walking into the box); a refusal's ends the
    /// box for good — the typing changed (「拒否が解けて下りた補足は出し直さない」).
    function unask(a) {
        if (machine.targetKey === a.key && a.typed) {
            machine.takeDown("typing", false)
            return
        }
        machine.unpoint(a.key)
    }
    /// The target took its ask back for a reason of its own: down now, no beat, no put-back — and no latch, since
    /// the target will ask afresh when its reason is gone.
    function drop(a) {
        if (machine.targetKey === a.key)
            machine.takeDown("withheld", false)
        else
            machine.unpoint(a.key)
    }
    /// The target is going (a delegate destroyed): as `drop`.
    function forget(a) {
        seat.drop(a)
    }
    /// The rows the target moves with moved under the hand (`TipAsk.hand`): the box out on it, or its rest, goes now —
    /// no beat, no latch (`HoverSeat.displace`). A target still asking is asked again from where it stands now, with
    /// its rest: one under a hand the rows moved under has already let go (`TipAsk.live`), so what keeps asking is a
    /// target the hand is not on — a stand-in a run wrote, a hand the surface cannot hear. Another target's box, or an
    /// ask kept for one behind a menu, is not this target's to touch.
    function slid(a) {
        if (machine.target === a)
            machine.displace("slid")
    }

    readonly property HoverSeat machine: HoverSeat {
        id: machine
        card: box
        // The window's hand: a box the reader dismissed — or a menu took down — comes back only once the hand has
        // moved (the rest is the judgement, as on every seat that rests).
        hand: seat.hand
        restMs: Metrics.tipDelayMs
        graceMs: Metrics.hoverKeepMs
        // What hover did not open is not the bar's to close, nor to keep from coming out.
        ignoresBar: machine.target !== null && machine.target.typed
        barExempt: ask => ask.typed
        // Behind a menu standing in the window nothing comes out, and what is out goes the moment it stands (デザイン規約
        // §メニュー「メニューが立っている間、その裏の hover は引っ込む」) — but for a menu's own rows, which are not behind
        // it, and a refusal of what is typed, which no hover opened.
        blockedBy: OutsidePress.standsOver(seat.host) ? "menu" : ""
        exempt: ask => ask.typed || OutsidePress.holds(ask.target)
        onOpenAsked: target => {
            box.parent = target.target
            seat.freeze()
            box.open()
        }
    }
    // The hand reached the box: a hold; letting go starts the beat.
    onPointedChanged: {
        machine.hold("inside", seat.pointed)
        seat.cover()
    }
    /// The hand in the box is over the rows its target moves with, though they do not hear it — as the hand in a card
    /// is (`HoverSeat.covering`): the rows moving meanwhile hold it from the first place heard once the box is gone,
    /// so the row the box's going leaves under it does not ask (`TipAsk.hand`).
    property HandWatch covering: null
    function cover() {
        if (seat.covering !== null) {
            seat.covering.uncovered()
            seat.covering = null
        }
        const ask = machine.target
        if (seat.pointed && machine.shown && ask !== null && ask.hand !== null) {
            ask.hand.covered++
            seat.covering = ask.hand
        }
    }
    // A refusal of what is typed stands wherever the hand goes.
    readonly property bool typedStanding: machine.target !== null && machine.target.typed
    onTypedStandingChanged: machine.hold("typed", seat.typedStanding)

    Component.onCompleted: Tips.enter(seat)
    Component.onDestruction: Tips.leave(seat)

    // ---- where the box stands ----------------------------------------------------------------------------------
    /// Binds the seat, again at every coming out (`freeze`). Bindings: a popup is not its final size in the frame it
    /// gets its words, and assigned, a box above its target would come out one box too low
    /// (rules-refs/app-ui.md「出す前に採寸する」). Again: where the target stands is a call (`targetAt`) they do not
    /// follow, so a target moved while its box was down, under a hand back on the column it rested on, would come
    /// out where the target stood when they last ran.
    function seatBox() {
        box.x = Qt.binding(seat.seatX)
        box.y = Qt.binding(seat.seatY)
    }
    /// The target's top-left in `host` coordinates. A method, so nothing binds to it — the seat moves with the sizes
    /// and the anchor, and is read against the target's place again each time a box comes out.
    function targetAt() {
        const at = box.parent
        return at !== null ? at.mapToItem(seat.host, 0, 0) : Qt.point(0, 0)
    }
    /// Beside the hand (the target's middle when there is none), inside the window. A box at least as wide as its
    /// target sits on the target's left edge instead, so neighbouring tabs' boxes do not open at the same x
    /// (規約 §hover のツールチップ). A row among rows stands it beside the row instead (`rowSeatX`).
    function seatX() {
        const at = box.parent
        const ask = machine.target
        if (at === null || ask === null)
            return 0
        const p = seat.targetAt()
        // Beside it: flush against its right edge, pulled back inside the window.
        if (ask.beside) {
            const beside = p.x + at.width
            const last = seat.host.width - box.implicitWidth - Theme.spaceXs
            return Math.max(Theme.spaceXs, Math.min(beside, last)) - p.x
        }
        if (ask.rowSide !== "")
            return seat.rowSeatX(box.width, at, p, ask.rowSide) - p.x
        const edge = Theme.spaceXs
        const room = seat.host.width - box.implicitWidth - edge
        // A field's refusal starts where the field does, whatever its length.
        if (ask.typed)
            return Math.max(edge, Math.min(p.x, room)) - p.x
        const mid = seat.anchorKnown ? seat.anchorX : p.x + at.width / 2
        const want = box.implicitWidth >= at.width ? p.x : mid - box.implicitWidth / 2
        return Math.max(edge, Math.min(want, room)) - p.x
    }
    /// A row's box, in `host` x: flush against the row's named side, else its other side — out of the list, so the rows
    /// above and below stay readable (規約 §hover のツールチップ「行の的は、行の横に立つ」). When neither outside holds the
    /// box, it stands on the row itself beside the hand, a step off it: the pointer stays on the row under the step,
    /// and the box is not under the pointer.
    function rowSeatX(width, at, p, side) {
        const edge = Theme.spaceXs
        const last = seat.host.width - width - edge
        const firstLeft = side === "left"
        const leftSeat = p.x - width
        const rightSeat = p.x + at.width
        if (leftSeat >= edge && (firstLeft || rightSeat > last))
            return leftSeat
        if (rightSeat <= last)
            return rightSeat
        const hand = seat.anchorKnown ? seat.anchorX : p.x + at.width / 2
        const handLeft = hand - edge - width
        const handRight = hand + edge
        const goesLeft = firstLeft ? handLeft >= edge : handRight > last
        return Math.max(edge, Math.min(goesLeft ? handLeft : handRight, last))
    }
    /// Flush against the target: above while there is room, else below. When neither fits (a wrapped name in a short
    /// window), the roomier side — "below" alone would hang out of the window with `margins` off.
    function seatY() {
        const at = box.parent
        const ask = machine.target
        if (at === null || ask === null)
            return 0
        const p = seat.targetAt()
        // Beside it: level with the hand, not the target's top — the asker may sit a row above the target, and a
        // straight sideways move must reach the box (rules-refs/app-ui.md「的の脇に立つ箱」). Runs keep the top.
        if (ask.beside) {
            const last = seat.host.height - box.implicitHeight - Theme.spaceXs
            const want = seat.anchorDown ? seat.anchorY - box.implicitHeight / 2 : p.y
            return Math.max(Theme.spaceXs, Math.min(want, last)) - p.y
        }
        // A row's box: on the row's own band, centred on it — the rows either side keep all but the box's overhang.
        if (ask.rowSide !== "") {
            const last = seat.host.height - box.implicitHeight - Theme.spaceXs
            const want = p.y + (at.height - box.implicitHeight) / 2
            return Math.max(Theme.spaceXs, Math.min(want, last)) - p.y
        }
        const above = p.y
        const below = seat.host.height - (p.y + at.height)
        return box.implicitHeight <= above || above >= below ? -box.implicitHeight : at.height
    }
    /// Reads the hand's seat off for a box coming out now, and again a turn later, before anything is drawn
    /// (rules-refs/app-ui.md「`TipSeat` の席は 2 度読む」).
    function freeze() {
        seat.readAnchor()
        seat.seatBox()
        Qt.callLater(seat.settleAnchor)
    }
    function settleAnchor() {
        if (box.visible)
            seat.readAnchor()
    }
    function readAnchor() {
        const ask = machine.target
        // A field's refusal: the hand may be anywhere in the window.
        if (ask === null || ask.typed) {
            seat.anchorKnown = false
            seat.anchorDown = false
            return
        }
        if (seat.handAcross >= 0) {
            seat.anchorKnown = true
            seat.anchorDown = false
            seat.anchorX = seat.targetAt().x + ask.target.width * seat.handAcross
            return
        }
        seat.anchorKnown = seat.hand.known
        seat.anchorDown = seat.anchorKnown
        if (seat.anchorKnown) {
            const at = seat.host.mapFromItem(null, seat.hand.at.x, seat.hand.at.y)
            seat.anchorX = at.x
            seat.anchorY = at.y
        }
    }

    // ---- the box ------------------------------------------------------------------------------------------------
    ToolTip {
        id: box
        // The seat counts the rest and the beat itself.
        delay: 0
        timeout: -1
        padding: Theme.spaceSm
        // The seat clamps itself: `Popup`'s push-back knows nothing of the target and leaves the hand behind.
        margins: -1
        // The style's policy reads a press on the box — a selection starting — as a press outside the target.
        closePolicy: Popup.CloseOnEscape
        text: machine.target !== null ? machine.target.words : ""
        background: Rectangle {
            /// The sweep hand, named so a run can reach it at `box.background.pad`.
            property alias pad: tipPad
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            HoverHandler {
                id: groundHover
                onHoveredChanged: seat.groundPointed = groundHover.hovered
            }
            // Every press on the box, its words' included, as on a card's face (`AppCardFace`): the window's watcher
            // never hears a press inside a popup (`Hand.acted`).
            PointHandler {
                acceptedButtons: Qt.AllButtons
                onActiveChanged: if (active) Hand.acted()
            }
            // Under the words, so it gets only the presses they did not take (規約 §hover のツールチップ).
            SweepPad {
                id: tipPad
                anchors.fill: parent
                content: box.contentItem
            }
        }
        contentItem: CardText {
            id: tipWords
            text: box.text
            readonly property var ask: machine.target
            readonly property string markWord: tipWords.ask !== null ? tipWords.ask.markWordNow : ""
            /// Where the one name stands: a tip names it last, after its conclusion (`Words.tipNameSeat`) — unless the
            /// ask seats its names itself (`TipAsk.marksNow`).
            readonly property int markAt: Words.tipNameSeat(box.text, tipWords.markWord)
            // Each a sign on its name (デザイン規約 §アイコンの席 の修飾), in a room the field opens; the sentence's own
            // space before the word stays the gap.
            marks: tipWords.ask === null ? []
                 : tipWords.ask.marksNow.length > 0 ? tipWords.ask.marksNow
                 : tipWords.markAt >= 0
                 ? [[tipWords.markAt, tipWords.ask.markKindNow, undefined, tipWords.markWord.length]]
                 : []
            // The pointer step is a colour, which can only ride inside the field's markup (`CardText.place`).
            place: tipWords.ask !== null ? tipWords.ask.place : ""
            placeHref: tipWords.ask !== null ? tipWords.ask.href : ""
            placeTint: tipWords.pointedLink !== "" ? Theme.textSecondary : Theme.textMuted
            onLinkAsked: href => seat.linkAsked(href)
            pixelSize: Theme.fontMd
            color: Theme.textPrimary
            // Half the window, as a graph row's card: names are carried in full and wrap rather than cut.
            width: Math.min(implicitWidth, seat.host.width / 2)
            onPointedChanged: seat.wordPointed = pointed
        }
    }
}

import QtQuick
import platitude.ui

/// The hand over one surface — a pane's rows, the window — read where Qt's hover cannot say it: where it is, and
/// whether it was the hand that moved or the rows under it (デザイン規約 §左メニューの所作「手が動いていない所へ来た行は
/// 開かない」). The rows light and open off this, never off their own hover: Qt hands hover to whatever arrives under a
/// still pointer, and tells the ancestor's handler the point again when the layout moves
/// (`tests/qml/tst_hoverunderstillhand.qml`), so a move is the place changing, not the telling.
///
/// A passive `HoverHandler` on the surface itself, which sees the hand over every child and loses it under a popup
/// (rules-refs/app-ui.md「ペイン全体の hover を測る handler はペイン自身に置く」). A surface outside it that belongs to the
/// same rows (the folded rail's peek) feeds the same count through `stir`.
HoverHandler {
    id: watch

    /// Where the hand is, in scene coordinates; (-1, -1) while it is on no surface this watches, under a popup, or
    /// parked by a headless run (`Hand.away`).
    property point at: Qt.point(-1, -1)
    readonly property bool known: watch.at.x >= 0
    /// Another handler whose hover counts as this surface's: a popup over the rows that belongs to them (the folded
    /// rail's open section), which this handler never sees into. Read where this surface has no hand of its own.
    property var alsoOver: null
    /// How many cards hung off these rows have the hand inside them (`HoverSeat`): a hand in one of them is over the
    /// rows though no surface of theirs hears it, and the card going brings it back over them without a walk.
    property int covered: 0
    /// The hand is over these rows — heard, lost under something standing over them, or in a card hung off them.
    readonly property bool present: watch.known || watch.hidden || watch.covered > 0
    /// The rows moving now came of no reader's doing — a reread, the history growing — rather than of a scroll, a
    /// search or the keyboard (デザイン規約 §hover のツールチップ「外乱では閉じない」). Read while `rowsShifted` is told
    /// and while the rows read the hand again (`shifts`): a row a read took from under the hand has not slid away from
    /// a reader (`NavItemDelegate.syncHand`).
    property bool disturbed: false
    /// Whether something stands over the hand now: a card hung off these rows (`covered`), a popup in this window
    /// (`OutsidePress.handIn`) — with the hand inside. Asked, not bound: a popup telling it lost the hand
    /// (`OutsidePress.handLeft`) may run before a binding on the same change is weighed again.
    function underCover() {
        return watch.covered > 0 || OutsidePress.handIn(watch.parent)
    }
    /// The hand was lost under something standing over this surface and has not been heard since: Qt hands nothing
    /// under a popup its hover, so it may be the same hand, unmoved, and what is told meanwhile is no leave of its own.
    /// Out of the popup to somewhere this surface does not reach, it has left (`weighCover`).
    property bool hidden: false
    /// How often the hand itself has moved. Walking off counts: the row left would keep its light.
    property int moves: 0
    signal moved(point at)

    /// The rows moved under a hand that did not (`rowsMoved`): whatever is under it now arrived there, and nothing it
    /// asks for opens until the hand's own next move — one place apart from where it was then (`heldAt`), a tremor
    /// included; heard again at the same place is no move, nor is a menu or a card over the hand and going (`hidden`) —
    /// the hand unheard under them is not a hand that moved. Gone from the surface with nothing over it, or out from
    /// under one to elsewhere, the hand did move. As Qt's tooltips wake on the mouse's own moves alone
    /// (`QApplication::notify`): a scroll, a search or the keyboard's walk under a still pointer opens nothing over
    /// what the reader is looking at (デザイン規約 §左メニューの所作「手が動いたかの見切り」).
    property bool stillHeld: false
    /// Where the hand was when the rows moved; (-1, -1) while it was in a card then — the first place heard is the
    /// place (a card's line pressed sends the rows moving, and the card going brings the hand back over them).
    property point heldAt: Qt.point(-1, -1)
    /// How often the rows have moved under a hand over them — the rows read the hand again on each, as on a move: the
    /// light goes to the row under it (`NavItemDelegate.syncHand`, `GraphRowDelegate.syncHand`).
    property int shifts: 0
    /// The rows under the hand moved — what hung off them is told (`RowHoverHost`).
    signal rowsShifted()
    /// A read is moving these rows, weighed by its keeper as it lands (`RowKeep.busy`): a row it pushes past its view's
    /// cache is still there, and the box the hand is reading off it stays (`Tips.dropIn`).
    property bool reading: false
    /// …and that row is built again where it stood (`RowKeep.bringBack`): the box held off its pooled delegate comes
    /// back onto it (`TipAsk`, `Tips.takeBack`).
    signal rowBack(Item row)

    /// The rows under the hand moved — a scroll, rows inserted above, the list stepping down the pane. What hangs off
    /// them is told either way; the hold, and the rows reading the hand again, are for a hand over them — heard, or in
    /// a card hung off them (from the first place heard then). A hand elsewhere, or none, has left every row already
    /// and comes back onto them by walking: nothing to hold, nothing to read again — and no read per row for every
    /// row the move took.
    /// **What hangs off them is told first** (`rowsShifted`: what is out goes, an ask standing for it is kept), **then
    /// the rows read the hand again** (`shifts`): the row now under it asks after that, and its ask is the one kept —
    /// told the other way round, the seat would keep the ask of the row that slid away over the new one.
    /// `disturbed`: the move came of no reader's doing, and what the hand is inside stays (`disturbed`).
    function rowsMoved(disturbed) {
        const over = watch.present
        if (over) {
            watch.heldAt = watch.at
            watch.stillHeld = true
        }
        watch.disturbed = disturbed === true
        watch.rowsShifted()
        if (over)
            watch.shifts++
        watch.disturbed = false
    }
    /// The view put itself back over the rows it was showing after a reread (`GraphList.keepingRows`): nothing moved
    /// under the hand and nothing hung off the rows goes, but other delegates stand under it now, and they read the
    /// hand again (`shifts`) — the row under it asks for what stands, as the one there before did.
    function rowsKept() {
        if (watch.present)
            watch.shifts++
    }
    /// A place the hand was heard at, in scene coordinates, or (-1, -1) for gone; counted only where it differs from
    /// the last. **The rows read the place before the hold lifts** (`moves`, then `stillHeld`): what the hold kept is
    /// answered for the row the hand is on now — lifted first, the ask of the row it has just left would open.
    function stir(scenePoint) {
        const gone = scenePoint === undefined || scenePoint === null || scenePoint.x < 0
        const next = gone ? Qt.point(-1, -1) : Qt.point(scenePoint.x, scenePoint.y)
        if (next.x === watch.at.x && next.y === watch.at.y)
            return
        watch.at = next
        // Gone under something standing over the surface, the hand is hidden there. Qt hands what the hand came onto
        // its hover before it takes it from this surface, so the popup has it by now.
        watch.hidden = gone && watch.underCover()
        let lifts = false
        if (watch.stillHeld) {
            if (gone)
                // Gone with nothing over it, the hand has left the surface — a move of its own; hidden, it may be the
                // same still hand.
                lifts = !watch.hidden
            else if (watch.heldAt.x < 0)
                watch.heldAt = next
            else
                lifts = watch.apart(next, watch.heldAt) >= 1
        }
        watch.moves++
        if (lifts)
            watch.stillHeld = false
        watch.moved(next)
    }
    /// A popup has the hand no more (`OutsidePress.handLeft`) — the hand walked out, or the popup went from under it.
    /// Qt hands the hover to what the hand is on before it takes it from the popup — a popup gone included, on the next
    /// hover it delivers — so a hand still over this surface has been heard by now, whether it walked back or the popup
    /// went from over it. One still hidden, that nothing holds now, is elsewhere — out of the window, over another
    /// surface, under the part of a popup that reached past this one — and has left, as a hand gone with nothing over
    /// it: the hold lifts, and what this surface latched is told the hand went (`moved`).
    function weighCover() {
        if (!watch.hidden || watch.underCover())
            return
        watch.hidden = false
        watch.stillHeld = false
        watch.moved(watch.at)
    }
    property Connections coverWatch: Connections {
        target: OutsidePress
        function onHandLeft() { watch.weighCover() }
    }
    /// A card or a tip hung off these rows has the hand no more (`covered`, one down): weighed as a popup losing it is
    /// (`weighCover`).
    function uncovered() {
        watch.covered--
        watch.weighCover()
    }
    /// How far two places stand apart, in the larger of the two directions — a pixel either way is a move.
    function apart(a, b) {
        return Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y))
    }
    /// Whether the hand is on `item` — for things with no `hovered` of their own (a row is not a `Control`).
    function over(item) {
        if (!watch.known || item === null || item === undefined)
            return false
        const p = item.mapFromItem(null, watch.at.x, watch.at.y)
        return p.x >= 0 && p.y >= 0 && p.x < item.width && p.y < item.height
    }
    /// Where the hand is heard now: on this surface, else on the one that counts as it (`alsoOver`), else nowhere.
    /// The other surface is asked first when this one has lost the hand — Qt tells the surface the hand came onto
    /// before the one it left, so a hand crossing between the two is never heard as gone.
    function here() {
        if (Hand.away)
            return Qt.point(-1, -1)
        if (watch.hovered)
            return watch.point.scenePosition
        if (watch.alsoOver !== null && watch.alsoOver !== undefined && watch.alsoOver.hovered)
            return watch.alsoOver.point.scenePosition
        return Qt.point(-1, -1)
    }

    onPointChanged: watch.stir(watch.here())
    onHoveredChanged: watch.stir(watch.here())
}

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The shared tooltip — the one popup nobody declares, built by the attached property in Fusion's clothes. Dressing it
/// once through `host` carries to every `ToolTip.text` in the tree. Three things are put right
/// (規約 §hover のツールチップ):
///
/// * **The words can be taken away** — the content is a `CardText`.
/// * **The hand can get to them** — it opens beside the hand and flush against the target: a gap is a band the
///   pointer crosses while touching neither.
/// * **It waits** — the site's binding falls as the hand starts walking into the tip, so the tip comes back up and a
///   beat (`hoverKeepMs`) decides.
///
/// An Item: the Components below are children, and a QtObject has nowhere to put a child.
Item {
    id: shared

    /// The item the attached tooltip is read off and every placement is measured in. Required: `sharedTip` is read
    /// while this is built.
    required property Item host
    /// Where the hand is, window-wide; silent while a popup covers the pointer, which is when the tip falls back to its
    /// target.
    required property PointerWatch hand

    readonly property var sharedTip: shared.host.ToolTip.toolTip

    /// The word inside the tip that is a place to go, and its href — read off the target, since the attached property
    /// carries only the plain sentence (`HoverToolButton.tipPlace`).
    readonly property string tipPlace: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipPlace !== undefined ? at.tipPlace : ""
    }
    readonly property string tipHref: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipHref !== undefined ? at.tipHref : ""
    }
    /// The word the drawn tree mark stands in front of (a working copy's name, 規約 §ref の種別), read off the target
    /// like the two above; the markup opens a gap for the mark (`Words.roomInSentence`).
    readonly property string tipMarkWord: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipMarkWord !== undefined ? at.tipMarkWord : ""
    }
    /// The place word was pressed; `Main` hands the href to the page that owns it.
    signal linkAsked(string href)

    /// Hover stand-in for runs with no pointer: where the hand is on the target, as a share of its width (0 = left
    /// edge, 1 = right). Negative — the ordinary case — means no stand-in.
    property real handAcross: -1

    /// The pointer is on the tip's ground or its words — two handlers, since background and content are siblings
    /// (rules-refs/app-ui.md「hover で開くものの 5 つの罠」(2), the pair `AppCard` carries).
    property bool groundPointed: false
    property bool wordPointed: false
    readonly property bool pointed: shared.groundPointed || shared.wordPointed

    /// The tip is up only because this part put it back. Cleared when it really goes or another target takes over.
    property bool keeping: false
    /// This part is taking it down, so the fall is not put back.
    property bool dropping: false

    /// The target the box came out on — what the beat holds it for; an instance handed to another target is not the
    /// box the hand was reaching for (`handOver`).
    property Item standing: null

    /// Where the tip was opened beside, in `host` coordinates. Frozen when it comes out: a seat that followed the
    /// pointer would slide from under the hand walking into it.
    property real anchorX: 0
    property bool anchorKnown: false
    /// The same for y, read only by a box beside its target (`tipBeside`). Separate because a run's stand-in has no
    /// height; those runs seat the box at the target's top.
    property real anchorY: 0
    property bool anchorDown: false

    /// Whether a hand could be walking into the tip — what the beat waits for. Never in a run, told by `handAcross`
    /// (rules-refs/app-ui.md「`SharedToolTip` の保持は自動化では止める」).
    readonly property bool walkable: shared.handAcross < 0 && shared.hand.seen

    /// Puts the app's own card on it, binds the seat, and hands the words a field that can be selected.
    function dressToolTip() {
        const tip = shared.sharedTip
        // Built under this item (a popup is not an Item, so it cannot be the parent), then the seat is given straight
        // back: a popup adopts `contentItem` only when it has no visual parent, or the words paint at this item's
        // corner (rules-refs/app-ui.md「ツールチップの地と字は、席を返してから渡す」). The `QObject` owner stays.
        const ground = tipGround.createObject(shared)
        const word = tipWord.createObject(shared)
        ground.parent = null
        word.parent = null
        tip.background = ground
        tip.contentItem = word
        tip.padding = Theme.spaceSm
        // The seat clamps itself: `Popup`'s push-back knows nothing of the target and leaves the hand behind.
        tip.margins = -1
        // The style's policy reads a press on the tip — a selection starting — as a press outside the target.
        tip.closePolicy = Popup.CloseOnEscape
        // Bindings: a popup is not its final size in the frame it gets its words, and assigned, a tip above its
        // target would come out one tip too low (rules-refs/app-ui.md「出す前に採寸する」).
        tip.x = Qt.binding(shared.seatX)
        tip.y = Qt.binding(shared.seatY)
    }

    /// The target asks for the tip beside it, not over its own words (`NavRowFacts`). Read off the target like
    /// `tipPlace`.
    readonly property bool tipBeside: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipBeside === true
    }
    /// The target is a row among rows — a list's, a menu's, a diff's — and names the side its box stands on first,
    /// `"left"` or `"right"`; empty for any other target. Read off the target like `tipPlace`.
    readonly property string tipRowSide: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipRowSide !== undefined ? at.tipRowSide : ""
    }
    /// The target's tip answers what is being typed, not a hand (`NavNameBox`'s refusal), so a held scroll bar leaves
    /// it be (`yieldToBar`). Read off the target like `tipPlace`.
    readonly property bool tipTyped: {
        const at = shared.sharedTip.parent
        return at !== null && at.tipTyped === true
    }

    /// The target's top-left in `host` coordinates. A method, so nothing binds to it — the seat moves with the sizes
    /// and the anchor.
    function targetAt() {
        const at = shared.sharedTip.parent
        return at !== null ? at.mapToItem(shared.host, 0, 0) : Qt.point(0, 0)
    }

    /// Beside the hand (the target's middle when there is none), inside the window. A tip at least as wide as its
    /// target sits on the target's left edge instead, so neighbouring tabs' tips do not open at the same x
    /// (規約 §hover のツールチップ). A row among rows stands it beside the row instead (`rowSeatX`).
    function seatX() {
        const tip = shared.sharedTip
        const at = tip.parent
        if (at === null)
            return 0
        const p = shared.targetAt()
        // Beside it: flush against its right edge, pulled back inside the window.
        if (shared.tipBeside) {
            const beside = p.x + at.width
            const last = shared.host.width - tip.implicitWidth - Theme.spaceXs
            return Math.max(Theme.spaceXs, Math.min(beside, last)) - p.x
        }
        if (shared.tipRowSide !== "")
            return shared.rowSeatX(tip.width, at, p) - p.x
        const mid = shared.anchorKnown ? shared.anchorX : p.x + at.width / 2
        const want = tip.implicitWidth >= at.width ? p.x : mid - tip.implicitWidth / 2
        const edge = Theme.spaceXs
        const room = shared.host.width - tip.implicitWidth - edge
        return Math.max(edge, Math.min(want, room)) - p.x
    }

    /// A row's box, in `host` x: flush against the row's named side, else its other side — out of the list, so the rows
    /// above and below stay readable (規約 §hover のツールチップ「行の的は、行の横に立つ」). When neither outside holds the
    /// box, it stands on the row itself beside the hand, a step off it: the pointer stays on the row under the step,
    /// and the box is not under the pointer.
    function rowSeatX(width, at, p) {
        const edge = Theme.spaceXs
        const last = shared.host.width - width - edge
        const firstLeft = shared.tipRowSide === "left"
        const leftSeat = p.x - width
        const rightSeat = p.x + at.width
        if (leftSeat >= edge && (firstLeft || rightSeat > last))
            return leftSeat
        if (rightSeat <= last)
            return rightSeat
        const hand = shared.anchorKnown ? shared.anchorX : p.x + at.width / 2
        const handLeft = hand - edge - width
        const handRight = hand + edge
        const goesLeft = firstLeft ? handLeft >= edge : handRight > last
        return Math.max(edge, Math.min(goesLeft ? handLeft : handRight, last))
    }

    /// Flush against the target: above while there is room, else below. When neither fits (a wrapped name in a short
    /// window), the roomier side — "below" alone would hang out of the window with `margins` off.
    function seatY() {
        const tip = shared.sharedTip
        const at = tip.parent
        if (at === null)
            return 0
        const p = shared.targetAt()
        // Beside it: level with the hand, not the target's top — the asker may sit a row above the target, and a
        // straight sideways move must reach the box (rules-refs/app-ui.md「的の脇に立つ箱」). Runs keep the top.
        if (shared.tipBeside) {
            const last = shared.host.height - tip.implicitHeight - Theme.spaceXs
            const want = shared.anchorDown ? shared.anchorY - tip.implicitHeight / 2 : p.y
            return Math.max(Theme.spaceXs, Math.min(want, last)) - p.y
        }
        // A row's box: on the row's own band, centred on it — the rows either side keep all but the box's overhang.
        if (shared.tipRowSide !== "") {
            const last = shared.host.height - tip.implicitHeight - Theme.spaceXs
            const want = p.y + (at.height - tip.implicitHeight) / 2
            return Math.max(Theme.spaceXs, Math.min(want, last)) - p.y
        }
        const above = p.y
        const below = shared.host.height - (p.y + at.height)
        return tip.implicitHeight <= above || above >= below ? -tip.implicitHeight : at.height
    }

    /// Reads the hand's seat off for a tip coming out now, and again a turn later, before anything is drawn
    /// (rules-refs/app-ui.md「`SharedToolTip` の閉じ待ちは 3 つの実測の上に乗っている」(3)).
    function freeze() {
        shared.keeping = false
        keep.stop()
        shared.standing = shared.sharedTip.parent
        shared.readAnchor()
        Qt.callLater(shared.settleAnchor)
    }
    function settleAnchor() {
        if (shared.sharedTip.visible && !shared.keeping)
            shared.readAnchor()
    }
    function readAnchor() {
        const at = shared.sharedTip.parent
        if (at === null) {
            shared.anchorKnown = false
            shared.anchorDown = false
            return
        }
        if (shared.handAcross >= 0) {
            shared.anchorKnown = true
            shared.anchorDown = false
            shared.anchorX = shared.targetAt().x + at.width * shared.handAcross
            return
        }
        shared.anchorKnown = shared.hand.known
        shared.anchorDown = shared.anchorKnown
        if (shared.anchorKnown) {
            shared.anchorX = shared.hand.handX
            shared.anchorY = shared.hand.handY
        }
    }

    /// Whether anything still wants the tip where it is: the hand inside it, the hand back on the target, or the
    /// target saying so itself (a `Control` under a menu, where the window's own watch cannot see the pointer).
    function wanted() {
        if (shared.pointed)
            return true
        const at = shared.sharedTip.parent
        if (at === null)
            return false
        if (at.hovered === true)
            return true
        return shared.hand.over(at)
    }

    /// Takes it down for good. `dropping` is cleared where the fall is heard, not here: a style with an exit transition
    /// would announce the fall later, and this part would put its own close back up. Unmarked when nothing is open.
    function drop() {
        shared.keeping = false
        if (!shared.sharedTip.visible)
            return
        shared.dropping = true
        shared.sharedTip.close()
    }

    /// A scroll bar was taken (`Hand.heldBar`): the targets are about to slide from under a hand Qt no longer hears, so
    /// the box goes now, and a rest already counting goes with it — `drop` leaves a box that is not up yet, and closing
    /// that one stops its count (規約 §hover のツールチップ「スクロールバーを掴んだら、hover で開いたものは閉じる」). A box
    /// that answers typing is no hover's, and stays (`tipTyped`).
    function yieldToBar() {
        if (shared.tipTyped)
            return
        shared.drop()
        shared.sharedTip.close()
    }

    /// Puts it back after a fall the hand may be walking into — a turn later, because an `open()` from inside
    /// `Popup`'s own teardown is silently dropped. `Qt.callLater` runs before the next draw, so nothing blinks.
    function reopen() {
        if (!shared.keeping || shared.sharedTip.visible)
            return
        // Only onto the target it came out on: another target's ask may have moved the instance first (`handOver`).
        if (shared.sharedTip.parent !== shared.standing) {
            shared.keeping = false
            return
        }
        // No delay (規約 §hover のツールチップ「出ているものの的へ戻る手は即通す」): through it, the beat would run
        // out first and take the tip down under the hand. The attached property writes the delay again on the next ask.
        shared.sharedTip.delay = 0
        shared.sharedTip.open()
        keep.restart()
    }

    /// The instance was handed to another target while it stood: Qt swaps `parent` and words without closing or
    /// counting the delay, so the box goes down and the ask is made again
    /// (規約 §hover のツールチップ「隣の的へは箱を下ろしてから移る」). A turn later, because this runs from the middle
    /// of the attached property's own show.
    function handOver() {
        const tip = shared.sharedTip
        if (!tip.visible || tip.parent === shared.standing)
            return
        shared.drop()
        // On the delay, which starts the new target's count; a hand that left the row closes it through the site's
        // own binding.
        if (tip.parent !== null)
            tip.open()
    }

    property Timer keep: Timer {
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!shared.wanted())
                shared.drop()
        }
    }

    // The hand reached the tip: the beat has nothing left to decide. Letting go starts it again.
    onPointedChanged: {
        if (shared.pointed)
            keep.stop()
        else if (shared.sharedTip.visible)
            keep.restart()
    }

    Connections {
        target: shared.sharedTip
        // A target letting go is answered with a beat; a target taking over, with a close — it is owed its own delay.
        function onVisibleChanged() {
            if (shared.sharedTip.visible) {
                if (!shared.keeping)
                    shared.freeze()
                return
            }
            if (shared.dropping) {
                shared.dropping = false
                return
            }
            if (!shared.walkable)
                return
            shared.keeping = true
            Qt.callLater(shared.reopen)
        }
        function onParentChanged() {
            if (shared.sharedTip.visible)
                Qt.callLater(shared.handOver)
        }
        // Asked for while a bar is held — a row reused under the hand takes the hover afresh, a rest runs out: down
        // again before anything is drawn. Here, not in `onVisibleChanged`: that is heard from the middle of the
        // opening, which carries on after a close made there (`QQuickPopupPrivate::prepareEnterTransition`).
        function onOpened() {
            if (Hand.heldBar !== null)
                shared.yieldToBar()
        }
    }
    Connections {
        target: Hand
        function onHeldBarChanged() {
            if (Hand.heldBar !== null)
                shared.yieldToBar()
        }
    }

    Component {
        id: tipGround
        Rectangle {
            /// The sweep hand, named so a run can reach it at `tip.background.pad` — an id inside a `Component` is
            /// invisible outside.
            property alias pad: tipPad
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            HoverHandler {
                id: groundHover
                onHoveredChanged: shared.groundPointed = groundHover.hovered
            }
            // Under the words, so it gets only the presses they did not take (規約 §hover のツールチップ).
            SweepPad {
                id: tipPad
                anchors.fill: parent
                content: shared.sharedTip.contentItem
            }
        }
    }
    Component {
        id: tipWord
        CardText {
            id: tipWords
            text: shared.sharedTip.text
            /// How many spaces the mark's ink takes. The sentence's own space before the word is the gap; these are
            /// for the ink alone.
            readonly property int markRoom:
                shared.tipMarkWord === "" || spaceRuler.advanceWidth <= 0
                ? 0 : Math.ceil(tipMark.inkWidth / spaceRuler.advanceWidth)
            /// Where that gap came out (empty when none). One binding over its inputs, since `charRect` is a call
            /// (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
            readonly property rect markSeat: {
                const at = shared.sharedTip.text.indexOf(shared.tipMarkWord)
                if (tipWords.markRoom <= 0 || at < 0 || tipWords.markup === ""
                        || tipWords.width <= 0 || tipWords.height <= 0)
                    return Qt.rect(0, 0, 0, 0)
                return tipWords.charRect(at + tipWords.markRoom)
            }
            // Built here: the pointer step is a colour, which can only ride inside the markup. A tip carries a place
            // or a mark, never both.
            markup: shared.tipMarkWord !== ""
                    ? Words.roomInSentence(shared.sharedTip.text, shared.tipMarkWord, tipWords.markRoom)
                    : Words.placeInSentence(shared.sharedTip.text, shared.tipPlace, shared.tipHref,
                                            pointedLink !== "" ? Theme.textSecondary : Theme.textMuted)
            onLinkAsked: href => shared.linkAsked(href)
            pixelSize: Theme.fontMd
            color: Theme.textPrimary
            // Half the window, as a graph row's card: names are carried in full and wrap rather than cut.
            width: Math.min(implicitWidth, shared.host.width / 2)
            onPointedChanged: shared.wordPointed = pointed
            /// A space in the field's own font, so a family with a wider space opens a wider gap.
            TextMetrics {
                id: spaceRuler
                font.family: Theme.uiFamily
                font.pixelSize: tipWords.pixelSize
                text: " "
            }
            /// Set against the word, so mark and name read as one (規約 §ref の種別「名前の印」): the ink ends where
            /// the word begins.
            NavIcon {
                id: tipMark
                visible: tipWords.markSeat.width > 0 || tipWords.markSeat.height > 0
                kind: "tree"
                tint: Theme.textSecondary
                width: Theme.iconSm
                height: Theme.iconSm
                x: tipWords.markSeat.x - (Theme.iconSm + tipMark.inkWidth) / 2
                y: tipWords.markSeat.y + (tipWords.markSeat.height - Theme.iconSm) / 2
            }
        }
    }
}

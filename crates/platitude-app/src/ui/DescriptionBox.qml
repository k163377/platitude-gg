import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The description box, wherever a commit message is written or read, and the one box whose corner grip pulls it
// taller (デザイン規約 §コミットメッセージの 2 つの枠). The pane owns the bound (`room` / `owed`).
Rectangle {
    id: box

    /// How much taller the pane can still let the box grow.
    property real room: 0
    /// How much of the pull the pane needs back.
    property real owed: 0

    property alias text: area.text
    property alias readOnly: area.readOnly
    property alias placeholderText: area.placeholderText
    /// The colour the text is painted in — the side the smoke hooks report (rules-refs/app-ui.md).
    readonly property color textColor: area.color
    /// Being written in: the caret is here, or this box's own menu stands over it (`FieldMenuSeat.holding`).
    readonly property bool focused: area.activeFocus || menuSeat.holding
    /// Automation: the bar inside the box (a run reads its opacity, the painted side) and how far the text stands.
    readonly property alias bar: textBar
    readonly property real textAt: textView.contentItem ? textView.contentItem.contentY : 0
    /// Automation only: the middle-button hand, driven without a pointer (a middle button cannot be injected).
    readonly property alias hand: hand

    /// What the pane allots this box at rest, before any pull.
    property real restHeight: Theme.messageMaxHeight
    /// How much taller than that the hand has pulled it; the pane adds it to the block it lays out. A plain property:
    /// `setBoxHeight` assigns to it, which would end a binding for good.
    property real extra: 0
    /// What the text would take if nothing held it in.
    readonly property real wants: area.implicitHeight + Theme.spaceSm
    /// What the box was actually given, read back off the layout.
    readonly property real boxHeight: box.height
    /// The ceiling a pull can reach: its own text, capped at what the pane has left to lend.
    readonly property real ceiling: Math.min(Math.max(box.wants, box.restHeight), box.height + box.room)
    /// Reported to the smoke hooks.
    readonly property real cap: box.restHeight + box.extra
    /// Whether the grip is offered: while the text runs past the box, and while a pull stands (putting it back is the
    /// same grip).
    readonly property bool grips:
        box.wants > box.height + Theme.borderWidth && (box.room > 0 || box.extra > 0) || box.extra > 0

    /// The one place the pull is recorded, as an offset from the pane's allotment, so a summary that gains a line
    /// moves the box with it.
    function setBoxHeight(want) {
        box.extra = Math.max(0, Math.min(want - box.restHeight, box.wants - box.restHeight, box.extra + box.room))
    }
    /// Every pull — the drag and the smoke hooks — comes through here, so the clamp and the refusal are one answer.
    function pullTo(want) {
        box.unpin()
        box.askedHeight = want
        // Judged before the pull lands: the bound is `boxHeight + room`, and `room` only updates on the next layout,
        // so read afterwards it has grown by what the box took and always says the ask fitted.
        box.askedPast = want > box.ceiling + Theme.splitterWidth || want < box.restHeight - Theme.splitterWidth
        box.setBoxHeight(want)
    }
    /// Smoke hook: pull the grip down by dy, through the drag's clamp.
    function grow(dy) {
        box.pullTo(box.height + dy)
    }
    /// Put the pull and the reading position back — called wherever the pane swaps in a different message (デザイン規約
    /// §コミットメッセージの 2 つの枠). The caret goes first: assigning `text` leaves it at the end and the `TextArea`
    /// scrolls to it after layout, so setting `contentY` alone does not hold.
    function resetForNewMessage() {
        box.extra = 0
        area.cursorPosition = 0
        box.pinnedTop = true
        box.toTop()
    }
    function toTop() {
        if (textView.contentItem)
            textView.contentItem.contentY = 0
    }
    /// Held at the top until the reader moves it: one assignment loses to the caret's scroll, which lands later the
    /// longer the text.
    property bool pinnedTop: false
    /// The reader took the box over: a wheel, a caret, or a pull.
    function unpin() {
        box.pinnedTop = false
    }
    Connections {
        target: textView.contentItem
        enabled: box.pinnedTop
        function onContentYChanged() {
            if (box.pinnedTop && textView.contentItem.contentY !== 0)
                textView.contentItem.contentY = 0
        }
        function onContentHeightChanged() {
            if (box.pinnedTop)
                textView.contentItem.contentY = 0
        }
    }

    // ---- a pull the box has nothing left to answer with -----------------
    /// What the last pull asked for, clamped or not — the only thing that can tell a refusal from a rest (規約
    /// §掴める境界は答える).
    property real askedHeight: 0
    /// Where the hand is, in scene coordinates — the badge is drawn in `RepoPage`'s overlay.
    property point gripPoint: Qt.point(0, 0)
    /// Automation only: stands in for the press the hooks cannot make.
    property bool gripHeld: false
    readonly property bool gripDragging: grip.pressed || box.gripHeld
    /// Whether the last ask was past what the box could give — only `pullTo` can judge it.
    property bool askedPast: false
    /// Whether a pull is asking for a height the box cannot be, either way. Letting go ends it, so no badge is
    /// stranded.
    readonly property bool gripRefused: box.gripDragging && box.askedPast
    /// How far into the text's inset the bar stops while the grip is offered: at the mark's highest ink, so the down
    /// arrow stands wholly above the grip, never down in the mark's square beside its slant, and keeps from it the air
    /// its end holds below it anywhere (デザイン規約 §スクロールバーの矢印). Rounded up: an end reaching into the mark
    /// would bring the arrow down onto it.
    readonly property int gripYield: Math.ceil(textView.y + textView.height - (grip.y + gripMark.y + gripMark.inkTop))
    /// Automation: a pull carried past one of the two ends (`PGG_AUTO_ACT=divider-refuse`, cases `desc-max` /
    /// `desc-min`).
    function pullPast(down) {
        const over = 4 * Theme.splitterWidth
        box.gripHeld = true
        box.pullTo(down ? box.ceiling + over : box.restHeight - over)
        // Written after the pull, off `boxHeight` (the anchored grip only moves on the next layout): written before,
        // it leaves the badge at the corner the grip has since left.
        box.gripPoint = box.mapToItem(null, box.width - gripMark.width / 2, box.boxHeight - gripMark.height / 2
        + (down ? over : -over))
    }
    // The badge is the page's (`RepoPage`): a refused hand is outside this box, and a badge parented here is
    // composited under the author card and the file list.
    /// A wheel this box had nothing left to do with, in pixels. Whoever put the box on a scrolling surface moves that
    /// surface by it — the box covers most of it, which could otherwise not be reached by wheel.
    signal wheelPastEnd(real pixels)
    /// Escape with the caret in here; whoever owns the text decides what it means (`MessageEditor`).
    signal escaped()
    /// The wheel is taken here because a flickable at its end keeps the event and says nothing.
    readonly property real wheelStep: Metrics.wheelRows * Theme.fontMdLine
    function rollBy(dy) {
        box.unpin()
        const flick = textView.contentItem
        const pixels = dy / 120 * box.wheelStep
        const max = Math.max(0, flick.contentHeight - flick.height)
        // From where the notch in flight is aiming, or two quick notches land as one (`WheelGlide.at`).
        const from = glide.at
        const next = Math.max(0, Math.min(max, from - pixels))
        if (Math.abs(next - from) > 0.5) {
            glide.sendTo(next)
            return
        }
        box.wheelPastEnd(pixels)
    }
    WheelGlide {
        id: glide
        view: textView.contentItem
    }
    /// The middle button's drift — the words and nothing else. Unlike the wheel it hands nothing to the surface under
    /// the box: the box travels with that surface and would carry the ring away from the pointer that set it.
    function driftText(dy) {
        box.unpin()
        // A notch still in flight would pull against the drift for its last beat (`WheelGlide.halt`).
        glide.halt()
        const flick = textView.contentItem
        flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, flick.contentY + dy))
    }
    /// Smoke hook: the caret in the box, the way a click puts it there. Not `focus()`: that name resolves to Item's
    /// `focus` property and the call is a TypeError.
    function takeCaret() {
        box.unpin()
        area.forceActiveFocus()
    }
    /// The caret leaves the box (Escape).
    function dropCaret() {
        area.focus = false
    }
    // Given back at once, before the pane's column runs under its edge.
    onOwedChanged: {
        if (box.owed > 0)
            box.setBoxHeight(box.height - box.owed)
    }
    // A viewport that grows without scrolling leaves TextEdit drawing only the old height; nudging `contentY` and
    // putting it back makes it draw again (rules-refs/app-ui.md).
    onHeightChanged: Qt.callLater(box.repaint)
    function repaint() {
        const flick = textView.contentItem
        const was = flick.contentY
        flick.contentY = was + 1
        flick.contentY = was
    }

    // Takes whatever the summary left of the pane's fixed-height block (デザイン規約 §コミットメッセージの 2 つの枠).
    Layout.fillWidth: true
    Layout.fillHeight: true
    Layout.minimumHeight: 0
    color: Theme.bgBase
    radius: Theme.radiusMd
    border.color: Theme.borderSubtle
    border.width: Theme.borderWidth

    // Declared first: under the view and the grip, it takes only the inset's presses neither took (`SweepBand`).
    SweepBand {
        anchors.fill: parent
        field: area
    }

    ScrollView {
        id: textView
        anchors.fill: parent
        // All four sides, the bar's included — no gutter; the words run under the thumb (デザイン規約 §余白).
        anchors.margins: Theme.spaceXs
        // Upright only; the sideways bar is refused by policy — `AsNeeded` takes presses at the foot, `null` logs a
        // `TypeError` (rules-refs/app-ui.md「`ScrollView` の要らない側のバー」, `tst_messageband`).
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        // A bar handed to a `ScrollView` gets no geometry, so the three numbers are written here and its flickable is
        // named (`AutoScrollBar.view`). Same block in `MessageEditor`.
        ScrollBar.vertical: AutoScrollBar {
            id: textBar
            view: textView.contentItem
            x: textView.width - width
            y: textView.topPadding
            // Short of the grip's ink while it is offered (`gripYield`).
            height: textView.availableHeight - (box.grips ? box.gripYield : 0)
            lengthAlong: textBar.height
            stepGlide: glide
            // Taking the bar is the reader taking the box over, as the wheel is.
            onHeldChanged: if (textBar.held) box.unpin()
        }
        // Reached once it exists. Not `interactive`: the flickable would answer the wheel as well as the handler and
        // move the text twice; `rollBy` is the only thing that moves it.
        Component.onCompleted: {
            contentItem.boundsBehavior = Flickable.StopAtBounds
            contentItem.interactive = false
        }
        TextArea {
            id: area
            wrapMode: TextArea.Wrap
            font.pixelSize: Theme.fontMd
            Keys.onEscapePressed: event => {
                box.escaped()
                event.accepted = true
            }
            // Lit only while being written; a read-only caret is for selecting (デザイン規約 §コミットメッセージの 2 つの枠).
            color: !area.readOnly && box.focused ? Theme.textPrimary : Theme.textSecondary
            background: null
            padding: 0
            onActiveFocusChanged: if (area.activeFocus) box.unpin()
            // The product's right-click menu, not the style's (`FieldMenuSeat`).
            ContextMenu.menu: null
            ContextMenu.onRequested: menuSeat.offer()
            FieldMenuSeat {
                id: menuSeat
                editor: area
            }
            // On the text, so it is offered the wheel before the flickable can keep it (`rollBy`).
            WheelHandler {
                acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                onWheel: event => box.rollBy(event.angleDelta.y)
            }
        }
    }
    // Declared after the ScrollView, or the bar's end takes the corner. The mark hangs off the square's lower-right, on
    // the text's own inset. While the bar stands, the down arrow's end reaches into the square (`gripYield`): the part
    // of the square the bar covers is the bar's, the rest the grip's.
    MouseArea {
        id: grip
        visible: box.grips
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: Theme.iconMd
        height: Theme.iconMd
        // A mask is the whole of the hit test: Qt asks it instead of the item's own rectangle, so it says the square
        // itself — a point outside it answered true takes presses from anywhere in the window.
        containmentMask: QtObject {
            function contains(point: point): bool {
                if (point.x < 0 || point.y < 0 || point.x > grip.width || point.y > grip.height)
                    return false
                return !textBar.visible || !textBar.contains(grip.mapToItem(textBar, point.x, point.y))
            }
        }
        hoverEnabled: true
        cursorShape: Qt.SizeVerCursor
        /// Where the pull started, in the pane's coordinates — the grip travels with the edge it moves.
        property real fromY: 0
        property real fromHeight: 0
        onPressed: mouse => {
            grip.fromY = mapToItem(box.parent, 0, mouse.y).y
            grip.fromHeight = box.boxHeight
            // The last drag's answer is not this one's (it would show the badge a frame early).
            box.askedPast = false
        }
        onPositionChanged: mouse => {
            box.gripPoint = grip.mapToItem(null, mouse.x, mouse.y)
            if (!grip.pressed)
                return
            box.pullTo(grip.fromHeight + mapToItem(box.parent, 0, mouse.y).y - grip.fromY)
        }
        NavIcon {
            id: gripMark
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            kind: "grip"
            tint: grip.containsMouse || grip.pressed ? Theme.textSecondary : Theme.borderStrong
        }
    }
    // The middle button's hand, over the words — last, so a gesture under way also stands over the grip. Only while
    // the words have somewhere to go (the bar's answer); otherwise a middle press is the surface's. Where the middle
    // button pastes, the words keep the press (`MiddleAutoScroll.claimedAt`).
    MiddleAutoScroll {
        id: hand
        anchors.fill: textView
        visible: textBar.visible
        onDrifted: dy => box.driftText(dy)
    }
}

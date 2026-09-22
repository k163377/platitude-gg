import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The description box, in both places a commit message is written or read (デザイン規約 §コミットメッセージの 2 つの枠). Dim until a caret
// is in it, and the one box in the app that can be pulled taller by the grip in its corner: a description worth reading
// is routinely longer than the room it is given, while a summary that long is an accident.
//
// Its height comes from the layout. The summary and this box are two halves of one block whose height is fixed, so
// this half is simply what the layout hands it once the summary has taken its lines — which is why the pair keeps the
// same total whether the summary runs to one line or three, and why nothing here can disagree with what the pane drew.
//
// The pane it sits in owns the bound. It measures how much room is still free below the box (`room`) and how far it is
// already past its own edge (`owed`) — nothing here knows what a file list or an author card is.
Rectangle {
    id: box

    /// How much taller the box may still be drawn before the pane runs out of what it was willing to lend.
    property real room: 0
    /// How much the pane needs back. The far side of the same measure: the window shrinks, a panel opens, a row
    /// appears, and what the hand was given has to be returned.
    property real owed: 0

    property alias text: area.text
    property alias readOnly: area.readOnly
    property alias placeholderText: area.placeholderText
    /// What the box paints — read by the smoke hooks, which report the painted side (see
    /// app-ui.md).
    readonly property color textColor: area.color
    readonly property bool focused: area.activeFocus
    /// Automation: the bar inside the box, and how far the text stands. A bar in here is the one the style keeps
    /// (デザイン規約 §色 スクロールバー), and what it paints is its own opacity — the side a run reads (verify-ui).
    readonly property alias bar: textBar
    readonly property real textAt: textView.contentItem ? textView.contentItem.contentY : 0

    /// What the pane has allotted this box at rest, before any pull. The pane works it out: the
    /// summary above it and this box are laid out inside one block of a single height (デザイン規約 §コミットメッセージの 2 つの枠), so
    /// what is left for this half is the pane's answer.
    property real restHeight: Theme.messageMaxHeight
    /// How much taller than that the hand has pulled it. The pane adds this to the block it lays out, so this box's own
    /// height is always simply what the layout handed it — the pane's is the only opinion about the height
    /// here.
    ///
    /// A plain property: `setBoxHeight` assigns to it, and a JS assignment ends a QML binding for
    /// good, so a height expressed as a binding would silently stop tracking the pane the first time anything wrote
    /// through it.
    property real extra: 0
    /// What the text would take if nothing held it in.
    readonly property real wants: area.implicitHeight + Theme.spaceSm
    /// What the box was actually given. Read back off the layout, so nothing downstream can
    /// disagree with what is on the screen.
    readonly property real boxHeight: box.height
    /// The ceiling a pull can reach: its own text, capped at what the pane has left to lend.
    readonly property real ceiling: Math.min(Math.max(box.wants, box.restHeight), box.height + box.room)
    /// Reported for the smoke hooks, which read a height.
    readonly property real cap: box.restHeight + box.extra
    /// Offered exactly while the box is what stands between the reader and the rest of the text — and it keeps standing
    /// there once the text is out, because putting the box back is the same grip.
    readonly property bool grips:
        box.wants > box.height + Theme.borderWidth && (box.room > 0 || box.extra > 0) || box.extra > 0

    /// The one place the pull is recorded. The drag and the smoke hook both come through here, so neither can reach a
    /// height the other is refused. Stored as the offset from the pane's own allotment, so a commit whose summary
    /// needs another line moves the box with it, and the pull keeps meaning what the hand
    /// asked for.
    function setBoxHeight(want) {
        box.extra = Math.max(0, Math.min(want - box.restHeight, box.wants - box.restHeight, box.extra + box.room))
    }
    /// Where a pull leaves the ceiling, and what it asked for on the way. The drag and the smoke hooks all come through
    /// here, so the clamp and the refusal below are one answer.
    function pullTo(want) {
        box.unpin()
        box.askedHeight = want
        // Judged here, against the bound as it stood when the hand asked. Unlike a column divider's, this bound moves
        // under its own ask: it is `boxHeight + room`, and `room` is what the pane has left to lend, which the pane
        // only recomputes on the next layout. Read back afterwards it has grown by exactly what the box just took, so a
        // comparison made later always says the ask fitted.
        box.askedPast = want > box.ceiling + Theme.splitterWidth || want < box.restHeight - Theme.splitterWidth
        box.setBoxHeight(want)
    }
    /// Smoke hook: pull the grip down by dy, the way a drag does, and through the same clamp. Headless has no pointer
    /// at all.
    function grow(dy) {
        box.pullTo(box.height + dy)
    }
    /// Put the pull and the reading position back — called wherever the pane swaps in a different message (デザイン規約
    /// §コミットメッセージの 2 つの枠). Both belong to the message they were made on: a box held open for a long description would
    /// otherwise stand open over the short one after it.
    ///
    /// The caret goes first, and it is the half that matters. Assigning `text` leaves the caret at the end of what was
    /// assigned, and a `TextArea` keeps its caret in view by scrolling the flickable it sits in — so a message longer
    /// than the box opened at its **last** line, mid-sentence, every time (measured, on a 853-byte body:
    /// `contentY=420 cursor=853`). Putting the caret back to the top is what stops that; setting `contentY` alone does
    /// not, because the scroll happens later, when the text lays itself out.
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
    /// Held at the top until the reader moves it themselves. A single assignment is not enough: the scroll to the caret
    /// happens when the text lays itself out, which is after this returns, and how long after depends on how much text
    /// there is — one message came back to the top and the next one, measured in the same run, did not (`contentY=560`
    /// on a 994-byte body). Pinning states the intent.
    property bool pinnedTop: false
    /// Whatever the reader does to the text is the end of the pin: a wheel over it, a caret put in it, or a pull on its
    /// corner.
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
    /// Where the hand is, **in scene coordinates** — the badge is drawn a long way from here (`RepoPage`'s overlay),
    /// and scene is the one frame both ends already share.
    property point gripPoint: Qt.point(0, 0)
    /// Automation only: stands in for the press the hooks cannot make.
    property bool gripHeld: false
    readonly property bool gripDragging: grip.pressed || box.gripHeld
    /// Whether the last ask was past what the box could give. Written by `pullTo`, which is the only moment the bound
    /// is still the one the ask was measured against.
    property bool askedPast: false
    /// Whether a pull is asking for a height the box cannot be — either way, since a hand that has run out has run out
    /// whichever way it was going. The dragging half is a binding, so letting go ends the answer whatever the last ask
    /// was: nothing here can strand a badge.
    readonly property bool gripRefused: box.gripDragging && box.askedPast
    /// Automation: a pull carried past one of the two ends (`PGG_AUTO_ACT=divider-refuse`, cases `desc-max` /
    /// `desc-min`).
    function pullPast(down) {
        const over = 4 * Theme.splitterWidth
        box.gripHeld = true
        box.pullTo(down ? box.ceiling + over : box.restHeight - over)
        // Where the hand got to — read after the pull, and past the bound by what the pull asked for. A real drag
        // writes this every move, so it is always the hand; a hook that wrote it beforehand would leave the badge at
        // the corner the grip has since left (the picture then shows it stranded halfway up the box).
        //
        // Off `boxHeight`, which settles with the clamp — the grip is anchored and moves on the
        // next layout.
        box.gripPoint = box.mapToItem(null, box.width - grip.width / 2, box.boxHeight - grip.height / 2
        + (down ? over : -over))
    }
    // The badge belongs to the page. A pull that has run out has the hand somewhere outside this box — below it
    // for the ceiling, above it for the floor — and a badge parented to the box lands outside its own parent, where it
    // is composited with the box among the pane's children: under the author card, under the file list (observed).
    // The page draws the one badge in a layer over everything, and this only says where the hand is and whether it
    // is being refused (`RepoPage`).
    /// A wheel this box had nothing left to do with, in pixels. Whoever put the box on a surface that scrolls moves
    /// that surface by it.
    ///
    /// Without this the box is a hole in the pane behind it: a wheel over it is answered by text that will not move,
    /// and the box covers most of what it stands on, so the surface underneath cannot be reached by wheel at all.
    signal wheelPastEnd(real pixels)
    /// Escape was pressed while the caret was in here. What that costs is the text's, so whoever owns
    /// the text decides (`MessageEditor`).
    signal escaped()
    /// The wheel is taken here, because a flickable at its end keeps the event and says nothing.
    /// One notch is the same `wheelRows` every list in this application moves by, counted
    /// in lines.
    readonly property real wheelStep: Metrics.wheelRows * Theme.fontMdLine
    function rollBy(dy) {
        box.unpin()
        const flick = textView.contentItem
        const pixels = dy / 120 * box.wheelStep
        const max = Math.max(0, flick.contentHeight - flick.height)
        // **Measured from where the notch in flight is aiming**, not from where the text is right now: two notches in
        // a row would otherwise both measure from the same place and land as one (`WheelGlide.at`).
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
    /// Smoke hook: the caret in the box, the way a click puts it there. Called `takeCaret` -- Item already has a
    /// `focus` property, and the name `focus()` resolves to that one, so the call is a TypeError at the point it is
    /// made.
    function takeCaret() {
        box.unpin()
        area.forceActiveFocus()
    }
    /// The other way round: the caret leaves, and this box stops being the thing being written in (Escape).
    function dropCaret() {
        area.focus = false
    }
    // The pane takes its room back before its own column runs under its edge — the accident the shared cap
    // was put there to stop, and one no screenshot shows.
    onOwedChanged: {
        if (box.owed > 0)
            box.setBoxHeight(box.height - box.owed)
    }
    // TextEdit draws only the part of itself its viewport can see, and a viewport that grows without scrolling never
    // tells it so: the box opens and the text stops on the line the old height ended at, with the rest of the frame
    // empty (measured). Nudging the flickable and putting it straight back is what says "look again" -- the text item
    // is watching for the viewport to move under it, which is the one thing a resize does not do.
    onHeightChanged: Qt.callLater(box.repaint)
    function repaint() {
        const flick = textView.contentItem
        const was = flick.contentY
        flick.contentY = was + 1
        flick.contentY = was
    }

    // The pane lays this box out inside a block of one fixed height, and this half takes whatever the summary above it
    // did not (デザイン規約 §コミットメッセージの 2 つの枠). Filling is the whole point: the two boxes are
    // split by the layout from a single number, so no arithmetic here has to agree with any arithmetic there.
    Layout.fillWidth: true
    Layout.fillHeight: true
    Layout.minimumHeight: 0
    color: Theme.bgBase
    radius: Theme.radiusMd
    border.color: Theme.borderSubtle
    border.width: Theme.borderWidth

    // The band the inset below leaves between this frame and the words. Declared first, so it is under both the view
    // and the grip and reaches only a press neither of them took (`SweepBand`).
    SweepBand {
        anchors.fill: parent
        field: area
    }

    ScrollView {
        id: textView
        anchors.fill: parent
        // One value on all four sides, the bar's side included — no gutter is taken for it, so the words run under the
        // thumb the way the graph's messages do, and the text stands between two equal margins. **The right
        // included**: taken to the frame's own line the text reads as stuck to it, which is the answer the graph's
        // right edge already gives (by design — デザイン規約 §余白).
        anchors.margins: Theme.spaceXs
        // **Upright only.** The words wrap, so there is never anything to send that way — but the style
        // hands every `ScrollView` one regardless, and an `AsNeeded` bar with `size` at 1 is laid out and left
        // *visible*: a 10px strip across the foot of the viewport that takes every press landing on it. On a
        // one-line summary that is the whole lower half of the box, answering nothing (qmltestrunner measured,
        // `tst_messageband`: `ScrollBar[0,9 312x10] orient=1 policy=0 size=1.00` under a click that focused nothing).
        // The vertical one is `AutoScrollBar`, which is drawn only where it has somewhere to go.
        //
        // **Turned off.** Handed `null`, the style's own upright bar is left reading
        // `control.ScrollBar.horizontal.active` off nothing — one `TypeError` per box on every start (Fusion
        // `ScrollView.qml:22`). `AlwaysOff` is answered with `visible: false`, and an invisible bar takes no
        // press, which is all this line was ever for (`tst_messageband`).
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        // A bar handed to a `ScrollView` arrives half-wired: `size` and `position` track the text, but **no geometry
        // comes with it** — it is laid out at `x 0 y 0` at its own implicit 10x10, which draws a dot in the box's top
        // corner (qmltestrunner measured, ; the same bar on a plain `Flickable` gets `x 267 h 112` for free). So
        // the three numbers are written here, and the flickable it answers for is named (`AutoScrollBar.view`), since
        // the bar is parented to the view and the flickable sits inside it. `MessageEditor` puts the same block
        // over the summary box.
        ScrollBar.vertical: AutoScrollBar {
            id: textBar
            view: textView.contentItem
            x: textView.width - width
            y: textView.topPadding
            height: textView.availableHeight
        }
        // ScrollView keeps its Flickable private -- reach it once it exists.
        //
        // `interactive` goes with the hard stop: the flickable answers the wheel itself as well as letting the handler
        // above see it, so the two moved the text twice — the handler's jump, and then the flickable's own animation
        // settling somewhere else, which reads as the text going up and being dragged back. Off,
        // `rollBy` is the only thing that moves this text, and dragging inside a text box means selecting it anyway.
        Component.onCompleted: {
            contentItem.boundsBehavior = Flickable.StopAtBounds
            contentItem.interactive = false
        }
        TextArea {
            id: area
            wrapMode: TextArea.Wrap
            font.pixelSize: Theme.fontMd
            // Escape leaves the box, and whoever owns the text decides what that means (`MessageEditor.escaped`).
            Keys.onEscapePressed: event => {
                box.escaped()
                event.accepted = true
            }
            // Dimmer than the summary while it is being read -- that pair is the message's own hierarchy -- and lit
            // while it is being written: text under a caret is what the eye is on, and secondary is the shade this
            // theme spends on what the eye is not on. Read-only does not count as writing it: a stash and a commit off
            // this line take a caret for selecting, and nothing typed there would land.
            color: !area.readOnly && area.activeFocus ? Theme.textPrimary : Theme.textSecondary
            background: null
            padding: 0
            // A caret put in the text by hand is the reader taking the box over, so the top stops being held for them
            // (`pinnedTop`).
            onActiveFocusChanged: if (area.activeFocus) box.unpin()
            // On the text: a handler here is offered the wheel before the flickable under
            // it decides to keep it, which is the whole point (`rollBy`).
            WheelHandler {
                acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                onWheel: event => box.rollBy(event.angleDelta.y)
            }
        }
    }
    // Declared after the ScrollView, so the corner belongs to the grip. Hit area and ink are the one 16 square: the
    // mark hangs off its lower-right, which puts the ink on the
    // same inset the text keeps from the frame.
    MouseArea {
        id: grip
        visible: box.grips
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: Theme.iconMd
        height: Theme.iconMd
        hoverEnabled: true
        // Down is the only way this goes, so the cursor says so. A browser's corner drag is a different
        // promise.
        cursorShape: Qt.SizeVerCursor
        /// Where the pull started, in the pane's own coordinates — the grip itself travels with the edge it is moving.
        property real fromY: 0
        property real fromHeight: 0
        onPressed: mouse => {
            grip.fromY = mapToItem(box.parent, 0, mouse.y).y
            grip.fromHeight = box.boxHeight
            // A grab is not yet an ask, and the last drag's answer is not this one's. Forgetting this shows the badge a
            // frame early on the next grab, never longer — the dragging half still ends it.
            box.askedPast = false
        }
        onPositionChanged: mouse => {
            box.gripPoint = grip.mapToItem(null, mouse.x, mouse.y)
            if (!grip.pressed)
                return
            box.pullTo(grip.fromHeight + mapToItem(box.parent, 0, mouse.y).y - grip.fromY)
        }
        NavIcon {
            anchors.fill: parent
            kind: "grip"
            // A structural line at rest, since what it marks is an edge; the pointer brings it up to the
            // shade the pane spends on things being read.
            tint: grip.containsMouse || grip.pressed ? Theme.textSecondary : Theme.borderStrong
        }
    }
}

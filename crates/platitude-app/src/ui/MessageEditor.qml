import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit message pair: a summary box over a dimmer description box, one block of one height
// (デザイン規約 §コミットメッセージの 2 つの枠). The commit editor and the details pane both use it, so they cannot drift.
//
// The pair's height is one number the layout splits: the description fills what the summary leaves, so the total is
// exact whatever the line height rounds to.
//
// What stands around the pair is the pane's and comes in as numbers (`listHeight` / `blockRoom` / `blockHeight`).
// The readouts go back out under the panes' names — automation reads the panes, and the panes forward.
ColumnLayout {
    id: editor

    // ---- what the pane says about the boxes -------------------------
    /// Whether the boxes refuse typing (the details pane on a commit that cannot be rewritten from here).
    property bool readOnly: false
    /// Why they refuse, in one line ("" when they do not).
    property string blockedTip: ""
    /// Stands in for the pointer on the summary box — hover cannot be injected.
    property bool summaryPointedAt: false
    /// What the empty boxes say when a message stands behind them (a stopped merge): leaving them empty commits
    /// exactly that (デザイン規約 §進行中の操作から出る). Empty falls back to the boxes' own names.
    property string standingSubject: ""
    property string standingBody: ""

    // ---- the mark a reader arrives at -------------------------------
    /// A reader was sent here from the graph row's hover card (デザイン規約 §hover のツールチップ); the frames say so
    /// until their next press. Only the pane that is sent to sets it.
    property bool attention: false
    function callAttention() { editor.attention = true }
    function dropAttention() { editor.attention = false }

    // ---- what the pane says about its own geometry ------------------
    /// The pane's file list height — the one thing under the block that gives. What it has past two rows is all this
    /// pair may borrow (デザイン規約 §コミットメッセージの 2 つの枠).
    property real listHeight: 0
    /// How much of the pane the block holding this pair may take — the pane's to say; the two panes differ.
    property real blockRoom: 0
    /// What that block asks for (its column's implicitHeight); `descOwed` says why it is held against `blockRoom`.
    property real blockHeight: 0

    // ---- what the pair holds ----------------------------------------
    readonly property string subjectText: summaryArea.text
    readonly property string bodyText: descBox.text
    /// The read-only tooltip is up (the output side).
    readonly property bool summaryTipShown: summaryArea.ToolTip.visible
    /// What the description box paints (the output side).
    readonly property color descriptionColor: descBox.textColor
    readonly property bool descriptionFocused: descBox.focused
    /// A caret is in either box — when the pane keeps the row that saves on screen (規約 §コミットメッセージの 2 つの枠).
    readonly property bool anyFocused: summaryArea.caretHeld || descBox.focused

    // -- the message pair is one block of one height --
    /// What the summary box needs for its own lines, capped: nothing in git bounds a summary, and an uncapped box
    /// pushes the description off the pane. Ceiled, so a box that fits its text shows no scroll bar.
    readonly property real summaryNeed:
        Math.min(Math.ceil(summaryArea.implicitHeight) + Theme.spaceSm,
                 Theme.messageMaxHeight)
    /// The description gives up its lines to a wrapping summary down to two, and no further.
    readonly property real descFloor: 2 * Theme.fontMdLine + Theme.spaceSm
    /// At rest: one summary line and five description lines, with the gap between them.
    readonly property real pairRest:
        Theme.fontLgLine + Theme.spaceSm + Theme.spaceXs + Theme.messageMaxHeight
    /// What the pair is laid out at: once the description is at its floor, the block grows, up to the summary's cap.
    readonly property real pairBase:
        Math.max(editor.pairRest,
                 editor.summaryNeed + Theme.spaceXs + editor.descFloor)
    readonly property real descRest:
        editor.pairBase - editor.summaryNeed - Theme.spaceXs

    // -- what the pane lends the description box --
    // The box carries the pull and the grip (`DescriptionBox`); the bound is the pane's (`listHeight`).
    readonly property real descRoom:
        Math.max(0, editor.listHeight - 2 * Theme.rowHeight)
    /// How far past the bound the pane already is — what the hand has to give back. The second term is the block's
    /// overflow past the pane's edge: the list stops answering at zero, and without it the give-back stalls short.
    ///
    /// Held against the room the block is *allowed* (`blockRoom`), not a laid-out height: that height follows the
    /// box, so comparing with it is a ring — the box is told it owes last frame's growth and gives it all back.
    readonly property real descOwed:
        Math.max(0, 2 * Theme.rowHeight - editor.listHeight)
        + Math.max(0, editor.blockHeight - editor.blockRoom)
    /// Whether the grip is refusing a pull, and where the hand is while it does (scene coordinates). The page draws the
    /// badge (`RepoPage.refusalSource`).
    readonly property alias descRefuses: descBox.gripRefused
    readonly property alias descPoint: descBox.gripPoint
    readonly property bool descGrips: descBox.grips
    readonly property real descHeight: descBox.boxHeight
    readonly property real descWants: descBox.wants
    readonly property real descCap: descBox.cap
    /// How many rows the file list is left with — the bound the pull stops at. Rounded: the layout hands out fractions.
    readonly property int descListRows:
        Math.round(editor.listHeight / Theme.rowHeight)
    /// Whether the block is taller than the room it was given — which is also "has the box given back what it owes".
    /// A readout: a headless run cannot see a scroll bar (`PGG_AUTO_ACT=window-floor wip`).
    readonly property bool blockScrolls:
        editor.blockHeight > editor.blockRoom + 1
    readonly property bool descKeeps: !editor.blockScrolls
    /// What the pane lays the pair out at: the block plus the grip's pull. Bind it to `Layout.preferredHeight` on the
    /// instance — the layout's own implicitHeight is the engine's: a binding there loses to the next recompute, and the
    /// description box collapses to its 0 implicit height.
    readonly property real pairHeight: editor.pairBase + descBox.extra

    /// A wheel neither box could use, in pixels — whoever owns the block moves it by this, or the block under the
    /// boxes cannot be wheeled at all.
    signal wheelPastEnd(real pixels)

    /// Swap in a different message: it opens at its rest height and first line — a pull and a reading position belong
    /// to the message they were made on (`DescriptionBox.resetForNewMessage`).
    function setMessage(subject, description) {
        summaryArea.text = subject
        descBox.text = description
        summaryArea.cursorPosition = 0
        editor.summaryPinned = true
        editor.summaryToTop()
        descBox.resetForNewMessage()
    }
    /// The summary held at its first line until the reader moves it (`DescriptionBox.pinnedTop`, same reason): the
    /// caret a new `text` leaves at the end scrolls a long subject to its last line, and a one-off reset loses to a
    /// late layout.
    property bool summaryPinned: false
    function unpinSummary() {
        editor.summaryPinned = false
    }
    Connections {
        target: summaryView.contentItem
        enabled: editor.summaryPinned
        function onContentYChanged() {
            if (editor.summaryPinned && summaryView.contentItem.contentY !== 0)
                summaryView.contentItem.contentY = 0
        }
        function onContentHeightChanged() {
            if (editor.summaryPinned)
                summaryView.contentItem.contentY = 0
        }
    }
    /// Write the texts alone (a revert, a smoke hook): the caret, the scroll and the pull stay put.
    function setTexts(subject, description) {
        summaryArea.text = subject
        descBox.text = description
    }
    /// Escape in one of the boxes: the caret goes, so the row that saves comes down. The text is the owner's to keep
    /// or drop — the details pane drops its draft, the commit editor keeps it (デザイン規約 §コミットメッセージの 2 つの枠).
    signal escaped()
    function leaveBoxes() {
        summaryArea.focus = false
        descBox.dropCaret()
        editor.escaped()
    }
    function summaryToTop() {
        if (summaryView.contentItem)
            summaryView.contentItem.contentY = 0
    }
    /// The summary's wheel, in `DescriptionBox.rollBy`'s two steps: its own text while there is text to move, then
    /// the block.
    function rollSummary(dy) {
        // A wheel is the reader taking over from the pin.
        editor.unpinSummary()
        const flick = summaryView.contentItem
        const pixels = dy / 120 * (Metrics.wheelRows * Theme.fontMdLine)
        const max = Math.max(0, flick.contentHeight - flick.height)
        // Measured from where the notch in flight is aiming (`DescriptionBox.rollBy`, same reason).
        const from = summaryGlide.at
        const next = Math.max(0, Math.min(max, from - pixels))
        if (Math.abs(next - from) > 0.5) {
            summaryGlide.sendTo(next)
            return
        }
        editor.wheelPastEnd(pixels)
    }
    WheelGlide {
        id: summaryGlide
        view: summaryView.contentItem
    }
    /// Automation only: the boxes' middle-button hands, driven without a pointer.
    readonly property alias summaryHand: summaryHand
    /// Automation: the summary box itself, for what a hand does to it before a right-click (its text and selection).
    readonly property alias summaryBox: summaryArea
    readonly property alias descriptionHand: descBox.hand
    /// The middle button's drift over the summary: the words only (`DescriptionBox.driftText`).
    function driftSummary(dy) {
        editor.unpinSummary()
        summaryGlide.halt()
        const flick = summaryView.contentItem
        flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, flick.contentY + dy))
    }
    /// Smoke hook: the caret in the description box, the way a click in it puts it there.
    function focusDescription() {
        descBox.takeCaret()
    }
    // -- smoke hooks, forwarded to the box --
    function growDescription(dy) { descBox.grow(dy) }
    function pullDescriptionPast(down) { descBox.pullPast(down) }
    /// Smoke hook: the wheel over the description box, through the one door a notch comes in by
    /// (`DescriptionBox.rollBy`).
    function rollDescription(dy) { descBox.rollBy(dy) }
    /// Smoke hook: the hand inside the box, which keeps its bar bright — hover is not injectable
    /// (`AutoScrollBar.inArea`).
    function holdDescriptionBar(on) { descBox.bar.inArea = on }
    /// How much ink is on the box's own bar, and how far the text stands.
    readonly property real descriptionBarInk: descBox.bar.opacity
    readonly property real descriptionAt: descBox.textAt
    readonly property real summaryAt: summaryView.contentItem ? summaryView.contentItem.contentY : 0

    spacing: Theme.spaceXs

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: editor.summaryNeed
        color: Theme.bgBase
        radius: Theme.radiusMd
        border.color: editor.attention ? Theme.accent : Theme.borderDefault
        border.width: Theme.borderWidth
        // The band the inset leaves between frame and words. Declared first, so it is under the view and gets only a
        // press the text did not take (`SweepBand`).
        SweepBand {
            anchors.fill: parent
            field: summaryArea
        }
        ScrollView {
            id: summaryView
            anchors.fill: parent
            anchors.margins: Theme.spaceXs
            // The same inset, own bar and no sideways bar as `DescriptionBox`, whose reasons are there.
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AutoScrollBar {
                id: summaryBar
                view: summaryView.contentItem
                x: summaryView.width - width
                y: summaryView.topPadding
                height: summaryView.availableHeight
                stepGlide: summaryGlide
                onHeldChanged: if (summaryBar.held) editor.unpinSummary()
            }
            // ScrollView keeps its Flickable private -- reach it once it exists. Not interactive: answering the same
            // wheel as the handler moves the text twice.
            Component.onCompleted: {
                contentItem.boundsBehavior = Flickable.StopAtBounds
                contentItem.interactive = false
            }
            SummaryArea {
                id: summaryArea
                readOnly: editor.readOnly
                placeholderText: editor.readOnly ? ""
                                 : editor.standingSubject !== "" ? editor.standingSubject
                                 : qsTr("Commit summary")
                // A caret in the box also takes it over from the pin.
                onActiveFocusChanged: if (summaryArea.activeFocus) editor.unpinSummary()
                Keys.onEscapePressed: event => {
                    editor.leaveBoxes()
                    event.accepted = true
                }
                ToolTip.visible: (hovered || editor.summaryPointedAt) && editor.blockedTip !== ""
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: editor.blockedTip
                WheelHandler {
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onWheel: event => editor.rollSummary(event.angleDelta.y)
                }
            }
        }
        // The middle button's hand over the summary, on the terms `DescriptionBox` gives its own.
        MiddleAutoScroll {
            id: summaryHand
            anchors.fill: summaryView
            visible: summaryBar.visible
            onDrifted: dy => editor.driftSummary(dy)
        }
    }
    // Always shown, even empty, and taking whatever the summary above it left of the block.
    DescriptionBox {
        id: descBox
        readOnly: editor.readOnly
        placeholderText: editor.readOnly ? ""
                         : editor.standingBody !== "" ? editor.standingBody
                         : qsTr("Description")
        border.color: editor.attention ? Theme.accent : Theme.borderSubtle
        restHeight: editor.descRest
        room: editor.descRoom
        owed: editor.descOwed
        onWheelPastEnd: pixels => editor.wheelPastEnd(pixels)
        onEscaped: editor.leaveBoxes()
    }
}

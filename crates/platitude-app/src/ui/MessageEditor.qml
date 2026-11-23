import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit message pair: a summary box over a dimmer description box, one block of one height. The commit editor and
// the details pane both use it, so the four boxes keep one rule and cannot drift. The summary is `textPrimary`; the
// description `textSecondary` at rest and `textPrimary` only while written in (`DescriptionBox`) — the step is the
// message's own hierarchy.
//
// **One block of one height**: at rest one summary line over five description lines (`Theme.messageMaxHeight`), and
// while the description has empty room to give that total does not move with either text — a summary that wraps takes
// its lines from that empty room, so the CHANGES band, the file list and the commit button beside the pair stay put.
// Written words are not cut for it: a summary that would cut them grows the block into the room the list spares, as
// the version corner and empty rows are lower than the words (`descWordRoom`). A pane too short for the whole block
// takes what it lacks from the description, so the buttons stay on screen (`descShare`). The description gives way
// down to its floor (`descFloor` — where its text counts: an empty box gives more than one with words in it); past
// that the block itself grows, until the summary's own cap (`messageMaxHeight`) stops it. The pair's height is one
// number the layout splits: the description fills what the summary leaves (`fillHeight`). Two heights computed apart
// and meant to add up would break the sum quietly on a line height's fraction, a layout's rounding or a clamp.
//
// A new message always opens from its first line: a text that overflows leaves the caret at its end and the box scrolls
// there, so it would start mid-text with nobody having moved it. Held at the top until the reader moves it — a wheel,
// a caret in the box, the grip (`setMessage`, `DescriptionBox.pinnedTop`).
//
// The `Co-authored-by:` lines stay inside the description: the box is the amend's editor, and taken out of it the box
// and the saved message would disagree, with no way left to remove or fix a co-author.
//
// What stands around the pair is the pane's and comes in as numbers (`listHeight` / `listFloor` / `blockRoom` /
// `blockHeight` / `besidePair`).
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
    /// A reader was sent here from the graph row's hover card (デザイン規約 §hover のツールチップ「送った先は名乗る」); the
    /// frames say so until their next press. Only the pane that is sent to sets it. Only the frames' colour changes, to
    /// `accent`, on both boxes at once so the message stands as one target; the ground stays — lifted, it would be a
    /// second selection-blue in the right pane (user decisions — ask before changing). Down at the next press outside a
    /// card or menu (a hand still reading one is not done) and at Escape, once nothing above it stands
    /// (`RepoPage.escapePressed`).
    property bool attention: false
    function callAttention() { editor.attention = true }
    function dropAttention() { editor.attention = false }

    // ---- what the pane says about its own geometry ------------------
    /// The pane's file list height — the one thing under the block that gives. What it has past `listFloor` is all this
    /// pair may borrow (`DescriptionBox`'s grip).
    property real listHeight: 0
    /// How short that list may go, in `listHeight`'s terms — the pane's to say (`WipPane.listFloor`,
    /// `DetailsPane.listFloor`).
    property real listFloor: 2 * Theme.rowHeight
    /// How much of the pane the block holding this pair may take — the pane's to say; the two panes differ. Unsaid,
    /// the pair stands as in a pane with room.
    property real blockRoom: Infinity
    /// What that block asks for (its column's implicitHeight); `descOwed` says why it is held against `blockRoom`.
    property real blockHeight: 0
    /// What stands in the block beside the pair — its margins, its steps and the rows over and under the pair — read
    /// off those rows by the block (`WipCommitBlock.besidePair`, `DetailsMessageBlock.besidePair`): the column's own
    /// height carries the pair's, and reading it here would ring.
    property real besidePair: 0
    /// How tall the pair may stand before the block scrolls.
    readonly property real pairRoom: editor.blockRoom - editor.besidePair

    // ---- what the pair holds ----------------------------------------
    readonly property string subjectText: summaryArea.text
    readonly property string bodyText: descBox.text
    /// The read-only tooltip is up (the output side).
    readonly property bool summaryTipShown: summaryTip.shown
    /// What the description box paints (the output side).
    readonly property color descriptionColor: descBox.textColor
    readonly property bool descriptionFocused: descBox.focused
    /// A caret is in either box — when the pane keeps the row that saves on screen (`MessageActionsRow`).
    readonly property bool anyFocused: summaryArea.caretHeld || descBox.focused

    // -- the message pair is one block of one height --
    /// What the summary box needs for its own lines, capped: nothing in git bounds a summary, and an uncapped box
    /// pushes the description off the pane. Ceiled, so a box that fits its text shows no scroll bar.
    readonly property real summaryNeed:
        Math.min(Math.ceil(summaryArea.implicitHeight) + Theme.spaceSm,
                 Theme.messageMaxHeight)
    /// The least the description stands at, to a short pane or a wrapping summary: empty, one line — its name or the
    /// standing message's first; with words in it, a line and the top half of the next, enough to read that more lines
    /// follow (its own lines where it has fewer). Written words hold their room longer than an empty box does (user
    /// decisions — ask before changing).
    readonly property real descFloor: descBox.text === ""
        ? Theme.fontMdLine + Theme.spaceSm
        : Math.min(1.5 * Theme.fontMdLine + Theme.spaceSm, descBox.wants)
    /// The description's share at rest: five lines, less only what the block lacks once the list is down to its floor
    /// — the description gives way to keep the block's buttons on screen, and nothing to a list that has room already;
    /// the block scrolls only once the description is at its floor (user decision — ask before changing).
    readonly property real descShare:
        Math.max(editor.descFloor, Math.min(Theme.messageMaxHeight,
                                            editor.pairRoom - Theme.fontLgLine - Theme.spaceSm - Theme.spaceXs))
    /// At rest: one summary line and the description's share, with the gap between them.
    readonly property real pairRest:
        Theme.fontLgLine + Theme.spaceSm + Theme.spaceXs + editor.descShare
    /// What a wrapping summary may not take from the description: its words — their own lines, up to its share (an
    /// empty box, its floor). The summary takes the box's empty room and the band stays; past that the block grows into
    /// the list's spare room, as the version corner and the empty rows are lower than written words, down to the list's
    /// floor — only there do the words give way, to `descFloor` (user decision — ask before changing).
    readonly property real descWordRoom: descBox.text === ""
        ? editor.descFloor
        : Math.max(editor.descFloor, Math.min(editor.descShare, descBox.wants))
    /// What the pair is laid out at: the rest height while the summary takes only the description's empty room; past
    /// that the summary over the description's words, as far as the pane has room; never less than the summary over the
    /// description's floor, up to the summary's cap.
    readonly property real pairBase:
        Math.max(editor.pairRest,
                 editor.summaryNeed + Theme.spaceXs + editor.descFloor,
                 Math.min(editor.summaryNeed + Theme.spaceXs + editor.descWordRoom, editor.pairRoom))
    readonly property real descRest:
        editor.pairBase - editor.summaryNeed - Theme.spaceXs

    // -- what the pane lends the description box --
    // The box carries the pull and the grip (`DescriptionBox`); the bound is the pane's (`listHeight`).
    readonly property real descRoom:
        Math.max(0, editor.listHeight - editor.listFloor)
    /// How far past the bound the pane already is — what the hand has to give back. The second term is the block's
    /// overflow past the pane's edge: the list stops answering at zero, and without it the give-back stalls short.
    ///
    /// Held against the room the block is *allowed* (`blockRoom`), not a laid-out height: that height follows the
    /// box, so comparing with it is a ring — the box is told it owes last frame's growth and gives it all back.
    readonly property real descOwed:
        Math.max(0, editor.listFloor - editor.listHeight)
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
    /// A readout: a headless run cannot see a scroll bar (`_PGG_AUTO_ACT=window-floor wip`).
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
    /// or drop — the details pane drops its draft (`DetailsPane.syncMessage`), the commit editor keeps it
    /// (`WipCommitBlock`).
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
        const from = summarySend.at
        const next = Math.max(0, Math.min(max, from - pixels))
        if (Math.abs(next - from) > 0.5) {
            summarySend.glideTo(next)
            return
        }
        editor.wheelPastEnd(pixels)
    }
    readonly property ScrollSend summarySend: ScrollSend {
        id: summarySend
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
        summarySend.halt()
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
                stepSend: summarySend
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
                placeholder: editor.readOnly ? ""
                             : editor.standingSubject !== "" ? editor.standingSubject
                             : qsTr("Commit summary")
                // A caret in the box also takes it over from the pin.
                onActiveFocusChanged: if (summaryArea.activeFocus) editor.unpinSummary()
                Keys.onEscapePressed: event => {
                    editor.leaveBoxes()
                    event.accepted = true
                }
                readonly property TipAsk summaryTip: TipAsk {
                    id: summaryTip
                    target: summaryArea
                    asked: (summaryArea.hovered || editor.summaryPointedAt) && editor.blockedTip !== ""
                    words: editor.blockedTip
                }
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
        placeholder: editor.readOnly ? ""
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

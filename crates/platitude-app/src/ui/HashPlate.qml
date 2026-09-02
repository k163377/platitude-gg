import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit's own hash over its parent's, rows aligned right.
//
// **Both rows are a control and a value at once, and one gesture tells them apart**: a press that lets go where it
// landed is the control's — the whole plate copies the hash, the whole link goes to the parent — and a press that
// travels is the field's, so the digits are dragged over like any other text in this window (デザイン規約
// §右のペインの字は掴める). Neither the face nor the target moves: the hand is laid over the control's own whole face,
// so what is pressable is what it always was.
//
// **The hand is what the face answers with, not the field under it.** A `TextEdit` takes the press wherever it is
// drawn, `selectByMouse: false` and all, and the control above it then never hears a click at all (qmltestrunner
// `tst_hashplate`) — so the words are given no press of their own (`grabbable: false`) and the hand hands the drag
// down to them. The room around the plate reaches the same fields from the row's gaps (`SweepRoom`).
ColumnLayout {
    id: plate

    property string sha8: ""
    property string fullSha: ""
    property string parentSha: ""

    signal copyRequested(string text)
    signal parentClicked(string oidHex)

    /// The words a sweep may put a selection into, and the boxes a sweep must keep its hands off — the two are the
    /// same rows seen from either side (`SweepRoom`).
    readonly property var valueFields: [shaText, parentText]
    /// Whether a point in another item's coordinates lands on a control of this plate. A press there belongs to the
    /// control, whole: this is what keeps the hit areas the size they have always been.
    function claims(item, x, y) {
        return plate.inside(hashCopy, item, x, y) || (parentLink.visible && plate.inside(parentLink, item, x, y))
    }
    function inside(box, item, x, y) {
        const p = box.mapFromItem(item, x, y)
        return p.x >= 0 && p.y >= 0 && p.x < box.width && p.y < box.height
    }
    /// Where this plate begins, for whoever is looking for room beside it.
    function leftEdge(item) {
        return plate.mapToItem(item, 0, 0).x
    }
    function fieldFor(which) {
        return which === "parent" ? parentText : shaText
    }
    /// The middle of the control that owns that row's presses, in another item's coordinates — what a run aims at to
    /// ask whether the press there is still the control's.
    function controlPoint(which, item) {
        const box = which === "parent" ? parentLink : hashCopy
        return box.mapToItem(item, box.width / 2, box.height / 2)
    }

    /// What the two controls' handlers call, and the only way in (verify-ui §壊れない動詞の実装).
    function copyFullNow() {
        plate.copyRequested(plate.fullSha)
    }
    function goToParentNow() {
        if (plate.parentSha === "")
            return
        plate.parentClicked(plate.parentSha)
    }

    function selectSha() { shaText.selectAll() }
    function selectParent() { parentText.selectAll() }
    /// Nothing on this plate is holding a selection any more. **Both rows, from either of them**: the plate is one
    /// thing, and a press on it is where its gestures start (規約 §右のペインの字は掴める — 選択は窓に 1 つ).
    function dropValues() {
        shaText.deselect()
        parentText.deselect()
    }
    readonly property alias shaSelected: shaText.selected
    readonly property alias parentSelected: parentText.selected

    /// Automation: the two gestures as a hand makes them, entering the same three functions the hands below call — a
    /// pointer cannot be injected (verify-ui §壊れない動詞の実装). The tap presses and lets go on the middle of the
    /// control's face; the drag starts at the head of the value and runs off its far end.
    function tapAt(which) {
        const hand = plate.handFor(which)
        const at = plate.controlPoint(which, hand)
        hand.takeAt(at.x, at.y)
        hand.releaseNow()
    }
    function dragAcross(which) {
        const hand = plate.handFor(which)
        const field = plate.fieldFor(which)
        const near = field.mapToItem(hand, 0, field.height / 2)
        const far = field.mapToItem(hand, field.width, field.height / 2)
        hand.takeAt(near.x, near.y)
        hand.followAt(far.x, far.y)
        hand.releaseNow()
    }
    /// Automation: and that there is a hand at all — a run enters the functions above rather than the pointer, so a
    /// hand taken out, disabled or shrunk would answer every gesture it was asked and never see a press.
    function handStands(which) {
        const hand = plate.handFor(which)
        const face = which === "parent" ? parentLink : hashCopy
        return hand.enabled && hand.width === face.width && hand.height === face.height
    }
    function handFor(which) {
        return which === "parent" ? parentHand : hashHand
    }

    /// The hand a row that is both a control and a value answers with. **The press is the control's until it travels**
    /// — past that it is a drag, and a drag belongs to the words (デザイン規約 §右のペインの字は掴める). The distance
    /// is the platform's own (`drag.threshold`), and the selection is anchored where the press landed rather than
    /// where the threshold was crossed, so it begins under the finger.
    ///
    /// **A plain `MouseArea` and not a handler**, for the pair already measured for the row beneath this plate
    /// (`SweepRoom`): a passive `PointHandler` answers one move of a two-move drag inside the pane's Flickable, and a
    /// `TapHandler` never taps at all over a selectable field. `preventStealing` is what keeps that Flickable from
    /// taking the drag away part-way through.
    component RowHand: MouseArea {
        id: hand

        /// The words this row draws, and what a press that never travelled does.
        required property LineText field
        signal tapped()
        /// Raised where the press lands, before anything has been decided about it: whoever owns the values clears
        /// the board, so no gesture runs on top of what the last one left picked out.
        signal taken()

        /// Where the press landed, and whether it has since travelled far enough to be a drag.
        property real fromX: 0
        property real fromY: 0
        property bool dragging: false

        /// The three the handlers below call, in this hand's own coordinates — and the three a run enters, so a hand
        /// that was never wired up reports nothing (verify-ui §壊れない動詞の実装).
        function takeAt(x, y) {
            hand.fromX = x
            hand.fromY = y
            hand.dragging = false
            // **The press lets the last gesture's selection go**, whichever this one turns out to be — the same thing
            // a press in any other text does, and what keeps a copy from being taken under words still washed by the
            // drag before it.
            hand.taken()
        }
        function followAt(x, y) {
            if (!hand.dragging) {
                if (Math.abs(x - hand.fromX) < hand.drag.threshold
                        && Math.abs(y - hand.fromY) < hand.drag.threshold)
                    return
                hand.dragging = true
                // The caret comes with the anchor, which is also what takes the selection off whatever field was
                // holding one — there is one selection in this window (`LineText`).
                hand.field.anchorFrom(hand, hand.fromX, hand.fromY)
            }
            hand.field.extendFrom(hand, x, y)
        }
        function releaseNow() {
            const travelled = hand.dragging
            hand.dragging = false
            if (!travelled)
                hand.tapped()
        }

        anchors.fill: parent
        preventStealing: true
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => hand.takeAt(mouse.x, mouse.y)
        onPositionChanged: mouse => { if (hand.pressed) hand.followAt(mouse.x, mouse.y) }
        onReleased: hand.releaseNow()
        onCanceled: hand.dragging = false
    }

    spacing: 0

    // The hash is the button, not just the icon beside it — a 16px glyph was too small to aim at. Hovering underlines
    // the hash and lights the icon so the whole plate reads as one control. Not a HoverToolButton: the style's panel
    // would make the plate taller than one line and drop this hash out of step with the author name beside it, so it
    // draws the same wash over its own flat face.
    ToolButton {
        id: hashCopy
        Layout.alignment: Qt.AlignRight
        hoverEnabled: true
        leftPadding: Theme.spaceXs
        // Nothing on this side. The mark's ink is seated on the plate's own right edge below (`copyMark`), which is
        // where the parent hash ends and where the message box's frame under it stands — a padding here would hold the
        // one row of the pane's right column short of the line every other row is cut to.
        rightPadding: 0
        topPadding: 0
        bottomPadding: 0
        readonly property bool lit: hovered || visualFocus
        // The pointer's press is the hand's below, so the face has to be told when it is being pressed — and told to
        // stop once the press has travelled, because from there the gesture is the words'. The keyboard's own press
        // rides along: `Space` never travels.
        down: hashCopy.pressed || (hashHand.containsPress && !hashHand.dragging)
        ToolTip.visible: hovered
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: qsTr("Copy full hash")
        // The keyboard's way in. The pointer's is the hand below, which is the only one that can tell a click from a
        // drag.
        onClicked: plate.copyFullNow()
        background: Rectangle {
            radius: Theme.radiusSm
            color: hashCopy.down ? Theme.bgPressed : hashCopy.lit ? Theme.bgHover : "transparent"
            // Drawn here rather than as the label's font underline so the rule runs under the icon too — the hash and
            // the icon are one target, so they get one line.
            Rectangle {
                visible: hashCopy.lit
                color: Theme.textPrimary
                height: Theme.borderWidth
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: hashCopy.leftPadding
                anchors.rightMargin: hashCopy.rightPadding
            }
        }
        contentItem: RowLayout {
            spacing: Theme.spaceXs
            LineText {
                id: shaText
                text: plate.sha8
                mono: true
                pixelSize: Theme.fontMd
                color: Theme.textPrimary
                // The button owns every press on this face. The field is here to be written into, not to be pressed
                // (デザイン規約 §右のペインの字は掴める).
                grabbable: false
                Layout.alignment: Qt.AlignVCenter
            }
            // A seat drawn to the mark's ink rather than its box: the two squares fill nine of the sixteen, so the box
            // holds air either side of them and that air is what the eye measures (デザイン規約 §余白). Unseated it was
            // spent twice — once doubling the step after the hash, and once holding the mark short of the right edge
            // the parent hash under it is cut to.
            Item {
                Layout.preferredWidth: copyMark.inkWidth
                Layout.preferredHeight: Theme.iconMd
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: copyMark
                    anchors.centerIn: parent
                    kind: "copyicon"
                    tint: hashCopy.lit ? Theme.textPrimary : Theme.textSecondary
                }
            }
        }
        // Declared after the content so it lies over it: the words below take their own press wherever they are drawn,
        // and this face answers as one target or not at all.
        RowHand {
            id: hashHand
            field: shaText
            onTaken: plate.dropValues()
            onTapped: plate.copyFullNow()
        }
    }
    Item {
        id: parentLink
        visible: plate.parentSha !== ""
        Layout.alignment: Qt.AlignRight
        implicitWidth: parentRow.implicitWidth
        implicitHeight: parentRow.implicitHeight
        ToolTip.visible: parentHand.containsMouse
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: qsTr("Go to parent commit")
        RowLayout {
            id: parentRow
            anchors.fill: parent
            spacing: Theme.spaceXs
            // The mark is drawn, not typed. The fonts disagree about `←`: Cascadia Mono holds it in one cell (7px of
            // ink) where Noto Sans Mono CJK JP gives it a full-width one (12px), so Ubuntu grew a tail nobody chose —
            // the same way `⚑` came out a different shape on each of the three. The seat is the mark's ink, not its
            // box, so the `spaceXs` beside it lands where the eye measures it (規約 §余白).
            Item {
                Layout.preferredWidth: parentBack.inkWidth
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: parentBack
                    anchors.centerIn: parent
                    kind: "arrow"
                    // The family draws it leaving; this one points back, and turning the mark is how `FoldBlock` faces
                    // its chevrons too.
                    rotation: 180
                    tint: Theme.textLink
                    width: Theme.iconSm
                    height: Theme.iconSm
                    // The grid shrinks and the line shrinks with it, or the mark carries more weight than the digits
                    // beside it (§語の隣に立つ印).
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                }
            }
            LineText {
                id: parentText
                text: plate.parentSha.substring(0, 8)
                mono: true
                pixelSize: Theme.fontSm
                color: Theme.textLink
                grabbable: false
                Layout.alignment: Qt.AlignVCenter
            }
        }
        // One line under the mark and the hash: they are one target, the way the hash and its copy icon share theirs.
        Rectangle {
            visible: parentHand.containsMouse
            color: Theme.textLink
            height: Theme.borderWidth
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
        }
        // Declared last so it lies over the row: the words below take their own press wherever they are drawn, and
        // this link answers as one target or not at all. It is also what reports the pointer for the mark and the
        // rule above — nothing else on this row asks for hover.
        RowHand {
            id: parentHand
            field: parentText
            hoverEnabled: true
            onTaken: plate.dropValues()
            onTapped: plate.goToParentNow()
        }
    }
}

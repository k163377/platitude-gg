import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit's own hash over its parent's, rows aligned right. Each row is a control and a value: a press that lets go
// where it landed is the control's (copy the hash / go to the parent), one that travels drags over the digits
// (デザイン規約 §右のペインの字は掴める). A `TextEdit` takes the press wherever it is drawn, so the words take none
// (`grabbable: false`) and a hand over each face hands drags down to them (rules-refs/app-ui.md「右パネルの値は掴める字」).
ColumnLayout {
    id: plate

    property string sha8: ""
    property string fullSha: ""
    property string parentSha: ""
    /// How loud the hash is: full in the details pane, under the message in a list (デザイン規約 §複数のコミットを選ぶ).
    property color shaColor: Theme.textPrimary
    /// And how big: a list takes a step down, since mono at the words' size reads louder than they do.
    property real shaSize: Theme.fontMd

    signal copyRequested(string text)
    signal parentClicked(string oidHex)

    /// The fields a sweep may select into, and must keep its hands off (`SweepRoom`).
    readonly property var valueFields: [shaText, parentText]
    /// Whether a point in `item`'s coordinates lands on a control of this plate, whose press is the control's whole.
    function claims(item, x, y) {
        return plate.inside(hashCopy, item, x, y) || (parentLink.visible && plate.inside(parentLink, item, x, y))
    }
    function inside(box, item, x, y) {
        const p = box.mapFromItem(item, x, y)
        return p.x >= 0 && p.y >= 0 && p.x < box.width && p.y < box.height
    }
    /// Where this plate begins, in `item`'s x.
    function leftEdge(item) {
        return plate.mapToItem(item, 0, 0).x
    }
    function fieldFor(which) {
        return which === "parent" ? parentText : shaText
    }
    /// For the runs: the middle of that row's control, in `item`'s coordinates.
    function controlPoint(which, item) {
        const box = which === "parent" ? parentLink : hashCopy
        return box.mapToItem(item, box.width / 2, box.height / 2)
    }

    /// The two controls' actions; handlers and runs both enter here (verify-ui §壊れない動詞の実装).
    function copyFullNow() {
        plate.copyRequested(plate.fullSha)
        hashCopy.copied = true
    }
    function goToParentNow() {
        if (plate.parentSha === "")
            return
        plate.parentClicked(plate.parentSha)
    }

    function selectSha() { shaText.selectAll() }
    function selectParent() { parentText.selectAll() }
    /// Clears both rows' selection: a press on either starts a gesture on the whole plate.
    function dropValues() {
        shaText.deselect()
        parentText.deselect()
    }
    readonly property alias shaSelected: shaText.selected
    readonly property alias parentSelected: parentText.selected

    /// Automation: the two gestures, through the hand's own three functions. The tap lands mid-face; the drag runs
    /// from the value's head off its far end.
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
    /// Automation: the word the copy control offers (hover cannot be injected; the tip itself is `tst_hashplate`'s).
    function tipWords() {
        return hashCopy.ToolTip.text
    }
    /// Automation: what the shared tip shows for this control, empty while none is up — differs from `tipWords` when
    /// a standing tip missed a new text.
    function tipSaid() {
        return plate.tipStanding ? hashCopy.ToolTip.toolTip.text : ""
    }
    /// Whether the window's one shared tip is up for this control, not merely up.
    readonly property bool tipStanding: {
        const tip = hashCopy.ToolTip.toolTip
        return !!tip && tip.visible && tip.parent === hashCopy
    }
    /// The attached tip snapshots its text when `visible` rises, missing a copy made during the delay; re-set as it
    /// comes up, which covers both orders (rules-refs/app-ui.md「右パネルの値は掴める字」).
    onTipStandingChanged: {
        if (plate.tipStanding)
            hashCopy.ToolTip.toolTip.text = hashCopy.ToolTip.text
    }
    function forceTip(on) {
        hashCopy.tipForced = on
    }
    /// Automation: the hand is there, enabled and full-face — runs enter its functions, so a missing one would pass.
    function handStands(which) {
        const hand = plate.handFor(which)
        const face = which === "parent" ? parentLink : hashCopy
        return hand.enabled && hand.width === face.width && hand.height === face.height
    }
    function handFor(which) {
        return which === "parent" ? parentHand : hashHand
    }

    /// The hand over a row's face: the press is the control's until it travels past `drag.threshold`, then a drag over
    /// the words, anchored where it landed. A plain `MouseArea` (rules-refs/app-ui.md「手の実装は素の」);
    /// `preventStealing` keeps the pane's Flickable from taking the drag part-way.
    component RowHand: MouseArea {
        id: hand

        /// The words this row draws.
        required property LineText field
        /// A press that never travelled.
        signal tapped()
        /// Raised as the press lands, before it is decided: the owner clears the last gesture's selection.
        signal taken()

        /// Where the press landed, and whether it has since travelled far enough to be a drag.
        property real fromX: 0
        property real fromY: 0
        property bool dragging: false

        /// The three the handlers call and runs enter, in this hand's coordinates (verify-ui §壊れない動詞の実装).
        function takeAt(x, y) {
            hand.fromX = x
            hand.fromY = y
            hand.dragging = false
            hand.taken()
        }
        function followAt(x, y) {
            if (!hand.dragging) {
                if (Math.abs(x - hand.fromX) < hand.drag.threshold
                        && Math.abs(y - hand.fromY) < hand.drag.threshold)
                    return
                hand.dragging = true
                // Anchoring also takes the window's one selection off any other field (`LineText`).
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

    // The hash is the button (the 16px glyph alone is too small to aim at), lit as one control with its icon. Its own
    // flat wash: a HoverToolButton's panel would make it taller than one line, out of step with the author name.
    ToolButton {
        id: hashCopy
        Layout.alignment: Qt.AlignRight
        hoverEnabled: true
        leftPadding: Theme.spaceXs
        // 0: the mark's ink ends on the line the parent hash and the message frame below end on.
        rightPadding: 0
        topPadding: 0
        bottomPadding: 0
        readonly property bool lit: hovered || visualFocus
        /// The hash was just copied, and the tip's one word says so (規約 §hover のツールチップ「結論を先頭に、1 行で書く」).
        property bool copied: false
        // Arriving clears it as well as leaving: a keyboard copy leaves it standing with no pointer near.
        onHoveredChanged: hashCopy.copied = false
        /// Automation: raise the tooltip with no pointer behind it (verify-ui §hover の絵の撮り方).
        property bool tipForced: false
        // The hand takes the pointer's press, so the face sinks from it until the press travels; `Space` sinks it too.
        down: hashCopy.pressed || (hashHand.containsPress && !hashHand.dragging)
        ToolTip.visible: hashCopy.hovered || hashCopy.tipForced
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: hashCopy.copied ? qsTr("Copied!") : qsTr("Copy full hash")
        // The keyboard's way in; the pointer's press goes to the hand.
        onClicked: plate.copyFullNow()
        background: Rectangle {
            radius: Theme.radiusSm
            color: hashCopy.down ? Theme.bgPressed : hashCopy.lit ? Theme.bgHover : "transparent"
            // Under the icon too: hash and icon are one target, with one line.
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
                pixelSize: plate.shaSize
                color: plate.shaColor
                grabbable: false
                Layout.alignment: Qt.AlignVCenter
            }
            // Seated to the mark's ink, not its 16px box, so the air beside it is not spent twice (デザイン規約 §余白).
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
        // After the content, so it lies over the words.
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
            // Drawn rather than `←` (デザイン規約 §寸法「印は描いて出す」), seated to its ink so the `spaceXs` beside it
            // lands where the eye measures.
            Item {
                Layout.preferredWidth: parentBack.inkWidth
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: parentBack
                    anchors.centerIn: parent
                    kind: "arrow"
                    // The family draws it leaving; turned to point back.
                    rotation: 180
                    tint: Theme.textLink
                    width: Theme.iconSm
                    height: Theme.iconSm
                    // Line scaled with the grid (rules-refs/app-ui.md「語の隣に立つ印は `iconSm`」).
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
        // One underline for the mark and the hash: one target.
        Rectangle {
            visible: parentHand.containsMouse
            color: Theme.textLink
            height: Theme.borderWidth
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
        }
        // Last, so it lies over the row; also the row's only hover source (tooltip, underline).
        RowHand {
            id: parentHand
            field: parentText
            hoverEnabled: true
            onTaken: plate.dropValues()
            onTapped: plate.goToParentNow()
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit's own hash over its parent's, rows aligned right.
//
// **Both rows are controls, and they answer the pointer exactly as they always did**: the whole plate is the button
// that copies the hash, the whole link is the target that goes to the parent (デザイン規約 §右のペインの字は掴める —
// 押せる的の判定は狭めない). The digits are drawn in fields so that a selection can be *put* into them, but the fields
// take no press of their own: whoever presses here is pressing the control, and the selection arrives from the room
// around the plate instead (`SweepRoom`).
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
    readonly property alias shaSelected: shaText.selected
    readonly property alias parentSelected: parentText.selected

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
        ToolTip.visible: hovered
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: qsTr("Copy full hash")
        onClicked: plate.copyFullNow()
        background: Rectangle {
            radius: Theme.radiusSm
            color: hashCopy.down ? Theme.bgPressed : hashCopy.lit ? Theme.bgHover : "transparent"
            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.NoButton
                cursorShape: Qt.PointingHandCursor
            }
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
    }
    Item {
        id: parentLink
        visible: plate.parentSha !== ""
        Layout.alignment: Qt.AlignRight
        implicitWidth: parentRow.implicitWidth
        implicitHeight: parentRow.implicitHeight
        ToolTip.visible: parentHover.containsMouse
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
            visible: parentHover.containsMouse
            color: Theme.textLink
            height: Theme.borderWidth
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
        }
        MouseArea {
            id: parentHover
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: plate.goToParentNow()
        }
    }
}

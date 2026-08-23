import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit's own hash over its parent's, rows aligned right.
ColumnLayout {
    id: plate

    property string sha8: ""
    property string fullSha: ""
    property string parentSha: ""

    signal copyRequested(string text)
    signal parentClicked(string oidHex)

    spacing: 0

    // The hash is the button, not just the icon beside it — a 16px glyph was too small to aim at. Hovering underlines
    // the hash and lights the icon so the whole plate reads as one control. Not a HoverToolButton: the style's panel
    // would make the plate taller than one line and drop this hash out of step with the author name beside it, so it
    // draws the same wash over its own flat face.
    ToolButton {
        id: hashCopy
        Layout.alignment: Qt.AlignRight
        hoverEnabled: true
        text: plate.sha8
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
        onClicked: plate.copyRequested(plate.fullSha)
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
            Label {
                text: hashCopy.text
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontMd
                color: Theme.textPrimary
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
    Label {
        id: parentLink
        visible: plate.parentSha !== ""
        Layout.alignment: Qt.AlignRight
        text: plate.parentSha.substring(0, 8)
        font.family: Theme.monoFamily
        color: Theme.textLink
        font.pixelSize: Theme.fontSm
        // The mark is drawn, not typed. The fonts disagree about `←`: Cascadia Mono holds it in one cell (7px of ink)
        // where Noto Sans Mono CJK JP gives it a full-width one (12px), so Ubuntu grew a tail nobody chose — the same
        // way `⚑` came out a different shape on each of the three.
        //
        // The seat is the mark's ink, not its box, so the `spaceXs` lands where the eye measures it — the same gap the
        // plate above spends between its hash and copy icon (規約 §余白「印が自分で持っている 余白は、隣の詰めに数える」).
        leftPadding: parentBack.inkWidth + Theme.spaceXs
        ToolTip.visible: parentHover.containsMouse
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: qsTr("Go to parent commit")
        NavIcon {
            id: parentBack
            kind: "arrow"
            // The family draws it leaving; this one points back, and turning the mark is how `FoldBlock` faces its
            // chevrons too.
            rotation: 180
            tint: Theme.textLink
            width: Theme.iconSm
            height: Theme.iconSm
            // The grid shrinks and the line shrinks with it, or the mark carries more weight than the digits beside it
            // (§語の隣に立つ印).
            stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
            // Hung off the left by the air it keeps inside its box, so the ink starts where the link does and the line
            // under the pair starts with it.
            x: -(parentBack.width - parentBack.inkWidth) / 2
            anchors.verticalCenter: parent.verticalCenter
        }
        // One line under the mark and the hash: they are one target, the way the hash and its copy icon share theirs.
        // `font.underline` cannot reach the mark now that the mark is not a letter.
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
            onClicked: plate.parentClicked(plate.parentSha)
        }
    }
}

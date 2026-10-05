import QtQuick
import QtQuick.Layouts
import platitude.ui

// One badge's box in the band's state group: natural width until the group hands down a cap, and the word elides into
// whatever is left.
Rectangle {
    id: badge
    property bool filled: false
    /// The state this badge stands for (規約 §状態), carried by the frame
    /// (規約 §進行中・長押しの定数「警告の色は枠と印が持つ」).
    property color tint: Theme.warning
    /// This badge's width with nothing narrowed, handed in from the hidden measurement (`BadgeWord`) — this row is
    /// not laid out while the group is folded.
    property real naturalW: 0
    /// What the group narrowed all of its badges to; `Number.MAX_VALUE` is "nothing narrowed" (`BandStateGroup.cap`).
    property real cap: Number.MAX_VALUE
    /// Whether this badge is also a way somewhere. Only the identity one is, narrowed or not
    /// (デザイン規約 §ウィンドウの縁「押せる行は identity だけ」).
    property bool pressable: false
    /// The box every badge is cut to (`BandStateMetrics`): its depth, and where the words' line stands in it.
    required property BandStateMetrics box
    signal pressed()
    default property alias content: badgeRowInner.data

    implicitWidth: badge.naturalW
    width: Math.min(badge.naturalW, badge.cap)
    height: badge.box.depth
    radius: Theme.radiusSm
    color: badge.filled ? Theme.danger : badge.pressable && badgeHover.hovered ? Theme.bgHover : "transparent"
    border.color: badge.filled ? "transparent" : badge.tint
    border.width: badge.filled ? 0 : Theme.borderWidth
    // On the box, not the words' line below: the line is shallower than the box.
    HoverHandler {
        id: badgeHover
        enabled: badge.pressable
    }
    TapHandler {
        enabled: badge.pressable
        onTapped: badge.pressed()
    }
    // The words' line, not the box: the box is cut round the capitals, and a line centred in it rides high or low by
    // the face's descent.
    RowLayout {
        id: badgeRowInner
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        y: badge.box.wordTop
        spacing: Theme.spaceXs
    }
}

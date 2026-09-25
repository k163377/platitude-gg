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
    signal pressed()
    default property alias content: badgeRowInner.data

    implicitWidth: badge.naturalW
    width: Math.min(badge.naturalW, badge.cap)
    height: Theme.iconLg
    radius: Theme.radiusSm
    color: badge.filled ? Theme.danger : badge.pressable && badgeHover.hovered ? Theme.bgHover : "transparent"
    border.color: badge.filled ? "transparent" : badge.tint
    border.width: badge.filled ? 0 : Theme.borderWidth
    RowLayout {
        id: badgeRowInner
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // Handlers: an `Item` handed to a layout is given a seat in it, and the word beside it loses that room
        // (rules-refs/app-ui.md「`Layout` の子の hover は `HoverHandler`」).
        HoverHandler {
            id: badgeHover
            enabled: badge.pressable
        }
        TapHandler {
            enabled: badge.pressable
            onTapped: badge.pressed()
        }
    }
}

import QtQuick
import QtQuick.Layouts
import platitude.ui

// One badge's box in the band's state group: natural width until the
// group hands down a cap, and the word elides into whatever is left.
Rectangle {
    id: badge
    property bool filled: false
    /// What this badge is drawn at with nothing narrowed. Handed in from
    /// the hidden measurement rather than read off the row inside: that
    /// row is not laid out while the group is folded (`BadgeWord`).
    property real naturalW: 0
    /// What the group narrowed all of its badges to, together.
    /// `Number.MAX_VALUE` is "nothing is narrowed" (`BandStateGroup.cap`).
    property real cap: Number.MAX_VALUE
    /// Whether this badge is also a way somewhere. Only
    /// the identity one is, narrowed or not (規約 §identity).
    property bool pressable: false
    signal pressed()
    default property alias content: badgeRowInner.data

    implicitWidth: badge.naturalW
    width: Math.min(badge.naturalW, badge.cap)
    height: Theme.iconLg
    radius: Theme.radiusSm
    color: badge.filled ? Theme.danger
           : badge.pressable && badgeHover.hovered
             ? Theme.bgHover : "transparent"
    border.color: badge.filled ? "transparent" : Theme.warning
    border.width: badge.filled ? 0 : Theme.borderWidth
    RowLayout {
        id: badgeRowInner
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // Handlers, not a `MouseArea`: an `Item` handed to
        // a layout is given a seat in it, and the word
        // beside it loses that much room (measured here as
        // `SET IDENT…` in a window with 800px going spare).
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

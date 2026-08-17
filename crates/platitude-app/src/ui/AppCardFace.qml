import QtQuick
import platitude.ui

// The face every card, menu and dialog in the app is drawn on: the elevated ground and the frame that floats it above
// whatever it covers (規約 §メニュー). Menus and dialogs are not popups and popups are not menus, so the ground they share is
// a `background:` rather than a base type — `AppCard` is the popup that stands on this one.
//
// `SectionPeekPopup` and the sidebar's own band keep `bgSurface` instead: what is in them is a piece of the sidebar,
// and its header band would be lost against `bgElevated`.
Rectangle {
    id: face

    /// Whether the face has to report the pointer resting on it, for a card that closes when the hand leaves
    /// (`AppCard.pointerInside`).
    ///
    /// Off unless asked, and the handler stands on an item of its own rather than on the face: a `HoverHandler` turns
    /// hover on for the item it is declared under whether or not anyone reads it, and a face with hover on stops the
    /// hover reaching what the card is lying over. An invisible item is skipped by the delivery walk outright, so an
    /// unwatched face costs nothing.
    property bool tracksPointer: false
    /// The pointer is on the face — including the padding band, which the content stops short of.
    readonly property bool pointed: faceHover.hovered

    color: Theme.bgElevated
    radius: Theme.radiusMd
    border.color: Theme.borderDefault
    border.width: Theme.borderWidth

    Item {
        anchors.fill: parent
        visible: face.tracksPointer
        HoverHandler {
            id: faceHover
        }
    }
}

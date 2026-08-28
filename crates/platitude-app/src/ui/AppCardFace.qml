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

    /// The content whose words the reader may take away, for the hand that hands them over from the air around them
    /// (`SweepPad`, 規約 §hover のツールチップ). Set it and a press anywhere in this face that nothing else took starts
    /// a selection in the nearest field — the padding band, the step between two lines, the room beside a short one.
    ///
    /// Null everywhere else, which is every face that is not a card of words: menus and dialogs act on a press, and
    /// the two lists are out by the same rule that keeps the file rows out (規約: 選べる一覧は対象外). An unset pad is
    /// invisible, and an invisible item is skipped by the delivery walk outright, so it costs those nothing.
    property Item textContent: null

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
    /// The hand a range selection over this card's words is taken with, named so a run can enter it
    /// (`<card>.background.pad`, the way `tip-copy` reaches `tip.contentItem`).
    property alias pad: sweepHand
    // It stands in the ground rather than over the content, so it is reached only where nothing else took the press
    // (実測 qmltestrunner `tst_cardpad`).
    SweepPad {
        id: sweepHand
        anchors.fill: parent
        visible: face.textContent !== null
        content: face.textContent
    }
}

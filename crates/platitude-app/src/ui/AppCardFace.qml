import QtQuick
import platitude.ui

// The face every card, menu and dialog is drawn on: the elevated ground and its frame (規約 §メニュー), shared as a
// `background:` because menus, dialogs and popups are separate components. `SectionPeekPopup` keeps `bgSurface`: it is
// a piece of the sidebar, whose header band would be lost against `bgElevated`.
Rectangle {
    id: face

    /// Report the pointer resting on the face (`AppCard.pointerInside`). Off unless asked, on an item of its own: a
    /// `HoverHandler` turns hover on for its item whether read or not, taking it from what the card lies over; an
    /// invisible item is skipped by delivery outright.
    property bool tracksPointer: false
    /// The pointer is on the face — including the padding band, which the content stops short of.
    readonly property bool pointed: faceHover.hovered

    /// The content whose words may be taken (`SweepPad`, 規約 §hover のツールチップ): set, a press anywhere on this face
    /// that nothing else took starts a selection in the nearest field. Null on every face that is not a card of words
    /// (規約 §hover のツールチップ「選べる一覧も対象外」); unset, the pad is invisible and costs nothing.
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
    /// The selection hand, named so a run can reach it (`<card>.background.pad`).
    property alias pad: sweepHand
    // In the ground, so only a press nothing else took reaches it.
    SweepPad {
        id: sweepHand
        anchors.fill: parent
        visible: face.textContent !== null
        content: face.textContent
    }
}

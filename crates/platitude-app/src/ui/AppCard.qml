import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The popup the app's cards stand on: an `AppCardFace` for a ground, a padding band inside the frame, and escape or a
// press outside as ways out. Cards that close on the pointer alone drop `CloseOnPressOutside`, and the ones that hang
// off the thing they describe drop `margins` — both are ordinary overrides.
Popup {
    id: card

    /// Whether this card outlives the hover that opened it, and so has to be able to say the hand is on it
    /// (`pointerInside`). A card nobody asks stays out of the way of the hover underneath it — see
    /// `AppCardFace.tracksPointer`.
    property bool tracksPointer: false
    /// The content's half of `pointerInside`, for a card whose content accepts hover: set it from a `HoverHandler`
    /// declared in the `contentItem`.
    property bool contentPointed: false
    /// Whether the pointer is over the card — over the padding band the background covers, or over the content (規約
    /// §hover のツールチップ の罠 (2)).
    ///
    /// **Two handlers, because the background and the content are siblings**: "handlers are passive" holds down a
    /// subtree, and the content's children are in the content's subtree; the background stands beside them. Measured on
    /// `RefListPopup`'s rows: on a row, the content's handler reads true and the background's reads false; with only
    /// the background's, the list called itself empty of the pointer the instant the hand reached a row and went out
    /// from under it (observed). With only the content's, the card shuts in the width of its own padding, which the
    /// hand walking in over the border crosses first.
    readonly property bool pointerInside: cardFace.pointed || card.contentPointed
    /// The ground, for a card that is a piece of something else: `SectionPeekPopup` keeps
    /// the sidebar's `bgSurface`, against which its header band would otherwise be lost (`AppCardFace`).
    property alias faceColor: cardFace.color
    /// And the corners, for a card that opens flush against the thing it comes out of — there is nothing for the two
    /// corners on that side to round into.
    property alias faceRadius: cardFace.radius
    /// The words this card hands over, for the hand that takes them from the air around them
    /// (`AppCardFace.textContent`). A card of sentences sets it to its own content and every gap in it — the padding
    /// band included — becomes a place a selection can start; a card of rows leaves it alone, because there a press is
    /// the row's (規約 §hover のツールチップ「選べる一覧は対象外」).
    property alias textContent: cardFace.textContent

    padding: Theme.spaceSm
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: AppCardFace {
        id: cardFace
        tracksPointer: card.tracksPointer
    }
}

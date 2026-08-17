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
    /// **Two handlers, because the background and the content are siblings rather than parent and child**: "handlers
    /// are passive" holds down a subtree, and the content's children are in the content's subtree, but the background
    /// is not their parent — it is next to them. Measured on `RefListPopup`'s rows: on a row, the content's handler
    /// reads true and the background's reads false; with only the background's, the list called itself empty of the
    /// pointer the instant the hand reached a row and went out from under it (2026-08-09 report). With only the
    /// content's, the card shuts in the width of its own padding, which the hand walking in over the border crosses
    /// first.
    readonly property bool pointerInside: cardFace.pointed || card.contentPointed

    padding: Theme.spaceSm
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: AppCardFace {
        id: cardFace
        tracksPointer: card.tracksPointer
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The popup the app's cards stand on: an `AppCardFace` ground, a padding band, and escape or a press outside to close.
Popup {
    id: card

    /// Set for a card that outlives the hover that opened it and so reports `pointerInside`; off, it leaves the hover
    /// underneath alone (`AppCardFace.tracksPointer`).
    property bool tracksPointer: false
    /// The content's half of `pointerInside`, for a card whose content accepts hover: set it from a `HoverHandler`
    /// declared in the `contentItem`.
    property bool contentPointed: false
    /// The pointer is over the padding band or the content. Two handlers, because the background and the content are
    /// siblings (rules-refs/app-ui.md「`background` と `contentItem` は兄弟」).
    readonly property bool pointerInside: cardFace.pointed || card.contentPointed
    /// The ground, for a card that is a piece of something else (`SectionPeekPopup`; see `AppCardFace`).
    property alias faceColor: cardFace.color
    /// The corners, for a card that opens flush against what it comes out of.
    property alias faceRadius: cardFace.radius
    /// A card of sentences sets its content here so a press in any gap starts a selection (`AppCardFace.textContent`);
    /// a card of rows leaves it, since there a press is the row's (規約 §hover のツールチップ「選べる一覧も対象外」).
    property alias textContent: cardFace.textContent

    padding: Theme.spaceSm
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: AppCardFace {
        id: cardFace
        tracksPointer: card.tracksPointer
    }
}

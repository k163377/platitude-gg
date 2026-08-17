import QtQuick
import platitude.ui

// The beat between a hover card and the thing it hangs off: it holds the card up while the hand walks from one into the
// other, and takes it down once the hand is out of both.
//
// **A beat, not a turn of the event loop.** A card opens flush under what raised it, so walking in takes the pointer
// off that thing on the way and walking back out puts it on again. The two hovers change in different frames and in no
// fixed order, and `Qt.callLater` lands between them — the card shut under the hand that was reaching for it
// (2026-08-09 report; 規約 §hover のツールチップ の罠 (3)).
//
// Not an item: it draws nothing and is placed nowhere. The owner keeps the card, opens it where it belongs, and hands
// it here to be closed.
QtObject {
    id: keeper

    /// The card being kept.
    required property AppCard card
    /// The pointer is on the thing the card hangs off — the row, the name, the chip, the mark. The owner writes it as
    /// the pointer comes and goes, and the headless runs write the same property, hover being the one thing that cannot
    /// be injected.
    property bool pointedAt: false
    /// Something that is not a pointer is holding the card up: a menu standing on one of its rows, a pin. Most owners
    /// leave it false.
    property bool grace: false

    /// Anything at all is asking for the card — what it hangs off, the card itself, or the hold. The card goes a beat
    /// after this falls.
    readonly property bool lit: keeper.pointedAt || keeper.grace || keeper.card.pointerInside

    /// Start the beat by hand, for an owner that knows the answer has changed before `lit` can see it — a menu that has
    /// just dismissed, a row saying the pointer left it.
    function settle() {
        keep.restart()
    }

    property Timer keep: Timer {
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!keeper.lit)
                keeper.card.close()
        }
    }

    onLitChanged: keeper.settle()
}

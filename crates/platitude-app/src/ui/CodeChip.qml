import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The dress a git command wears wherever the UI says one (デザイン規約 §git 用語のコード表記): the spelling itself, in mono, on a
// faint ground that hangs half a gap off either end of the glyphs so the word keeps the place in the line it would have
// had bare. Wearing this is the whole statement — a chip says "this is the command", so a word that only describes one
// goes without.
//
// The toolbar's fetch / push and the menu rows draw their own, and stay that way: each carries machinery this has no
// business in — a box measured for the longest wording of a pair, a drawn rule for every dash, a column the rows of one
// menu share. What is here is what a question bar needs, where one command is said twice: at the head of the question,
// and on the pill that answers it.
Item {
    id: chip

    /// The command, spelled the way git spells it — and not translated, since what it names is the command rather than
    /// a word for it.
    property string word: ""
    /// The ink. The ground is the same everywhere; the word takes the colour of whatever is speaking, which for a
    /// question bar is the tone that says what answering costs.
    property color tint: Theme.textPrimary
    /// The weight of the run this sits in, so a chip inside a heading is not a lighter word in the middle of a heavier
    /// sentence.
    property int weight: Font.Normal
    /// And its size, for the same reason: a band whose own words are `fontSm` would otherwise carry one word a step
    /// larger than everything beside it.
    property int size: Theme.fontMd
    /// The widest this chip may be before its word starts giving way. Zero is no bound at all, which is what every
    /// chip on a line measured to its own content wants; a chip inside something of a fixed width (the commit button)
    /// hands in what is left for it, because a branch name has no length git will not take.
    property real cap: 0

    /// What the whole word would take, cap or no cap — what a caller sharing out room has to ask for, since the width
    /// below is already the answer to that sharing (reading it back closes a ring).
    readonly property real wantWidth: wordLabel.implicitWidth

    implicitWidth: chip.cap > 0 ? Math.min(wordLabel.paintedWidth, chip.cap) : wordLabel.implicitWidth
    implicitHeight: wordLabel.implicitHeight

    Rectangle {
        // Drawn to the ink, not to the cell: an elided word paints narrower than the width it was given, and a ground
        // stretched to that width leaves a tail of chip with nothing on it (2026-08-18 実測).
        anchors.left: wordLabel.left
        // The word's own step rather than the label's height, which is the mono family's line box — the one part of
        // this dress each OS settles differently (`ActionButtonLabel` carries the measurements).
        anchors.verticalCenter: wordLabel.verticalCenter
        height: chip.size + Theme.spaceXs / 2
        // Half a gap of ground either side, not a whole one: it eats into the air the line already had rather than
        // pushing the glyphs about, so the word starts where it would have started bare.
        anchors.leftMargin: -Theme.spaceXs / 2
        width: wordLabel.paintedWidth + Theme.spaceXs
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    Label {
        id: wordLabel
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // Bounded by the cap rather than by the chip's width, which comes from this — reading it back closes a loop.
        // A ref that will not fit gives way in the middle: the tail of a branch name is what tells two of them apart
        // (`feature/…-a` and `feature/…-b`), and eliding the right would leave the reader with the half they share.
        width: chip.cap > 0 ? Math.min(implicitWidth, chip.cap) : implicitWidth
        elide: chip.cap > 0 ? Text.ElideMiddle : Text.ElideNone
        text: chip.word
        color: chip.tint
        font.family: Theme.monoFamily
        // A command and its flag are one thing said, and a mono space is wider than the air the ground keeps at its own
        // ends — left alone the two drift apart and the chip reads as two words on one ground (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: -Theme.spaceXs
        font.pixelSize: chip.size
        font.weight: chip.weight
    }
}

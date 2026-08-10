import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The dress a git command wears wherever the UI says one (デザイン規約
// §git 用語のコード表記): the spelling itself, in mono, on a faint ground
// that hangs half a gap off either end of the glyphs so the word keeps the
// place in the line it would have had bare. Wearing this is the whole
// statement — a chip says "this is the command", so a word that only
// describes one goes without.
//
// The toolbar's fetch / push and the menu rows draw their own, and stay
// that way: each carries machinery this has no business in — a box
// measured for the longest wording of a pair, a drawn rule for every dash,
// a column the rows of one menu share. What is here is what a question bar
// needs, where one command is said twice: at the head of the question, and
// on the pill that answers it.
Item {
    id: chip

    /// The command, spelled the way git spells it — and not translated,
    /// since what it names is the command rather than a word for it.
    property string word: ""
    /// The ink. The ground is the same everywhere; the word takes the
    /// colour of whatever is speaking, which for a question bar is the
    /// tone that says what answering costs.
    property color tint: Theme.textPrimary
    /// The weight of the run this sits in, so a chip inside a heading is
    /// not a lighter word in the middle of a heavier sentence.
    property int weight: Font.Normal

    implicitWidth: wordLabel.implicitWidth
    implicitHeight: wordLabel.implicitHeight

    Rectangle {
        anchors.fill: wordLabel
        // Half a gap of ground either side, not a whole one: it eats into
        // the air the line already had rather than pushing the glyphs
        // about, so the word starts where it would have started bare.
        anchors.leftMargin: -Theme.spaceXs / 2
        anchors.rightMargin: -Theme.spaceXs / 2
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    Label {
        id: wordLabel
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        text: chip.word
        color: chip.tint
        font.family: Theme.monoFamily
        // A command and its flag are one thing said, and a mono space is
        // wider than the air the ground keeps at its own ends — left alone
        // the two drift apart and the chip reads as two words on one
        // ground (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: -Theme.spaceXs
        font.pixelSize: Theme.fontMd
        font.weight: chip.weight
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The dress a git command wears wherever the UI says one (デザイン規約 §git 用語のコード表記): the spelling in mono, on a
// faint ground hanging half a gap off either end so the word keeps its place in the line.
//
// The toolbar's fetch / push and the menu rows draw their own: each carries machinery this has no business in (a box
// for the longest wording of a pair, a drawn rule per dash, a column shared across a menu).
Item {
    id: chip

    /// The command, spelled the way git spells it — untranslated.
    property string word: ""
    /// The word's ink; the ground is the same everywhere.
    property color tint: Theme.textPrimary
    /// The weight and size of the run this sits in, so the chip is not a different word in its sentence.
    property int weight: Font.Normal
    property int size: Theme.fontMd
    /// The widest this chip may be before its word gives way; 0 = no bound. A chip in something of fixed width (the
    /// commit button) hands in what is left, since a branch name has no length limit.
    property real cap: 0

    /// Whether the word is a field a reader can drag over (規約 §右のペインの字は掴める). Off by default, and it has
    /// to be: a chip stands inside buttons as their `contentItem` (`ActionButtonLabel` / `StashActionsBand`), and a
    /// field takes the press wherever it is drawn, so the button under it could not be pressed
    /// (`tests/qml/tst_hashplate.qml`). On only where the chip is the value (規約 §設定の画面).
    property bool grabbable: false

    /// What the whole word would take, cap or no cap — what a caller sharing out room asks for, since `implicitWidth`
    /// is already the answer to that sharing (reading it back closes a ring).
    readonly property real wantWidth: wordLabel.implicitWidth

    implicitWidth: chip.cap > 0 ? Math.min(wordLabel.paintedWidth, chip.cap) : wordLabel.implicitWidth
    implicitHeight: wordLabel.implicitHeight

    Rectangle {
        // Drawn to the ink (`paintedWidth`): an elided word paints narrower than its width, which would leave an empty
        // tail of chip.
        anchors.left: wordLabel.left
        // Height from the word's size, not the label: the mono line box differs per OS (`ActionButtonLabel`).
        anchors.verticalCenter: wordLabel.verticalCenter
        height: chip.size + Theme.spaceXs / 2
        // Half a gap either side, eating into the line's own air so the word starts where it would without the chip.
        anchors.leftMargin: -Theme.spaceXs / 2
        width: wordLabel.paintedWidth + Theme.spaceXs
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    Label {
        id: wordLabel
        // Hidden where the field below draws the word, so it is drawn and pressed once; kept, since every measurement
        // this chip makes is read off it and a `Text` lays out while invisible.
        visible: !chip.grabbable
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // A ref that will not fit gives way in the middle: the tail tells two branch names apart.
        width: chip.cap > 0 ? Math.min(implicitWidth, chip.cap) : implicitWidth
        elide: chip.cap > 0 ? Text.ElideMiddle : Text.ElideNone
        text: chip.word
        color: chip.tint
        font.family: Theme.monoFamily
        // A mono space is wider than the air at the ground's ends, so a command and its flag would read as two words
        // (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: -Theme.spaceXs
        font.pixelSize: chip.size
        font.weight: chip.weight
    }
    /// Automation only: the word is in a field — a picture cannot say it (`settings-sweep`).
    readonly property bool grabbed: wordField.item !== null && wordField.item.grabs
                                    && wordField.item.text === chip.word && chip.word !== ""

    // The same word in a field (`grabbable`), loaded only where asked for: a chip is drawn per plan row and button,
    // and a field is six items. Spelled from the same values as the label, word spacing included, so the ground
    // measured off the label fits it.
    Loader {
        id: wordField
        active: chip.grabbable
        anchors.left: wordLabel.left
        anchors.verticalCenter: wordLabel.verticalCenter
        width: wordLabel.width
        height: wordLabel.implicitHeight
        sourceComponent: LineText {
            text: chip.word
            color: chip.tint
            mono: true
            pixelSize: chip.size
            weight: chip.weight
            wordSpacing: -Theme.spaceXs
            cutAt: "middle"
            // The chip's own ground, for the cut mark.
            ground: Theme.bgHover
        }
    }
}

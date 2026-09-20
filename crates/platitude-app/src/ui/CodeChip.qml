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

    /// The command, spelled the way git spells it — untranslated, since what it names is the command
    /// itself.
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

    /// Whether the word answers its own press, so a reader can drag over it and take it away
    /// (規約 §右のペインの字は掴める). **Off by default, and it has to be**: a chip stands inside buttons as their
    /// `contentItem` (`ActionButtonLabel` / `StashActionsBand`), and a field takes the press wherever it is drawn —
    /// measured, and measured the hard way: a click on the hash plate's digits stopped reaching the button under
    /// them (`tests/qml/tst_hashplate.qml`). So a chip that answered for itself everywhere would leave those
    /// buttons unable to be pressed at all.
    ///
    /// **On where the chip is the value and nothing else wants the press**: the git a settings screen says it is
    /// running, which is git's own spelling of its own version and the one line on that screen a reader takes away
    /// to a bug report (規約 §設定の画面).
    property bool grabbable: false

    /// What the whole word would take, cap or no cap — what a caller sharing out room has to ask for, since the width
    /// below is already the answer to that sharing (reading it back closes a ring).
    readonly property real wantWidth: wordLabel.implicitWidth

    implicitWidth: chip.cap > 0 ? Math.min(wordLabel.paintedWidth, chip.cap) : wordLabel.implicitWidth
    implicitHeight: wordLabel.implicitHeight

    Rectangle {
        // Drawn to the ink: an elided word paints narrower than the width it was given, and a ground
        // stretched to that width leaves a tail of chip with nothing on it (measured).
        anchors.left: wordLabel.left
        // The word's own step: the label's height is the mono family's line box — the one part of
        // this dress each OS settles differently (`ActionButtonLabel` carries the measurements).
        anchors.verticalCenter: wordLabel.verticalCenter
        height: chip.size + Theme.spaceXs / 2
        // Half a gap of ground either side: it eats into the air the line already had, so the word starts where it
        // would have started bare.
        anchors.leftMargin: -Theme.spaceXs / 2
        width: wordLabel.paintedWidth + Theme.spaceXs
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    Label {
        id: wordLabel
        // Hidden where the field below is drawing the word, and kept all the same: **every measurement this chip
        // makes is read off it** — the ground's width, the chip's own implicit size, what a caller sharing out room
        // asks for — and a `Text` lays out while invisible. An invisible item is skipped by the delivery walk
        // outright, so the word is drawn once and answers a press once.
        visible: !chip.grabbable
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // Bounded by the cap: the chip's width comes from this — reading it back closes a loop.
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
    /// The word is in a field, and the field has it. **A picture cannot say it** — a chip a reader can drag over is
    /// drawn exactly like one they cannot — so a run says it instead (`settings-sweep`). An automation-only
    /// exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property bool grabbed: wordField.item !== null && wordField.item.grabs
                                    && wordField.item.text === chip.word && chip.word !== ""

    // The same word, in a field, for the chip a reader takes away (`grabbable`). **Loaded only where it is asked
    // for**: a chip is drawn per plan row and per button in this window, and a field is six items.
    //
    // It is laid exactly where the word above is and spelled from the same values, so the ground measured off that
    // word fits this one — **including the word spacing**, which is the chip's own and would otherwise leave the
    // field four pixels wider than the ground under it.
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
            // The same half the `Label` gives way in, and for the same reason.
            cutAt: "middle"
            // The chip's own ground, so a cut mark is drawn on the chip rather than on whatever is behind it.
            ground: Theme.bgHover
        }
    }
}

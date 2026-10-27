import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The dress a git command or a name wears wherever the UI says one as code (デザイン規約 §git 用語のコード表記、§ref
// の種別「文の中の名前」): the spelling in mono, on a faint ground hanging half a gap off either end so the word keeps
// its place in the line — a name behind its kind's mark on the same ground.
//
// The toolbar's fetch / push and the menu rows draw their own: each carries machinery this has no business in (a box
// for the longest wording of a pair, a drawn rule per dash, a column shared across a menu).
Item {
    id: chip

    /// The command or the name, spelled the way git spells it — untranslated.
    property string word: ""
    /// The word's ink; the ground is the same everywhere.
    property color tint: Theme.textPrimary
    /// The weight and size of the run this sits in, so the chip is not a different word in its sentence.
    property int weight: Font.Normal
    property int size: Theme.fontMd
    /// The widest this chip may be before its word gives way; 0 = no bound. A chip in something of fixed width (the
    /// commit button) hands in what is left, since a branch name has no length limit.
    property real cap: 0
    /// The word gives way to the width a layout hands the chip instead — set once, never with a `cap`: a chip that
    /// read its own width only while its cap was still 0 would close a ring through `implicitWidth` when the cap came.
    property bool cutToWidth: false
    /// A mark ahead of the word on the same ground — the ref's kind where the word is a ref's name (デザイン規約 §ref の種別
    /// 「名前の印」), so mark and name read and give way as one; empty for none. A sign on the word (§アイコンの席 の修飾),
    /// in its kind's colour — the word's where that is grey or on a fill (`NavIcon.inkBeside`).
    property string mark: ""
    property color markTint: wordMark.inkBeside(chip.tint)
    /// The room the mark takes before the word: the seat (`MarkSeat`), or where chips stand under sentences, the seat
    /// as a sentence lays it out, so the names start on one x (`AppMenuItem.refCell` — デザイン規約 §メニュー). The mark
    /// stands in its middle, and the ground starts at its head: the seat holds the mark's air.
    property real markCell: wordMark.wordCell
    /// The mark's cell, nothing without one.
    readonly property real markRoom: chip.mark !== "" ? chip.markCell : 0

    /// Whether the word is a field a reader can drag over (規約 §右のペインの字は掴める). Off by default, and it has
    /// to be: a chip stands inside buttons as their `contentItem` (`ActionButtonLabel` / `StashActionsBand`), and a
    /// field takes the press wherever it is drawn, so the button under it could not be pressed
    /// (`tests/qml/tst_hashplate.qml`). On only where the chip is the value (規約 §設定の画面).
    property bool grabbable: false
    /// The chip is a button's word (`CodeGround.buttonWord`).
    property bool buttonWord: false

    /// Where a word that will not fit gives way: `middle` — the tail tells two branch names apart — or `end`: a
    /// remote's name, whose head is the remote (デザイン規約 §寸法「upstream 名だけは末尾で切る」), or a line cut at
    /// the end (`CodeLine`).
    property string cutAt: "middle"
    /// Whether the word gave way (`cap`, `cutToWidth`): the hover gives it back.
    readonly property bool cut: wordLabel.truncated
    /// What the whole word would take, cap or no cap — what a caller sharing out room asks for, since `implicitWidth`
    /// is already the answer to that sharing (reading it back closes a ring).
    readonly property real wantWidth: chip.markRoom + wordLabel.implicitWidth
    /// The word as drawn: cut where it gave way.
    readonly property string shownWord: chip.cut ? wordShown.elidedText : chip.word
    /// What the chip draws: the mark's room and the word as cut — short of the width it was given where the cut drops
    /// a whole glyph, so what stands after it closes up to its ground (`CodeLine`). A cut is read off the cut text, not
    /// `paintedWidth`: offscreen a cut label answered a pixel past the glyphs it kept (105 for 104, `Cascadia Mono`).
    readonly property real drawnWidth: chip.markRoom
                                       + (chip.cut ? wordFace.advanceWidth(chip.shownWord) : wordLabel.paintedWidth)

    implicitWidth: chip.markRoom + (chip.cap > 0 ? Math.min(wordLabel.paintedWidth, chip.cap - chip.markRoom)
                                                 : wordLabel.implicitWidth)
    implicitHeight: wordLabel.implicitHeight
    /// The word's, so a row can stand a sentence beside the chip on its baseline (`AskBar`, `OpExitCard`). The
    /// label's: a `grabbable` chip draws its word in a field instead, which stands its words by its own rule
    /// (`LineText`).
    baselineOffset: wordLabel.y + wordLabel.baselineOffset

    CodeGround {
        // A word alone: half a gap off either end of the drawn text, eating into the line's own air so the word starts
        // where it would without the chip. A name behind its mark: from the seat's head, which holds the mark's air,
        // to half a gap past the ink of the last glyph the name is drawn with — the `…` of a cut at the end
        // (`WordFace.groundStart` / `groundEnd`).
        x: chip.mark !== "" ? 0 : WordFace.groundStart(0)
        width: (chip.mark !== "" ? WordFace.groundEnd(wordFace, chip.shownWord, chip.drawnWidth)
                                 : chip.drawnWidth + Theme.spaceXs / 2) - x
        // The baseline of the word as drawn: the field's where it draws the word (`grabbable`).
        wordBaseline: wordField.item !== null ? wordField.y + wordField.item.baselineOffset
                                          : wordLabel.y + wordLabel.baselineOffset
        size: chip.size
        buttonWord: chip.buttonWord
    }
    /// The word's face, for where its ink ends (`WordFace.groundEnd`).
    FontMetrics {
        id: wordFace
        font: wordLabel.font
    }
    /// The word cut as the label cuts it (`shownWord`), laid out only where it was cut.
    TextMetrics {
        id: wordShown
        font: wordLabel.font
        elide: wordLabel.elide
        elideWidth: wordLabel.width
        text: chip.cut ? chip.word : ""
    }
    // The mark, its box the word's line, so the two stand on one baseline, and the room before the word.
    NavIcon {
        id: wordMark
        visible: chip.mark !== ""
        x: 0
        y: wordLabel.y
        width: chip.markCell
        height: wordLabel.height
        kind: chip.mark !== "" ? chip.mark : "branch"
        set: "sign"
        size: chip.size
        wordBase: wordLabel.baselineOffset
        nameAir: WordFace.nameAir(wordFace, chip.shownWord)
        tint: chip.markTint
    }
    Label {
        id: wordLabel
        // Hidden where the field below draws the word, so it is drawn and pressed once; kept, since every measurement
        // this chip makes is read off it and a `Text` lays out while invisible.
        visible: !chip.grabbable
        anchors.left: parent.left
        anchors.leftMargin: chip.markRoom
        anchors.verticalCenter: parent.verticalCenter
        // A word that will not fit gives way where `cutAt` says.
        width: chip.cutToWidth ? Math.min(implicitWidth, chip.width - chip.markRoom)
             : chip.cap > 0 ? Math.min(implicitWidth, chip.cap - chip.markRoom) : implicitWidth
        elide: !chip.cutToWidth && chip.cap <= 0 ? Text.ElideNone
             : chip.cutAt === "end" ? Text.ElideRight : Text.ElideMiddle
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

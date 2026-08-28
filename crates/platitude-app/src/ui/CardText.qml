import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A line a card puts out to be read, in a field the reader can take away with them: pressed, dragged over and copied
// like any other text in this window (規約 §hover のツールチップ「出したものは持ち帰れる」).
//
// **A `TextEdit` rather than a `Label`**, because selecting is a text field's doing and a Label has none of it. What
// that costs is `elide`, which `TextEdit` has no property for — so **this part wraps and the caller caps the width**.
// That is the answer the cards wanted anyway: every field that used to elide in one of them is a name being shown
// *because* the row it came from had already cut it (規約 §hover のツールチップ「名前は文ではないので 1 行に収めない」), and a
// name cut a second time in the place it went to be read is a name nobody can read anywhere.
//
// **The height is the one cap that stays a cut.** A message has no length git enforces and a card that grows with one
// takes its own footer off the screen, so a caller may hand this part a `capHeight`; what will not fit is clipped and
// the mark below says so. Nothing is taken out of the field itself — the whole of it is still there to be selected,
// which is what separates this from an elide.
Item {
    id: cardText

    /// What the field holds, in full — including whatever the cap below is hiding.
    property string text: ""
    property color color: Theme.textPrimary
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// How tall the field may grow before the rest is left behind. 0 = no cap, which is every one-line field.
    property real capHeight: 0
    /// The card this stands on, for the mark's own ground: the mark is drawn over the last line it cuts, and needs
    /// something opaque under it (`AppCardFace` paints `bgElevated`, which is what every card here is).
    property color ground: Theme.bgElevated

    /// The pointer is on the words. The card ORs this into its own `pointerInside`, the same as any other content that
    /// takes hover (`AppCard.contentPointed` — 規約 §hover のツールチップ の罠 (2)).
    readonly property alias pointed: wordHover.hovered
    /// The cap left something behind. The output side, and what a headless run reads in place of a mark it cannot see.
    readonly property bool clipped: cardText.capHeight > 0 && field.implicitHeight > cardText.capHeight + 0.5
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText

    /// Puts the whole field in the selection, for the automation and for `Ctrl+A` (see `selectByKeyboard`).
    function selectAll() {
        field.selectAll()
    }

    /// The width the words want with nothing to wrap them — measured off a ruler rather than off the field, because
    /// **the field's own `implicitWidth` follows its `width` once it wraps**, and a natural width read from there is a
    /// binding standing on its own answer (規約 §QML 実装ルール). Rounded up: a layout hands an item the whole pixel
    /// below a fractional width, and a line asking for 79.28 given 79 wraps against a box meant to hold it.
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: cardText.capHeight > 0
                    ? Math.min(field.implicitHeight, cardText.capHeight)
                    : field.implicitHeight
    // Only the height ever cuts, and only when a caller asked for a cap.
    clip: cardText.clipped

    Text {
        id: ruler
        visible: false
        text: cardText.text
        font: field.font
        wrapMode: Text.NoWrap
    }

    TextEdit {
        id: field
        width: cardText.width
        text: cardText.text
        color: cardText.color
        font.family: Theme.uiFamily
        font.pixelSize: cardText.pixelSize
        font.weight: cardText.weight
        wrapMode: Text.Wrap
        // Read, not written: the card is a view of a commit, a file or a person, and none of them is edited from here.
        readOnly: true
        // `selectByKeyboard` is false by default on a read-only field, and `Ctrl+A` goes with it — the one gesture a
        // reader who wants the whole line reaches for first.
        selectByMouse: true
        selectByKeyboard: true
        // **Never `persistentSelection`.** There is one selection in this window, and it belongs to whatever the caret
        // is in — a card holding several of these would otherwise stay lit in every line the reader had swept, and so
        // would the pane behind it (2026-08-28 ユーザー報告, on the pane's own fields). It was kept for the camera, and
        // the camera never needed it: a field that has not taken focus answers `selectedText` after `selectAll()` all
        // the same, which is the whole of what `tip-copy` reads (実測 qmltestrunner `tst_twofields`).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        // A field of its own width, with no room around it: the card's padding is the card's.
        padding: 0
        textMargin: 0
        HoverHandler {
            id: wordHover
        }
    }

    // What the cap left behind. On the card's own ground rather than over the words: the last line it cuts through is
    // drawn under this, and a mark read through a sentence is not a mark.
    Rectangle {
        visible: cardText.clipped
        color: cardText.ground
        width: markLabel.implicitWidth + Theme.spaceXs
        height: markLabel.implicitHeight
        x: cardText.width - width
        y: cardText.height - height
        Label {
            id: markLabel
            anchors.right: parent.right
            text: "…"
            color: cardText.color
            font.family: Theme.uiFamily
            font.pixelSize: cardText.pixelSize
        }
    }
}

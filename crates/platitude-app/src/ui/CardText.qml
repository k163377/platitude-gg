import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A line a card puts out to be read, in a field the reader can select and copy
// (規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
//
// A `TextEdit`, since a Label cannot select. `TextEdit` has no `elide`, so this part wraps and the caller caps the
// width (規約 §hover のツールチップ「名前は折り返して全文を出す」).
//
// A press on the words is the field's own; a press in the card's air (padding, between lines, beside a short one) is
// taken by the pad under the card and driven in from there (`SweepPad`). Both land on the same selection.
//
// The height is the one cap that cuts: by room (`capHeight`) or by lines (`capRows`), spent in whole lines either way
// (`capLines`). What is clipped stays in the field and can still be selected.
Item {
    id: cardText

    /// The full text, including whatever the cap hides.
    property string text: ""
    /// The same line as rich text, where one word wears a colour of its own (`Words.nameInSentence`); empty elsewhere.
    /// `text` stays the plain sentence either way — that is what callers measure and read back. The ruler measures
    /// whichever is drawn, or a width would be bid for the tags as well.
    property string markup: ""
    property color color: Theme.textPrimary
    /// The anchor the pointer is on (empty on none), and the press on it — an anchor, since only the field knows where
    /// a run of glyphs inside a sentence begins and ends. Its colour is whatever the markup gave it: `linkColor` is
    /// `Text`'s (on a `TextEdit` the type fails to load) and `palette.link` is not read (the style's blue comes out).
    readonly property alias pointedLink: field.hoveredLink
    signal linkAsked(string href)
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// The whole field underlined in its own colour (`NavFactLine.goes`). The font's rule, not a drawn line: the field
    /// wraps, and a line at the foot of the box would underline the last line alone.
    property bool underline: false
    /// Spelled in git's own family (規約 §git 用語のコード表記). A bool, since a family handed in by a caller would skip
    /// the per-OS fallback Theme resolves (規約 §QML 実装ルール).
    property bool mono: false
    /// Centred where a screen's whole content is the sentence (`NoticeLine`); left wherever a card stacks lines.
    property int horizontalAlignment: Text.AlignLeft
    /// The height cap; 0 = none.
    property real capHeight: 0
    /// The same cap in lines; wins over `capHeight`, 0 = not asked for.
    property int capRows: 0
    /// The mark's ground, opaque over the line it cuts (`AppCardFace` paints `bgElevated`).
    property color ground: Theme.bgElevated

    /// The pointer is on the words. The card ORs this into its `pointerInside` (`AppCard.contentPointed` —
    /// rules-refs/app-ui.md「hover で開くものの 5 つの罠」の (2)).
    readonly property alias pointed: wordHover.hovered
    /// One line's height: the field is laid out in one font, so its height divides by its line count exactly.
    readonly property real lineHeight: field.lineCount > 0 ? field.contentHeight / field.lineCount : 0
    /// How many whole lines the cap leaves room for. A height taken at its word cuts a line through the waist, with the
    /// mark on a baseline of its own; rounded down, the field ends the way an elide does. At least one line, or a cap
    /// shorter than a line leaves a blank strip with a mark.
    readonly property int capLines: cardText.capRows > 0
                                    ? cardText.capRows
                                    : (cardText.capHeight > 0 && cardText.lineHeight > 0
                                       ? Math.max(1, Math.floor(cardText.capHeight / cardText.lineHeight))
                                       : 0)
    /// The cap left something behind — what a headless run reads in place of a mark it cannot see.
    readonly property bool clipped: cardText.capLines > 0 && field.lineCount > cardText.capLines
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText
    /// A value a sweep from the gaps can land on: `SweepPad` walks the card for this, so a new field is reachable
    /// without being listed anywhere.
    readonly property bool sweepable: true
    /// Whether the keyboard is here — what `Ctrl+C` needs, and what a selection alone does not say.
    readonly property alias hasCaret: field.activeFocus

    /// Where the mark's ground begins, rounded out to a character boundary — a glyph cut down its middle is the same
    /// broken line `capLines` avoids, on its side.
    readonly property real markLeft: {
        const want = cardText.width - markLabel.implicitWidth - Theme.spaceXs
        // The guard is also the dependency list: `positionAt` is a call, so the reads that decide it are made here
        // (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
        if (field.text === "" || field.width <= 0 || field.contentHeight <= 0 || want <= 0)
            return Math.max(0, want)
        const under = field.positionAt(want, cardText.height - cardText.lineHeight / 2)
        // `positionAt` answers the *nearest* boundary; one that overshot the mark is walked back by one.
        const near = field.positionToRectangle(under).x
        const left = near > want && under > 0 ? field.positionToRectangle(under - 1).x : near
        return Math.max(0, Math.min(want, left))
    }

    /// Where one character stands, in this part's coordinates (the field fills it with no margin). What a mark drawn
    /// in the gap the markup opens for it is placed against (`SharedToolTip`, `Words.roomInSentence`). A call, so a
    /// caller binding to it reads `width` / `text` in the same binding, as `markLeft` does.
    function charRect(pos) {
        return field.positionToRectangle(pos)
    }

    /// For the automation; `Ctrl+A` reaches the field itself (`selectByKeyboard`).
    function selectAll() {
        field.selectAll()
    }
    function deselect() {
        field.deselect()
    }

    // ---- driven from outside ----------------------------------------
    // A gesture begun in the card's air: the pad under the card hands points down and the field turns them into
    // characters (`SweepPad`). The anchor is set once where the drag began, as the field's own drag does.
    /// Where the gesture started, in characters.
    property int grabAnchor: 0
    /// A point in another item's coordinates, clamped into this box on both axes: outside it `positionAt` answers about
    /// the line alone, so a sweep from the padding picks the same character at both ends (`LineText.onLine`).
    function inBox(item, x, y) {
        const p = cardText.mapFromItem(item, x, y)
        return Qt.point(Math.max(0, Math.min(cardText.width - 1, p.x)),
                        Math.max(0, Math.min(cardText.height - 1, p.y)))
    }
    function anchorFrom(item, x, y) {
        const p = cardText.inBox(item, x, y)
        // Taking focus is what clears whichever field held the selection (one selection in the window); a press on
        // the words does the same on its own (`activeFocusOnPress`).
        field.forceActiveFocus()
        cardText.grabAnchor = field.positionAt(p.x, p.y)
        field.select(cardText.grabAnchor, cardText.grabAnchor)
    }
    function extendFrom(item, x, y) {
        const p = cardText.inBox(item, x, y)
        field.select(cardText.grabAnchor, field.positionAt(p.x, p.y))
    }

    /// The unwrapped width, off a ruler: the field's own `implicitWidth` follows its `width` once it wraps
    /// (rules-refs/app-ui.md「自然幅は隠した `Text` の ruler で測る」). Rounded up: a layout floors a fractional width,
    /// and a line asking for 79.28 given 79 wraps.
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: cardText.capLines > 0
                    ? Math.min(field.implicitHeight, cardText.capLines * cardText.lineHeight)
                    : field.implicitHeight
    clip: cardText.clipped

    Text {
        id: ruler
        visible: false
        text: cardText.markup !== "" ? cardText.markup : cardText.text
        textFormat: cardText.markup !== "" ? Text.RichText : Text.PlainText
        font: field.font
        wrapMode: Text.NoWrap
    }

    TextEdit {
        id: field
        width: cardText.width
        text: cardText.markup !== "" ? cardText.markup : cardText.text
        // Pinned: AutoText guesses by whether a string looks like markup, and git's words can (a branch called `<b>`
        // would vanish). `markup` is escaped by the caller that built it.
        textFormat: cardText.markup !== "" ? TextEdit.RichText : TextEdit.PlainText
        color: cardText.color
        onLinkActivated: href => cardText.linkAsked(href)
        font.family: cardText.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: cardText.pixelSize
        font.weight: cardText.weight
        font.underline: cardText.underline
        horizontalAlignment: cardText.horizontalAlignment
        wrapMode: Text.Wrap
        readOnly: true
        // `selectByKeyboard` is off by default on a read-only field, and `Ctrl+A` goes with it.
        selectByMouse: true
        selectByKeyboard: true
        // No `persistentSelection`: one selection in the window, held by the field with the caret
        // (rules-refs/app-ui.md「`persistentSelection` を使わない」).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        // The card's padding is the card's.
        padding: 0
        textMargin: 0
        HoverHandler {
            id: wordHover
        }
    }

    // The cut mark, on the tail of the last kept line, over the card's ground so the line under it does not show
    // through. A line tall so the mark sits on that line's baseline.
    Rectangle {
        visible: cardText.clipped
        color: cardText.ground
        width: cardText.width - cardText.markLeft
        height: cardText.lineHeight
        x: cardText.markLeft
        y: cardText.height - height
        Label {
            id: markLabel
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: "…"
            color: cardText.color
            // The field's font, so `mono` reaches the mark too.
            font: field.font
        }
    }
}

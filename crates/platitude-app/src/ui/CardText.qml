import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A line a card puts out to be read, in a field the reader can take away with them: pressed, dragged over and copied
// like any other text in this window (規約 §hover のツールチップ「出したものは持ち帰れる」).
//
// **A `TextEdit`**, because selecting is a text field's doing and a Label has none of it. What
// that costs is `elide`, which `TextEdit` has no property for — so **this part wraps and the caller caps the width**.
// That is the answer the cards want anyway: every field a card shows is a name being shown *because* the row it came
// from had already cut it (規約 §hover のツールチップ「名前は折り返して全文を出す」), and a name cut a second
// time in the place it went to be read is a name nobody can read anywhere.
//
// **Two ways in, the pair the right pane's values carry.** Where the pointer is on the words they answer their own
// press, which is Qt's machinery and needs nothing from us; where it is in the card's own air — the padding band, the
// step between two lines, the room beside a short one — the press is taken by the pad lying under the card and driven
// in from there (`SweepPad`, 規約 §hover のツールチップ). Both land on the same selection.
//
// **The height is the one cap that stays a cut.** A message has no length git enforces and a card that grows with one
// takes its own footer off the screen, so a caller may cap this part — by the room there is (`capHeight`) or by a count
// of lines (`capRows`), whichever its bound actually is. What will not fit is clipped and the mark below says so, and
// **the cap is spent in whole lines either way** (`capLines`). Nothing is taken out of the field itself — the whole of
// it is still there to be selected, which is what separates this from an elide.
Item {
    id: cardText

    /// What the field holds, in full — including whatever the cap below is hiding.
    property string text: ""
    /// The same line spelled as the markup rich text reads, where one word in it wears a colour of its own — a ref
    /// name inside the sentence (`Words.nameInSentence`). Empty everywhere else, and the field takes `text` instead.
    /// **`text` stays the plain sentence either way**, since that is what a caller measuring or reading the line back
    /// is handed. The ruler measures whichever of the two is drawn, or a width would be bid for the tags as well.
    property string markup: ""
    property color color: Theme.textPrimary
    /// The word the pointer is on, empty when it is on none of them, and the press on it.
    ///
    /// **An anchor** (`CommitHoverCard`'s note is the other shape): that note
    /// is a whole line and this one is a run of glyphs inside a sentence, and only the field knows where that run
    /// begins and ends. What the field draws for it is the word underlined — **and in whatever colour the markup
    /// gave it**, since an anchor's ink is the document's: `linkColor` belongs to `Text` and this is a `TextEdit`
    /// (measured — the type would not load at all), and `palette.link` is not read here either (measured — the
    /// anchor came out in the style's blue).
    readonly property alias pointedLink: field.hoveredLink
    signal linkAsked(string href)
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// The whole field underlined, in its own colour — what a line that is a way somewhere wears under the pointer
    /// (`NavFactLine.goes`). **The font's own rule, not one drawn under the box**: the field wraps, and a line drawn
    /// at the foot of the box would underline the last line alone and run on past its words.
    property bool underline: false
    /// Spelled in git's own family — a path git printed, a hash (規約 §git 用語のコード表記).
    /// The pair `LineText` carries, and chosen by name for the same reason: a family handed in from a caller would
    /// skip the per-OS fallback Theme resolves (規約 §QML 実装ルール).
    property bool mono: false
    /// How the lines sit across the field's own width. Centred where the surface centres its words — a screen whose
    /// whole content is the sentence in the middle of it (`NoticeLine`) — and left everywhere a card stacks them.
    property int horizontalAlignment: Text.AlignLeft
    /// How tall the field may grow before the rest is left behind, as a height. 0 = no cap, which is every one-line
    /// field. Spent in whole lines all the same — see `capLines`.
    property real capHeight: 0
    /// The same cap said in lines, for a caller who means a number of lines.
    /// Wins over `capHeight` when both are set; 0 = not asked for.
    property int capRows: 0
    /// The card this stands on, for the mark's own ground: the mark is drawn over the last line it cuts, and needs
    /// something opaque under it (`AppCardFace` paints `bgElevated`, which is what every card here is).
    property color ground: Theme.bgElevated

    /// The pointer is on the words. The card ORs this into its own `pointerInside`, the same as any other content that
    /// takes hover (`AppCard.contentPointed` — 規約 §hover のツールチップ の罠 (2)).
    readonly property alias pointed: wordHover.hovered
    /// One line of the field, which is the size of every line in it: the whole of it is laid out in one font, so its
    /// height divides by its line count exactly.
    readonly property real lineHeight: field.lineCount > 0 ? field.contentHeight / field.lineCount : 0
    /// How many whole lines the cap leaves room for — **a cap is always spent in whole lines**. A height taken at its
    /// word lands inside a line and leaves a row of glyphs cut through the waist, with the mark floating beside it on a
    /// baseline of its own: a message that broke. Rounded down to the line
    /// below, the field ends the way an elide ends — a whole last line with the mark standing on its tail. One line
    /// at the least: a cap shorter than a line still has to show the line it is cutting, or all that is left is a blank
    /// strip with a mark on it.
    readonly property int capLines: cardText.capRows > 0
                                    ? cardText.capRows
                                    : (cardText.capHeight > 0 && cardText.lineHeight > 0
                                       ? Math.max(1, Math.floor(cardText.capHeight / cardText.lineHeight))
                                       : 0)
    /// The cap left something behind. The output side, and what a headless run reads in place of a mark it cannot see.
    readonly property bool clipped: cardText.capLines > 0 && field.lineCount > cardText.capLines
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText
    /// This is a value a sweep from the gaps around it can land on (`SweepPad`, 規約 §hover のツールチップ). The pad
    /// walks the card it stands under looking for exactly this, so a field added to a card is reachable from the
    /// card's own air the day it is added and nobody has to remember to list it.
    readonly property bool sweepable: true
    /// Whether the keyboard is here — which is what `Ctrl+C` needs, and the half a selection alone does not say.
    readonly property alias hasCaret: field.activeFocus

    /// Where the mark's ground begins: the near edge of the glyph it would otherwise stand on the right half of. Asking
    /// the field which position sits under that edge, and then where that position is, rounds the band out to a
    /// character boundary — a letter cut down its middle is the same broken line `capLines` is for, laid on its side.
    readonly property real markLeft: {
        const want = cardText.width - markLabel.implicitWidth - Theme.spaceXs
        // The guard is also the dependency list: `positionAt` is a call, and a call tells QML nothing about when its
        // answer went stale, so the reads that decide it have to be made here (規約 §QML 実装ルール).
        if (field.text === "" || field.width <= 0 || field.contentHeight <= 0 || want <= 0)
            return Math.max(0, want)
        const under = field.positionAt(want, cardText.height - cardText.lineHeight / 2)
        // `positionAt` answers with the *nearest* boundary, which is the one past the glyph as often as the one before
        // it; the band wants the last boundary that still starts left of the mark, so a nearest answer that overshot is
        // walked back by one.
        const near = field.positionToRectangle(under).x
        const left = near > want && under > 0 ? field.positionToRectangle(under - 1).x : near
        return Math.max(0, Math.min(want, left))
    }

    /// Where one character of the text stands, in this part's own coordinates — the field fills it and keeps no
    /// margin of its own, so the two are the same frame. **What a mark drawn inside the sentence is placed against**
    /// (`SharedToolTip`): a mark is not a character, so the markup opens a gap for it (`Words.roomInSentence`) and
    /// this says where that gap came out. A call, so a caller binding to it reads the field's own answers
    /// (`width`, `text`) in the same binding — the way `markLeft` above does.
    function charRect(pos) {
        return field.positionToRectangle(pos)
    }

    /// Puts the whole field in the selection, for the automation and for `Ctrl+A` (see `selectByKeyboard`).
    function selectAll() {
        field.selectAll()
    }
    function deselect() {
        field.deselect()
    }

    // ---- driven from outside ----------------------------------------
    // For a gesture that began in the card's own air: the pad under the card hands it down
    // in its own coordinates and the field turns it into characters (`SweepPad`, the pair `LineText` carries for the
    // right pane). The anchor is set once, where the drag began, and every move reads from there — the same shape the
    // field's own drag has.
    /// Where the gesture started, in characters.
    property int grabAnchor: 0
    /// A point in another item's coordinates, brought inside this field's own box. **Both axes are clamped**: a field
    /// asked for a position outside its box answers about the line alone, so a sweep that
    /// arrived from the padding beside it picked the same character at both ends of its drag and came away with
    /// nothing (`LineText.onLine`, the same measurement).
    function inBox(item, x, y) {
        const p = cardText.mapFromItem(item, x, y)
        return Qt.point(Math.max(0, Math.min(cardText.width - 1, p.x)),
                        Math.max(0, Math.min(cardText.height - 1, p.y)))
    }
    function anchorFrom(item, x, y) {
        const p = cardText.inBox(item, x, y)
        // The caret comes here, which is also what takes the selection off whatever field was holding one: a field
        // drops its own the moment it loses focus (nothing here is persistent — one selection in the window). A press
        // on these words does the same thing on its own (`activeFocusOnPress`, measured qmltestrunner `tst_cardpad`).
        field.forceActiveFocus()
        cardText.grabAnchor = field.positionAt(p.x, p.y)
        field.select(cardText.grabAnchor, cardText.grabAnchor)
    }
    function extendFrom(item, x, y) {
        const p = cardText.inBox(item, x, y)
        field.select(cardText.grabAnchor, field.positionAt(p.x, p.y))
    }

    /// The width the words want with nothing to wrap them — measured off a ruler, because
    /// **the field's own `implicitWidth` follows its `width` once it wraps**, and a natural width read from there is a
    /// binding standing on its own answer (規約 §QML 実装ルール). Rounded up: a layout hands an item the whole pixel
    /// below a fractional width, and a line asking for 79.28 given 79 wraps against a box meant to hold it.
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: cardText.capLines > 0
                    ? Math.min(field.implicitHeight, cardText.capLines * cardText.lineHeight)
                    : field.implicitHeight
    // Only the height ever cuts, and only when a caller asked for a cap.
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
        // Pinned. What these fields carry is git's own words as often as this application's,
        // and AutoText decides by guessing whether a string looks like markup — a branch called `<b>` would vanish.
        // The one line that is markup says so because a caller built it and escaped everything that went into it.
        textFormat: cardText.markup !== "" ? TextEdit.RichText : TextEdit.PlainText
        color: cardText.color
        onLinkActivated: href => cardText.linkAsked(href)
        font.family: cardText.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: cardText.pixelSize
        font.weight: cardText.weight
        font.underline: cardText.underline
        horizontalAlignment: cardText.horizontalAlignment
        wrapMode: Text.Wrap
        // Read only: the card is a view of a commit, a file or a person, and none of them is edited from here.
        readOnly: true
        // `selectByKeyboard` is false by default on a read-only field, and `Ctrl+A` goes with it — the one gesture a
        // reader who wants the whole line reaches for first.
        selectByMouse: true
        selectByKeyboard: true
        // **One selection in this window, and it belongs to whatever the caret is in.**
        // A card holding several of these would otherwise stay lit in every line the reader had swept, and so
        // would the pane behind it (observed, on the pane's own fields). It was kept for the camera, and
        // the camera never needed it: a field that has not taken focus answers `selectedText` after `selectAll()` all
        // the same, which is the whole of what `tip-copy` reads (measured, qmltestrunner `tst_twofields`).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        // A field of its own width, with no room around it: the card's padding is the card's.
        padding: 0
        textMargin: 0
        HoverHandler {
            id: wordHover
        }
    }

    // What the cap left behind, on the tail of the last line the cap kept — where an elide puts it. On the card's own
    // ground: the line runs on under this, and a mark read through a sentence is not a mark.
    // The band is a line tall so the mark sits on that line's own baseline.
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
            // The field's own, so the mark is cut from the same glyphs as the line it stands on — `mono` moves both.
            font: field.font
        }
    }
}

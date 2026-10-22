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
    /// The same line as rich text the caller builds (`Words.placeInSentence`); empty elsewhere. It replaces the field's
    /// own markup, names and place included. `text` stays the plain sentence either way — that is what callers measure
    /// and read back. The ruler measures whichever is drawn, or a width would be bid for the tags as well.
    property string markup: ""
    /// One word of the line that is a place to go, drawn as a link in `placeTint` (`namedMarkup`) with the line's names
    /// still set as code round it; empty for none. Found outside the names, where the line first says it.
    property string place: ""
    property string placeHref: ""
    property color placeTint: Theme.textMuted
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
    /// Names in the line that wear their kind's mark in front of them (デザイン規約 §ref の種別「名前の印」):
    /// `[at, kind, nameColor, length]` each — `at` where the name begins in `text`, the kind a `NavIcon`'s (empty for a
    /// name drawn apart with no mark), the name's own colour where it says where the reader is (none for any other) and
    /// its length — in the line's order. Where the length is given, the name is set as code (`namedMarkup`): the code
    /// face, in `Words.nameInk`, its mark on the same ground (`codeGrounds`) in its kind's colour
    /// (`NavIcon.inkBeside`).
    /// The field opens a room of no-break spaces at each seat, the mark's cell wide (`roomedText`), and draws the mark
    /// in it — so no line ends between a mark and its name. A copy leaves the rooms out (デザイン規約 §アイコンの席
    /// 「コピー」).
    property var marks: []
    /// Names past the name ceiling (`Theme.nameMaxChars`) cut in the middle — a remote's at its end (デザイン規約 §寸法
    /// 「upstream 名だけは末尾で切る」) — and the whole line given back by a hand on a cut name — a heading's
    /// (デザイン規約 §寸法「名前の天井」). Off, every name is drawn whole and wraps.
    property bool holdsNames: false
    /// The line as drawn, before the rooms: `text` with each name the marks give a length for cut where
    /// `holdsNames` cuts it, the marks moved onto it, and which of them were cut — `{ text, marks, cuts }`.
    readonly property var held: {
        const marks = cardText.marks
        const cap = capRuler.advanceWidth
        // Read so a change of face cuts again: `advanceWidth` and `elidedText` are calls, which a binding does not
        // follow (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
        const face_ = codeFace.font
        let line = ""
        let from = 0
        let shift = 0
        const moved = []
        const cuts = []
        marks.forEach(mark => {
            const length = mark[3] > 0 ? mark[3] : 0
            const name = cardText.text.substr(mark[0], length)
            const shown = cardText.holdsNames && length > 0 && face_ !== null && codeFace.advanceWidth(name) > cap
                          ? codeFace.elidedText(name, mark[1] === "remote" ? Qt.ElideRight : Qt.ElideMiddle, cap)
                          : name
            line += cardText.text.substring(from, mark[0]) + shown
            moved.push([mark[0] + shift, mark[1], mark[2], length > 0 ? shown.length : mark[3]])
            if (shown !== name)
                cuts.push(moved.length - 1)
            shift += shown.length - name.length
            from = mark[0] + length
        })
        return { text: line + cardText.text.substring(from), marks: moved, cuts: cuts }
    }
    /// Each mark's cell, as its drawing reports it: the room is priced off the mark it holds.
    property var markCells: []
    function noteMarkCell(i, cell) {
        const cells = cardText.markCells.slice()
        cells[i] = cell
        cardText.markCells = cells
    }
    /// Each mark's room, the mark's cell wide (`Words.roomFill`) — none until its drawing has said how wide it is.
    readonly property var markRoomTexts: cardText.held.marks.map((mark, i) => {
        const cell = i < cardText.markCells.length && cardText.markCells[i] ? cardText.markCells[i] : 0
        return mark[1] === "" ? "" : Words.roomFill(cell, markSpace.advanceWidth, markThin.advanceWidth)
    })
    /// How many characters each room takes in the field.
    readonly property var markRooms: cardText.markRoomTexts.map(room => room.length)
    /// The drawn line (`held`) with those rooms opened: what the field draws where no `markup` is given.
    readonly property string roomedText: {
        const line = cardText.held.text
        if (cardText.held.marks.length === 0)
            return line
        let out = ""
        let from = 0
        cardText.held.marks.forEach((mark, i) => {
            out += line.substring(from, mark[0]) + cardText.markRoomTexts[i]
            from = mark[0]
        })
        return out + line.substring(from)
    }
    /// The colour mark `i`'s name is drawn in: the rule's where the mark gives the name's length, else the one it gives
    /// (or the line's) — a name not drawn apart keeps the line's colour, and its mark with it.
    function nameInkAt(i) {
        const mark = cardText.held.marks[i]
        if (mark[3] > 0)
            return Words.nameInk(cardText.color, mark[2])
        return mark[2] !== undefined ? mark[2] : cardText.color
    }
    /// `roomedText` as markup, with each name the marks give a length for set as code (`Words.codeMarkup`) in its
    /// colour (`nameInkAt`) and the `place` as its link; empty where neither is in the line, and wherever the caller
    /// hands its own `markup`.
    readonly property string namedMarkup: {
        const line = cardText.held.text
        if (cardText.markup !== "" || (cardText.held.marks.length === 0 && cardText.placeAt < 0))
            return ""
        let out = ""
        let from = 0
        let apart = cardText.placeAt >= 0
        cardText.held.marks.forEach((mark, i) => {
            out += cardText.wordsMarkup(from, mark[0]) + Words.inked(cardText.markRoomTexts[i])
            from = mark[0]
            if (mark[3] > 0) {
                out += Words.codeMarkup(line.substr(mark[0], mark[3]), cardText.nameInkAt(i))
                from = mark[0] + mark[3]
                apart = true
            }
        })
        return apart ? out + cardText.wordsMarkup(from, line.length) : ""
    }
    /// Where the line as drawn first says `place` outside its names; -1 for nowhere.
    readonly property int placeAt: {
        const line = cardText.held.text
        if (cardText.place === "")
            return -1
        let at = line.indexOf(cardText.place)
        while (at >= 0 && cardText.held.marks.some(mark => mark[3] > 0 && at < mark[0] + mark[3]
                                                     && at + cardText.place.length > mark[0]))
            at = line.indexOf(cardText.place, at + 1)
        return at
    }
    /// The words of the drawn line from `from` to `to` as markup, the place's link where it falls among them.
    function wordsMarkup(from, to) {
        const line = cardText.held.text
        const at = cardText.placeAt
        if (at < from || at + cardText.place.length > to)
            return Words.inked(line.substring(from, to))
        return Words.inked(line.substring(from, at))
             + "<a href=\"" + cardText.placeHref + "\"><font color=\"" + cardText.placeTint + "\">"
             + Words.inked(cardText.place) + "</font></a>" + Words.inked(line.substring(at + cardText.place.length, to))
    }
    /// Where each name's code ground runs (`CodeGround`), a piece per line the name takes: `{x, y, width}` in this
    /// part's coordinates, `y` the line's middle. From the mark's ink — or the name's start where it has none — to the
    /// name's end, half a gap off the ink at both ends (`WordFace.groundStart` / `groundEnd`); a piece a wrap ends or
    /// starts stands half a gap off the glyphs there. One binding over its inputs, since `positionToRectangle` and the
    /// marks' items are calls (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
    readonly property var codeGrounds: {
        const rooms = cardText.markRooms
        const drawn = lineMarks.count
        if (cardText.namedMarkup === "" || field.text === "" || cardText.width <= 0 || cardText.height <= 0)
            return []
        const half = Theme.spaceXs / 2
        const line = cardText.held.text
        // The field takes a new line after the marks made for it: until it has, a name can lie past its end, and
        // Qt warns at every position asked for there. The binding runs again once the field has the line.
        const length = field.length
        const pieces = []
        cardText.held.marks.forEach((mark, i) => {
            const start = cardText.markRoomAt(i) + rooms[i]
            const end = start + mark[3]
            if (!(mark[3] > 0) || end > length)
                return
            const icon = rooms[i] > 0 && i < drawn ? lineMarks.itemAt(i) : null
            let left = icon ? WordFace.groundStart(icon.x + icon.inkLeft) : field.positionToRectangle(start).x - half
            for (let at = start; at < end; at++) {
                const here = field.positionToRectangle(at)
                const next = field.positionToRectangle(at + 1)
                const sameLine = Math.abs(next.y - here.y) < 1
                if (at + 1 < end && sameLine)
                    continue
                // The name's end on this line: where the next character stands, or past this one's own advance where
                // the field wraps after it — and at the name's end, half a gap off its ink, as before the mark.
                const right = sameLine ? next.x : here.x + codeFace.advanceWidth(line.charAt(mark[0] + at - start))
                const end_ = at + 1 === end && icon
                             ? WordFace.groundEnd(codeFace, line.substr(mark[0], mark[3]), right) : right + half
                pieces.push({ x: left, y: here.y + here.height / 2, width: end_ - left })
                left = next.x - half
            }
        })
        return pieces
    }
    /// What the field draws, and whether as markup.
    readonly property string drawnText: cardText.markup !== "" ? cardText.markup
                                      : cardText.namedMarkup !== "" ? cardText.namedMarkup : cardText.roomedText
    readonly property bool drawnRich: cardText.markup !== "" || cardText.namedMarkup !== ""
    /// Where the first mark's name came out (empty while it has no room), and that mark's cell — what a run reads
    /// (`NoticeBar.markSeat`).
    readonly property rect firstMarkSeat: {
        const rooms = cardText.markRooms
        if (rooms.length === 0 || rooms[0] <= 0 || field.text === "" || cardText.width <= 0 || cardText.height <= 0
                || cardText.markRoomAt(0) + rooms[0] > field.length)
            return Qt.rect(0, 0, 0, 0)
        return cardText.charRect(cardText.markRoomAt(0) + rooms[0])
    }
    readonly property real firstMarkCell: cardText.markCells.length > 0 && cardText.markCells[0]
                                          ? cardText.markCells[0] : 0
    /// Where mark `i`'s room starts in the field, every room before it counted.
    function markRoomAt(i) {
        let at = cardText.held.marks[i][0]
        for (let j = 0; j < i; j++)
            at += cardText.markRooms[j]
        return at
    }
    /// Every room the line holds, in the field's characters: `[at, length]` each.
    readonly property var allRooms: {
        const all = []
        cardText.markRooms.forEach((room, i) => {
            if (room > 0)
                all.push([cardText.markRoomAt(i), room])
        })
        return all
    }

    /// The pointer is on the words. The card ORs this into its `pointerInside` (`AppCard.contentPointed` —
    /// rules-refs/app-ui.md「hover で開くものの 5 つの罠」の (2)).
    readonly property alias pointed: wordHover.hovered
    /// A hand on a name `holdsNames` cut — what puts the whole line up. One binding over its inputs, since
    /// `positionAt` is a call.
    readonly property bool cutPointed: {
        const cuts = cardText.held.cuts
        if (cuts.length === 0 || !wordHover.hovered || field.text === "" || field.width <= 0)
            return false
        const at = field.positionAt(wordHover.point.position.x, wordHover.point.position.y)
        return cuts.some(i => {
            const start = cardText.markRoomAt(i) + cardText.markRooms[i]
            return at >= start && at <= start + cardText.held.marks[i][3]
        })
    }
    /// The whole line's names behind their marks, for the tip the cut puts up (`SharedToolTip.tipMarks`).
    readonly property var tipMarks: cardText.held.cuts.length > 0 ? cardText.marks : []
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
    /// in its room is placed against (`marks`). A call, so a caller binding to it reads `width` / `text` in the same
    /// binding, as `markLeft` does.
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
    /// The selection as copied: the line as drawn (`held` — a cut name goes out cut, as read) over the same span, the
    /// rooms taken out.
    function copiedText() {
        const plain = pos => {
            let taken = 0
            for (const room of cardText.allRooms)
                taken += Math.max(0, Math.min(room[1], pos - room[0]))
            return pos - taken
        }
        return cardText.held.text.substring(plain(field.selectionStart), plain(field.selectionEnd))
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
    /// The baseline of the line a character stands on (`charRect`): what a mark set in the sentence is drawn against
    /// (`NavIcon.wordBase`). The field files the face's leading under each line (`QTextDocumentLayout`), so the
    /// line's top is `ascent` over its baseline, as a `Text`'s is.
    function baselineAt(rect) {
        return rect.y + face.ascent
    }
    /// The first line's, so a row can stand the sentence on another item's baseline (`Qt.AlignBaseline`), as
    /// `LineText`'s.
    baselineOffset: field.y + field.baselineOffset
    clip: cardText.clipped
    // The whole line, where a name was cut (`holdsNames`).
    ToolTip.visible: cardText.cutPointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: cardText.text

    FontMetrics {
        id: face
        font: field.font
    }
    /// The face the names are set in (`Words.codeMarkup`), at the line's size and weight.
    FontMetrics {
        id: codeFace
        font.family: Theme.monoFamily
        font.pixelSize: cardText.pixelSize
        font.weight: cardText.weight
    }
    Text {
        id: ruler
        visible: false
        text: cardText.drawnText
        textFormat: cardText.drawnRich ? Text.RichText : Text.PlainText
        font: field.font
        wrapMode: Text.NoWrap
    }

    // The names' code grounds, under the words (`codeGrounds`).
    Repeater {
        model: cardText.codeGrounds
        CodeGround {
            required property var modelData
            x: modelData.x
            y: Math.round(modelData.y - height / 2)
            width: modelData.width
            size: cardText.pixelSize
        }
    }
    TextEdit {
        id: field
        width: cardText.width
        text: cardText.drawnText
        // Pinned: AutoText guesses by whether a string looks like markup, and git's words can (a branch called `<b>`
        // would vanish). `markup` is escaped by the caller that built it, `namedMarkup` here.
        textFormat: cardText.drawnRich ? TextEdit.RichText : TextEdit.PlainText
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
        // Copied here where the line holds a room: the field's own copy would carry its spaces out.
        Keys.onPressed: event => {
            if (copier.item && event.matches(StandardKey.Copy) && field.selectedText !== "") {
                copier.item.copy(cardText.copiedText())
                event.accepted = true
            }
        }
    }
    // The name ceiling, in the names' own face (`holdsNames`).
    TextMetrics {
        id: capRuler
        font: codeFace.font
        text: "n".repeat(Theme.nameMaxChars)
    }
    /// A narrow no-break space, the room's finer step (`Words.roomFill`).
    TextMetrics {
        id: markThin
        font: field.font
        text: "\u202f"
    }
    /// A space in the field's own font and weight, so a family with a wider space opens a wider room.
    TextMetrics {
        id: markSpace
        font: field.font
        text: "\u00a0"
    }
    // The marks, each in the room opened in front of its name, against that name's baseline (`baselineAt`) with a
    // step of the box under it for a stem that runs past it.
    Repeater {
        id: lineMarks
        model: cardText.marks.length
        NavIcon {
            id: lineMark
            required property int index
            /// Where the name came out (empty when it has no room yet). One binding over its inputs, since `charRect`
            /// is a call (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
            readonly property rect seat: {
                const rooms = cardText.markRooms
                if (lineMark.index >= rooms.length || rooms[lineMark.index] <= 0 || field.text === ""
                        || cardText.width <= 0 || cardText.height <= 0
                        || cardText.markRoomAt(lineMark.index) + rooms[lineMark.index] > field.length)
                    return Qt.rect(0, 0, 0, 0)
                return cardText.charRect(cardText.markRoomAt(lineMark.index) + rooms[lineMark.index])
            }
            /// A name drawn apart with no mark of its own (`marks`' kind left empty).
            readonly property bool unmarked: lineMark.index < cardText.marks.length
                                             && cardText.marks[lineMark.index][1] === ""
            /// The room this mark takes: its cell, its air both sides.
            readonly property real room: lineMark.wordCell
            visible: !lineMark.unmarked && (lineMark.seat.width > 0 || lineMark.seat.height > 0)
            kind: lineMark.index < cardText.marks.length && !lineMark.unmarked ? cardText.marks[lineMark.index][1]
                                                                               : "tree"
            // A sign on its name (デザイン規約 §アイコンの席 の修飾), a hair off it (`NavIcon.wordAir`).
            set: "sign"
            size: cardText.pixelSize
            tint: lineMark.inkBeside(lineMark.index < cardText.marks.length ? cardText.nameInkAt(lineMark.index)
                                                                            : cardText.color)
            wordBase: Math.ceil(lineMark.size)
            width: lineMark.wordCell
            height: lineMark.wordBase + Theme.spaceXs
            x: Math.round(lineMark.seat.x - lineMark.width)
            y: Math.round(cardText.baselineAt(lineMark.seat)) - lineMark.wordBase
            onRoomChanged: cardText.noteMarkCell(lineMark.index, lineMark.room)
            Component.onCompleted: cardText.noteMarkCell(lineMark.index, lineMark.room)
        }
    }
    // Built only on a line that holds a room: a helper is a second field, and most lines copy as they are.
    Loader {
        id: copier
        active: cardText.allRooms.length > 0
        sourceComponent: ClipboardHelper {}
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

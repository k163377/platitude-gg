import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line of words with its names set as code (デザイン規約 §ref の種別「文の中の名前」), never wrapped: the words and
// the names side by side, each name behind its kind's mark where it has one. Where the column is too narrow it is cut
// one of two ways (`cutAt`):
// - `end`: the piece the column's edge falls in cut there with `…` (`CutName` / `CodeChip`, `cutAt: "end"`), nothing
//   after it — a discard list row, whose own mark says the kind and whose hover card says the rest. What was done stays
//   readable; the name gives way.
// - `middle`: the names give way in the middle, never the words round them, the longer first down to an even share
//   (デザイン規約 §メニュー「入りきらない行は名前だけを真ん中で縮める」, a remote's at its end — §寸法) — a pane's line
//   (`DiffFileNotices`' one-sided conflict), whose whole line a hand on it gets back.
Item {
    id: line

    property string text: ""
    /// Where the names stand in `text`: `[at, kind, own, length]`, as `CardText.marks` — an empty kind for a name with
    /// no mark of its own.
    property var marks: []
    property real pixelSize: Theme.fontMd
    property color color: Theme.textPrimary
    /// `end` or `middle` — see above.
    property string cutAt: "end"

    /// The words and the names in order: `{text, code, kind, own}`.
    readonly property var pieces: {
        const out = []
        let from = 0
        const named = line.marks.filter(mark => mark[3] > 0).sort((a, b) => a[0] - b[0])
        for (const mark of named) {
            if (mark[0] > from)
                out.push({ text: line.text.substring(from, mark[0]), code: false, kind: "", own: undefined })
            out.push({ text: line.text.substr(mark[0], mark[3]), code: true, kind: mark[1], own: mark[2] })
            from = mark[0] + mark[3]
        }
        if (from < line.text.length)
            out.push({ text: line.text.substring(from), code: false, kind: "", own: undefined })
        return out
    }
    /// Whether the line was cut to the column.
    readonly property bool cutting: row.natural > line.width + 0.5
    /// The width each piece asks for: its own, or `middle`'s share of what the words leave for a name. A piece is
    /// still held to what is left of the column where it stands.
    readonly property var wants: {
        const naturals = []
        for (let i = 0; i < row.built; i++) {
            const piece = pieceRepeater.itemAt(i)
            naturals.push(piece === null ? 0 : piece.natural)
        }
        if (line.cutAt !== "middle" || !line.cutting)
            return naturals
        const names = []
        let room = line.width
        naturals.forEach((natural, i) => {
            if (i < line.pieces.length && line.pieces[i].code)
                names.push(i)
            else
                room -= natural
        })
        // The narrowest keep their width while it is under an even share of what is left; the rest take that share.
        names.sort((a, b) => naturals[a] - naturals[b])
        const out = naturals.slice()
        for (let k = 0; k < names.length; k++) {
            const even = Math.max(0, room) / (names.length - k)
            if (naturals[names[k]] <= even) {
                room -= naturals[names[k]]
                continue
            }
            for (let j = k; j < names.length; j++)
                out[names[j]] = even
            break
        }
        return out
    }
    /// The whole line's names behind their marks, for the tip a cut line puts up (`TipAsk.marksNow`).
    readonly property var tipMarks: line.marks

    implicitHeight: baseRef.implicitHeight
    // Every piece stands on the words' baseline.
    baselineOffset: baseRef.baselineOffset
    /// The whole line, where a name was cut (`middle`): the ask for the window's tooltip, whose names behind their
    /// marks the seat reads off this line (`tipMarks`).
    readonly property alias tip: lineTip
    readonly property TipAsk lineTip: TipAsk {
        id: lineTip
        target: line
        asked: line.cutAt === "middle" && line.cutting && lineHover.hovered
        words: line.text
    }

    HoverHandler {
        id: lineHover
        enabled: line.cutAt === "middle"
    }
    // The words' line and baseline in this box, for the chips to stand on and a row's mark to be set against.
    CutName {
        id: baseRef
        visible: false
        width: line.width
        height: line.height
        text: "x"
        pixelSize: line.pixelSize
    }
    Row {
        id: row
        /// How many pieces have been built — what the bindings reading the pieces walk, not the count: `itemAt` is a
        /// call, so a binding hears only what it walks by, and a line built in the background (a list's row past its
        /// edge) is counted before its pieces are built.
        property int built: 0
        /// The whole line's width uncut.
        readonly property real natural: {
            let sum = 0
            for (let i = 0; i < row.built; i++) {
                const piece = pieceRepeater.itemAt(i)
                sum += piece === null ? 0 : piece.natural
            }
            return sum
        }
        height: line.height
        Repeater {
            id: pieceRepeater
            model: line.pieces
            onItemAdded: row.built++
            onItemRemoved: row.built--
            delegate: Item {
                id: piece
                required property var modelData
                required property int index
                readonly property real natural: piece.modelData.code ? chip.wantWidth : words.implicitWidth
                /// What it asks for, no further than what is left of the column at this piece — nothing once the edge
                /// is passed. Kept in the row even then (a `Row` leaves a hidden child where it was): what it draws
                /// goes instead.
                readonly property real room: Math.max(0, Math.min(piece.index < line.wants.length
                                                                  ? line.wants[piece.index] : piece.natural,
                                                                  line.width - piece.x))
                // A name cut in the middle takes only what it draws: its cut drops whole glyphs, and the words after
                // it close up. One cut at the end keeps the rest of the column, so nothing after it is drawn.
                width: line.cutAt === "middle" && piece.modelData.code && chip.cut
                       ? Math.min(piece.room, chip.drawnWidth) : piece.room
                height: line.height
                CutName {
                    id: words
                    visible: !piece.modelData.code && piece.room > 0
                    anchors.fill: parent
                    text: piece.modelData.code ? "" : piece.modelData.text
                    cutAt: "end"
                    pixelSize: line.pixelSize
                    color: line.color
                }
                CodeChip {
                    id: chip
                    visible: piece.modelData.code && piece.room > 0
                    y: baseRef.baselineOffset - chip.baselineOffset
                    width: piece.room
                    cutToWidth: true
                    cutAt: line.cutAt === "middle" && piece.modelData.kind !== "remote" ? "middle" : "end"
                    word: piece.modelData.code ? piece.modelData.text : ""
                    mark: piece.modelData.kind
                    size: line.pixelSize
                    tint: Words.nameInk(line.color, piece.modelData.own)
                }
            }
        }
    }
}

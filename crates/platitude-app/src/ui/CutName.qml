import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A name in a column narrower than itself, cut so the column keeps both of its edges
// (規約 §寸法「列に並ぶ名前の省略は、余りを切れ目へ入れる」). `Text.ElideMiddle` alone draws what fits from the left, so the
// unused width piles up at the right edge, differently on every row — the column's edge frays, and the hex tails that
// tell generated branch names apart stop lining up; and it can stop up to two characters short, as it gives one back
// to each end at a time. So the cut is filled out here a character at a time, the column used to its edge, and the
// leftover (under one character) goes into the cut, half either side of the mark: there it reads as "something is
// missing", never as a space inside a word, and neither edge moves.
//
// A sentence (`cutAt: "end"`, a commit's subject) keeps only its head, filled the same way, with the mark pinned to
// the column's right edge (規約 §寸法「文の省略は末尾で、印は列の右辺に貼る」): a sentence has no telling tail to keep,
// and a mark set right after the text lands where the elide stopped — a character short on some rows — so the `…`
// would stand at a different x on every row. The leftover goes in front of the mark.
//
// The name in full stays the row's hover (規約 §hover のツールチップ).
Item {
    id: cut

    /// The name in full.
    property string text: ""
    property color color: Theme.textPrimary
    property int weight: Font.Normal
    property real pixelSize: Theme.fontMd
    /// Letter spacing (the tab strip's tracking, `TabMetrics.titleTracking`). Set it here, not on the labels: the
    /// rulers measure through this font, so spacing set past them draws wider than it measured.
    property real letterSpacing: 0
    /// Set in git's own family, as a name set as code is (`NameRun`, デザイン規約 §ref の種別「文の中の名前」).
    property bool mono: false
    /// Where the mark falls: `middle` for a name (told apart by both of its ends), `end` for a sentence (read from the
    /// left, and nothing after the cut is worth keeping).
    property string cutAt: "middle"
    /// Whether the pieces are drawn. False where something else draws the same name here (`NameCell.whole`); the cut
    /// is still worked out, so the column keeps its width and `cutting` keeps answering.
    property bool inked: true

    /// The two halves that are drawn, and whether the mark stands between them — `cutting` is how a caller asks
    /// whether the name was cut (`Text.truncated` means nothing here). Written by `relayout()`: a binding that writes
    /// to the ruler it reads would retrigger itself forever.
    property string headText: ""
    property string tailText: ""
    property bool cutting: false
    /// Where the ink ends — the whole column once there is a cut in the name, the name's own width while there is not.
    /// What a mark on the name's shoulder is placed against.
    readonly property real inkWidth: cut.cutting ? cut.width : headLabel.implicitWidth

    function relayout() {
        const whole = cut.text
        // The whole name goes onto `measure` here, before any branch, so `implicitWidth` never lags the text (a
        // binding would answer `textChanged` in an unwritten order). The fit test below reuses it, and an unchanged
        // text (a column drag) costs no layout.
        measure.font = headLabel.font
        measure.text = whole
        // Both rulers take the face here too: a run from the label's `fontChanged` comes before a binding on that font
        // has answered, and would cut for the face the label has just left (a menu's family arrives after its rows).
        probe.font = headLabel.font
        // Read after that write: a caller sizing the box off `implicitWidth` (a tab is as wide as its name:
        // `TabItemDelegate.width` → `titleEase` → this item's margin) re-enters this function from those two lines,
        // and a width read earlier would lay a cut for a column that is gone back over the nested answer. `TabPin`
        // runs the same arithmetic and comes out right — "one name, one width, crushed in one place only" is this
        // defect's shape. It shows in no width and no picture: `crushed=` / `kept=` of `tab-widths` / `tab-pin` /
        // `tab-pin-go` say it (`TabProbe.tabNamesCrushed`, `nameKept`). Pinned without a window by `tst_tabwidths.qml`
        // (`test_a_name_is_cut_against_the_column_it_ends_up_in_not_the_one_it_replaced`).
        const box = cut.width
        // No name or no column yet (a delegate is built before layout): the anchored labels would otherwise draw the
        // whole name over the row's neighbours.
        if (whole === "" || box <= 0) {
            cut.headText = ""
            cut.tailText = ""
            cut.cutting = false
            return
        }
        if (measure.advanceWidth <= box) {
            cut.headText = whole
            cut.tailText = ""
            cut.cutting = false
            return
        }
        // Start from the elide's answer: it breaks only where text may break (never inside a surrogate pair or a
        // cluster).
        probe.elideWidth = box
        probe.text = whole
        const elided = probe.elidedText
        const at = cut.cutPoint(whole, elided)
        const toEnd = cut.cutAt === "end"
        let head = at >= 0 ? at : 0
        let tail = !toEnd && at >= 0 ? elided.length - at - 1 : 0
        const room = box - cut.inkOf("…")
        let headInk = cut.inkOf(whole.substring(0, head))
        let tailInk = cut.inkOf(whole.substring(whole.length - tail))
        // …which can run past the column (a middle cut of a small face, by a few pixels — `fontSm`, the WORKTREES rows'
        // branch column, `NavRowBody.branchSeat`): the tail would then be drawn over the mark. Taken back a character
        // at a time from the longer half until it fits; `tst_cutname.qml` walks the width a pixel at a time.
        while (headInk + tailInk > room && head + tail > 0) {
            if (head >= tail) {
                head -= cut.stepBack(whole, head)
                headInk = cut.inkOf(whole.substring(0, head))
            } else {
                tail -= cut.stepOn(whole, whole.length - tail)
                tailInk = cut.inkOf(whole.substring(whole.length - tail))
            }
        }
        // Fill the column, the tail first: a generated name is told apart by its end
        // (規約 §git 用語のコード表記「ref の省略は中央」). A cut at the end grows only the head.
        // A character at a time, not a binary search (built, measured, no difference): the elide's answer starts within
        // two characters of the fit, so this goes round two or three times. Propose a search again only with a new
        // measurement; what cutting costs is measuring the whole name (`measure`, and `probe`'s elide), not this loop.
        for (;;) {
            let grew = false
            const back = toEnd ? 0 : cut.stepBack(whole, whole.length - tail)
            if (back > 0 && head + tail + back <= whole.length) {
                const ink = cut.inkOf(whole.substring(whole.length - tail - back))
                if (headInk + ink <= room) {
                    tail += back
                    tailInk = ink
                    grew = true
                }
            }
            const on = cut.stepOn(whole, head)
            if (on > 0 && head + tail + on <= whole.length) {
                const ink = cut.inkOf(whole.substring(0, head + on))
                if (ink + tailInk <= room) {
                    head += on
                    headInk = ink
                    grew = true
                }
            }
            if (!grew)
                break
        }
        cut.headText = whole.substring(0, head)
        cut.tailText = tail > 0 ? whole.substring(whole.length - tail) : ""
        cut.cutting = true
    }

    /// The advance of a piece of the name. Only `advanceWidth` equals the labels' `implicitWidth`; TextMetrics' other
    /// measures do not (`BandWidest`).
    function inkOf(part) {
        probe.text = part
        return probe.advanceWidth
    }

    /// One character on from `at`, and one back — in code points so a surrogate pair is never halved, and past
    /// anything with no advance of its own, so a combining accent stays with its letter.
    function stepOn(s, at) {
        let n = 0
        while (at + n < s.length) {
            const c = s.charCodeAt(at + n)
            n += c >= 0xD800 && c <= 0xDBFF && at + n + 1 < s.length ? 2 : 1
            if (cut.inkOf(s.substring(at, at + n)) > 0)
                break
        }
        return n
    }
    function stepBack(s, at) {
        let n = 0
        while (at - n > 0) {
            const c = s.charCodeAt(at - n - 1)
            n += c >= 0xDC00 && c <= 0xDFFF && at - n - 1 > 0 ? 2 : 1
            if (cut.inkOf(s.substring(at - n, at)) > 0)
                break
        }
        return n
    }
    /// Where the elide put its mark — searched for, since a name may hold its own `…` (a stash is named by its
    /// message): the first mark with a real head and tail of the name around it.
    function cutPoint(whole, elided) {
        for (let i = elided.indexOf("…"); i >= 0; i = elided.indexOf("…", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return i
        }
        return -1
    }

    // Rounded up once here for every caller (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
    implicitWidth: Math.ceil(measure.advanceWidth)
    implicitHeight: markLabel.implicitHeight
    // The label's own, its centring in the box included: a mark set beside the name stands on it (`NavIcon.wordBase`).
    baselineOffset: headLabel.baselineOffset
    onTextChanged: cut.relayout()
    onWidthChanged: cut.relayout()
    Component.onCompleted: cut.relayout()

    /// Two rulers, both written by `relayout()` — text and face: `measure` holds the whole name (the fit test and
    /// `implicitWidth`), `probe` the pieces — set a dozen times per relayout, so nothing binds to it.
    TextMetrics {
        id: measure
    }
    TextMetrics {
        id: probe
        elide: cut.cutAt === "end" ? Text.ElideRight : Text.ElideMiddle
    }

    // The three pieces, centred vertically (`Text.AlignVCenter`): a `Label` draws at the top of its box, and callers
    // pass boxes taller than the line on purpose — `CarriedPane`'s name seat, `DiffPaneHeader`, `CloneDialog`'s framed
    // field — where the name alone would float up off the heading beside it and off a tree mark set by it. Checked by
    // `carried-stand topic --preset carried`, `carried-read <name> --preset carried`, `clone-dialog` (a modal box:
    // overlay.png).
    Label {
        id: headLabel
        visible: cut.inked
        anchors.fill: parent
        verticalAlignment: Text.AlignVCenter
        text: cut.headText
        textFormat: Text.PlainText
        color: cut.color
        font.family: cut.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: cut.pixelSize
        font.weight: cut.weight
        font.letterSpacing: cut.letterSpacing
        // The family arrives with the window and the size with the caller; either changes what fits.
        onFontChanged: cut.relayout()
    }
    Label {
        id: tailLabel
        anchors.fill: parent
        horizontalAlignment: Text.AlignRight
        verticalAlignment: Text.AlignVCenter
        visible: cut.inked && cut.tailText !== ""
        text: cut.tailText
        textFormat: Text.PlainText
        color: cut.color
        font: headLabel.font
    }
    Label {
        id: markLabel
        y: 0
        height: cut.height
        visible: cut.inked && cut.cutting
        verticalAlignment: Text.AlignVCenter
        text: "…"
        color: cut.color
        font: headLabel.font
        // In the middle of the cut, the sub-character leftover split either side of the mark, and never left of the
        // head's end (a hair-wide measure would draw the mark over it). A cut at the end pins it to the right edge.
        x: cut.cutAt === "end"
           ? cut.width - markLabel.implicitWidth
           : Math.max(headLabel.implicitWidth,
                      (headLabel.implicitWidth + cut.width - tailLabel.implicitWidth - markLabel.implicitWidth) / 2)
    }
}

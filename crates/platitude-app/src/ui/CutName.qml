import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A name standing in a column narrower than itself: cut in the middle, and cut so that **the column keeps both of its
// edges**. `Text.ElideMiddle` on its own keeps neither — it hands back whatever fits and the Label draws that from the
// left, so everything it could not use piles up at the right edge. Down a list of names the pile is a different width
// on every row and the column's right edge frays: the six hex a generated branch name is told apart by stop lining up.
//
// The leftover is real, and bigger than it looks: the elide gives a character back to *each* end at a time, so it can
// stop as much as two characters short of what fits (measured, Yu Gothic UI `fontMd`, boxes 40..240: 1〜18px).
// So it is moved twice. **The cut is worked out here rather than left to the elide**, one character at a time, until
// the name fills its column — 規約 §git 用語のコード表記「句は枠いっぱいまで使う — 余りを残して先に省略しない」, read on a
// name. What is left after that is under one character, and it goes **into the cut**, where the mark already says
// something was taken out. Neither edge of the column moves, and no row's ink stops short of it.
//
// **A sentence is cut at its end instead** (`cutAt: "end"` — a commit's subject). It has no tail the reader tells it
// apart by, so nothing is kept past the mark; but the elide frays the same way, one character at a time, so the head is
// filled out here in the same loop and **the mark is pinned to the column's right edge**. Down the graph every `…`
// stands at one x (規約 §タイポグラフィ「文の省略は末尾で、印は列の右辺に貼る」).
//
// The name in full stays the row's hover (規約 §hover のツールチップ) — this part only decides what is on screen.
Item {
    id: cut

    /// The name in full. What is drawn is a part of it whenever the column is narrower.
    property string text: ""
    property color color: Theme.textPrimary
    property int weight: Font.Normal
    property real pixelSize: Theme.fontMd
    /// How far apart the letters are set — 0 everywhere but the tab strip, which opens a short name's tracking
    /// (`TabMetrics.titleTracking`). It arrives here rather than on the drawn half alone because the cut is worked out
    /// through these labels' own font: spacing set past the rulers would measure one name and draw a wider one.
    property real letterSpacing: 0
    /// Where the mark falls: `middle` for a name (told apart by both of its ends), `end` for a sentence (read from the
    /// left, and nothing after the cut is worth keeping).
    property string cutAt: "middle"

    /// The two halves that are drawn, and whether the mark stands between them — the output side, which is also how a
    /// caller asks whether this name was cut at all (`Text.truncated` has no meaning here: neither half is elided).
    ///
    /// **Written by `relayout()` rather than bound.** Working the cut out means measuring candidate strings, and a
    /// binding that writes to the ruler it reads would retrigger itself for ever.
    property string headText: ""
    property string tailText: ""
    property bool cutting: false
    /// Where the ink ends — the whole column once there is a cut in the name, the name's own width while there is not.
    /// What a mark on the name's shoulder is placed against.
    readonly property real inkWidth: cut.cutting ? cut.width : headLabel.implicitWidth

    /// Decide the cut and hand the two halves to the labels below.
    function relayout() {
        const whole = cut.text
        // The whole name goes onto `measure` here, imperatively — written before any branch so the ruler behind
        // `implicitWidth` never lags the text, and written by this handler rather than bound because a binding and
        // this handler answer the same `textChanged` in an order nobody has written down. Owning the write is also
        // what lets the fitness test below read `measure` instead of laying the whole name out a *second* time on
        // `probe` — and a relayout whose text has not changed (a column drag, a window resize) lays it out no times:
        // TextMetrics does nothing for a value it already holds.
        measure.font = headLabel.font
        measure.text = whole
        // The column, read **after** that write and never before it. `implicitWidth` is this ruler's answer, so a
        // caller that sizes the box off the name's own width — a tab is drawn as wide as what it is called
        // (`TabItemDelegate`) — moves the column from inside those two lines and re-enters this function on the way.
        // The nested run settles the name against the column it now has; read first, this one would lay its own
        // answer, worked out against the column that is gone, back over it. What that leaves is a name cut for a box
        // narrower than the one it is drawn in — the mark standing alone where two letters fit (`TabProbe`,
        // `PGG_AUTO_ACT=tab-widths`).
        const box = cut.width
        // No name, or no column yet — a delegate is built before the layout has given it one, and the labels below
        // are anchored to the item rather than elided into it, so a name drawn against a box of nothing would be
        // drawn in full over whatever the row keeps beside it.
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
        // Where to start: the elide's own answer. It fits, and it falls where the text may be broken — which is what
        // keeps the loop below off the inside of a surrogate pair or a cluster.
        probe.elideWidth = box
        probe.text = whole
        const elided = probe.elidedText
        const at = cut.cutPoint(whole, elided)
        const toEnd = cut.cutAt === "end"
        let head = at >= 0 ? at : 0
        let tail = !toEnd && at >= 0 ? elided.length - at - 1 : 0
        // Fill the column. The tail grows first: a generated name is told apart by its end (規約 §git 用語のコード表記
        // 「ref の省略は中央 — ブランチ名は末尾で見分ける」). A cut at the end has no tail at all, so only the head grows
        // and what the fill cannot use is left in front of the mark, which is already standing at the column's edge.
        const room = box - cut.inkOf("…")
        let headInk = cut.inkOf(whole.substring(0, head))
        let tailInk = cut.inkOf(whole.substring(whole.length - tail))
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

    /// The advance of a piece of the name, measured the way the labels below draw it — `TextMetrics.advanceWidth` and
    /// `Label.implicitWidth` are the same number here (measured, eight names, diff 0 at every width). The other
    /// measures TextMetrics carries are not (`BandWidest`), so nothing else is read off it.
    function inkOf(part) {
        probe.text = part
        return probe.advanceWidth
    }

    /// One character on from `at`, and one back from it — counted in code points so a surrogate pair is never halved,
    /// and carried on past anything that has no advance of its own: a combining accent belongs to the letter it stands
    /// on, and a cut taken between the two would open the tail with a mark on nothing.
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
    /// Where the elide put its mark. **Looked for rather than assumed** — `…` is a character a name may hold of its own
    /// (a stash is named by its message, 規約 §左メニューの所作), so the answer is the first mark that leaves a real head
    /// and a real tail of the name behind it.
    function cutPoint(whole, elided) {
        for (let i = elided.indexOf("…"); i >= 0; i = elided.indexOf("…", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return i
        }
        return -1
    }

    // Rounded up: a layout hands an item the whole pixel below a fractional width, and a name asking for 79.28 given
    // 79 cuts itself against a column meant to hold it (app-ui.md §自然幅の上限は切り上げる — the same fault the author
    // row hit, answered once here for every column this part stands in).
    implicitWidth: Math.ceil(measure.advanceWidth)
    implicitHeight: markLabel.implicitHeight
    onTextChanged: cut.relayout()
    onWidthChanged: cut.relayout()
    Component.onCompleted: cut.relayout()

    /// Two rulers, both written by `relayout()`. `measure` carries the whole name — laid out once there, read for
    /// the fitness test and, through the `implicitWidth` binding above, for the item's own size, so the name is never
    /// laid out twice for one relayout. `probe` is set and read a dozen times inside one `relayout()` on pieces of
    /// the name, and nothing binds to it.
    TextMetrics {
        id: measure
    }
    TextMetrics {
        id: probe
        font: headLabel.font
        elide: cut.cutAt === "end" ? Text.ElideRight : Text.ElideMiddle
    }

    // The three pieces. Head and tail fill the item and let their own alignment hold them to its edges — the same
    // shape a single Label had, so a column that stretches this part draws its name exactly where it drew it before.
    Label {
        id: headLabel
        anchors.fill: parent
        text: cut.headText
        color: cut.color
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
        visible: cut.tailText !== ""
        text: cut.tailText
        color: cut.color
        font: headLabel.font
    }
    Label {
        id: markLabel
        y: 0
        height: cut.height
        visible: cut.cutting
        text: "…"
        color: cut.color
        font: headLabel.font
        // In the middle of the cut. What the fill could not use is under one character, and halving it either side of
        // the mark keeps it from reading as a space in the name. Never left of the head — a measure that came out a
        // hair wide would otherwise draw the mark over it.
        //
        // **A cut at the end puts it on the column's right edge instead**, which is the whole point of that mode: a
        // mark set against the head lands on a different x every row and frays the column. The leftover — under one
        // character — falls in front of it, where the mark already says something was taken out.
        x: cut.cutAt === "end"
           ? cut.width - markLabel.implicitWidth
           : Math.max(headLabel.implicitWidth,
                      (headLabel.implicitWidth + cut.width - tailLabel.implicitWidth - markLabel.implicitWidth) / 2)
    }
}

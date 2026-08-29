pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One git invocation in the command log: the clock, the command, and how it went. A failure keeps git's own parting
// words under it — the whole point of the panel is that nothing is rephrased.
//
// **The three are columns of one line** (デザイン規約 §git が言ったことを読む場所): a drag over the log picks the line's own
// characters out of them, so each column is a run of the mono font at a place this row can name, and the two gaps
// between them are the tabs a copy comes out with. That is why the command's two halves are a character apart rather
// than a spacing token — `git` and what follows it are one string, drawn in two colours.
Rectangle {
    id: row

    required property int index
    required property string clock
    required property string args
    required property string full
    required property string state
    required property string result
    required property string duration
    required property string output
    /// Where the reader's own selection falls on this row (`CommandsModel.sel`): a run for each of the three columns
    /// and a last field saying whether the line was taken end to end, which is what brings git's words with it.
    /// Empty on a row the selection does not reach.
    required property string sel

    /// One measured column of the mono font and what a wide glyph costs beyond the two it is counted as — the pair
    /// the wash is placed with, handed down so the hit and the wash agree (`CommandsPane.charW` / `wideDelta`).
    required property real charW
    required property real wideDelta

    readonly property bool failed: row.state === "failed"
    readonly property bool showsOutput: row.failed && row.output !== ""

    // ---- where this row's columns are drawn ---------------------------------------------------------------------
    /// The near edge of each column and the far edge of the two a press can run off the end of. The hand over the list
    /// reads these to decide which column a press landed in (`CommandsTextSelect`).
    readonly property real clockX: clock_.x
    readonly property real clockEnd: clock_.x + clock_.implicitWidth
    readonly property real cmdX: program.x
    readonly property real cmdEnd: Math.min(argsText.x + argsText.implicitWidth, outcome.x)
    readonly property real outX: outcome.x
    readonly property real outEnd: outcome.x + outcome.width
    /// How tall the command's own line is — below it is the block of words a failure carries.
    readonly property real lineHeight: line.height

    // Width comes from the view; the height grows with the output block.
    height: Theme.rowHeight + (row.showsOutput ? outputText.implicitHeight + 2 * Theme.spaceXs : 0)
    color: row.failed || rowHover.containsMouse ? Theme.bgElevated : "transparent"

    // What went wrong is carried by the edge as well as by the words, so a failure is findable while scrolling past at
    // speed.
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 2 * Theme.borderWidth
        color: Theme.danger
        visible: row.failed
    }

    // The pointer, for the lit ground and the one line the row has no room for. It takes no buttons: the presses over
    // this list belong to the hand that picks its text (`CommandsTextSelect`), which is laid over the whole of it.
    MouseArea {
        id: rowHover
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
        ToolTip.visible: containsMouse && row.state !== "running"
        ToolTip.delay: Metrics.tipDelayMs
        // The reproducible form is long; it is here rather than in the row so the log stays one line per command.
        ToolTip.text: row.full
    }

    // ---- the reader's own selection -----------------------------------------------------------------------------
    /// The three runs and the flag, taken apart once: `<clock>|<command>|<outcome>|<whole>`.
    readonly property var picked: row.sel === "" ? [] : row.sel.split("|")
    readonly property bool tookLine: row.picked.length === 4 && row.picked[3] === "1"

    /// Where a run begins and ends in this row's own pixels. A run is `col:wides:width:wides` — columns and the wide
    /// glyphs standing in them, because a wide glyph comes from a fallback that need not advance two mono columns
    /// (`CommandsPane.wideDelta`).
    function runFrom(run, baseX) {
        const at = run.split(":")
        return baseX + Number(at[0]) * row.charW + Number(at[1]) * row.wideDelta
    }
    function runTo(run, baseX) {
        const at = run.split(":")
        return baseX + (Number(at[0]) + Number(at[2])) * row.charW + (Number(at[1]) + Number(at[3])) * row.wideDelta
    }
    /// The wash is one rectangle from the first column the selection reaches into to the last, which is what puts it
    /// over the gaps as well — a tab is a character of the line, and a terminal washes it like one.
    function washEdge(wantFirst) {
        const bases = [row.clockX, row.cmdX, row.outX]
        for (let i = 0; i < 3; i++) {
            const at = wantFirst ? i : 2 - i
            if (row.picked[at] === undefined || row.picked[at] === "")
                continue
            return wantFirst ? row.runFrom(row.picked[at], bases[at]) : row.runTo(row.picked[at], bases[at])
        }
        return 0
    }
    readonly property real washX: row.picked.length === 0 ? 0 : row.washEdge(true)
    // Clamped to the row: a command too long for its column is drawn elided, and a wash running on past the last
    // character anybody can see would be washing the panel rather than the text.
    readonly property real washRight: row.picked.length === 0
                                      ? 0 : Math.min(row.washEdge(false), row.width - Theme.spaceMd)

    Rectangle {
        x: row.washX
        y: 0
        width: Math.max(0, row.washRight - row.washX)
        height: line.height
        color: Theme.bgSelected
    }
    // git's words come with a command line taken whole, so they are washed with it (デザイン規約 §git が言ったことを読む場所).
    Rectangle {
        x: outputText.x
        y: outputText.y
        width: outputText.implicitWidth
        height: outputText.implicitHeight
        visible: row.showsOutput && row.tookLine
        color: Theme.bgSelected
    }

    Item {
        id: line
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.rowHeight

        Label {
            id: clock_
            x: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            text: row.clock
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // The program never varies, so it is drawn rather than read: what changes from row to row is the part after it.
        Label {
            id: program
            x: row.clockEnd + Theme.spaceLg
            anchors.verticalCenter: parent.verticalCenter
            text: "git"
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Label {
            id: argsText
            // One character along from `git`, not one spacing token: the two are one string with one space in it, and
            // a selection that runs through them has to land on the same pixels the copy names. **Its width is
            // written rather than anchored to the outcome** — an anchor on the right decides the x as well, and this
            // one has to begin where the command's own column does (measured, the args went to the far edge).
            x: program.x + 4 * row.charW
            width: Math.max(0, outcome.x - Theme.spaceMd - x)
            anchors.verticalCenter: parent.verticalCenter
            text: row.args
            elide: Text.ElideRight
            color: Theme.textPrimary
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Row {
            id: outcome
            anchors.right: parent.right
            anchors.rightMargin: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            // The same one space the copy puts between them: two words about how it went, not two columns.
            spacing: row.charW
            Label {
                text: row.result
                visible: text !== ""
                color: Theme.danger
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
            Label {
                // Exit 0 says nothing that the absence of a complaint has not already said, so only the time it took is
                // kept.
                text: row.state === "running" ? qsTr("running…") : row.duration
                color: row.state === "running" ? Theme.accent : Theme.textMuted
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // Indented to the command it belongs to. Positioned rather than anchored: the column it lines up with lives inside
    // `line`, which makes it neither parent nor sibling of this.
    Label {
        id: outputText
        visible: row.showsOutput
        x: program.x
        y: line.height + Theme.spaceXs
        width: row.width - program.x - Theme.spaceMd
        text: row.output
        wrapMode: Text.Wrap
        color: Theme.textSecondary
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        lineHeight: Theme.fontSmLine
        lineHeightMode: Text.FixedHeight
    }
}

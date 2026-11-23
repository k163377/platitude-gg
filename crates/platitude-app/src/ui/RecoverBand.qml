pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui
import platitude.ui.bars
import platitude.ui.face
import platitude.ui.words

// The band over the graph while an entry of the discard log is on it: what the operation was and when,
// what each of its parts would bring back and as what, the press that brings it back, and the way out of the showing
// — the left panel only names the entry. The rebase plan's head is the model: a mode, named over its rows and
// underlined in the accent (`RebasePlanPane`).
//
// The presses say what the Windows Recycle Bin says for the same act: `Restore` for one thing, `Restore all` for every
// part of an entry whose parts each come back alone too. The branches one rebase moved come back together, under the
// one `Restore` (`byPart`); a single commit of theirs is a cherry-pick from the rows shown on the graph.
//
// One rebase can move any number of branches, and the band describes a graph it must not take the place of: it stands
// at `Theme.recoverBandMaxHeight` at the most, whatever the window, and leaves the graph its rows. Past it the lines
// scroll (`CappedLines`); the heading and the date stay put, a list whose heading has scrolled away no longer saying
// what it lists (`DetailsMessageBlock`), and so do the controls. The heading is cut at `Theme.recoverBandHeadRows`, its
// whole under the hand: past those it is a rebase's branches, each said on a line of its own below, or long names in a
// graph at its narrowest. A column short of room takes it off the band, down to one line of each kind (user decision —
// ask before changing) — never off the graph's rows, which the window's floor holds (`leastHeight`, `floorHeight`;
// `PageLayout.panesMinHeight`). A report or a question comes before it: the band steps aside while either stands, one
// bar over the middle at a time (`RepoPage.centreColumn`, user decision).
//
// The band, its lines and its presses are user decisions — ask before changing.
Rectangle {
    id: band

    /// The entry on the graph, as the list words it (`RecoverEntries.entryOf`); null for none.
    property var entry: null
    /// The moment in full, then how long ago (`RecoverEntries.whenWords`).
    property string when: ""
    /// Per part, whether what it would bring back is on the graph: a part whose tip the graph has not walked to cannot
    /// come back until it is (`RepoPage.recoverReach`). Empty while the walk that would tell is still out: nothing is
    /// pressed, and nothing is said to be out of reach.
    property var reach: []
    /// The graph can walk further than it has, `step` commits at a press (`GraphTailFooter`).
    property bool canWalkFurther: false
    property int step: 0
    /// A write is out: the presses wait for it.
    property bool busy: false
    /// What its column has for it with the graph's rows kept (`Theme.graphRowsKept`).
    property real room: Theme.recoverBandMaxHeight

    /// The press that brings it back — every part, or the one `part` (-1 for all).
    signal restoreAsked(int part)
    /// The showing is put down.
    signal dismissed()
    /// Walk the graph further, toward what the entry would bring back.
    signal furtherAsked()

    readonly property var parts: band.entry === null ? [] : band.entry.parts
    /// Each part can be brought back alone as well (`Discard::restores_by_part`).
    readonly property bool byPart: band.entry !== null && band.entry.byPart
    readonly property bool outOfReach: band.reach.some(inReach => !inReach)
    /// Every part can be pressed: the walk has told, and all of it is on the graph.
    readonly property bool allInReach: band.parts.length > 0 && band.reach.length === band.parts.length
                                       && !band.outOfReach
    readonly property real ceiling: Math.min(Theme.recoverBandMaxHeight, band.room)
    /// A line of the parts at the least: one that sets the name it brings back as (`lineRuler`).
    readonly property real partLine: lineRuler.setLineHeight
    /// The band at its least: a line of the heading, the date at `dateHeight`, a line of the parts and `reach` under
    /// them (the line saying the press cannot be had, where it stands), with the press on the middle of those lines
    /// (`controls`). In whole pixels, up: the press stands on a whole one, and a least under the band's own would come
    /// off the graph.
    function leastOf(headLine, dateHeight, partLine, reach) {
        const lines = partLine + reach
        return Math.ceil(2 * Theme.spaceSm + headLine + Theme.spaceXs + dateHeight + Theme.spaceXs
                         + Math.max(lines, (lines + Theme.controlHeight) / 2))
    }
    /// The least as it stands: what the panes give way to under the window's floor (`PageLayout.panesLeastHeight`).
    /// Read off the lines (`bandBody`), so none without an entry.
    readonly property real leastHeight: bandBody.item !== null ? bandBody.item.leastHeight : 0
    /// The least the window's floor holds for it, whatever entry is on the graph: its lines read off lines that set a
    /// name (`headRuler`, `lineRuler`), the date on one, every part on the graph (`PageLayout.panesMinHeight`). Past
    /// that the graph gives up rows: an entry the graph has not walked to stands its line saying so, and another
    /// worktree's entry its date with the worktree's name, which can wrap where the graph is at its narrowest.
    readonly property real floorHeight: band.leastOf(headRuler.setLineHeight, lineRuler.setLineHeight,
                                                     band.partLine, 0)
    /// The entry across reads: a read makes a new object of the one on the graph (`RecoverEntries.entries`), and a
    /// reader's place in its lines stays through that.
    readonly property string entryKey: band.entry === null ? "" : band.entry.key

    /// The line under the title where the entry was another worktree's: when, then where.
    function whereWords(when, worktree) {
        return qsTr("%1 · in %2").arg(when).arg(worktree)
    }
    /// A part's line: what it took, then what the press makes of it.
    function partWords(taken, restore) {
        return qsTr("%1 — %2").arg(taken).arg(restore)
    }

    /// The band's own height: its lines and its margins.
    readonly property real ownHeight: bandBody.implicitHeight + 2 * Theme.spaceSm
    /// The height of the bar the band takes the place of, gone in the same frame (`BarTravel`). Read once and put down
    /// — the very turn, should the band not come back in it; 0 for none, where the band stands at once, as on a pick.
    property real enterFrom: 0
    onEnterFromChanged: if (band.enterFrom > 0) enterFromLater.restart()
    readonly property TurnLater enterFromLater: TurnLater {
        id: enterFromLater
        onTriggered: band.enterFrom = 0
    }
    onVisibleChanged: {
        if (band.visible && band.enterFrom > 0)
            BarTravel.standThenTravel(band, band.enterFrom, () => band.ownHeight)
        band.enterFrom = 0
    }
    /// Only that travel animates (`AskBar.travelling`): a pick, another entry or a read stands the band at its
    /// height at once.
    property bool travelling: false
    onImplicitHeightChanged: if (band.travelling && band.implicitHeight === band.ownHeight) band.travelling = false
    Behavior on implicitHeight {
        enabled: band.travelling
        NumberAnimation { duration: Metrics.animStandardMs }
    }

    implicitHeight: band.ownHeight
    // Only while it travels: at a height short of its lines, they would run over the graph.
    clip: band.travelling
    color: Theme.bgElevated
    // Another entry's lines start at their top: the band stays standing from pick to pick. The bar's step in flight
    // goes with the lines it was sending (rules-refs/app-ui.md「ホイールの 1 ノッチは送られる、跳ばない」).
    onEntryKeyChanged: if (bandBody.item !== null) bandBody.item.toTop()

    // The lines and the presses, built while an entry is on the graph and taken down with it (rules-refs/app-ui.md —
    // the `WindowDialogSeat` line): the band stands only then, and what the window's floor holds for it is read off
    // the rulers below, never off these. An entry hidden behind a diff or the plan keeps them, and its place in them.
    Loader {
        id: bandBody
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceMd
        anchors.topMargin: Theme.spaceSm
        active: band.entry !== null
        sourceComponent: bandRowComponent
    }
    Component {
        id: bandRowComponent
        RowLayout {
            id: bandRow
            /// The line saying the press cannot be had, with its gap, where it stands: under the parts, among the lines
            /// the press stands on the middle of (`lines`).
            readonly property real reachRoom: band.outOfReach ? Theme.spaceXs + reachRow.implicitHeight : 0
            /// What stands over and under the parts' lines however few they are, with the heading at `rows` lines: the
            /// band's margins, the date, and the line saying the press cannot be had.
            function aroundParts(rows) {
                return 2 * Theme.spaceSm + rows * heading.lineHeight + Theme.spaceXs + whenLine.implicitHeight
                       + Theme.spaceXs + bandRow.reachRoom
            }
            /// The heading's lines: as many as still leave a line of the parts, one where they do not.
            readonly property int headRows: band.ceiling >= band.leastOf(Theme.recoverBandHeadRows * heading.lineHeight,
                                                                         whenLine.implicitHeight, band.partLine,
                                                                         bandRow.reachRoom)
                                            ? Theme.recoverBandHeadRows : 1
            /// What the ceiling leaves the parts' lines, a line at the least. The press stands on the middle of the
            /// parts and the line under them (`controls`): on lines shorter than it, it reaches past them by half the
            /// difference, so each pixel off the band takes two off the parts.
            readonly property real partsRoom: {
                const left = band.ceiling - bandRow.aroundParts(Math.min(bandRow.headRows, heading.lineCount))
                return Math.max(band.partLine, left + bandRow.reachRoom >= Theme.controlHeight
                                               ? left : 2 * left + bandRow.reachRoom - Theme.controlHeight)
            }
            /// The band's least as it stands ([`RecoverBand.leastHeight`]).
            readonly property real leastHeight: band.leastOf(heading.lineHeight, whenLine.implicitHeight, band.partLine,
                                                             bandRow.reachRoom)
            function toTop() {
                partsSeat.toTop()
            }
            spacing: Theme.spaceSm
            ColumnLayout {
                id: column
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                spacing: Theme.spaceXs
                // What was done, then when and where — the heading's two halves, as a commit's card gives subject and
                // date. Every name behind its kind's mark (デザイン規約 §ref の種別「名前の印」, `RecoverEntries.titleMarksOf`).
                CardText {
                    id: heading
                    Layout.fillWidth: true
                    text: band.entry === null ? "" : band.entry.title
                    marks: band.entry === null ? [] : band.entry.titleMarks
                    // A heading: a long name is cut at the name ceiling, whole under the hand (デザイン規約 §寸法「名前の天井」).
                    holdsNames: true
                    capRows: bandRow.headRows
                    tipsCap: true
                    color: Theme.textPrimary
                    pixelSize: Theme.fontMd
                    weight: Theme.fontWeightStrong
                }
                CardText {
                    id: whenLine
                    Layout.fillWidth: true
                    text: band.entry === null || band.entry.worktree === ""
                          ? band.when : band.whereWords(band.when, band.entry.worktree)
                    marks: band.entry === null || band.entry.worktree === "" ? []
                           : Words.marksOf(worktree => band.whereWords(band.when, worktree),
                                           [[band.entry.worktree, band.entry.worktreeMark]])
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                }
                // What the press acts on: the lines it stands on the middle of (`restoreEntry`), the date not among
                // them.
                ColumnLayout {
                    id: lines
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    // One line a part, a list's pitch apart: its mark in the left panel's tint, what it took, and what
                    // the press makes of it — and, where the parts come back alone, the press for that one. The bar
                    // stands in the gap before the press, where a part's own press is not.
                    CappedLines {
                        id: partsSeat
                        Layout.fillWidth: true
                        cap: bandRow.partsRoom
                        gap: bandRow.spacing
                        spacing: Theme.spaceXs
                        Repeater {
                            model: band.parts
                            delegate: RowLayout {
                                id: partLine
                                required property var modelData
                                required property int index
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                // The head of the line's words, on their capitals' middle (デザイン規約 §アイコンの席 の
                                // 文頭記号) — the words' top is the row's (they are its tallest item).
                                RecoverMark {
                                    Layout.preferredWidth: implicitWidth
                                    Layout.preferredHeight: implicitHeight
                                    Layout.alignment: Qt.AlignTop
                                    Layout.topMargin: Math.round(partText.baselineOffset
                                                                 - WordFace.capitalMiddle(partText.pixelSize)
                                                                 - implicitHeight / 2)
                                    size: partText.pixelSize
                                    kind: partLine.modelData.mark
                                    tint: partLine.modelData.tint
                                    badged: partLine.modelData.badged
                                }
                                // Wrapped, never cut at the name ceiling (user decision — ask before changing): the
                                // line says what the press brings back as, down to the name's tail, and the band's
                                // ceiling answers for how tall that gets.
                                CardText {
                                    id: partText
                                    Layout.fillWidth: true
                                    text: band.partWords(partLine.modelData.text, partLine.modelData.restore)
                                    // The restore half's names, seated after what the part took.
                                    marks: {
                                        const lead = Words.standSeats(
                                            band.partWords(partLine.modelData.text, Words.stand),
                                            partLine.modelData.restore)
                                        return lead.length === 0 ? []
                                             : partLine.modelData.restoreMarks
                                                   .map(mark => [mark[0] + lead[0], mark[1], mark[2], mark[3]])
                                    }
                                    color: Theme.textPrimary
                                    pixelSize: Theme.fontSm
                                }
                                // The words' own height, the press on the middle of their lines, its word on theirs in
                                // whole pixels where they are one line: a button's height would open the lines apart,
                                // and centred on the line its word stands under the line's by half the leading a field
                                // files under its line (2〜3px on macOS). The seat is centred in the row as the words
                                // are, so its top is theirs.
                                Item {
                                    visible: band.byPart
                                    Layout.preferredWidth: partRestore.implicitWidth
                                    Layout.preferredHeight: partText.implicitHeight
                                    HoverToolButton {
                                        id: partRestore
                                        y: Math.round(partText.baselineOffset - partRestore.baselineOffset
                                                      + (partText.implicitHeight - partText.lineHeight) / 2)
                                        // Down to the wash's own height (`HoverToolButton`), which the lines' spacing
                                        // holds.
                                        topPadding: 0
                                        bottomPadding: 0
                                        enabled: band.reach[partLine.index] === true && !band.busy
                                        text: qsTr("Restore")
                                        font.pixelSize: Theme.fontSm
                                        onClicked: band.restoreAsked(partLine.index)
                                    }
                                }
                            }
                        }
                    }
                    // Not yet on the graph: the press waits, and the way there is offered in the words the graph's own
                    // end offers it with (`GraphTailFooter`). Out of the scroll: it says why the press is dead. The
                    // press stands right after the words; where the two do not fit on one line it goes under them, its
                    // word starting where theirs do, and the words keep a line of their own rather than wrap beside a
                    // press that stands on one (user decision — ask before changing).
                    GridLayout {
                        id: reachRow
                        /// The words and the press do not fit on one line: the press stands under the words.
                        readonly property bool pressUnder: band.canWalkFurther
                            && reachWords.implicitWidth + reachRow.columnSpacing + walkFurther.implicitWidth
                               > lines.width
                        visible: band.outOfReach
                        Layout.fillWidth: true
                        Layout.maximumWidth: implicitWidth
                        columns: reachRow.pressUnder ? 1 : 2
                        columnSpacing: Theme.spaceSm
                        rowSpacing: Theme.spaceXs
                        // On one line, on one baseline in whole pixels, as a part's press is: both hang from the top
                        // and the one whose word stands higher drops to the other's.
                        CardText {
                            id: reachWords
                            Layout.fillWidth: true
                            text: qsTr("Not loaded on the graph yet")
                            color: Theme.warning
                            pixelSize: Theme.fontSm
                            Layout.alignment: Qt.AlignTop
                            Layout.topMargin: band.canWalkFurther && !reachRow.pressUnder
                                              ? Math.max(0, Math.round(walkFurther.baselineOffset
                                                                       - reachWords.baselineOffset))
                                              : 0
                        }
                        // Beside the words, the press's own height on their baseline. Under them, a line of theirs: the
                        // seat is a line of the words tall, the press's word on that line's baseline and its wash
                        // reaching into the lines' spacing, as a part's press does — the press's box a line apart would
                        // open the lines apart. Its word starts where theirs do; its wash reaches into the band's
                        // margin.
                        Item {
                            visible: band.canWalkFurther
                            Layout.alignment: Qt.AlignTop | Qt.AlignLeft
                            Layout.preferredWidth: walkFurther.implicitWidth
                            // The words' own height, a line of theirs where the press stands under them (they have the
                            // row to themselves): their `lineHeight` can read many lines' height for one while a width
                            // moves.
                            Layout.preferredHeight: reachRow.pressUnder ? reachWords.implicitHeight
                                                                        : walkFurther.implicitHeight
                            Layout.topMargin: reachRow.pressUnder
                                              ? 0 : Math.max(0, Math.round(reachWords.baselineOffset
                                                                           - walkFurther.baselineOffset))
                            Layout.leftMargin: reachRow.pressUnder ? -walkFurther.leftPadding : 0
                            HoverToolButton {
                                id: walkFurther
                                y: reachRow.pressUnder
                                   ? Math.round(reachWords.baselineOffset - walkFurther.baselineOffset) : 0
                                // The step is a count: said as a number, the button's words stay the app's whatever the
                                // band hands it (a tool button leaves its words to the style, which guesses their
                                // format).
                                text: Words.loadFurther(Number(band.step))
                                font.pixelSize: Theme.fontSm
                                onClicked: band.furtherAsked()
                            }
                        }
                    }
                }
            }
            // The controls' column (user decision — ask before changing): the `✕` up in the corner, as the question
            // bar's (`AskBar`), and the press on the middle of the lines it acts on, as that bar's stands beside its
            // one line, the heading and the date keeping it clear of the `✕`. Their right edges are one line. Centred
            // on one line the press reaches below the words, and this column sets the band's height.
            Item {
                id: controls
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: restoreEntry.implicitWidth
                implicitHeight: restoreEntry.y + restoreEntry.height
                ActionButton {
                    id: restoreEntry
                    anchors.right: parent.right
                    y: Math.round(lines.y + (lines.height - restoreEntry.height) / 2)
                    height: Theme.controlHeight
                    enabled: band.allInReach && !band.busy
                    text: band.byPart ? qsTr("Restore all") : qsTr("Restore")
                    frameColor: Theme.accent
                    onActivated: band.restoreAsked(-1)
                }
                // The way out of the showing, as the list's own `✕` is (`RecoverPane`): the mark's right edge on the
                // press's and its top on the heading's, the rest of its seat hanging past them.
                CloseToolButton {
                    id: stopShowing
                    x: controls.width - stopShowing.width + (stopShowing.width - stopShowing.markSize) / 2
                    y: -(stopShowing.height - stopShowing.markSize) / 2
                    Accessible.name: qsTr("Stop showing it")
                    onClicked: band.dismissed()
                }
            }
        }
    }
    // A line of the heading and one of the small words, each with a name on it, never shown: a name is set in git's
    // face beside the words' and stands the line taller than words alone, so a field's own line moves with what it
    // says (`CardText.lineHeight`). What the band stands at the least whatever entry it shows is read here
    // (`floorHeight`, `CardText.setLineHeight`).
    CardText {
        id: headRuler
        visible: false
        text: "main"
        marks: [[0, "branch", Theme.textLink, 4]]
        holdsNames: true
        pixelSize: Theme.fontMd
        weight: Theme.fontWeightStrong
    }
    CardText {
        id: lineRuler
        visible: false
        text: "main"
        marks: [[0, "branch", Theme.textLink, 4]]
        pixelSize: Theme.fontSm
    }
    // The mode's own underline, as the plan's head draws it.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: Theme.accent
    }
}

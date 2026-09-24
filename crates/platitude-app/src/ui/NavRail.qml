pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The sidebar folded down to its spine: one cell per section, the icon saying what is in it and the number under it how
// much. The pointer resting on a cell opens that one section beside the rail; a click puts the whole list back with
// that section expanded in it.
//
// The cells only report the pointer. The list that opens is the sidebar's, because it has to outlive the hover that
// raised it — the pointer walking into it comes off the cell on the way (the same reason the graph's chips hand their
// popup to the page).
Rectangle {
    id: rail

    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel
    /// Whether the graph is showing tags. The rail is the only place the answer can be while the list is folded — the
    /// switch itself is in the TAGS section, which is a hover away.
    required property bool tagsShown

    /// The section the sidebar has open beside the rail: its cell keeps the hover look while the pointer is down in the
    /// list.
    property string openKind: ""
    /// The `+` this rail carries is held (`SidebarPane.doorsHeld`). **Only the `+`** — the cells go on opening their
    /// sections to the pointer and putting the list back to a click, which is how the folded pane is read.
    property bool addHeld: false

    /// The pointer came to rest on a cell. `top` is where that cell sits, so the section can open level with the icon
    /// it belongs to.
    signal peekRequested(string kind, real top)
    /// It left a cell. Which one it was is the whole message: the cell being left on the way to another has already
    /// been replaced, and only the one whose section is open can take it away.
    signal peekLeft(string kind)
    /// A click landed on a cell: the section it opened goes away, and a second click brings it back. The list stays
    /// folded — a stray click on the rail would take an open file down with it.
    signal peekToggled(string kind, real top)
    /// Put the list back, with whatever was open in it still open.
    signal unfoldRequested()
    /// The REMOTES cell with nothing in it was clicked: a remote is to be written down. That cell has no section to
    /// open, and writing the first remote down is what a repository with none is for — a local `init` that now wants
    /// what is on the far side. The open band carries the same `+` at the end of it (NavHeader).
    signal addRemoteRequested()

    // The sections in the order the open sidebar stacks them, wearing the tints their headers wear (NavHeader).
    readonly property var sections: [
        { kind: "branch", icon: "branch", tint: Theme.accent },
        { kind: "remote", icon: "remote", tint: Theme.textSecondary },
        { kind: "worktree", icon: "tree", tint: Theme.success },
        { kind: "stash", icon: "stash", tint: Theme.textSecondary },
        { kind: "tag", icon: "tag", tint: Theme.refTag }
    ]

    /// A section's icon and tint, so the one the sidebar opens beside the rail wears the same mark as the cell that
    /// opened it. An unnamed section answers with the first one's look — the caller is a binding that outlives the
    /// choice.
    function sectionOf(kind) {
        for (let i = 0; i < rail.sections.length; i++) {
            if (rail.sections[i].kind === kind)
                return rail.sections[i]
        }
        return rail.sections[0]
    }

    function modelOf(kind) {
        return kind === "branch" ? rail.branchesModel
             : kind === "remote" ? rail.remotesModel
             : kind === "worktree" ? rail.worktreesModel
             : kind === "stash" ? rail.stashesModel : rail.tagsModel
    }

    /// How many rows a section has — and none at all once there is no model to ask. Closing a tab takes the page's
    /// models down before the rail that reads them (a delegate's children go first), so every binding here outlives
    /// what it is bound to; without this the last thing a closed tab does is throw on `total` of null.
    function countOf(kind) {
        const section = rail.modelOf(kind)
        return section ? section.total : 0
    }

    /// What a cell does when the pointer arrives on it, leaves it, or is clicked on it. The cells' own handlers call
    /// these and nothing else, so the smoke hooks (PGG_AUTO_ACT=nav-peek / nav-peek-away / nav-peek-shut) reach the real
    /// answer by calling the same three — neither hover nor a click can be injected (verify-ui スキル), and a second copy
    /// of the answer for them to call is a second answer.
    ///
    /// `top` is where the cell sits; a hook that has no cell to read it off asks `topOf`, which computes the same
    /// number.
    function enterAt(kind, top) {
        // An empty section opens nothing, and takes away what the cell before it opened: a list left standing beside a
        // cell it does not belong to would be pointing at the wrong section.
        if (rail.countOf(kind) === 0)
            rail.peekLeft(kind)
        else
            rail.peekRequested(kind, top)
    }
    function leaveAt(kind) {
        rail.peekLeft(kind)
    }
    function tapAt(kind) {
        // The one cell that answers a click with no rows behind it. Everywhere else an empty cell is unavailable —
        // there is nothing to open and nothing to do — but this one has the operation that fills it (`addable`), for
        // as long as that operation is not one of the doors being held.
        if (rail.addableAt(kind) && !rail.addHeld)
            rail.addRemoteRequested()
        else if (rail.countOf(kind) > 0)
            rail.peekToggled(kind, rail.topOf(kind))
    }
    /// Whether a cell's whole answer is "write a remote down": REMOTES, and only while it is empty. With rows in it
    /// the click belongs to the section, and the `+` is on the band the hover opens.
    function addableAt(kind) {
        return kind === "remote" && rail.countOf(kind) === 0
    }

    /// How tall one cell stands: `toolbarHeight`, which is a mark of `iconXl` with the `fontSm` line of its word under
    /// it — what a cell holds, measured. The five of this column all answer to it, because a reach that changes
    /// partway down a column of one kind of thing is a difference nobody can see and everybody feels.
    ///
    /// **Not the ☰'s cell.** That one stands in the tab band, which is slimmer than this, with the operation panel
    /// between them (`Theme.toolbarHeight` — TopBar): they are not one column, so they do not have to be one reach.
    ///
    /// `railWidth` is the window's own outer edge, and what sets it is the mark at the head of this
    /// column plus a step either side (§左メニューを畳む). Two questions with two answers — a square cell would have
    /// the narrower of them decide the reach.
    readonly property int cellHeight: Theme.toolbarHeight

    /// Where a section's cell sits, for anything that has to line up with it without the pointer having been there (the
    /// smoke hook).
    function topOf(kind) {
        for (let i = 0; i < rail.sections.length; i++) {
            if (rail.sections[i].kind === kind)
                return Theme.headerHeight + i * rail.cellHeight
        }
        return Theme.headerHeight
    }

    implicitWidth: Theme.railWidth
    color: Theme.bgSurface

    Column {
        anchors.fill: parent
        spacing: 0

        // The pane's header band, which folded is the fold control and nothing else — the same block that sits at the
        // end of the band when the list is open, holding the whole width because there is no filter beside it to divide
        // it from. The panes either side still start where this ends (規約 §QML 実装ルール).
        FoldBlock {
            width: parent.width
            height: Theme.headerHeight
            folds: false
            divided: false
            onActivated: rail.unfoldRequested()
            // The hairline the filter's underline leaves behind, so the band reads as the same band it was before it
            // folded.
            BandRule {}
        }

        Repeater {
            model: rail.sections
            delegate: Rectangle {
                id: cell
                required property var modelData
                readonly property int sectionCount: rail.countOf(cell.modelData.kind)
                readonly property bool open: rail.openKind === cell.modelData.kind
                // Tags are the one section that can be kept out of the graph, so its cell is the one that says whether
                // they are in it.
                readonly property bool taggable: cell.modelData.kind === "tag"
                readonly property bool offGraph: cell.taggable && !rail.tagsShown
                /// Nothing in there, so there is nothing to open: a list of no rows offers no operation, and the number
                /// under the mark has already said as much. The cell keeps its place — what is countable is worth
                /// counting at zero — and goes unavailable (規約 §無効).
                readonly property bool empty: cell.sectionCount === 0
                /// Except this one. An empty REMOTES cell has an operation after all — writing the first remote down —
                /// so it answers the pointer and the click, and wears the `+` that says which (`rail.addableAt`).
                readonly property bool addable: rail.addableAt(cell.modelData.kind)
                /// …and whether that operation can be reached right now. The mark stays on the cell either way — it is
                /// what the cell is for — and goes grey with the rest of it while the doors are held (規約 §無効).
                readonly property bool addLive: cell.addable && !rail.addHeld

                width: Theme.railWidth
                height: rail.cellHeight
                // The open section keeps the hover wash while the pointer is away in its list: what is on screen has to
                // say which cell put it there. An empty one washes for nobody — unavailable does not answer the pointer
                // (規約 §無効) — unless it is the one with something to press.
                color: (cellHover.hovered && (!cell.empty || cell.addLive)) || cell.open
                       ? Theme.bgHover : "transparent"
                // The `+` is the cell's only name while it is standing in for the whole band, and a mark with no word
                // beside it has nowhere else to carry one (規約 §hover のツールチップ). The wording is the band's own, so
                // the two doors into the dialog name it the same (デザイン規約 §リモートを書き留める).
                ToolTip.visible: cell.addLive && cellHover.hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: Words.addRemote

                Column {
                    // All the cell's slack above the mark, none under the number. The slack is the `spaceXs` the
                    // spacing below takes back, and split in two it left the first cell with half of what the others
                    // have between them: the band's mark sits right on the cell's own top edge, so the top cell pays
                    // one half where every other pair of cells pays two. Put on one side it is the same gap
                    // everywhere — fold block to the first mark, and each number to the next mark
                    // (by design. デザイン規約 §左メニューを畳む).
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Theme.spaceXs
                    // Taken back: the box keeps half its unused grid as air under the mark's ink, and the number's
                    // line box keeps its own leading above the digit, so a zero here still reads as a gap — and once
                    // the mark took the cell's whole remainder, a wider one than the cells keep between them. The
                    // pair then reads as two things rather than one (measured on Windows, ink to ink: at zero, 6px
                    // from the mark to its own digit against 5px from that digit to the next cell's mark — the gap
                    // inside the pair is the larger of the two; 2 against 9 with this). Subtracting the air a mark
                    // holds is what デザイン規約 §余白 says to do beside a word — under one it is the same sum.
                    spacing: -Theme.spaceXs
                    NavIcon {
                        id: sectionIcon
                        anchors.horizontalCenter: parent.horizontalCenter
                        kind: cell.modelData.icon
                        // Off the graph is a state, and a state is said by dropping the mark a step — grey text is what
                        // unavailable looks like (デザイン規約 §暗く落とした段). Which is what an empty one is, so grey is
                        // exactly what it wears. An empty REMOTES cell greys too: what is unavailable there is the
                        // section, and the mark and the number are what report it — the `+` beside them is the
                        // part that can be pressed, and it keeps its colour.
                        tint: cell.empty ? Theme.textMuted : cell.offGraph ? Theme.refTagDim : cell.modelData.tint
                        // The cell's whole height above the number, with no padding written around the box: the mark
                        // carries its own air inside it (デザイン規約 §余白 — a mark's own margin counts towards the gap
                        // beside it), and these marks draw some 11 of their 16 grid. A `spaceXs` written on either
                        // side of the box was that air twice over, and what the eye measures is the ink.
                        //
                        // The remainder of `cellHeight` once the number's line and the slack above are out of it, one
                        // step up from the marks that stand alone in a band: this one is the cell (デザイン規約 §寸法 —
                        // 印がセルの中身そのものである時は別の規則).
                        width: Theme.iconXl
                        height: Theme.iconXl
                        // The eye the TAGS header carries, worn as a mark on the corner: folded, this is the only place
                        // "are tags in the graph" can be answered. Up in both states, and struck through once they are
                        // out — the same pair the open header wears (NavHeader), so folding keeps the mark that says
                        // it. A badge that is only there in one state leaves the other reading as a cell that never
                        // carried one, and two steps of colour is not a state anyone reads at this size: the colour is
                        // what the stroke is read against.
                        // An empty section is the exception — with no tags to keep out of the graph the switch has
                        // nothing to answer for, and the cell is unavailable anyway (規約 §無効).
                        // Smaller than the section's own mark, because it is about the mark it sits on.
                        NavIcon {
                            visible: cell.taggable && !cell.empty
                            kind: rail.tagsShown ? "eye" : "eye-off"
                            tint: sectionIcon.tint
                            width: Theme.iconSm
                            height: Theme.iconSm
                            // Sideways it hangs off the box, into the air the cell keeps either side of the mark;
                            // downwards it starts at the box's own top. A `spaceXs` off both, back towards the mark it
                            // belongs to: hung off the corner outright it stood on the cell's right edge with the air
                            // all on the other side, and the step up spends the slack the column keeps above it, so
                            // the badge's own box starts where the cell does. Centred on the
                            // corner, half the badge would stand in the cell above — beside a number that counts a
                            // different section.
                            anchors.horizontalCenter: parent.right
                            anchors.horizontalCenterOffset: -Theme.spaceXs
                            anchors.top: parent.top
                            anchors.topMargin: -Theme.spaceXs
                        }
                        // The `+` the open band carries at the end of it (NavHeader), worn on the same corner as the
                        // eye and only while the section is empty: with rows in it the band is a hover away and
                        // carries its own. It keeps the section's colour against the greyed mark it sits on — what
                        // cannot be pressed is the section (規約 §無効).
                        NavIcon {
                            visible: cell.addable
                            kind: "plus"
                            tint: cell.addLive ? cell.modelData.tint : Theme.textMuted
                            width: Theme.iconSm
                            height: Theme.iconSm
                            // The eye's seat exactly (規約 §左メニューを畳む names one corner for both) — the two are
                            // never on screen together, so a step between them would read as two different corners.
                            anchors.horizontalCenter: parent.right
                            anchors.horizontalCenterOffset: -Theme.spaceXs
                            anchors.top: parent.top
                            anchors.topMargin: -Theme.spaceXs
                        }
                    }
                    // How many are in there. It stands in for the caption as well as the count, so it takes the
                    // caption's colour (NavHeader — a count wears the dimmer one where a word is already carrying
                    // the section).
                    Label {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: cell.sectionCount
                        color: cell.empty ? Theme.textMuted : Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                        lineHeightMode: Text.FixedHeight
                        lineHeight: Theme.fontSmLine
                    }
                }
                // Passive, so that walking from a cell into the list it opened does not have the list's own hover taken
                // away.
                HoverHandler {
                    id: cellHover
                    onHoveredChanged: {
                        if (cellHover.hovered)
                            rail.enterAt(cell.modelData.kind, cell.y)
                        else
                            rail.leaveAt(cell.modelData.kind)
                    }
                }
                TapHandler {
                    onTapped: rail.tapAt(cell.modelData.kind)
                }
            }
        }
    }
}

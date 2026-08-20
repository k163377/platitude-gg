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

    /// The pointer came to rest on a cell. `top` is where that cell sits, so the section can open level with the icon
    /// it belongs to.
    signal peekRequested(string kind, real top)
    /// It left a cell. Which one it was is the whole message: the cell being left on the way to another has already
    /// been replaced, and only the one whose section is open can take it away.
    signal peekLeft(string kind)
    /// A click landed on a cell: the section it opened goes away, and a second click brings it back. It does not put
    /// the list back — a stray click on the rail would take an open file down with it.
    signal peekToggled(string kind, real top)
    /// Put the list back, with whatever was open in it still open.
    signal unfoldRequested()

    // The sections in the order the open sidebar stacks them, wearing the tints their headers wear (NavHeader).
    readonly property var sections: [
        { kind: "branch", icon: "branch", tint: Theme.accent },
        { kind: "remote", icon: "remote", tint: Theme.textSecondary },
        { kind: "worktree", icon: "tree", tint: Theme.success },
        { kind: "stash", icon: "stash", tint: Theme.textSecondary },
        { kind: "tag", icon: "tag", tint: Theme.refTag }
    ]

    /// A section's icon and tint, so the one the sidebar opens beside the rail wears the same mark as the cell that
    /// opened it. An unnamed section answers with the first one's look rather than nothing — the caller is a binding
    /// that outlives the choice.
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
    /// these and nothing else, so the smoke hooks (PG_AUTO_ACT=nav-peek / nav-peek-away / nav-peek-shut) reach the real
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
        if (rail.countOf(kind) > 0)
            rail.peekToggled(kind, rail.topOf(kind))
    }

    /// Where a section's cell sits, for anything that has to line up with it without the pointer having been there (the
    /// smoke hook).
    function topOf(kind) {
        for (let i = 0; i < rail.sections.length; i++) {
            if (rail.sections[i].kind === kind)
                return Theme.headerHeight + i * Theme.railWidth
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
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: Theme.borderSubtle
            }
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

                width: Theme.railWidth
                height: Theme.railWidth
                // The open section keeps the hover wash while the pointer is away in its list: what is on screen has to
                // say which cell put it there. An empty one washes for nobody — unavailable does not answer the pointer
                // (規約 §無効).
                color: (cellHover.hovered && !cell.empty) || cell.open ? Theme.bgHover : "transparent"

                Column {
                    anchors.centerIn: parent
                    // Taken back, not written: the box keeps half its unused grid as air under the mark's ink, and the
                    // number's line box keeps its own leading above the digit, so a zero here still reads as a gap.
                    // Once the mark filled the cell that gap grew to the width of the one between two cells, and the
                    // pair stopped reading as a pair (measured on Windows: 7px from ink to digit against 8px from the
                    // digit to the next cell's mark; 4 against 9 with this). Subtracting the air a mark holds is what
                    // デザイン規約 §余白 says to do beside a word — under one it is the same sum.
                    spacing: -Theme.spaceXs
                    NavIcon {
                        id: sectionIcon
                        anchors.horizontalCenter: parent.horizontalCenter
                        kind: cell.modelData.icon
                        // Off the graph is a state, and a state is said by dropping the mark a step, not by greying it
                        // — grey text is what unavailable looks like (デザイン規約 §暗く落とした段). Which is what an empty one is,
                        // so grey is exactly what it wears.
                        tint: cell.empty ? Theme.textMuted : cell.offGraph ? Theme.refTagDim : cell.modelData.tint
                        // The cell's whole height above the number, with no padding written around the box: the mark
                        // carries its own air inside it (デザイン規約 §余白 — a mark's own margin counts towards the gap
                        // beside it), and these marks draw some 11 of their 16 grid. A `spaceXs` written on either
                        // side of the box was that air twice over, and what the eye measures is the ink.
                        width: Theme.iconXl
                        height: Theme.iconXl
                        // The eye the TAGS header carries, worn as a mark on the corner: folded, this is the only place
                        // "are tags in the graph" can be answered. It is open or it is not there — a mark that is
                        // always up says nothing, and two steps of colour is not a state anyone reads at this size.
                        // Smaller than the section's own mark, because it is about the mark rather than beside it.
                        NavIcon {
                            visible: cell.taggable && rail.tagsShown
                            kind: "eye"
                            tint: sectionIcon.tint
                            width: Theme.iconSm
                            height: Theme.iconSm
                            // Sideways it hangs off the box, into the air the cell keeps either side of the mark;
                            // downwards it starts at the box's own top, because the box now reaches the top of the
                            // cell. Centred on that corner, half the badge would stand in the cell above — beside a
                            // number that counts a different section.
                            anchors.horizontalCenter: parent.right
                            anchors.top: parent.top
                        }
                    }
                    // How many are in there. It stands in for the caption as well as the count, so it takes the
                    // caption's colour rather than the dimmer one a count wears when a word is already carrying the
                    // section (NavHeader).
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

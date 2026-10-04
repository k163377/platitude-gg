pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The sidebar folded to its spine: one cell per section, its icon over its count. Resting on a cell opens that
// section beside the rail; a click shuts or reopens it. The cells only report the pointer: the list that opens is
// the sidebar's, since it must outlive the hover that raised it.
Rectangle {
    id: rail

    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel
    /// Whether the graph shows tags — folded, the rail is the only place that says so.
    required property bool tagsShown

    /// The section open beside the rail; its cell keeps the hover wash.
    property string openKind: ""
    /// Holds the `+` only (`SidebarPane.doorsHeld`); the cells keep opening their sections.
    property bool addHeld: false

    /// The pointer rested on a cell; `top` lines the section up with it.
    signal peekRequested(string kind, real top)
    /// It left a cell — named, so only the cell whose section is open can shut it.
    signal peekLeft(string kind)
    /// A click on a cell shuts the section it opened, or reopens it. The list stays folded: unfolding on a stray
    /// click would take an open file down with it.
    signal peekToggled(string kind, real top)
    /// Put the list back, with whatever was open in it still open.
    signal unfoldRequested()
    /// The empty REMOTES cell was clicked: add the first remote (the open band's `+`, NavHeader).
    signal addRemoteRequested()

    // The sections in the order the open sidebar stacks them, wearing the tints their headers wear (NavHeader).
    readonly property var sections: [
        { kind: "branch", icon: "branch", tint: Theme.accent },
        { kind: "remote", icon: "remote", tint: Theme.textSecondary },
        { kind: "worktree", icon: "tree", tint: Theme.success },
        { kind: "stash", icon: "stash", tint: Theme.textSecondary },
        { kind: "tag", icon: "tag", tint: Theme.refTag }
    ]

    /// A section's icon and tint, for the section opened beside the rail. An unknown kind gets the first — the
    /// caller's binding outlives the choice.
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

    /// A section's row count, 0 without a model: closing a tab takes the models down before this rail, and its
    /// bindings would throw on null.
    function countOf(kind) {
        const section = rail.modelOf(kind)
        return section ? section.total : 0
    }

    /// A cell's whole answer to arrival, leaving and a click. The handlers call only these, so the smoke hooks
    /// (PGG_AUTO_ACT=nav-peek / nav-peek-away / nav-peek-shut) reach the real answer; they get `top` from `topOf`.
    function enterAt(kind, top) {
        // Empty: open nothing, and shut the last cell's section rather than leave it beside the wrong cell.
        if (rail.countOf(kind) === 0)
            rail.peekLeft(kind)
        else
            rail.peekRequested(kind, top)
    }
    function leaveAt(kind) {
        rail.peekLeft(kind)
    }
    function tapAt(kind) {
        // Of the empty cells, only REMOTES answers a click, unless the doors are held.
        if (rail.addableAt(kind) && !rail.addHeld)
            rail.addRemoteRequested()
        else if (rail.countOf(kind) > 0)
            rail.peekToggled(kind, rail.topOf(kind))
    }
    /// Whether a cell's click adds a remote: REMOTES while empty. The click, the wash and the `+` all read this.
    function addableAt(kind) {
        return kind === "remote" && rail.countOf(kind) === 0
    }

    /// One cell's height: an `iconXl` mark over a `fontSm` line (デザイン規約 §寸法). Not square — `railWidth` is set
    /// by the ☰ above, a different question (§左メニューを畳む).
    readonly property int cellHeight: Theme.toolbarHeight
    /// The ground under the last number, which a cell otherwise keeps none of (デザイン規約 §左メニューを畳む). The last
    /// cell's own, so its wash and its hand reach down to what lies under the rail.
    readonly property int tailRoom: Theme.spaceXs / 2
    /// The fold block and every cell. The rail is not clipped, so a pane shorter than this draws the last cells over
    /// what lies under it (`SidebarPane.floorHeight`).
    readonly property int wholeHeight: Theme.headerHeight + rail.sections.length * rail.cellHeight + rail.tailRoom

    /// Where a section's cell sits, for the smoke hooks.
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

        // Folded, the header band is only the fold control, full width. The panes beside still start where it ends
        // (規約 §QML 実装ルール).
        FoldBlock {
            width: parent.width
            height: Theme.headerHeight
            folds: false
            divided: false
            onActivated: rail.unfoldRequested()
            // The filter's underline, so the band reads the same folded.
            BandRule {}
        }

        Repeater {
            model: rail.sections
            delegate: Rectangle {
                id: cell
                required property var modelData
                required property int index
                readonly property int sectionCount: rail.countOf(cell.modelData.kind)
                readonly property bool open: rail.openKind === cell.modelData.kind
                // Only tags can be kept out of the graph.
                readonly property bool taggable: cell.modelData.kind === "tag"
                readonly property bool offGraph: cell.taggable && !rail.tagsShown
                /// Nothing to open: the cell keeps its place (a zero is worth counting) and goes unavailable
                /// (規約 §無効).
                readonly property bool empty: cell.sectionCount === 0
                /// …except an empty REMOTES cell, which adds a remote and wears the `+` (`rail.addableAt`).
                readonly property bool addable: rail.addableAt(cell.modelData.kind)
                /// …and whether it can now; while the doors are held the `+` stays, greyed (規約 §無効).
                readonly property bool addLive: cell.addable && !rail.addHeld

                width: Theme.railWidth
                height: rail.cellHeight + (cell.index === rail.sections.length - 1 ? rail.tailRoom : 0)
                // The open section's cell keeps the wash, to say which cell opened it. An empty cell does not wash
                // unless it can be pressed (規約 §無効).
                color: (cellHover.hovered && (!cell.empty || cell.addLive)) || cell.open
                       ? Theme.bgHover : "transparent"
                // A wordless `+` names itself by tooltip (規約 §hover のツールチップ), in the band's own words
                // (デザイン規約 §リモートを書き留める).
                ToolTip.visible: cell.addLive && cellHover.hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: Words.addRemote

                Column {
                    // All the slack above the mark, none under the number, so the fold block to the first mark is
                    // the same gap as between cells (デザイン規約 §左メニューを畳む).
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Theme.spaceXs
                    // Negative: the mark's air under its ink and the digit's leading would make the gap inside the
                    // pair wider than between cells (デザイン規約 §左メニューを畳む).
                    spacing: -Theme.spaceXs
                    NavIcon {
                        id: sectionIcon
                        anchors.horizontalCenter: parent.horizontalCenter
                        kind: cell.modelData.icon
                        // Off the graph drops a step (デザイン規約 §暗く落とした段). Empty is unavailable, so grey —
                        // REMOTES included; its `+` keeps its colour.
                        tint: cell.empty ? Theme.textMuted : cell.offGraph ? Theme.refTagDim : cell.modelData.tint
                        // The cell's remainder above the number (デザイン規約 §寸法), with no padding around the box:
                        // the mark's own air is the gap (§余白).
                        width: Theme.iconXl
                        height: Theme.iconXl
                        // The TAGS header's eye (NavHeader) on the shoulder, in both states — struck through when
                        // off — since a badge in one state only reads as absent in the other. Not on an empty section
                        // (規約 §無効). A `spaceXs` in and up from the corner: centred on it, half the badge would sit
                        // in the cell above (デザイン規約 §左メニューを畳む).
                        ShoulderBadge {
                            visible: cell.taggable && !cell.empty
                            kind: rail.tagsShown ? "eye" : "eye-off"
                            tint: sectionIcon.tint
                        }
                        // The band's `+` (NavHeader), while the section is empty, in the eye's seat; it keeps the
                        // section's colour over the greyed mark (規約 §無効).
                        ShoulderBadge {
                            visible: cell.addable
                            kind: "plus"
                            tint: cell.addLive ? cell.modelData.tint : Theme.textMuted
                        }
                    }
                    // Caption and count at once, so the caption's colour (NavHeader).
                    Label {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: cell.sectionCount
                        color: cell.empty ? Theme.textMuted : Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                        lineHeightMode: Text.FixedHeight
                        lineHeight: Theme.fontSmLine
                    }
                }
                // Passive, so the list it opened keeps its own hover.
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

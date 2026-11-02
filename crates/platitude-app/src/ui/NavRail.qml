pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The sidebar folded to its spine: one cell per section, its icon over its count — the mark says what, the number
// under it how many. Nothing else in a cell: folding is for width, and no word fits in `railWidth`. Resting on a cell
// opens that section beside the rail; a click shuts or reopens it. The cells only report the pointer: the list that
// opens is the sidebar's, since it must outlive the hover that raised it.
//
// A section with no rows keeps its cell, counting zero, unavailable (規約 §無効: mark and number to `textMuted`,
// ground and frame as they are, the hover passing through) and opens nothing — gone, "none" and "no such section"
// would look alike. What is greyed is availability: a dimmed kind colour would not say "cannot press"
// (§暗く落とした段).
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
    /// It left a cell — named, so only the cell whose section is open can shut it. A QML signal handed too few
    /// arguments gets `undefined` without a word, and a section no leave can shut.
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
    /// bindings would throw on null. WORKTREES is never 0 (git lists the main checkout too); TAGS counts every tag
    /// whatever the graph shows (`tagsModel.total` is a feed apart from the walk), so the eye shuts nothing.
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
    /// Whether a cell's click adds a remote: REMOTES while empty, the one section where "no rows, nothing to do" does
    /// not hold — no remote at all is when the first is written down. The list stays folded (unfolding would close
    /// the diff). The click, the wash and the `+` all read this one answer — written apart, a cell takes the click
    /// without washing, or wears the `+` with nothing behind it. Arrival is untouched: there are still no rows to open
    /// (`enterAt`). Only the empty cell wears the `+`: with rows, the hover opens the band, which carries its own.
    /// Headless: `nav-add-remote folded`.
    function addableAt(kind) {
        return kind === "remote" && rail.countOf(kind) === 0
    }

    /// One cell's height: an `iconXl` mark over a `fontSm` line (デザイン規約 §寸法) — the ☰ cell's (`toolbarHeight`),
    /// so the height a hand can press does not change down the column (user decision — ask before changing). Not
    /// square: width and height answer two questions. The width is the window's edge — the rail is the one column
    /// flush against it — and is the ☰ mark at the column's head plus the usual step (`railWidth`; a section's mark
    /// plays no part). The height is the hit, set by the band that ☰ stands on. Squared, the narrow edge would bind
    /// the height or the wide hit would push the edge out.
    readonly property int cellHeight: Theme.toolbarHeight
    /// The ground under the last number, which a cell otherwise keeps none of (user decision — ask before changing):
    /// at zero, a window at its floor presses the last number onto the `>_` row under it (or the log's band). The last
    /// cell's own, so its wash and its hand reach down to what lies under the rail with no unpainted strip between.
    /// Half a step, so the wash stops where it still reads level with the other cells'; a whole `spaceXs` makes that
    /// cell look taller. The floor counts it (`wholeHeight`, `SidebarPane.floorHeight`).
    readonly property int tailRoom: Theme.spaceXs / 2
    /// The foot of a cell's mark box. The number's line stands on the cell's foot and reaches `spaceXs` up into the
    /// box — the mark's air under its ink and the line's own would make the gap inside the pair wider than between
    /// cells, and mark and number would stop reading as a pair. The number keeps `fontSm` (it is the section's heading
    /// too): the pairing is done by the gap. All the cell's slack goes above the mark, none under the number (user
    /// decision — ask before changing): split above and below, the top cell would pay half, since the fold block sits
    /// flush on it — one share where every other cell pays two (below + above). The ink may still differ; the gap
    /// written is one.
    readonly property int markFoot: rail.cellHeight - Theme.fontSmLine + Theme.spaceXs

    /// Where a count's ink starts in its cell, inside the number's line: centred between the mark's box and half a step
    /// over the line's foot — the same gap under the mark whatever the face, inside the line, so the gaps written above
    /// (`markFoot`) stand. Asked of the ink, not left to the face: a line's baseline
    /// falls its ascent down — at 12px about 11px in Hiragino Sans (macOS), 13 in Yu Gothic UI, 14 in Noto Sans JP —
    /// and the shallow one lifts the digits into the mark. Exact, fraction and all; the label's whole-pixel place
    /// rounds it, a half down the cell, so the odd pixel of a whole room goes above. The room is halved as it is: a
    /// face whose ink is a fraction short of it (Hiragino Sans's digits, 9.8px in 10) has no pixel to give, and a
    /// fraction counted as one pushes the ink under the room's foot. Not a fixed line box (`Text.FixedHeight`): that
    /// fixes the height only, the baseline still falls the ascent, and digits stacked under the mark bite into it on
    /// macOS alone.
    ///
    /// Headless `nav-fold`'s `counts=` says whether the ink stands centred under the mark's box (the `fonts` stage is
    /// what the macOS push's pictures read). Headless Windows draws in Noto Sans JP, not Yu Gothic UI (verify-ui
    /// windows.md, the `.ttc` line); Yu Gothic UI's figures come from an offscreen `qmltestrunner` handed
    /// `C:/Windows/Fonts/YuGothM.ttc` / `YuGothR.ttc` by `FontLoader`. A real window's DirectWrite at 100% hints fully
    /// and rounds the ascent to whole pixels (`QFontEngine::initializeHeightMetrics`); scaled up, it does not.
    function countInkTop(ink) {
        const room = rail.cellHeight - Theme.spaceXs / 2 - rail.markFoot - ink.tightBoundingRect.height
        return rail.markFoot + room / 2
    }
    /// The fold block and every cell. The rail is not clipped, so a pane shorter than this draws the last cells over
    /// what lies under it (`SidebarPane.floorHeight`).
    readonly property int wholeHeight: Theme.headerHeight + rail.sections.length * rail.cellHeight + rail.tailRoom

    /// Automation-only, like `GraphPane.view` (rules-refs/app-ui.md): the cells, each handing over its mark and its
    /// count (PGG_AUTO_ACT=nav-fold).
    readonly property alias autoCells: cells

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
            id: cells
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
                /// Automation: the mark and the count, for `NavProbe.countsInk`.
                readonly property alias autoMark: sectionIcon
                readonly property alias autoCount: countLabel

                width: Theme.railWidth
                height: rail.cellHeight + (cell.index === rail.sections.length - 1 ? rail.tailRoom : 0)
                // The open section's cell keeps the wash, to say which cell opened it. An empty cell does not wash
                // unless it can be pressed (規約 §無効).
                color: (cellHover.hovered && (!cell.empty || cell.addLive)) || cell.open
                       ? Theme.bgHover : "transparent"
                // A wordless `+` names itself by tooltip (規約 §hover のツールチップ), in the band's own words
                // (デザイン規約 §リモートを書き留める).
                readonly property TipAsk addTip: TipAsk {
                    target: cell
                    asked: cell.addLive && cellHover.hovered
                    words: Words.addRemote
                }

                NavIcon {
                    id: sectionIcon
                    anchors.horizontalCenter: parent.horizontalCenter
                    // Hung from `markFoot`: all the slack above the mark, so the fold block to the first mark is the
                    // same gap as between cells.
                    y: rail.markFoot - sectionIcon.height
                    kind: cell.modelData.icon
                    // Off the graph drops a step (デザイン規約 §暗く落とした段). Empty is unavailable, so grey —
                    // REMOTES included; its `+` keeps its colour.
                    tint: cell.empty ? Theme.textMuted : cell.offGraph ? Theme.refTagDim : cell.modelData.tint
                    // The cell's remainder above the number (`iconXl` = 40 less the `fontSm` line; デザイン規約 §寸法),
                    // with no padding around the box: the box already holds `(iconXl − the mark's step) / 2` of air
                    // (§余白), and a `spaceXs` outside it would take that room twice and shrink the one ink a wordless
                    // cell has.
                    width: Theme.iconXl
                    height: Theme.iconXl
                    // The TAGS header's eye (NavHeader) on the shoulder, in both states — struck through when off —
                    // since a badge in one state only reads as absent in the other. Not on an empty section (規約 §無効).
                    // A `spaceXs` in and up from the corner (`ShoulderBadge`): half out of the corner as drawn plain,
                    // it stands at the cell's right edge with all the air on one side; centred on the corner, half
                    // of it would sit in the cell above, beside another section's count. Both badges share the
                    // corner — one moved alone would make two corners.
                    ShoulderBadge {
                        visible: cell.taggable && !cell.empty
                        kind: rail.tagsShown ? "eye" : "eye-off"
                        tint: sectionIcon.tint
                    }
                    // The band's `+` (NavHeader), while the section is empty, in the eye's seat; it keeps the section's
                    // colour over the greyed mark (規約 §無効).
                    ShoulderBadge {
                        visible: cell.addable
                        kind: "plus"
                        tint: cell.addLive ? cell.modelData.tint : Theme.textMuted
                    }
                }
                // Caption and count at once, so the caption's colour (NavHeader). Seated by its ink (`countInkTop`), in
                // whole pixels: a half pixel smears the digits. `Math.round` takes a half down the cell
                // (`countInkTop` says why that way).
                PlainTextLabel {
                    id: countLabel
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: Math.round(rail.countInkTop(countInk) - countLabel.baselineOffset - countInk.tightBoundingRect.y)
                    text: cell.sectionCount
                    color: cell.empty ? Theme.textMuted : Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                // Every digit, so the counts stand level whatever they hold. Five digits (45,000 tags, the reference
                // repository's) fill the cell's width exactly; the step stays — the ink only touches the edge, and
                // only a repository with tens of thousands of refs counts that high.
                TextMetrics {
                    id: countInk
                    font: countLabel.font
                    text: "0123456789"
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

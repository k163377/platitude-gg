pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The folded sidebar's one open section, and the bookkeeping that says
// when it is open. Which one it is, and whether it still has the
// pointer, are held here rather than on a cell: the cell is left behind
// the moment the pointer walks into what it opened.
AppCard {
    id: peek

    required property var repoTab
    required property var workTree
    /// The folded rail: which model and which mark each section has.
    required property var rail
    /// Who the rows report their gestures to (`NavList.gestures`).
    required property var gestures
    /// The pane this comes out of — the panel opens flush against its
    /// edge and stops inside it — and the width the list goes back to.
    required property real paneW
    required property real paneH
    required property real listW
    /// What holds it open with the pointer elsewhere: a menu raised from
    /// one of its rows is standing over it, and taking the row away from
    /// under an open menu reads as the row having gone.
    required property bool pinned
    /// The `+` on the band above these rows is held (`SidebarPane.doorsHeld`) — the same band, so the same answer.
    property bool addHeld: false

    /// Which section is open, and where the cell that opened it begins.
    property string kind: ""
    property real top: 0
    /// The pointer is still on that cell.
    property bool wanted: false

    signal refActivated(string oidHex)
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    /// Right-click on the row a remote itself stands on — the same menu the open list raises.
    signal remoteMenuRequested(string name)
    signal addRemoteRequested()

    readonly property var sectionModel: peek.kind === "" ? null : peek.rail.modelOf(peek.kind)
    // The section headers' own words, said again for the one section the
    // folded list shows: the rail has only an icon to name it with.
    readonly property string caption:
        peek.kind === "branch" ? qsTr("BRANCHES")
        : peek.kind === "remote" ? qsTr("REMOTES")
        : peek.kind === "worktree" ? qsTr("WORKTREES")
        : peek.kind === "stash" ? qsTr("STASHES")
        : peek.kind === "tag" ? qsTr("TAGS") : ""

    function openAt(kind, top) {
        peek.kind = kind
        peek.top = top
        peek.wanted = true
        peek.open()
    }
    /// A click landed on the cell the pointer is resting on. The section
    /// standing beside the rail goes away, and a second click brings it
    /// back — the hover cannot, because the pointer has not moved and so
    /// nothing about it has changed.
    function toggleAt(kind, top) {
        // Asked of `kind` rather than the popup: on the way out it is
        // still visible, and a click that arrived then would close what it
        // was meant to open.
        if (peek.kind === kind)
            peek.shut()
        else
            peek.openAt(kind, top)
    }
    /// The pointer left a cell. Only the cell whose section is open can
    /// take it away — the one being left on the way to another has
    /// already been replaced by the time this runs, in whichever order
    /// the two arrive.
    function leaveAt(kind) {
        if (peek.kind === kind)
            peek.wanted = false
        peek.settle()
    }
    /// Ask for the beat by hand, where the answer has changed somewhere
    /// `lit` cannot see it — a cell saying the pointer left it.
    function settle() {
        peekKeeper.settle()
    }
    function shut() {
        peek.close()
    }
    // What the section leaves behind, whichever way it went: the beat
    // running out, a click on the cell that opened it, Escape from
    // inside it.
    //
    // `aboutToHide` rather than `closed`: `kind` has to be clear before
    // the next click can arrive, and the popup is still visible for as
    // long as it takes to go — `toggleAt` reads `kind` for exactly that
    // reason.
    onAboutToHide: {
        // The mark on the row a click last landed on goes with the rows
        // it was on: the rename gesture's second click has to be aimed
        // at a mark that stayed on screen (デザイン規約 §左メニューの所作),
        // and this list comes and goes with the pointer. Kept, it makes
        // the first click of a later peek a second one, which puts the
        // whole list back over the diff the fold was made for. `gestures`
        // is asked for because closing a tab takes the page's pieces down
        // before this popup.
        if (peek.kind !== "" && peek.gestures)
            peek.gestures.forgetClicks()
        peek.kind = ""
        peek.wanted = false
        // The list it was in has gone, so the pointer is not in it
        // whatever the last hover said.
        peek.contentPointed = false
    }
    /// Smoke hooks (PG_AUTO_ACT=nav-reclick): the three the open list
    /// answers, for the section standing beside the folded rail.
    function clickRow(index) {
        return peekList.clickRow(index)
    }
    function rowArmed(index) {
        return peekList.rowArmed(index)
    }
    function rowGuarded(index) {
        return peekList.rowGuarded(index)
    }
    function rowFocused(index) {
        return peekList.rowFocused(index)
    }
    function rowInView(index) {
        return peekList.rowInView(index)
    }
    function scrollToEnd() {
        peekList.scrollToEnd()
    }
    // The section opens flush against the rail, so walking into it takes
    // the pointer off the cell, and walking back out puts it on again —
    // the beat between the two is the same one every hover card in the
    // app waits (`HoverCardHost`, デザイン規約 §hover のツールチップ). What
    // holds it up is the cell, the menu standing over it, or the pointer
    // being down in the section itself (`pointerInside`).
    HoverCardHost {
        id: peekKeeper
        card: peek
        pointedAt: peek.wanted
        grace: peek.pinned
    }

    // Flush against the rail, with nothing in between for the pointer
    // to fall through, and starting level with the cell that opened
    // it. It only ever grows downwards from there: a section with more
    // rows than the pane can hold would otherwise be laid out from the
    // top edge of the pane, nowhere near the cell it came out of
    // (a repository with 45,000 tags puts every peek up there —
    // observed). The rows that do not fit scroll.
    x: peek.paneW
    y: peek.top
    width: peek.listW
    // As tall as it has rows, and never past the foot of the pane it
    // comes out of. It never opens with no rows at all — a cell
    // holding a zero does not open (NavRail).
    height: Math.min(Theme.headerHeight + peekList.count * Theme.rowHeight + Theme.borderWidth,
                     Math.max(0, peek.paneH - peek.top))
    padding: 0
    margins: 0
    // Leaving it is what closes it (above). Escape is for the reader
    // whose pointer is already inside it.
    closePolicy: Popup.CloseOnEscape

    // It keeps the list's own ground rather than a menu's: what is in
    // it is the sidebar, and the header band would be lost against
    // `bgElevated`. The frame is what floats it (規約 §メニュー), and
    // there is no rounding on a panel that starts flush against the
    // rail.
    faceColor: Theme.bgSurface
    faceRadius: 0

    contentItem: ColumnLayout {
        spacing: 0
        HoverHandler {
            id: peekHover
            // The other half of leaving. The cells can only see the
            // way back over themselves; walking out of the list the
            // other way — right into the diff or the graph, or off
            // its top or bottom edge — is an exit no cell is told
            // about, and without this it raises no event at all.
            //
            // The card's own `contentPointed`, written rather than
            // bound: the smoke hooks write the same one, so a headless
            // run and a real pointer come to a single answer (the same
            // shape as the diff's hunk hover — 規約 §diff の中のステージ).
            onHoveredChanged: peek.contentPointed = peekHover.hovered
        }
        // The open list's own band, carrying what that section carries
        // wherever it stands — the tags eye, the `+` that writes a remote
        // down. The one thing it drops is the fold arrow: this list is
        // the only thing on screen, so there is nothing to fold away to,
        // and a mark that answers nothing is worse than no mark
        // (デザイン規約 §長押し).
        NavHeader {
            caption: peek.caption
            iconKind: peek.rail.sectionOf(peek.kind).icon
            iconTint: peek.rail.sectionOf(peek.kind).tint
            count: peek.sectionModel ? peek.sectionModel.total : 0
            foldable: false
            showTagToggle: peek.kind === "tag"
            tagsShown: peek.repoTab.tagsShown
            onTagsToggled: shown => peek.repoTab.setTagsShown(shown)
            showAddRemote: peek.kind === "remote"
            addHeld: peek.addHeld
            onAddRemoteRequested: peek.addRemoteRequested()
        }
        NavList {
            id: peekList
            sectionModel: peek.sectionModel
            expanded: true
            kindHint: peek.kind
            gestures: peek.gestures
            stretch: true
            headTracks: peek.kind === "branch" && peek.workTree.upstream !== ""
            headAhead: peek.workTree.ahead
            headBehind: peek.workTree.behind
            remotesPacked: peek.repoTab.remoteNames
            markedRemote: peek.repoTab.pushDefault
            onRefActivated: oidHex => peek.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => peek.refMenuRequested(kind, name, full, oidHex)
            onRemoteMenuRequested: name => peek.remoteMenuRequested(name)
        }
    }
}

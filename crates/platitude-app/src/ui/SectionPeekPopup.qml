pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The folded sidebar's one open section, and the bookkeeping that says
// when it is open. Which one it is, and whether it still has the
// pointer, are held here rather than on a cell: the cell is left behind
// the moment the pointer walks into what it opened.
Popup {
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

    /// Which section is open, and where the cell that opened it begins.
    property string kind: ""
    property real top: 0
    /// The pointer is still on that cell.
    property bool wanted: false
    /// The pointer is down in the open section itself. The popup's own
    /// hover writes this and so do the smoke hooks, so a headless run and
    /// a real pointer come to one answer (the same shape as the diff's
    /// hunk hover — 規約 §diff の中のステージ). What happens when it goes
    /// false hangs off the change rather than off the hover, so there is
    /// no way to say "gone" without the settle that has to follow.
    property bool entered: false

    signal refActivated(string oidHex)
    signal refMenuRequested(string kind, string name, string full, string oidHex)

    onEnteredChanged: {
        if (!peek.entered)
            peek.settle()
    }
    // The menu that was standing over it has gone: whether the pointer
    // came back in the meantime decides what happens now.
    onPinnedChanged: peek.settle()

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
    // The section opens flush against the rail, so walking into it takes
    // the pointer off the cell, and walking back out puts it on again.
    // The two hovers change in different frames and in no fixed order —
    // between them the pointer is on neither, and `Qt.callLater` lands
    // there and closes the section under the hand. So the answer waits
    // a beat (デザイン規約 §hover のツールチップ), by which time
    // whichever of the two now holds the pointer has said so.
    function settle() {
        peekSettle.restart()
    }
    function shut() {
        // The mark on the row a click last landed on goes with the rows
        // it was on: the rename gesture's second click has to be aimed
        // at a mark that stayed on screen (デザイン規約 §左メニューの所作),
        // and this list comes and goes with the pointer. Kept, it makes
        // the first click of a later peek a second one, which puts the
        // whole list back over the diff the fold was made for
        // (2026-08-18 ユーザー報告). `gestures` is asked for because
        // closing a tab takes the page's pieces down before this popup.
        if (peek.kind !== "" && peek.gestures)
            peek.gestures.activeKey = ""
        peek.close()
        peek.kind = ""
        peek.wanted = false
        // The list it was in has gone, so the pointer is not in it
        // whatever the last hover said.
        peek.entered = false
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
    Timer {
        id: peekSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!peek.entered && !peek.wanted && !peek.pinned)
                peek.shut()
        }
    }

    // Flush against the rail, with nothing in between for the pointer
    // to fall through, and starting level with the cell that opened
    // it. It only ever grows downwards from there: a section with more
    // rows than the pane can hold would otherwise be laid out from the
    // top edge of the pane, nowhere near the cell it came out of
    // (a repository with 45,000 tags puts every peek up there —
    // reported 2026-08-08). The rows that do not fit scroll.
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
    background: Rectangle {
        color: Theme.bgSurface
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }

    contentItem: ColumnLayout {
        spacing: 0
        HoverHandler {
            id: peekHover
            // The other half of leaving. The cells can only see the
            // way back over themselves; walking out of the list the
            // other way — right into the diff or the graph, or off
            // its top or bottom edge — is an exit no cell is told
            // about, and without this it raises no event at all.
            onHoveredChanged: peek.entered = peekHover.hovered
        }
        NavHeader {
            caption: peek.caption
            iconKind: peek.rail.sectionOf(peek.kind).icon
            iconTint: peek.rail.sectionOf(peek.kind).tint
            count: peek.sectionModel ? peek.sectionModel.total : 0
            // Nothing to fold away to: this list is the only thing on
            // screen. What closes it is the pointer leaving.
            foldable: false
            showTagToggle: peek.kind === "tag"
            tagsShown: peek.repoTab.tagsShown
            onTagsToggled: shown => peek.repoTab.setTagsShown(shown)
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
            onRefActivated: oidHex => peek.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => peek.refMenuRequested(kind, name, full, oidHex)
        }
    }
}

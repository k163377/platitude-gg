pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The folded sidebar's one open section and when it is open. Which one, and whether it still has the pointer, are
// held here, not in the rail cell: the pointer leaves the cell the moment it walks into the section.
AppCard {
    id: peek

    required property var repoTab
    required property var workTree
    /// The folded rail: which model and which mark each section has.
    required property var rail
    /// Who the rows report their gestures to (`NavList.gestures`).
    required property var gestures
    /// The pane this opens flush against and stays inside, and the list's width.
    required property real paneW
    required property real paneH
    required property real listW
    /// Holds it open with the pointer elsewhere: a menu raised from one of its rows stands over it, and taking the
    /// row from under an open menu reads as the row having gone.
    required property bool pinned
    /// The `+` on the band above these rows is held (`SidebarPane.doorsHeld`) — the same band, so the same answer.
    property bool addHeld: false

    /// Which section is open, and where the cell that opened it begins.
    property string kind: ""
    property real top: 0
    /// The pointer is still on that cell.
    property bool wanted: false
    /// The section whose row last raised a menu: the one a menu raised here holds open (`SidebarPane`'s `pinned`). A
    /// rest on another cell while that menu stands opens that section, which goes as a hover does.
    property string menuKind: ""
    /// Where this section last saw the hand, in its own coordinates — tells a hand that moved from a layout that
    /// moved under it (`SidebarPane`, and the handler below).
    property point handAt: Qt.point(-1, -1)

    signal refActivated(string oidHex)
    signal refMenuRequested(string kind, string name, string full, string oidHex, string aim)
    /// Right-click on the row a remote itself stands on — the same menu the open list raises.
    signal remoteMenuRequested(string name)
    signal addRemoteRequested()

    readonly property var sectionModel: peek.kind === "" ? null : peek.rail.modelOf(peek.kind)
    // The section headers' words: the rail names a section only by icon.
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
    /// A click on the cell under the pointer toggles the section — the hover cannot bring it back, as the pointer
    /// has not moved.
    function toggleAt(kind, top) {
        // By `kind`, not `visible`: the popup is still visible on the way out, and a click then would close what it
        // meant to open.
        if (peek.kind === kind)
            peek.shut()
        else
            peek.openAt(kind, top)
    }
    /// The pointer left a cell. Only the open section's cell can take it away — a cell left for another has already
    /// been replaced, whichever order the two arrive in.
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
    // Cleanup for every way out (the beat, a click on the cell, Escape). In `aboutToHide`: `kind` must be clear
    // before the next click, while the popup is still visible (`toggleAt`).
    onAboutToHide: {
        // The last-click mark goes with these rows: kept, a later peek's first click would count as the rename
        // gesture's second (デザイン規約 §左メニューの所作). `gestures` is checked because closing a tab takes it
        // down before this popup.
        if (peek.kind !== "" && peek.gestures)
            peek.gestures.forgetClicks()
        peek.kind = ""
        peek.wanted = false
        peek.menuKind = ""
        // The list has gone, whatever the last hover said.
        peek.contentPointed = false
    }
    /// Smoke hooks (PGG_AUTO_ACT=nav-reclick): the open list's, for the section beside the folded rail.
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
    /// Rest the pointer on one of these rows (PGG_AUTO_ACT=nav-peek-open), through the list's `pointedTipRow`.
    function pointTipAt(index) {
        peekList.pointedTipRow = index
    }
    function rowNameAt(index) {
        return peekList.rowNameAt(index)
    }
    /// The open row's words and the row itself, as the sections answer them.
    function openWords() {
        return peekList.openWords()
    }
    function openFactsItem() {
        return peekList.openFactsItem()
    }
    function openShown() {
        return peekList.openShown()
    }
    function scrollToEnd() {
        peekList.scrollToEnd()
    }
    /// Automation only: the list itself, for a run that sends it by its own hand and reads where it went.
    readonly property alias list: peekList
    /// Automation only: the beat after a leave is still running — what a run waits out to read whether the section
    /// was kept (PGG_AUTO_ACT=menu-peek).
    readonly property bool settling: peekKeeper.keep.running
    HoverCardHost {
        id: peekKeeper
        card: peek
        pointedAt: peek.wanted
        grace: peek.pinned
    }

    // Flush against the rail, level with the cell that opened it, and only growing downwards — a section taller than
    // the pane would otherwise be laid out from the pane's top, away from its cell. Rows that do not fit scroll.
    x: peek.paneW
    y: peek.top
    width: peek.listW
    // Header band (`NavHeader`, `rowHeight`) + rows + the frame's two lines, at most to the pane's foot. Never
    // empty: a cell holding a zero does not open (NavRail).
    height: Math.min(Theme.rowHeight + peekList.count * Theme.rowHeight + 2 * Theme.borderWidth,
                     Math.max(0, peek.paneH - peek.top))
    // Keeps the frame out from under the content: `Popup` lays content over the whole face, and at zero the opaque
    // header band paints the frame out.
    padding: Theme.borderWidth
    margins: 0
    // The hand walking in off the rail crosses that padding, which the content's handler cannot see; a hand resting
    // on it would otherwise close the section (デザイン規約 §左メニューを畳む).
    tracksPointer: true
    // Leaving closes it (above); Escape is for a pointer already inside.
    closePolicy: Popup.CloseOnEscape

    // The sidebar's own ground (the header band would be lost on `bgElevated`); the frame floats it (規約 §メニュー),
    // and square corners since it starts flush against the rail.
    faceColor: Theme.bgSurface
    faceRadius: 0

    contentItem: ColumnLayout {
        spacing: 0
        HoverHandler {
            id: peekHover
            // The other half of leaving: an exit to the right or off the top or bottom edge passes no cell. Writes the
            // card's `contentPointed`, as the smoke hooks do, so headless and a real pointer agree.
            onHoveredChanged: {
                peek.contentPointed = peekHover.hovered
                // Heard for the rows in here, which re-read their hover only on a move of the hand
                // (`NavItemDelegate.syncHover`): a popup stands outside the panel, whose watch never sees in here.
                peek.handAt = Qt.point(-1, -1)
                if (peek.gestures !== null)
                    peek.gestures.handStirred()
            }
            // By position, not notification, as `SidebarPane` weighs it: a row growing in here reports a fresh point.
            onPointChanged: {
                if (peekHover.point.position.x === peek.handAt.x
                        && peekHover.point.position.y === peek.handAt.y)
                    return
                peek.handAt = peekHover.point.position
                if (peek.gestures !== null)
                    peek.gestures.handStirred()
            }
        }
        // The section's own band (the tags eye, the remote `+`) without the fold arrow: there is nothing to fold away
        // to here (デザイン規約 §左メニューの所作).
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
            // Rows open under themselves as in the sections — all but STASHES, which keeps the tooltip. Working copies
            // are read off their own section whichever list the row is in.
            offersFacts: peek.kind === "branch" || peek.kind === "remote" || peek.kind === "worktree"
                         || peek.kind === "tag"
            worktreesModel: peek.rail !== null ? peek.rail.modelOf("worktree") : null
            // For REMOTES: where the branch reading a remote-tracking row stands, as the sections hand it
            // (`NavSections`).
            branchesModel: peek.rail !== null && peek.kind === "remote" ? peek.rail.modelOf("branch") : null
            pushRemote: peek.repoTab.defaultRemote
            stretch: true
            remoteNames: peek.repoTab.remoteNames
            markedRemote: peek.repoTab.pushDefault
            originRemote: peek.repoTab.markedOrigin
            onRefActivated: oidHex => peek.refActivated(oidHex)
            // Marked before the page raises the menu, so the hold is there as it comes up.
            onRefMenuRequested: (kind, name, full, oidHex, aim) => {
                peek.menuKind = peek.kind
                peek.refMenuRequested(kind, name, full, oidHex, aim)
            }
            onRemoteMenuRequested: name => {
                peek.menuKind = peek.kind
                peek.remoteMenuRequested(name)
            }
        }
    }
}

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
    required property var worktree
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

    /// Which section is open, and where the cell that opened it begins — the seat's target, followed (`follow`).
    property string kind: ""
    property real top: 0
    /// The panel's hand (`HandWatch`), which the rows in here read as the panel's rows do; this popup stands outside
    /// the panel, so its own handler counts as the panel's surface (`HandWatch.alsoOver` ← `hover`).
    property HandWatch paneHand: null
    readonly property alias hover: peekHover

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

    /// The pointer came onto a cell (`NavRail.enterAt`): that section opens at once (規約 §hover のツールチップ「展開は
    /// 即時に開く」), in place of the one open. **Each section is a target of its own** (`keyOf`) — the popup is one,
    /// what it shows is not: a section held by its own rows (a menu raised on one, a name box open on one — `pinned`)
    /// is not handed over, and the cell the hand is on meanwhile is an ask kept for the hold lifting.
    function openAt(kind, top) {
        peekKeeper.point(peek.targetOf(kind, top), true)
    }
    /// A click on the cell under the pointer toggles the section — closed, the hover cannot bring it back, as the
    /// pointer has not moved (the seat's latch); a click can (`HoverSeat.press`), whatever the latch says.
    function toggleAt(kind, top) {
        // By `kind`, not `visible`: the popup is still visible on the way out, and a click then would close what it
        // meant to open.
        if (peek.kind === kind)
            peekKeeper.takeDown("click", true)
        else
            peekKeeper.press(peek.targetOf(kind, top))
    }
    /// The pointer left a cell: the section's own leave — the open one's lets it go (a cell left for another has
    /// already been replaced, whichever order the two arrive in), and a kept ask's takes that ask with it.
    function leaveAt(kind) {
        peekKeeper.unpoint(peek.keyOf(kind))
    }
    function keyOf(kind) {
        return "peek:" + kind
    }
    function targetOf(kind, top) {
        return { "key": peek.keyOf(kind), "kind": kind, "top": top }
    }
    /// Down now, as the owner asks: a menu raised elsewhere, the rail unfolding.
    function shut() {
        peekKeeper.takeDown("owner", false)
    }
    function follow(target) {
        peek.kind = target.kind
        peek.top = target.top
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
    readonly property bool settling: peekKeeper.leaving
    readonly property HoverSeat peekKeeper: HoverSeat {
        id: peekKeeper
        card: peek
        hand: peek.paneHand
        // The section opens the moment the hand is on its cell; a cell asks only as the hand comes onto it, so a
        // section the reader closed comes back with the hand leaving the cell and coming back, or a click.
        expands: true
        onOpenAsked: target => {
            peek.follow(target)
            peek.open()
        }
        // The open section's cell asked again — where it now begins, the rail having moved, is followed.
        onTargetChanged: {
            if (peekKeeper.shown && peekKeeper.target !== null)
                peek.follow(peekKeeper.target)
        }
    }
    // A menu raised from one of its rows, a box open on one of them: held with the pointer elsewhere.
    onPinnedChanged: peekKeeper.hold("pinned", peek.pinned)
    Component.onCompleted: peekKeeper.hold("pinned", peek.pinned)
    /// The section open has rows to show; emptied under the open popup (its last branch deleted), the popup goes as
    /// the owner's close — a cell holding a zero opens nothing (`NavRail`), and keeps nothing.
    readonly property bool sectionStands: peek.kind === ""
                                          || (peek.sectionModel !== null && peek.sectionModel.total > 0)
    onSectionStandsChanged: {
        if (!peek.sectionStands)
            peekKeeper.forget(target => target.kind === peek.kind, "gone")
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
    // Leaving closes it (above).
    closesOnPressOutside: false

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
                // Heard for the rows in here, which read the panel's hand (`NavItemDelegate.syncHand`): a popup
                // stands outside the panel, whose own handler never sees in here.
                if (peek.paneHand !== null)
                    peek.paneHand.stir(peek.paneHand.here())
            }
            // By position (`HandWatch.stir` counts a place, not a telling): a row growing in here reports a fresh
            // point.
            onPointChanged: {
                if (peek.paneHand !== null)
                    peek.paneHand.stir(peekHover.point.scenePosition)
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
            paneHand: peek.paneHand
            // Rows open under themselves as in the sections — all but STASHES, which keeps the tooltip. Worktrees
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
            // The page notes where the menu was raised before it comes up, so the hold (`pinned`) is there as it does.
            onRefMenuRequested: (kind, name, full, oidHex, aim) => peek.refMenuRequested(kind, name, full, oidHex, aim)
            onRemoteMenuRequested: name => peek.remoteMenuRequested(name)
        }
    }
}

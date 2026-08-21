import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Navigation sidebar: fixed section headers, each section scrolls
// inside its own list. Owns its filter text and per-section fold
// state; what a click means (jump, menu, open a tab) is reported
// upward.
Rectangle {
    id: sidebar

    required property var repoTab
    required property var workTree
    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel

    /// Folded down to the rail. Held by the page: what folds it is going
    /// to include opening a diff, and that is the page's to know.
    required property bool collapsed

    signal refActivated(string oidHex)
    /// Right-click on a row. `kind` is the section it came from, `name`
    /// what the row shows and `full` what git knows it by.
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    signal worktreeActivated(string path)
    /// Double-click on a branch row: `kind` is the chip letter the page's
    /// dispatcher reads ("L" local / "R" remote).
    signal refSwitchRequested(string kind, string name)
    /// A name was typed for a new branch on a tag's commit.
    signal branchAtRequested(string oidHex, string name)
    /// A row was renamed. `kind` is the section ("branch" / "tag" /
    /// "stash"), `id` what git knows the row by.
    signal renameSubmitted(string kind, string id, string name)
    signal foldRequested(bool collapse)
    /// The `+` on the REMOTES band was pressed: a remote is to be written
    /// down. Raised from the open list and from the section the folded
    /// rail opens alike — one band, wherever it is standing.
    signal addRemoteRequested()
    /// Right-click on the row a remote itself stands on: what to do with that remote, rather than with a ref.
    signal remoteMenuRequested(string name)

    // ---- the row gestures ------------------------------------------
    // Held beside the lists rather than in one (`SidebarRowGestures`).
    // The pane keeps the names its own callers already reach for: the
    // page opens the box from the row menu, and the smoke hooks read
    // which row has one.
    SidebarRowGestures {
        id: rowGestures
        host: sidebar
        repoTab: sidebar.repoTab
        remotesModel: sidebar.remotesModel
    }
    property alias activeKey: rowGestures.activeKey
    property alias editKey: rowGestures.editKey
    /// A menu raised from one of the folded list's rows is standing over
    /// it. The page's to set — the menus are its.
    property alias menuOpen: rowGestures.menuOpen
    function stopEdit() {
        rowGestures.stopEdit()
    }
    function submitEdit(text) {
        rowGestures.submitEdit(text)
    }
    function beginRename(kind, id, text) {
        rowGestures.beginRename(kind, id, text)
    }
    function beginBranchAt(kind, id, oidHex) {
        rowGestures.beginBranchAt(kind, id, oidHex)
    }
    function activateRow(kind, name, full, oidHex) {
        rowGestures.activateRow(kind, name, full, oidHex)
    }

    /// Smoke hook (PG_AUTO_ACT=nav-filter): type into the filter band.
    /// Written into the field itself, so what the sections are asked is
    /// what a typist asks them.
    function typeFilter(text) {
        refFilter.text = text
        return refFilter.text
    }

    /// Smoke hook (PG_AUTO_ACT=nav-peek): rest on one section's cell.
    /// Named rather than hovered — hover cannot be injected (verify-ui
    /// スキル). Whether that opens anything is the cell's answer: an
    /// empty section answers no.
    function peekAt(kind) {
        rail.enterAt(kind, rail.topOf(kind))
    }
    /// Smoke hooks (PG_AUTO_ACT=nav-peek-away / nav-peek-shut): walk the
    /// pointer off the cell that opened it, and click that cell. All of
    /// these go in at the rail, so what the cells decide and report is
    /// part of what is being tested (NavRail.enterAt).
    function peekAway(kind) {
        rail.leaveAt(kind)
    }
    function peekTap(kind) {
        rail.tapAt(kind)
    }
    /// Smoke hooks (PG_AUTO_ACT=nav-peek-into / nav-peek-out): the pointer
    /// walked off the cell down into the open section, and then out of the
    /// section the other way (into the diff or the graph) instead of back
    /// over the cell. They write the same `peekEntered` the popup's own
    /// hover writes — the leaving and the being-inside are one state, and
    /// a headless run that cannot say "inside" cannot tell the exit that
    /// closes it from the one that must not.
    function peekInto(kind) {
        rail.leaveAt(kind)
        sidebar.peekEntered = true
    }
    function peekOut() {
        sidebar.peekEntered = false
    }

    /// Smoke hook (PG_AUTO_ACT=nav-close): close one section by putting a
    /// click in where the header band takes one — a hook that set
    /// `expTags` itself would be a second answer, and would open a
    /// section the band itself refuses to (`NavHeader.tap`).
    function closeSection(kind) {
        const head = sidebar.headOf(kind)
        if (head)
            head.tap()
    }
    function headOf(kind) {
        return kind === "branch" ? branchHead
            : kind === "remote" ? remoteHead
            : kind === "worktree" ? worktreeHead
            : kind === "stash" ? stashHead
            : kind === "tag" ? tagHead : null
    }
    /// Smoke hook (PG_AUTO_ACT=nav-add-remote): press the `+` at the end
    /// of the REMOTES band. It goes in at the band's own signal, so what
    /// answers is the page's wiring and not a second way in.
    function tapAddRemote() {
        remoteHead.addRemoteRequested()
    }
    /// Smoke hook (PG_AUTO_ACT=tags-eye): press the eye at the end of the
    /// TAGS band, at the button's own press (`NavHeader.tapTags`).
    function tapTagEye() {
        tagHead.tapTags()
    }
    /// Where a section's header band has come to rest, and where the
    /// ground under the last section begins — what PG_AUTO_ACT=nav-close
    /// reads to see the sections packed against the top.
    function headerTopOf(kind) {
        const head = sidebar.headOf(kind)
        return head ? head.y : -1
    }
    readonly property real groundTop: ground.y
    /// Where the section the folded rail has open begins and ends, for the
    /// smoke hooks (PG_AUTO_ACT=nav-peek). The panel is a popup, so a
    /// headless run reads its placement here rather than off the picture:
    /// it starts at the top edge of the cell that opened it and stops
    /// inside the pane, whatever the section's row count.
    readonly property real peekY: peek.y
    readonly property real peekBottom: peek.y + peek.height

    /// Smoke hooks (PG_AUTO_ACT=nav-tip): rest the pointer on one
    /// section's row, and read back what that row answers about a
    /// tooltip. Hover cannot be injected (verify-ui スキル), so it goes in
    /// at the same `pointedTipRow` the file lists carry, and what comes
    /// out is the row's own attached ToolTip and the shared instance.
    function listOf(kind) {
        return kind === "branch" ? branchList
            : kind === "remote" ? remoteList
            : kind === "worktree" ? worktreeList
            : kind === "stash" ? stashList
            : kind === "tag" ? tagList : null
    }
    function pointTipAt(kind, row) {
        const list = sidebar.listOf(kind)
        if (list)
            list.pointedTipRow = row
    }
    function tipWordsAt(kind, row) {
        const list = sidebar.listOf(kind)
        return list ? list.rowTipWords(row) : ""
    }
    function tipNameAt(kind, row) {
        const list = sidebar.listOf(kind)
        return list ? list.rowNameAt(row) : ""
    }
    /// The current branch's sticky stand-in, under the same pointer: it
    /// rides the edge its own row went out of, so resting on that row is
    /// resting on this (`HeadPinRow`). Named separately as well, since the
    /// row it stands for has no place in the list at all while a filter
    /// hides it — which is one of the two ways the stand-in is on screen.
    property bool headPinPointed: false
    readonly property bool headPinLit: headPin.visible && headPin.pointed
    readonly property string headPinWords: headPin.tipWords

    /// Smoke hook (PG_SCROLL_TO=nav-bottom): jump the branches list to
    /// its end. The current branch's sticky row only changes edges
    /// under scroll, which a headless run cannot produce otherwise.
    function scrollBranchesToEnd() {
        branchList.contentY = Math.max(0, branchList.contentHeight - branchList.height)
    }

    // The width the list goes back to. Read off the pane as it folds
    // rather than fixed, so one that has been widened comes back the
    // width it was left (規約 §レイアウト初期値 is only where it starts).
    property real openWidth: 260
    /// Narrower than this is not a width anybody dragged to.
    readonly property int minOpenWidth: 180
    SplitView.preferredWidth: sidebar.openWidth
    SplitView.minimumWidth: sidebar.minOpenWidth
    color: Theme.bgSurface

    // Folding is a size, and a size is the splitter's business: pinning
    // both ends to the rail's width is what takes the drag away while it
    // is folded. Assigned rather than bound — a drag writes the same
    // attached property, and a binding here would be gone after the first
    // one (leaving the fold with nothing to set).
    onCollapsedChanged: sidebar.applyFold()
    function applyFold() {
        // Whichever way it goes, the one section the rail had open goes
        // with the rail — including when what put the list back was a
        // row in that very section (startEdit).
        sidebar.closePeek()
        // And the box goes with the list it stood in: folded, one left
        // open comes back up under the pointer on a row nobody clicked
        // (2026-08-18 ユーザー報告). `startEdit` puts the list back before
        // opening its own box, so that one is never this one.
        sidebar.stopEdit()
        if (sidebar.collapsed) {
            // Only a width the splitter has actually handed over is worth
            // going back to. A page built already folded — one restored
            // from the last session — has not been laid out yet and
            // reports 0, and folding that in would lose the width the
            // session was left at.
            if (sidebar.width >= sidebar.minOpenWidth)
                sidebar.openWidth = sidebar.width
            sidebar.SplitView.minimumWidth = Theme.railWidth
            sidebar.SplitView.maximumWidth = Theme.railWidth
            sidebar.SplitView.preferredWidth = Theme.railWidth
        } else {
            sidebar.SplitView.minimumWidth = sidebar.minOpenWidth
            sidebar.SplitView.maximumWidth = Number.POSITIVE_INFINITY
            sidebar.SplitView.preferredWidth = sidebar.openWidth
        }
    }

    // Section expansion (filter reveals collapsed sections). What a
    // section is actually showing is its own band's answer
    // (`NavHeader.showsRows`) — one with no rows stays folded whatever
    // is written here, and the fold survives that: a repository with no
    // remotes leaves this alone, so the next one that has some opens the
    // way this reader left it.
    property bool expBranches: true
    property bool expRemotes: true
    property bool expWorktree: true
    property bool expStashes: true
    property bool expTags: true

    // How the height nobody needs is handed out. Every section takes
    // what its own rows want and no more; the ground at the foot of the
    // column takes the rest — spare height inside a section would push
    // everything under it down. Both ends have to name a pull: Qt reads
    // an unset one as a zero and hands it nothing at all. At this
    // distance the ground keeps none of the height while any section
    // still has rows it cannot show, on a pane of any height (measured).
    readonly property int sectionPull: 10000
    readonly property int groundPull: 1

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: !sidebar.collapsed
        SidebarFilterRow {
            id: refFilter
            Layout.fillWidth: true
            onTextChanged: {
                sidebar.branchesModel.setFilter(refFilter.text)
                sidebar.remotesModel.setFilter(refFilter.text)
                sidebar.worktreesModel.setFilter(refFilter.text)
                sidebar.stashesModel.setFilter(refFilter.text)
                sidebar.tagsModel.setFilter(refFilter.text)
            }
            onFoldRequested: sidebar.foldRequested(true)
        }

        NavHeader {
            id: branchHead
            caption: qsTr("BRANCHES")
            iconKind: "branch"
            iconTint: Theme.accent
            count: sidebar.branchesModel.total
            expanded: sidebar.expBranches || refFilter.text !== ""
            onToggled: sidebar.expBranches = !sidebar.expBranches
        }
        NavList {
            id: branchList
            sectionModel: sidebar.branchesModel
            expanded: branchHead.showsRows
            kindHint: "branch"
            gestures: rowGestures
            Layout.verticalStretchFactor: sidebar.sectionPull
            headTracks: sidebar.workTree.upstream !== ""
            headAhead: sidebar.workTree.ahead
            headBehind: sidebar.workTree.behind
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)

            HeadPinRow {
                id: headPin
                // The list is a Flickable: children declared in one are
                // adopted by its content item and scroll away with it.
                // Parenting to the list itself is what keeps this still,
                // so the adoption is written here rather than inside the
                // component (app-ui.md).
                parent: branchList
                pointed: sidebar.headPinPointed
                         || (branchList.pointedTipRow >= 0
                             && branchList.pointedTipRow === sidebar.branchesModel.headRow)
                branchesModel: sidebar.branchesModel
                workTree: sidebar.workTree
                contentY: branchList.contentY
                viewHeight: branchList.height
                width: branchList.width
                onActivated: oidHex => sidebar.refActivated(oidHex)
            }
        }

        NavHeader {
            id: remoteHead
            caption: qsTr("REMOTES")
            iconKind: "remote"
            iconTint: Theme.textSecondary
            count: sidebar.remotesModel.total
            expanded: sidebar.expRemotes || refFilter.text !== ""
            onToggled: sidebar.expRemotes = !sidebar.expRemotes
            // The only band that carries a way to make its own rows, and
            // the only one whose control outlives the section going
            // unavailable (デザイン規約 §左メニューの所作).
            showAddRemote: true
            onAddRemoteRequested: sidebar.addRemoteRequested()
        }
        NavList {
            id: remoteList
            sectionModel: sidebar.remotesModel
            expanded: remoteHead.showsRows
            kindHint: "remote"
            gestures: rowGestures
            remotesPacked: sidebar.repoTab.remoteNames
            markedRemote: sidebar.repoTab.pushDefault
            Layout.verticalStretchFactor: sidebar.sectionPull
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
            onRemoteMenuRequested: name => sidebar.remoteMenuRequested(name)
        }

        // git worktrees (checkouts); the changed-file lists live in the
        // right pane's WIP view. Clicking one opens it as a new tab.
        NavHeader {
            id: worktreeHead
            caption: qsTr("WORKTREES")
            iconKind: "tree"
            iconTint: Theme.success
            count: sidebar.worktreesModel.total
            expanded: sidebar.expWorktree || refFilter.text !== ""
            onToggled: sidebar.expWorktree = !sidebar.expWorktree
        }
        NavList {
            id: worktreeList
            sectionModel: sidebar.worktreesModel
            expanded: worktreeHead.showsRows
            kindHint: "worktree"
            gestures: rowGestures
            Layout.verticalStretchFactor: sidebar.sectionPull
        }

        NavHeader {
            id: stashHead
            caption: qsTr("STASHES")
            iconKind: "stash"
            iconTint: Theme.textSecondary
            count: sidebar.stashesModel.total
            expanded: sidebar.expStashes || refFilter.text !== ""
            onToggled: sidebar.expStashes = !sidebar.expStashes
        }
        NavList {
            id: stashList
            sectionModel: sidebar.stashesModel
            expanded: stashHead.showsRows
            kindHint: "stash"
            gestures: rowGestures
            Layout.verticalStretchFactor: sidebar.sectionPull
            // A stash is a commit: clicking shows its stashed changes
            // in the details pane.
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        NavHeader {
            id: tagHead
            caption: qsTr("TAGS")
            iconKind: "tag"
            iconTint: Theme.refTag
            count: sidebar.tagsModel.total
            expanded: sidebar.expTags || refFilter.text !== ""
            onToggled: sidebar.expTags = !sidebar.expTags
            showTagToggle: true
            tagsShown: sidebar.repoTab.tagsShown
            onTagsToggled: shown => sidebar.repoTab.setTagsShown(shown)
        }
        NavList {
            id: tagList
            sectionModel: sidebar.tagsModel
            expanded: tagHead.showsRows
            kindHint: "tag"
            gestures: rowGestures
            Layout.verticalStretchFactor: sidebar.sectionPull
            onRefActivated: oidHex => sidebar.refActivated(oidHex)
            onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        }

        // The ground under the last section. Nothing stands on it — it
        // is here to be given the height the sections have no rows for,
        // so that they stay packed against the top whichever of them
        // are closed.
        Item {
            id: ground
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.verticalStretchFactor: sidebar.groundPull
        }
    }

    // ---- folded ------------------------------------------------------
    NavRail {
        id: rail
        anchors.fill: parent
        visible: sidebar.collapsed
        branchesModel: sidebar.branchesModel
        remotesModel: sidebar.remotesModel
        worktreesModel: sidebar.worktreesModel
        stashesModel: sidebar.stashesModel
        tagsModel: sidebar.tagsModel
        tagsShown: sidebar.repoTab.tagsShown
        openKind: peek.visible ? sidebar.peekKind : ""
        onPeekRequested: (kind, top) => sidebar.openPeek(kind, top)
        onPeekLeft: kind => sidebar.leavePeek(kind)
        onPeekToggled: (kind, top) => sidebar.togglePeek(kind, top)
        onUnfoldRequested: sidebar.foldRequested(false)
        onAddRemoteRequested: sidebar.addRemoteRequested()
    }

    // ---- the folded list's one open section --------------------------
    // The section itself, and the bookkeeping that says when it is open,
    // live in SectionPeekPopup. The pane keeps the names the rail, the
    // fold and the smoke hooks already call it by.
    property alias peekKind: peek.kind
    property alias peekTop: peek.top
    property alias peekEntered: peek.entered
    /// The open section itself, for the smoke hooks alone (the shape
    /// `GraphPane.view` already has): clicks cannot be injected, so
    /// PG_AUTO_ACT=nav-reclick puts one in at a row in here.
    readonly property var peekSection: peek

    function openPeek(kind, top) {
        peek.openAt(kind, top)
    }
    function togglePeek(kind, top) {
        peek.toggleAt(kind, top)
    }
    function leavePeek(kind) {
        peek.leaveAt(kind)
    }
    function closePeek() {
        peek.shut()
    }

    SectionPeekPopup {
        id: peek
        parent: sidebar
        repoTab: sidebar.repoTab
        workTree: sidebar.workTree
        rail: rail
        gestures: rowGestures
        paneW: sidebar.width
        paneH: sidebar.height
        listW: sidebar.openWidth
        // A box open on one of its rows holds it as firmly as a menu
        // does: it has taken the keyboard, and a name half typed into a
        // list the pointer walked away from is a name lost.
        pinned: sidebar.menuOpen || sidebar.editKey !== ""
        onRefActivated: oidHex => sidebar.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sidebar.remoteMenuRequested(name)
        onAddRemoteRequested: sidebar.addRemoteRequested()
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Navigation sidebar. Owns its per-section fold state; what a click means (jump, menu, open a tab) is reported
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

    /// Folded down to the rail. Held by the page: opening a diff or a rebase plan folds it (`RepoPage.foldForDiff` /
    /// `foldForPlan`).
    required property bool collapsed
    /// The plan's freeze: while a rebase is being composed the only way into a write is the run button, so the list and
    /// the rail wear the disabled step whole (規約 §無効; デザイン規約 §フル interactive rebase). The `>_` band at the
    /// foot stays pressable — it answers "what is git doing" — so dimming the pane whole would paint it as disabled.
    property bool frozen: false
    /// The doors alone are held while a write replays behind the screen (`RepoPage.doorsHeldWhy`): a switch (the row's
    /// double-click), the name box, the REMOTES `+`, and the menus' write rows. The pane keeps its ink — everything
    /// else goes on working, so `textMuted` over the rows would lie (規約 §無効); what cannot be pressed greys itself.
    property bool doorsHeld: false
    /// The page whose command log the foot row opens (`CommandsToggle`); null while no tab is open. The row goes while
    /// the log is up, whose header band takes its place (`CommandsPane`).
    property var commandsPage: null

    signal refActivated(string oidHex)
    /// Right-click on a row. `kind` is the section it came from, `name` what the row shows and `full` what git knows
    /// it by.
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    signal worktreeActivated(string path)
    /// Double-click on a branch row: `kind` is the word the page's dispatcher reads (`branch` / `remote`, the
    /// same word a chip goes out under).
    signal refSwitchRequested(string kind, string name)
    /// A name was typed for a new branch on the row's commit.
    signal branchAtRequested(string oidHex, string name)
    /// The same for a new tag.
    signal tagAtRequested(string oidHex, string name)
    /// A row was renamed. `kind` is the section ("branch" / "tag" / "stash"), `id` what git knows the row by.
    signal renameSubmitted(string kind, string id, string name)
    signal foldRequested(bool collapse)
    /// The REMOTES `+`, past the doors' hold (`askAddRemote`).
    signal addRemoteRequested()
    /// Right-click on the row a remote itself stands on: what to do with that remote.
    signal remoteMenuRequested(string name)

    // ---- the row gestures ------------------------------------------
    // The pane keeps the names its callers (the page's row menu, the smoke hooks) reach for.
    SidebarRowGestures {
        id: rowGestures
        host: sidebar
        // Only the doors' hold: the plan's freeze already takes the list out of the input path.
        held: sidebar.doorsHeld
        repoTab: sidebar.repoTab
        remotesModel: sidebar.remotesModel
    }
    /// Where this panel last saw the hand, in its own coordinates.
    property point handAt: Qt.point(-1, -1)
    // The hand, heard once on the panel — an ancestor of every row, since a sibling over them takes their hover. The
    // rows re-read their hover on its moves only, so a list moving under a still pointer cannot light a row nobody
    // aimed at (`NavItemDelegate.syncHover`; rules-refs/app-ui.md, the 左メニューの行の重ね色 line). Walking out of
    // the panel counts as a move, or the row it left would keep the light.
    HoverHandler {
        id: handWatch
        // A move is a new place, not a new telling: the point is handed again when the layout moves under a still
        // pointer, and counting that closes an opening row in a blink (`tests/qml/tst_hoverunderstillhand.qml`).
        onPointChanged: {
            if (handWatch.point.position.x === sidebar.handAt.x
                    && handWatch.point.position.y === sidebar.handAt.y)
                return
            sidebar.handAt = handWatch.point.position
            rowGestures.handStirred()
        }
        onHoveredChanged: {
            sidebar.handAt = Qt.point(-1, -1)
            rowGestures.handStirred()
        }
    }
    property alias activeKey: rowGestures.activeKey
    property alias editKey: rowGestures.editKey
    /// Automation only: whether the box refuses what is typed, and its line — a frame colour and a tooltip.
    readonly property bool editRefused: rowGestures.editRefused
    readonly property string editRefusedWhy: rowGestures.editRefusedWhy
    /// A menu raised from one of the folded list's rows stands over it; the page sets it, the menus being its.
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
    /// git's answer to a rename this pane's box sent (`SidebarRowGestures.submitEdit`): it landed, or git would not
    /// have the name and says so under the box the name is still in.
    function noteRenameLanded() {
        rowGestures.renameLanded()
    }
    function noteRenameRefused(why) {
        rowGestures.renameRefused(why)
    }
    function beginBranchAt(kind, id, oidHex) {
        rowGestures.beginBranchAt(kind, id, oidHex)
    }
    function beginTagAt(kind, id, oidHex) {
        rowGestures.beginTagAt(kind, id, oidHex)
    }
    function activateRow(kind, name, full, oidHex) {
        rowGestures.activateRow(kind, name, full, oidHex)
    }
    /// The REMOTES `+`, wherever it stands, held in one place: the bands grey their own `+` (規約 §無効), and this
    /// makes the refusal true whatever else reaches the signal.
    function askAddRemote() {
        if (!sidebar.doorsHeld)
            sidebar.addRemoteRequested()
    }

    /// Automation-only exposures, like `GraphPane.view` (rules-refs/app-ui.md); `auto/NavProbe.qml` composes its runs
    /// from these.
    readonly property alias autoRail: rail
    readonly property alias autoSections: sections
    readonly property alias autoPeek: peek

    /// The pointer is on the current branch's sticky stand-in (`HeadPinRow`). The pane's own, because its bindings
    /// read it: a headless run writes the one property a real hover writes.
    property bool headPinPointed: false

    // The width the list goes back to, read off the pane as it folds (規約 §レイアウト初期値 is only where it starts).
    property real openWidth: 260
    readonly property int minOpenWidth: 180
    SplitView.preferredWidth: sidebar.openWidth
    SplitView.minimumWidth: sidebar.minOpenWidth
    color: Theme.bgSurface

    // Assigned, not bound: a drag writes the same attached `SplitView` properties, and a binding would be gone after
    // the first one.
    onCollapsedChanged: sidebar.applyFold()
    function applyFold() {
        // Whichever way it goes, the rail's open section goes with the rail.
        sidebar.closePeek()
        // And the box with the list it stood in, or it comes back up on a row nobody clicked.
        // `SidebarRowGestures.startEdit` never moves the fold, so this never closes a box just opened.
        sidebar.stopEdit()
        if (sidebar.collapsed) {
            // A page restored already folded has not been laid out and reports 0, which would lose the width the
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

    // Section expansion (the filter reveals folded sections). What a section shows is its band's answer
    // (`NavHeader.showsRows`): one with no rows leaves this alone, so the next repository with rows there opens the
    // way the reader left it.
    property bool expBranches: true
    property bool expRemotes: true
    property bool expWorktree: true
    property bool expStashes: true
    property bool expTags: true

    /// Beside the flags, so the pane stays the one writer of its fold state (`NavSections.sectionToggled`).
    function toggleSection(kind) {
        if (kind === "branch")
            sidebar.expBranches = !sidebar.expBranches
        else if (kind === "remote")
            sidebar.expRemotes = !sidebar.expRemotes
        else if (kind === "worktree")
            sidebar.expWorktree = !sidebar.expWorktree
        else if (kind === "stash")
            sidebar.expStashes = !sidebar.expStashes
        else if (kind === "tag")
            sidebar.expTags = !sidebar.expTags
    }

    // ---- the log's seat ----------------------------------------------
    // The foot of the pane (デザイン規約 §git が言ったことを読む場所).
    CommandsToggle {
        id: commandsSeat
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        // Folded, the fold control's block at the rail's other end; open, one more of the sections' bands.
        height: sidebar.collapsed ? Theme.headerHeight : Theme.rowHeight
        captioned: !sidebar.collapsed
        markSize: sidebar.collapsed ? Theme.iconLg : Theme.iconMd
        visible: sidebar.commandsPage !== null && !commandsSeat.open
        curPage: sidebar.commandsPage
    }
    /// Automation: the colour the `>_` painted while the panel is down (`PGG_AUTO_ACT=commands-clear`).
    readonly property alias commandsMarkColor: commandsSeat.markColor
    readonly property real footRoom: commandsSeat.visible ? commandsSeat.height : 0
    /// How short the pane may be with the whole menu on it (デザイン規約 §窓の床): folded, the rail's every cell; open,
    /// the list's bands (its rows scroll). The foot row counts while it stands — the log up takes it away.
    readonly property real floorHeight:
        (sidebar.collapsed ? rail.wholeHeight : sections.bandsHeight) + sidebar.footRoom

    /// Which row has its facts open, in the sections below or the rail's section (`SectionPeekPopup`): one row for the
    /// pane, since only one pointer is ever resting (`SidebarRowGestures.openKey`).
    property alias openKey: rowGestures.openKey
    /// Automation: whether a row is open, and what it says — read off the row itself, so a run cannot go green with
    /// the wiring cut (PGG_AUTO_ACT=nav-open).
    readonly property bool rowFactsOpen: sidebar.openKey !== ""
    function rowFactsWords() {
        const said = sections.openWords()
        return said !== "" ? said : peek.openWords()
    }
    /// Where the open row and its lines landed, so a run reads the list having made room rather than the layout's
    /// word for it (PGG_AUTO_ACT=nav-open).
    function rowFactsGeom() {
        return sections.openGeom()
    }
    /// Whether the open row shows whole, its lines included (PGG_AUTO_ACT=nav-open-foot).
    function rowFactsShown() {
        return sections.openFactsItem() !== null ? sections.openShown() : peek.openShown()
    }
    /// For the runs that read what the hand did next (PGG_AUTO_ACT=nav-open-then).
    function openFactsItem() {
        const lines = sections.openFactsItem()
        return lines !== null ? lines : peek.openFactsItem()
    }

    // ---- the open list ----------------------------------------------
    NavSections {
        id: sections
        anchors.fill: parent
        anchors.bottomMargin: sidebar.footRoom
        visible: !sidebar.collapsed
        enabled: !sidebar.frozen
        opacity: sidebar.frozen ? Metrics.dimFade : 1
        addHeld: sidebar.doorsHeld
        repoTab: sidebar.repoTab
        branchesModel: sidebar.branchesModel
        remotesModel: sidebar.remotesModel
        worktreesModel: sidebar.worktreesModel
        stashesModel: sidebar.stashesModel
        tagsModel: sidebar.tagsModel
        gestures: rowGestures
        expBranches: sidebar.expBranches
        expRemotes: sidebar.expRemotes
        expWorktree: sidebar.expWorktree
        expStashes: sidebar.expStashes
        expTags: sidebar.expTags
        headPinPointed: sidebar.headPinPointed
        onSectionToggled: kind => sidebar.toggleSection(kind)
        onFoldRequested: collapse => sidebar.foldRequested(collapse)
        onRefActivated: oidHex => sidebar.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sidebar.remoteMenuRequested(name)
        onAddRemoteRequested: sidebar.askAddRemote()
    }

    // ---- folded ------------------------------------------------------
    NavRail {
        id: rail
        anchors.fill: parent
        anchors.bottomMargin: sidebar.footRoom
        visible: sidebar.collapsed
        enabled: !sidebar.frozen
        opacity: sidebar.frozen ? Metrics.dimFade : 1
        addHeld: sidebar.doorsHeld
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
        onAddRemoteRequested: sidebar.askAddRemote()
    }

    // ---- the folded list's one open section --------------------------
    // The pane keeps the names the rail, the fold and the smoke hooks call it by.
    property alias peekKind: peek.kind
    property alias peekTop: peek.top
    property alias peekEntered: peek.contentPointed
    /// For the smoke hooks alone (like `GraphPane.view`): PGG_AUTO_ACT=nav-reclick clicks a row in here.
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
        addHeld: sidebar.doorsHeld
        repoTab: sidebar.repoTab
        workTree: sidebar.workTree
        rail: rail
        gestures: rowGestures
        paneW: sidebar.width
        paneH: sidebar.height
        listW: sidebar.openWidth
        // A box open on one of its rows holds it as a menu does: a name half typed into a list the pointer walked away
        // from is a name lost.
        pinned: sidebar.menuOpen || sidebar.editKey !== ""
        onRefActivated: oidHex => sidebar.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sidebar.remoteMenuRequested(name)
        onAddRemoteRequested: sidebar.askAddRemote()
    }
}

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
    /// Every door on this pane is held down, and dimmed the way disabled
    /// things are; reading it stays free (規約 §無効).
    ///
    /// **This is the plan's freeze, and it is meant to be read as one.**
    /// While a rebase is being composed the only way into a write is the
    /// run button, and the pane is out for as long as the mode lasts — so
    /// it says so with the disabled step over the whole of itself
    /// (デザイン規約 §フル interactive rebase). A write that merely
    /// replays behind the screen is the other thing entirely, and holds
    /// the doors one at a time
    /// (`doorsHeld`).
    ///
    /// **The `>_` band at the foot stands free.** It is the one
    /// place that answers "what is git doing", which is exactly the
    /// question a reader has while a replay they cannot interrupt is
    /// running — so the hold is put on the list and the rail, and the
    /// band goes on standing at full weight. Dimming
    /// the pane whole would also have to be undone here: a band that is
    /// pressable while painted like a disabled one is a lie about itself.
    property bool frozen: false
    /// The doors alone are held: a write that replays a range a commit at
    /// a time is running behind the screen, and what the reader has to do
    /// is wait for it (`RepoPage.doorsHeldWhy`). Held are the ways a click
    /// here moves the history out from under it — a switch (the row's
    /// double-click), the name box a second click or a menu row opens,
    /// the `+` that writes a remote down, and every write row of the menus
    /// the rows raise.
    ///
    /// **The pane keeps its ink and its rows.** The lock comes
    /// off the moment git answers, so the pane has to be the same pane on
    /// both sides of it; and everything it is read with — choosing a row
    /// and jumping to it, scrolling, the filter, folding a section, hover
    /// — goes on working, which is what makes `textMuted` over the rows a
    /// lie (規約 §無効). A delete takes its row away without stopping the
    /// pane around it for exactly this reason (デザイン規約 §消す操作は先に
    /// 画面から消す). The two things that really cannot be pressed say so
    /// themselves: the `+` greys, and a menu row greys and gives its line
    /// on hover.
    property bool doorsHeld: false
    /// The page whose command log the row at the foot of this pane opens
    /// (`CommandsToggle`). Null while no tab is open — and the whole row
    /// goes once the log is up, since from then on it is the log's own
    /// header band (`CommandsPane`).
    property var commandsPage: null

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
    /// And the same for a tag on the commit whatever row was met stands on.
    signal tagAtRequested(string oidHex, string name)
    /// A row was renamed. `kind` is the section ("branch" / "tag" /
    /// "stash"), `id` what git knows the row by.
    signal renameSubmitted(string kind, string id, string name)
    signal foldRequested(bool collapse)
    /// The `+` on the REMOTES band was pressed: a remote is to be written
    /// down. Raised from the open list and from the section the folded
    /// rail opens alike — one band, wherever it is standing.
    signal addRemoteRequested()
    /// Right-click on the row a remote itself stands on: what to do with that remote.
    signal remoteMenuRequested(string name)

    // ---- the row gestures ------------------------------------------
    // Held beside the lists (`SidebarRowGestures`).
    // The pane keeps the names its own callers already reach for: the
    // page opens the box from the row menu, and the smoke hooks read
    // which row has one.
    SidebarRowGestures {
        id: rowGestures
        host: sidebar
        // Only the doors' hold reaches here: the plan's freeze takes the whole list out of the input path, so there
        // is no gesture left for these to refuse.
        held: sidebar.doorsHeld
        repoTab: sidebar.repoTab
        remotesModel: sidebar.remotesModel
    }
    property alias activeKey: rowGestures.activeKey
    property alias editKey: rowGestures.editKey
    /// Whether what is typed in that box cannot be taken, and the line the box says so with — automation-only
    /// exposures (app-ui.md), since the frame is a colour and the line is in a tooltip.
    readonly property bool editRefused: rowGestures.editRefused
    readonly property string editRefusedWhy: rowGestures.editRefusedWhy
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
    /// The `+` at the end of the REMOTES band, wherever it is standing — the open list's, the folded rail's cell, the
    /// section the rail opens beside itself. **One door, held in one place**: the bands grey their own `+` so the hand
    /// reads the refusal before it presses (規約 §無効), and this is what makes the refusal true whatever else reaches
    /// the signal — a smoke hook, or a fourth band added later.
    function askAddRemote() {
        if (!sidebar.doorsHeld)
            sidebar.addRemoteRequested()
    }

    /// The three the pane is made of, handed over whole. **Automation-only exposures**, the same one
    /// `GraphPane.view` is (app-ui.md): what a headless run does to the sidebar — typing into the filter, resting on
    /// a cell, closing a band, reading a row's tooltip back — is composed from these by `auto/NavProbe.qml`, and a
    /// pane that mirrored each of those would be twenty forwards that mean nothing to anyone reading it.
    readonly property alias autoRail: rail
    readonly property alias autoSections: sections
    readonly property alias autoPeek: peek

    /// The pointer is on the current branch's sticky stand-in, which rides the edge its own row went out of
    /// (`HeadPinRow`). The pane's own, because its bindings read it: hover is the input that cannot be injected, so a
    /// headless run writes the one property a real hover writes.
    property bool headPinPointed: false

    // The width the list goes back to. Read off the pane as it folds,
    // so one that has been widened comes back the
    // width it was left (規約 §レイアウト初期値 is only where it starts).
    property real openWidth: 260
    /// The narrowest width a drag can leave.
    readonly property int minOpenWidth: 180
    SplitView.preferredWidth: sidebar.openWidth
    SplitView.minimumWidth: sidebar.minOpenWidth
    color: Theme.bgSurface

    // Folding is a size, and a size is the splitter's business: pinning
    // both ends to the rail's width is what takes the drag away while it
    // is folded. Assigned — a drag writes the same
    // attached property, and a binding here would be gone after the first
    // one (leaving the fold with nothing to set).
    onCollapsedChanged: sidebar.applyFold()
    function applyFold() {
        // Whichever way it goes, the one section the rail had open goes
        // with the rail — including when what put the list back was a
        // row in that very section (startEdit).
        sidebar.closePeek()
        // And the box goes with the list it stood in: folded, one left
        // open comes back up under the pointer on a row nobody clicked.
        // `startEdit` puts the list back before opening its own box, so
        // that one is never this one.
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

    /// What a click on one of the column's header bands flips (`NavSections.sectionToggled`). Held beside the flags,
    /// so the pane stays the one writer of its own fold state.
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
    // The foot of the pane: one of the sections' own bands while the list
    // is open, the fold control's own block at the other end of the rail
    // while it is folded, and gone once the panel is up — from then on
    // this row is the log's own header band, run the width of the window
    // (デザイン規約 §git が言ったことを読む場所).
    CommandsToggle {
        id: commandsSeat
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        // Folded, it is the fold control's own block at the other end of the rail — same box, same step of mark.
        // Open, it is one more of the sections' bands.
        height: sidebar.collapsed ? Theme.headerHeight : Theme.rowHeight
        captioned: !sidebar.collapsed
        markSize: sidebar.collapsed ? Theme.iconLg : Theme.iconMd
        visible: sidebar.commandsPage !== null && !commandsSeat.open
        curPage: sidebar.commandsPage
    }
    /// Automation: the colour the `>_` painted while the panel is down
    /// (`PGG_AUTO_ACT=commands-clear`; the page picks the standing seat).
    readonly property alias commandsMarkColor: commandsSeat.markColor
    /// What the seat leaves for the list above it.
    readonly property real footRoom: commandsSeat.visible ? commandsSeat.height : 0

    /// Which row of the left panel has its facts open under it, wherever that row is standing: in the sections below,
    /// or in the one section the folded rail has open (`SectionPeekPopup`). One row for the pane — only one pointer
    /// is ever resting, and the row that holds the answer may be in either list (`SidebarRowGestures.openKey`).
    property alias openKey: rowGestures.openKey
    /// What the open row is saying, for a headless run to read: whether one is open at all, and the answers under it.
    /// Read off the row itself rather than off whatever asked for it, so a run cannot go green with the wiring cut
    /// (PGG_AUTO_ACT=nav-open).
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
    /// Whether the open row is showing whole, its lines included — the rule a row opening at the foot of a section
    /// that scrolls is judged on (PGG_AUTO_ACT=nav-open-foot).
    function rowFactsShown() {
        return sections.openFactsItem() !== null ? sections.openShown() : peek.openShown()
    }
    /// The lines that are open, for the runs that read what the hand did next (PGG_AUTO_ACT=nav-open-then) — they
    /// are where a sweep takes words from, and where a press that never moved goes.
    function openFactsItem() {
        const lines = sections.openFactsItem()
        return lines !== null ? lines : peek.openFactsItem()
    }

    // ---- the open list ----------------------------------------------
    // The filter band, the five sections and the ground under them, as
    // one column (`NavSections`). The column reports upward; the pane
    // answers with the state it owns.
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
    // The section itself, and the bookkeeping that says when it is open,
    // live in SectionPeekPopup. The pane keeps the names the rail, the
    // fold and the smoke hooks already call it by.
    property alias peekKind: peek.kind
    property alias peekTop: peek.top
    property alias peekEntered: peek.contentPointed
    /// The open section itself, for the smoke hooks alone (the shape
    /// `GraphPane.view` already has): clicks cannot be injected, so
    /// PGG_AUTO_ACT=nav-reclick puts one in at a row in here.
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
        // A box open on one of its rows holds it as firmly as a menu
        // does: it has taken the keyboard, and a name half typed into a
        // list the pointer walked away from is a name lost.
        pinned: sidebar.menuOpen || sidebar.editKey !== ""
        onRefActivated: oidHex => sidebar.refActivated(oidHex)
        onRefMenuRequested: (kind, name, full, oidHex) => sidebar.refMenuRequested(kind, name, full, oidHex)
        onRemoteMenuRequested: name => sidebar.remoteMenuRequested(name)
        onAddRemoteRequested: sidebar.askAddRemote()
    }
}

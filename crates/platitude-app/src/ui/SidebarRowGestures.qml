import QtQuick
import platitude
import platitude.ui

// The sidebar's row gestures: which row was clicked last, and which one has a name box open in it. Held beside the
// lists: only one row at a time is either, in whichever section, and both outlive the delegates that show them
// (デザイン規約 §左メニューの所作).
QtObject {
    id: gestures

    /// The pane these gestures belong to. Where a gesture leads is raised as the pane's signal — the page listens to
    /// the pane, and a second emitter would be a second answer.
    required property Item host
    required property RepoTab repoTab
    required property NavSectionModel remotesModel
    /// Asked by the new worktree's box: whether its name is a branch already, and what is where the worktree would go
    /// (`Words.worktreeNameRefused`).
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel

    /// One of the page's menus is standing, raised on these rows or anywhere else (`SidebarPane.menuRaisedOn`).
    property bool menuOpen: false

    /// The pane's write doors are held (`SidebarPane.doorsHeld`). Only the two gestures that write, or open a box to
    /// write with, answer for it; a click, a fold and a scroll go on working
    /// (デザイン規約 §消す操作は先に画面から消す「消すのは行だけ、周りは生きたまま」).
    property bool held: false

    /// The sidebar's one `ReclickGesture` (rules-refs/app-ui.md「間を空けた 2 回目のクリック」). What the wait was
    /// aimed at is read at the click (`p`): by the time it runs out the row may be showing another name.
    property ReclickGesture reclick: ReclickGesture {
        onRenameAsked: (key, p) => gestures.startEdit(p.kind, key, "rename", p.id, p.oid,
                                                      gestures.typedName(p.kind, p.id, p.name))
    }
    property alias activeKey: gestures.reclick.activeKey
    /// The name a row is typed and renamed by: a stash's message (git knows it by its selector), a remote branch
    /// without the remote it lives on, and the id for everything else.
    function typedName(kind, id, name) {
        return kind === "stash" ? name
             : kind === "remote" ? gestures.remoteBranchHalf(id) : id
    }
    property string editKey: ""
    property string editKind: ""
    property string editMode: ""
    property string editId: ""
    property string editOid: ""
    property string editText: ""
    /// The remote a remote row lives on (`origin`), empty for every other kind. Cut by the configured names, since a
    /// remote's own name may contain `/`; an unconfigured one cuts at the first slash (`GitFacts.remoteOfRef`).
    readonly property string editRemote: gestures.editKind !== "remote" ? ""
        : GitFacts.remoteOfRef(gestures.editId, gestures.repoTab.remoteNames)
    function remoteBranchHalf(id) {
        return GitFacts.branchOfRef(id, gestures.repoTab.remoteNames)
    }
    /// A rename to a name the remote already carries. Refused here: a plain push to an existing name fast-forwards
    /// it and reports success, moving somebody else's branch. Rename only — the new-branch box on a remote row makes
    /// a local branch.
    readonly property bool editTaken: gestures.editMode === "rename" && gestures.editRemote !== ""
        && gestures.editText.trim() !== ""
        && gestures.remotesModel.oidOfName(gestures.editRemote + "/" + gestures.editText.trim()) !== ""
    /// A new branch / tag / worktree box still empty since it opened. Not refused: the frame answers for what was typed
    /// (デザイン規約 §可否・警告の出し場所), and nothing has been. A rename rubbed out to nothing is refused.
    readonly property bool editUnanswered:
        (gestures.editMode === "branch" || gestures.editMode === "tag" || gestures.editMode === "worktree")
        && gestures.editText.trim() === ""
    /// Where the worktree named in the box would go (`NavSectionModel.newWorktreeFor`), asked per keystroke; undefined
    /// for every other box.
    readonly property var editWorktreePlace: gestures.editMode !== "worktree" ? undefined
        : gestures.worktreesModel.newWorktreeFor(gestures.editText.trim())
    /// What stands in a new worktree's way, said before the press — the graph's box says the same
    /// (`Words.worktreeNameRefused`).
    readonly property string editWorktreeWhy: gestures.editMode !== "worktree" ? ""
        : Words.worktreeNameRefused(gestures.editText,
                                gestures.branchesModel.oidOfName(gestures.editText.trim()) !== "",
                                gestures.editWorktreePlace)
    /// What is typed cannot be accepted. The rules are git's own, asked of core (a stash's label is free text).
    readonly property bool editRefused: gestures.editKey !== "" && !gestures.editUnanswered
        && (gestures.editTaken
            || gestures.editCaseOnly
            || gestures.editWorktreeWhy !== ""
            || gestures.editGitRefusal !== ""
            || !(gestures.editKind === "stash"
                 ? GitFacts.validStashMessage(gestures.editText)
                 : GitFacts.validRefName(gestures.editText)))
    /// A tag rename that changes only the letters' case. git writes a ref as a file, so on a case-insensitive disk the
    /// new name lands on the old one's and both are gone; core refuses it (`tag::rename`), and asking here puts the
    /// answer in the box (デザイン規約 §答えの要らない報せ). `tag` is the row's kind — mode `tag` is the new-tag box.
    readonly property bool editCaseOnly:
        gestures.editKind === "tag" && gestures.editMode === "rename"
        && gestures.editText.trim() !== gestures.editId
        && gestures.editText.trim().toLowerCase() === gestures.editId.toLowerCase()
    readonly property string editRefusedWhy: !gestures.editRefused ? ""
        // git's own words win: it answered about this very name; the rest were worked out before asking.
        : gestures.editGitRefusal !== ""
          ? gestures.editGitRefusal
        : gestures.editText.trim() === ""
          ? qsTr("A name is needed")
          : gestures.editWorktreeWhy !== ""
            ? gestures.editWorktreeWhy
          : gestures.editCaseOnly
            ? qsTr("Only the letter case differs — on this disk that deletes both names")
          : gestures.editTaken
            ? qsTr("%1 already has a branch called that").arg(gestures.editRemote)
            : gestures.editKind === "stash"
              ? qsTr("One line, and nothing invisible in it")
              : Words.notAName
    /// The worktree's folder in that line, which the box's tip stands the tree mark in front of
    /// (`Words.worktreeNameMark`); empty where the line names none.
    readonly property string editRefusedMark: Words.worktreeNameMark(gestures.editRefusedWhy,
                                                                     gestures.editWorktreePlace)

    function startEdit(kind, key, mode, id, oid, text) {
        // The one door into the box (the second click and the menu's four `begin*`), so the hold is asked here: a box
        // opened while the doors are held would take a name nothing can be done with.
        if (gestures.held)
            return
        // The box opens where the row is; folded, that is the peek beside the rail, and the list stays folded
        // (デザイン規約 §左メニューを畳む). The box holds the peek open the way a menu does (`pinned`).
        gestures.editKind = kind
        gestures.editMode = mode
        gestures.editId = id
        gestures.editOid = oid
        gestures.editText = text
        gestures.editKey = key
    }
    function stopEdit() {
        // A box coming down spends the gesture that opened it (as `GraphPane.stopNaming` does), or the next click on
        // the same row reopens it a window later — a blink.
        if (gestures.editKey !== "")
            gestures.forgetClicks()
        gestures.editKey = ""
        gestures.editText = ""
        gestures.editMode = ""
        gestures.editWaiting = false
        gestures.editGitRefusal = ""
    }
    onEditTextChanged: gestures.editGitRefusal = ""
    function submitEdit(text) {
        const kind = gestures.editKind
        const id = gestures.editId
        const oid = gestures.editOid
        const mode = gestures.editMode
        // Enter on a box nobody has typed in leaves it standing, as on a refused one (`editUnanswered`).
        if ((mode === "branch" || mode === "tag" || mode === "worktree") && text.trim() === "")
            return
        const was = kind === "remote" ? gestures.remoteBranchHalf(id) : id
        if (mode === "branch") {
            gestures.stopEdit()
            gestures.host.branchAtRequested(oid, text.trim())
            return
        }
        if (mode === "worktree") {
            gestures.stopEdit()
            gestures.host.worktreeAtRequested(oid, text.trim())
            return
        }
        if (mode === "tag") {
            gestures.stopEdit()
            gestures.host.tagAtRequested(oid, text.trim())
            return
        }
        // The name it opened with, unchanged: not a rename, just the way out.
        if (text.trim() === was) {
            gestures.stopEdit()
            return
        }
        // A rename keeps its box until git answers (デザイン規約 §答えの要らない報せ): git turns down names nothing here
        // could know about (one already taken), and a box closed first loses what was typed and leaves the answer
        // nowhere but the log. The page takes it down on the landing (`RepoPage.absorbWriteResult`).
        gestures.editWaiting = true
        gestures.host.renameSubmitted(kind, id, text.trim())
    }
    /// A rename is out and git has not answered; the answer takes the box down or writes its refusal under it
    /// (`editGitRefusal`).
    property bool editWaiting: false
    /// git's refusal of the name in the box, cleared by the next keystroke — it is about the name git was asked about.
    property string editGitRefusal: ""
    function renameLanded() {
        if (gestures.editWaiting)
            gestures.stopEdit()
    }
    function renameRefused(why) {
        if (!gestures.editWaiting)
            return
        gestures.editWaiting = false
        gestures.editGitRefusal = why
    }
    /// Which BRANCHES row has its facts open under it on hover (`NavRowFacts`), by its git name. Held here like the
    /// gestures above: delegates are recycled, only one row is ever open, and it may be in the sections, the sticky
    /// stand-in or the folded rail's peek.
    property string openKey: ""
    /// The last row to open and where the pointer stood when it did, in window coordinates. A row opens where the hand
    /// travelled to it, not where the list moved under a still hand: an open row is taller, so the rows below move,
    /// and opening whatever lands under a still pointer would have rows trading places (デザイン規約 §左メニューの所作).
    property string openedKey: ""
    property point openedAt: Qt.point(-1, -1)
    function openFacts(key, at) {
        // Nothing opens under a name box, a menu or a held scroll bar (`barWatch`).
        if (gestures.editKey !== "" || gestures.menuOpen || Hand.heldBar !== null)
            return
        // Only a different row arriving under a still hand is refused; the same row coming back is pointed at again.
        if (key !== gestures.openedKey && gestures.settled(at))
            return
        gestures.openKey = key
        gestures.openedKey = key
        gestures.openedAt = at
    }
    function closeFacts(key) {
        if (gestures.openKey === key)
            gestures.openKey = ""
    }
    /// Less than half a row from where the last row opened is the list having moved, not the hand.
    function settled(at) {
        return Math.abs(at.x - gestures.openedAt.x) < Theme.rowHeight / 2
            && Math.abs(at.y - gestures.openedAt.y) < Theme.rowHeight / 2
    }

    /// How often the hand itself has moved, counted by `SidebarPane` and `SectionPeekPopup` against the point last
    /// seen, not per event: a row growing and a list scrolling also reach those handlers as fresh points
    /// (`tests/qml/tst_hoverunderstillhand.qml`). The rows re-read their hover only on these
    /// (`NavItemDelegate.syncHover`) — Qt gives hover to whatever arrives under a still pointer, and opening a row
    /// moves the rows around it.
    property int handMoves: 0
    /// Where the hand was last heard over the rows, in the window's coordinates: what a row the reader scrolled weighs
    /// itself against (`NavItemDelegate.syncUnder`) — the hand has not moved, so this still holds. **A leave does not
    /// clear it** (`at` (-1, -1)): a hand crossing from the panel into the rail's peek is heard entering the one before
    /// leaving the other (`QQuickDeliveryAgentPrivate::deliverHoverEvent` sends the leaves last), and a row the hand
    /// has left is not pointed, so a stale place lights and opens nothing.
    property point handAt: Qt.point(-1, -1)
    function handStirred(at) {
        if (at.x >= 0)
            gestures.handAt = at
        gestures.handMoves++
    }

    /// A menu raised on the open row — its own line or its lines — which keeps it open: it is about that row. Every
    /// other menu closes it, or it would move rows under the hand reading the menu.
    property bool menuFromFacts: false
    function noteMenuFromFacts() {
        gestures.menuFromFacts = true
    }
    /// The menu so noted did not open (`NavItemDelegate.rowPressed`): nothing will close to clear the note.
    function dropMenuFromFacts() {
        gestures.menuFromFacts = false
    }
    /// Nothing opens over a name box or under a menu: opening moves the rows under both (デザイン規約 §左メニューの所作).
    onEditKeyChanged: if (gestures.editKey !== "") gestures.openKey = ""
    onMenuOpenChanged: {
        if (gestures.menuOpen && !gestures.menuFromFacts)
            gestures.openKey = ""
        if (!gestures.menuOpen)
            gestures.menuFromFacts = false
    }
    /// Nor while a scroll bar is held (`Hand.heldBar`), and taking one closes the open row
    /// (規約 §hover のツールチップ「スクロールバーを掴んだら、hover で開いたものは閉じる」). The rows re-read their hover
    /// only when the hand is heard to move, and nothing is heard while a bar is held: left to the row, the close would
    /// come after the drag — under another row's name once the delegate is reused, and with what the opening scrolled
    /// never given back (the drag moved the list: `NavList.openHeld`). At the press, the list gives back what the
    /// opening took before anything is dragged.
    property Connections barWatch: Connections {
        target: Hand
        function onHeldBarChanged() {
            if (Hand.heldBar !== null)
                gestures.openKey = ""
        }
    }

    /// A click landed on `key`: a box open on any other row is dropped without asking, typing and all. Which row was
    /// clicked is the gesture's own to remember (`reclick`).
    function noteClick(key) {
        if (gestures.editKey !== "" && gestures.editKey !== key)
            gestures.stopEdit()
    }
    /// A line of an open row's facts was pressed (`NavRowFacts`): the graph goes to the commit it names and the panel
    /// stays (デザイン規約 §左メニューの所作「行き先はグラフ」). The row `key` (`NavList.keyOf`) becomes the one last
    /// clicked but arms nothing — a line was pressed, not the row's name.
    function followLine(key, oidHex) {
        gestures.noteClick(key)
        gestures.reclick.land(key)
        gestures.host.refActivated(oidHex)
    }
    /// The rows this gesture was made on have gone (the folded rail's peek closed): the memory and the wait go with
    /// them, or the next visit's first click comes up as a second one
    /// (rules-refs/app-ui.md「それを載せている一覧と共に消える」).
    function forgetClicks() {
        gestures.reclick.forget()
    }
    /// Double-click: where the row leads (デザイン規約 §左メニューの所作).
    function activateRow(kind, name, full, oidHex) {
        // Held: a switch and the tag row's new-branch box, which write. A worktree row goes through — moving to another
        // worktree writes nothing, and the running write outlives the page that asked for it
        // (`RepoSession::close`, デザイン規約 §左メニューの所作 の replay の段).
        if (gestures.held && kind !== "worktree")
            return
        const id = full !== "" ? full : name
        if (kind === "branch" || kind === "remote")
            gestures.host.refSwitchRequested(kind, id)
        else if (kind === "worktree")
            gestures.host.worktreeActivated(full)
        else if (kind === "tag")
            // A tag is a mark: the row offers the branch that would make it somewhere to carry on from.
            gestures.startEdit(kind, "tag:" + id, "branch", id, oidHex, "")
        // A stash is a shelf, and a folder a heading: both stay put.
    }
    /// The menu's way into the rename box, for anyone who cannot use the two-click gesture.
    function beginRename(kind, id, text) {
        gestures.startEdit(kind, kind + ":" + id, "rename", id, "", text)
    }
    /// The new-branch box on any row that names a commit, not just a tag's.
    function beginBranchAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "branch", id, oidHex, "")
    }
    /// The same box for a new tag. One field, four modes, told apart by the placeholder (`NavNameBox`).
    function beginTagAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "tag", id, oidHex, "")
    }
    /// …and for a new branch out in a worktree of its own (`Create worktree here…`).
    function beginWorktreeAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "worktree", id, oidHex, "")
    }
}

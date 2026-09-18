import QtQuick
import platitude
import platitude.ui

// The sidebar's row gestures: which row was clicked last, and which one has a name box open in it. Held beside the
// lists — only one row at a time is either, whichever section it sits in, and both have to
// outlive the delegates that show them (デザイン規約 §左メニューの所作).
QtObject {
    id: gestures

    /// The pane these gestures belong to. Where a gesture leads is the pane's to raise: it is the pane the page
    /// listens to, and a second emitter would be a second answer.
    required property Item host
    required property RepoTab repoTab
    required property NavSectionModel remotesModel

    /// A menu raised from one of the folded list's rows is standing over it. The page's to set — the menus are its.
    property bool menuOpen: false

    /// The pane's write doors are held (`SidebarPane.doorsHeld`). Only the two gestures below answer for it — the ones
    /// that move the history or open a box to move it with. A click, a fold and a scroll are how this list is read,
    /// and they go on working (デザイン規約 §消す操作は先に画面から消す: 消すのは行だけで、周りは止めない).
    property bool held: false

    /// The two clicks a row answers with one gesture — one for the sidebar (`ReclickGesture`): a
    /// delegate is recycled the moment its row scrolls off, and both the memory and the wait have to outlive it.
    /// What the wait was aimed at is read at the click (`p`), because by the time it runs out the row may be showing
    /// another name.
    property ReclickGesture reclick: ReclickGesture {
        onRenameAsked: (key, p) => gestures.startEdit(p.kind, key, "rename", p.id, p.oid,
                                                      gestures.typedName(p.kind, p.id, p.name))
    }
    property alias activeKey: gestures.reclick.activeKey
    /// The name this row is typed and renamed by. A stash is named by its message and known to git by its selector;
    /// everything else answers to the name it shows. A remote branch is typed without the remote it is on —
    /// `origin/` is where the branch lives.
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
    /// The remote a row being renamed lives on (`origin`), empty for every other kind of row. The configured names
    /// say where the cut is — a remote's own name may contain `/`; an unconfigured one cuts at the first slash, so
    /// the duplicate-name refusal below keeps its remote to ask about (`GitFacts.remoteOfRef`).
    readonly property string editRemote: gestures.editKind !== "remote" ? ""
        : GitFacts.remoteOfRef(gestures.editId, gestures.repoTab.remoteNames)
    /// The name a remote row is typed and renamed by — the branch half, without the remote it lives on.
    function remoteBranchHalf(id) {
        return GitFacts.branchOfRef(id, gestures.repoTab.remoteNames)
    }
    /// A name the remote already carries. Refused here: a plain push to a name that exists
    /// fast-forwards it and reports success, so somebody else's branch would move instead of this one being renamed.
    /// Only ever a rename's rule: the box for a new branch's name opens on a remote row too, and what it makes is a
    /// local branch — a name the remote happens to carry is no answer to that.
    readonly property bool editTaken: gestures.editMode === "rename" && gestures.editRemote !== ""
        && gestures.editText.trim() !== ""
        && gestures.remotesModel.oidOfName(gestures.editRemote + "/" + gestures.editText.trim()) !== ""
    /// A box that opened empty and is still empty: the one for a new branch's name, before a word has been put in it.
    /// **Still open.** The frame answers for what was typed (デザイン規約 §可否・警告の出し場所), and nothing has been —
    /// a box that comes up already turned down is turning down the reader's arrival. A rename rubbed out to nothing is
    /// the other thing: a name was there and has been taken away, which git would refuse.
    readonly property bool editUnanswered:
        (gestures.editMode === "branch" || gestures.editMode === "tag")
        && gestures.editText.trim() === ""
    /// What is typed cannot be accepted. The rules are git's own, asked of core (a stash's label is free
    /// text).
    readonly property bool editRefused: gestures.editKey !== "" && !gestures.editUnanswered
        && (gestures.editTaken
            || gestures.editCaseOnly
            || gestures.editGitRefusal !== ""
            || !(gestures.editKind === "stash"
                 ? GitFacts.validStashMessage(gestures.editText)
                 : GitFacts.validRefName(gestures.editText)))
    /// Only the letters' case differs from the name the row already carries. **git writes a ref as a file**, so on a
    /// case-insensitive disk the new name lands on the old one's and both are gone — core refuses it outright
    /// (`tag::rename`), and asking here is what puts the answer in the box the name was typed into
    /// (デザイン規約 §答えの要らない報せ, by design).
    ///
    /// **`tag` is the kind here** — the mode says which of the box's three questions is being asked
    /// (`rename` / `branch` / `tag`), and this is only about the one that renames something that is already there.
    readonly property bool editCaseOnly:
        gestures.editKind === "tag" && gestures.editMode === "rename"
        && gestures.editText.trim() !== gestures.editId
        && gestures.editText.trim().toLowerCase() === gestures.editId.toLowerCase()
    readonly property string editRefusedWhy: !gestures.editRefused ? ""
        // git's own words win: it was asked about this very name and answered, where the rest are what this end
        // worked out before asking.
        : gestures.editGitRefusal !== ""
          ? gestures.editGitRefusal
        : gestures.editText.trim() === ""
          ? qsTr("A name is needed")
          : gestures.editCaseOnly
            ? qsTr("Only the letter case differs — on this disk that deletes both names")
          : gestures.editTaken
            ? qsTr("%1 already has a branch called that").arg(gestures.editRemote)
            : gestures.editKind === "stash"
              ? qsTr("One line, and nothing invisible in it")
              : qsTr("git will not take this as a name")

    function startEdit(kind, key, mode, id, oid, text) {
        // **The one door into the box, so the hold is asked once here** — the second click's own wait comes through,
        // and so do the menu's three ways in (`beginRename` / `beginBranchAt` / `beginTagAt`). Every one of them ends
        // in a write, and a box that opened while the doors are held would take a name nothing can be done with.
        if (gestures.held)
            return
        // The box opens where the row is — folded, that is the section standing beside the rail, and the list stays
        // folded (デザイン規約 §左メニューを畳む: a click in a peek keeps the fold — undoing it would take the
        // diff it was made for down). The hover that raised that section no longer decides how long it stands: the box
        // holds it open, the way a menu does (`pinned`).
        gestures.editKind = kind
        gestures.editMode = mode
        gestures.editId = id
        gestures.editOid = oid
        gestures.editText = text
        gestures.editKey = key
    }
    function stopEdit() {
        // **A box coming down spends the gesture that opened it** — the same rule the graph's box answers to
        // (`GraphPane.stopNaming`). Without it a row whose box was walked away from is still the row last clicked, so
        // the very next click on it opens the box again a window later, which reads as a blink.
        if (gestures.editKey !== "")
            gestures.forgetClicks()
        gestures.editKey = ""
        gestures.editText = ""
        gestures.editMode = ""
        gestures.editWaiting = false
        gestures.editGitRefusal = ""
    }
    // What git said is about the name it was asked about. One key on top of it and that is no longer the name in the
    // box, so the answer goes with it.
    onEditTextChanged: gestures.editGitRefusal = ""
    function submitEdit(text) {
        const kind = gestures.editKind
        const id = gestures.editId
        const oid = gestures.editOid
        const mode = gestures.editMode
        // Enter on a box nobody has typed in leaves it standing, the same as Enter on a refused one: it has not been
        // answered, and closing it would be answering for the reader (`editUnanswered`).
        if ((mode === "branch" || mode === "tag") && text.trim() === "")
            return
        // What the box opened with: a remote branch is typed without the remote it is on, so the name it answers to is
        // not what it shows.
        const was = kind === "remote" ? gestures.remoteBranchHalf(id) : id
        if (mode === "branch") {
            gestures.stopEdit()
            gestures.host.branchAtRequested(oid, text.trim())
            return
        }
        if (mode === "tag") {
            gestures.stopEdit()
            gestures.host.tagAtRequested(oid, text.trim())
            return
        }
        // The name it opened holding is not a rename: the box was left as it was found, so this is the way out of it.
        if (text.trim() === was) {
            gestures.stopEdit()
            return
        }
        // **A rename keeps its box until git answers** (デザイン規約 §答えの要らない報せ の色の軸): git turns names down that
        // nothing here could have known about — one that is already taken is the common one — and closing the box
        // first throws away what was typed and leaves the answer nowhere to go but the log. The page takes it down on
        // the landing (`RepoPage.absorbWriteResult`).
        gestures.editWaiting = true
        gestures.host.renameSubmitted(kind, id, text.trim())
    }
    /// A rename is out and git has not answered yet. The box stays as it is, and what comes back either takes it down
    /// or writes git's own refusal under it (`editGitRefusal`).
    property bool editWaiting: false
    /// What git said about the name in the box, kept until the reader types something else — at which point it is
    /// about a name nobody asked git about.
    property string editGitRefusal: ""
    /// The answer to that rename: it landed, so the box has done its job.
    function renameLanded() {
        if (gestures.editWaiting)
            gestures.stopEdit()
    }
    /// …or git would not have it, and the box is where that belongs.
    function renameRefused(why) {
        if (!gestures.editWaiting)
            return
        gestures.editWaiting = false
        gestures.editGitRefusal = why
    }
    /// Which row has its facts open under it, by the name git knows it by (`NavRowFacts`) — the hover expansion of a
    /// BRANCHES row. Held here for the reason the two gestures above are: a delegate is recycled the moment its row
    /// scrolls off, only one row is ever open, and the row that opens may be in the sections, in the sticky stand-in
    /// or in the section the folded rail has open.
    property string openKey: ""
    /// The last row to open, and where the pointer was standing when it did — in the window's own coordinates.
    /// **A row opens where the hand travelled to it, and not where the list travelled to the hand**: an open row is
    /// taller than a closed one, so the rows under it move, and a pointer that never left its place can be left over
    /// a row it was never aimed at. Opening that one would move the list again under a hand that asked for nothing,
    /// and the two would go on trading places (デザイン規約 §左メニューの所作).
    property string openedKey: ""
    property point openedAt: Qt.point(-1, -1)
    function openFacts(key, at) {
        if (gestures.editKey !== "" || gestures.menuOpen)
            return
        // The same row coming back under the same hand is the reader pointing at it again — it is a row arriving
        // under a hand that did not move that says nothing was asked for.
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
    /// Whether the pointer is still where the last row opened under it. Half a row of slack: a hand that has moved
    /// that far has moved on purpose, and anything less is the list having moved instead.
    function settled(at) {
        return Math.abs(at.x - gestures.openedAt.x) < Theme.rowHeight / 2
            && Math.abs(at.y - gestures.openedAt.y) < Theme.rowHeight / 2
    }
    /// A menu raised from the open row's own lines. **That one does not take them down** — it is about the row they
    /// belong to, and the reader who right-clicked them is reading about that row. Every other menu does: the
    /// pointer is over there, and what is open would be moving rows under the hand reading them.
    property bool menuFromFacts: false
    function noteMenuFromFacts() {
        gestures.menuFromFacts = true
    }
    /// Nothing opens over a name box or under a menu: the box is the mode the reader has entered and the menu is the
    /// rows they are reading, and what opens moves the rows under both (デザイン規約 §左メニューの所作).
    onEditKeyChanged: if (gestures.editKey !== "") gestures.openKey = ""
    onMenuOpenChanged: {
        if (gestures.menuOpen && !gestures.menuFromFacts)
            gestures.openKey = ""
        if (!gestures.menuOpen)
            gestures.menuFromFacts = false
    }

    /// A click landed somewhere: any box open elsewhere is walked away from (nothing is asked — what it costs is the
    /// typing). Which row it landed on is the gesture's own to remember — it is what tells its next click apart.
    function noteClick(key) {
        if (gestures.editKey !== "" && gestures.editKey !== key)
            gestures.stopEdit()
    }
    /// The rows this gesture was made on have gone (the folded rail's peek closed): the memory and the wait go with
    /// them, or the next visit's first click comes up as a second one (app-ui.md).
    function forgetClicks() {
        gestures.reclick.forget()
    }
    /// Double-click: where the row leads (デザイン規約 §左メニューの所作).
    function activateRow(kind, name, full, oidHex) {
        // The two of these that write: a switch, and the box a tag's row opens for a new branch's name. **A worktree
        // row goes through** — opening another working copy in a tab writes nothing in this one, and going
        // somewhere else to read while a rewrite runs is exactly what the hold is meant to leave alone
        // (デザイン規約 §左メニューの所作 の replay の段).
        if (gestures.held && kind !== "worktree")
            return
        const id = full !== "" ? full : name
        if (kind === "branch")
            gestures.host.refSwitchRequested("L", id)
        else if (kind === "remote")
            gestures.host.refSwitchRequested("R", id)
        else if (kind === "worktree")
            gestures.host.worktreeActivated(full)
        else if (kind === "tag")
            // A tag is a mark: the row offers the one thing that would make it somewhere to carry on from.
            gestures.startEdit(kind, "tag:" + id, "branch", id, oidHex, "")
        // A stash is a shelf, and a folder a heading: both stay put.
    }
    /// The menu's way into the same box, for anyone who does not know the gesture or cannot aim two separate clicks at
    /// one row.
    function beginRename(kind, id, text) {
        gestures.startEdit(kind, kind + ":" + id, "rename", id, "", text)
    }
    /// The same box on any row that names a commit, not just a tag's: a branch is most often started where another one
    /// already stands.
    function beginBranchAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "branch", id, oidHex, "")
    }
    /// And the same box again for a tag on that commit. One field, three modes: what is typed is a ref name either
    /// way, and the placeholder is the whole of what tells them apart (`NavNameBox`).
    function beginTagAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "tag", id, oidHex, "")
    }
}

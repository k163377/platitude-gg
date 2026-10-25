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
    /// Asked by the tag boxes, for the names already there (`nameClash`).
    required property NavSectionModel tagsModel

    /// The page's standing menu's door (`SidebarPane.menuDoor`), null while none stands — raised on these rows or
    /// anywhere else. One property: what is weighed is both at once, that a menu stands and whether it is the open
    /// row's.
    property var menuDoor: null
    readonly property bool menuOpen: gestures.menuDoor !== null

    /// The pane's write doors are held (`SidebarPane.doorsHeld`). Only the two gestures that write, or open a box to
    /// write with, answer for it; a click, a fold and a scroll go on working
    /// (デザイン規約 §消す操作は先に画面から消す「消すのは行だけ、周りは生きたまま」).
    property bool held: false

    /// The sidebar's one `ReclickGesture` (rules-refs/app-ui.md「間を空けた 2 回目のクリック」). What the wait was
    /// aimed at is read at the click (`p`): by the time it runs out the row may be showing another name.
    property ReclickGesture reclick: ReclickGesture {
        onRenameAsked: (key, p) => {
            if (p.withheld !== undefined)
                gestures.withholdRename(key, p.withheld, p.mark)
            else
                gestures.startEdit(p.kind, key, "rename", p.id, p.oid, gestures.typedName(p.kind, p.id, p.name))
        }
    }
    property alias activeKey: gestures.reclick.activeKey
    /// The name a row is typed and renamed by: a stash's message (git knows it by its selector), a worktree's folder
    /// (git knows it by its path), a remote branch without the remote it lives on, and the id for everything else.
    function typedName(kind, id, name) {
        return kind === "stash" || kind === "worktreeFolder" ? name
             : kind === "remote" ? gestures.remoteBranchHalf(id) : id
    }
    /// A spaced second click a worktree's lock or gone folder turned down (`GitFacts.worktreeRename`): the row, and
    /// why — said where the box would have stood, as a refused name's reason is, until the next press or key
    /// (デザイン規約 §worktree の名前を変える「箱を開かないもの」).
    property string withheldKey: ""
    property string withheldWhy: ""
    /// The worktree's folder in that line, which the tip stands the tree mark in front of.
    property string withheldMark: ""
    function withholdRename(key, why, mark) {
        if (gestures.held)
            return
        gestures.stopEdit()
        // The words before the row: the row builds its tip as the key arrives (`NavItemDelegate.withheldSeat`).
        gestures.withheldWhy = why
        gestures.withheldMark = mark
        gestures.withheldKey = key
    }
    function dropWithheld() {
        gestures.withheldKey = ""
        gestures.withheldWhy = ""
        gestures.withheldMark = ""
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
                                gestures.branchesModel.nameClash(gestures.editText.trim(), ""),
                                gestures.editWorktreePlace)
    /// The box names a tag: a new one, or a tag's rename. Every other ref box names a branch — a new one, a branch's
    /// rename, a remote branch's (a branch over there) — a stash's is free text, and a worktree's folder is a folder.
    readonly property bool editNamesTag: gestures.editMode === "tag"
        || (gestures.editMode === "rename" && gestures.editKind === "tag")
    /// git's rules for the kind of name the box takes, asked of core. A worktree's folder follows the disk's rules
    /// instead (`editFolderWhy`).
    readonly property bool editNameRuled: gestures.editMode === "rename" && gestures.editKind === "stash"
        ? GitFacts.validStashMessage(gestures.editText)
        : gestures.editMode === "rename" && gestures.editKind === "worktreeFolder" ? true
        : gestures.editNamesTag ? GitFacts.validTagName(gestures.editText.trim())
        : GitFacts.validBranchName(gestures.editText.trim())
    /// What git would refuse the name for beside the branches or tags already there (`Words.nameClashed`): a new
    /// branch or tag, or a local one's rename, the row's own name out of its own way. A remote branch's rename asks the
    /// remote's names instead (`editTaken`).
    readonly property string editClashWhy: {
        const name = gestures.editText.trim()
        if (name === "")
            return ""
        if (gestures.editMode === "branch")
            return Words.nameClashed(gestures.branchesModel.nameClash(name, ""), false)
        if (gestures.editMode === "tag")
            return Words.nameClashed(gestures.tagsModel.nameClash(name, ""), true)
        if (gestures.editMode !== "rename")
            return ""
        if (gestures.editKind === "branch")
            return Words.nameClashed(gestures.branchesModel.nameClash(name, gestures.editId), false)
        if (gestures.editKind === "tag")
            return Words.nameClashed(gestures.tagsModel.nameClash(name, gestures.editId), true)
        return ""
    }
    /// Where a worktree's folder would go renamed to what is typed (`NavSectionModel.renamedWorktreeFor`), asked per
    /// keystroke; undefined for every other box, and for a name that is not one folder's.
    readonly property var editFolderPlace: gestures.editKind !== "worktreeFolder" ? undefined
        : gestures.worktreesModel.renamedWorktreeFor(gestures.editId, gestures.editText.trim())
    /// What stands in that folder's way, said before the press — the graph's box says the same
    /// (`Words.worktreeRenameRefused`). A folder's name follows the disk's rules, not git's.
    readonly property string editFolderWhy: gestures.editKind !== "worktreeFolder" ? ""
        : Words.worktreeRenameRefused(gestures.editText, gestures.editFolderPlace)
    /// A branch being renamed that a rebase or bisect stopped in some worktree holds — this one's too: git renames none
    /// such, whatever the name (`NavSectionModel.heldByOperation`). Undefined for every other box.
    readonly property var editHeld: gestures.editMode === "rename" && gestures.editKind === "branch"
        ? gestures.worktreesModel.heldByOperation(gestures.editId) : undefined
    readonly property string editHeldWhy: gestures.editHeld === undefined ? ""
        : Words.heldByOperation(gestures.editHeld.by, gestures.editHeld.here,
                                gestures.worktreesModel.worktreeCalled(gestures.editHeld.path))
    /// What is typed cannot be accepted. The rules are git's own, asked of core (a stash's label is free text, a
    /// worktree's name a folder's).
    readonly property bool editRefused: gestures.editKey !== "" && !gestures.editUnanswered
        && (gestures.editTaken
            || gestures.editCaseOnly
            || gestures.editWorktreeWhy !== ""
            || gestures.editFolderWhy !== ""
            || gestures.editHeldWhy !== ""
            || !gestures.editNameRuled
            || gestures.editClashWhy !== "")
    /// A tag rename that changes only the letters' case. git writes a ref as a file, so on a case-insensitive disk the
    /// new name lands on the old one's and both are gone; core refuses it (`tag::rename`), and asking here puts the
    /// answer in the box (デザイン規約 §答えの要らない報せ). `tag` is the row's kind — mode `tag` is the new-tag box.
    readonly property bool editCaseOnly:
        gestures.editKind === "tag" && gestures.editMode === "rename"
        && gestures.editText.trim() !== gestures.editId
        && gestures.editText.trim().toLowerCase() === gestures.editId.toLowerCase()
    readonly property string editRefusedWhy: !gestures.editRefused ? ""
        : gestures.editFolderWhy !== ""
          ? gestures.editFolderWhy
        : gestures.editText.trim() === ""
          ? Words.nameNeeded
          : gestures.editHeldWhy !== ""
            ? gestures.editHeldWhy
          : gestures.editWorktreeWhy !== ""
            ? gestures.editWorktreeWhy
          : gestures.editCaseOnly
            ? qsTr("Only the letter case differs — on this disk that deletes both names")
          : gestures.editTaken
            ? Words.remoteHasBranch(gestures.editRemote)
          : !gestures.editNameRuled
            ? (gestures.editMode === "rename" && gestures.editKind === "stash"
               ? qsTr("One line, and nothing invisible in it")
               : Words.notAName)
            : gestures.editClashWhy
    /// The branch, tag or remote that line names, behind its mark (`Words.refusalMarks`) — asked again of the clash it
    /// was written from (`editClashWhy`).
    readonly property var editRefusedMarks: {
        // Nothing refused, nothing to seat — and no clash asked of the model for it.
        if (gestures.editRefusedWhy === "")
            return []
        const tag = gestures.editMode === "tag" || (gestures.editMode === "rename" && gestures.editKind === "tag")
        // A new worktree's box names the branch it makes, refused against the branches (`editWorktreeWhy`).
        const branch = gestures.editMode === "branch" || gestures.editMode === "worktree"
                       || (gestures.editMode === "rename" && gestures.editKind === "branch")
        const clash = !tag && !branch ? undefined
            : (tag ? gestures.tagsModel : gestures.branchesModel).nameClash(
                  gestures.editText.trim(), gestures.editMode === "rename" ? gestures.editId : "")
        return Words.refusalMarks(gestures.editRefusedWhy, clash, tag, gestures.editTaken ? gestures.editRemote : "")
    }
    /// The worktree's folder in that line, which the box's tip stands the tree mark in front of
    /// (`Words.worktreeNameMark`); empty where the line names none.
    readonly property string editRefusedMark: gestures.editKind === "worktreeFolder"
        ? Words.worktreeRenameMark(gestures.editRefusedWhy, gestures.editFolderPlace)
        : gestures.editHeldWhy !== "" && gestures.editRefusedWhy === gestures.editHeldWhy
          && !gestures.editHeld.here
        ? gestures.worktreesModel.worktreeCalled(gestures.editHeld.path)
        : Words.worktreeNameMark(gestures.editRefusedWhy, gestures.editWorktreePlace)

    function startEdit(kind, key, mode, id, oid, text) {
        // The one door into the box (the second click and the menu's four `begin*`), so the hold is asked here: a box
        // opened while the doors are held would take a name nothing can be done with.
        if (gestures.held)
            return
        gestures.dropWithheld()
        // The box opens where the row is; folded, that is the peek beside the rail, and the list stays folded
        // (デザイン規約 §左メニューを畳む). The box holds the peek open the way a menu does (`pinned`).
        gestures.editKind = kind
        gestures.editMode = mode
        gestures.editId = id
        gestures.editOid = oid
        gestures.editText = text
        gestures.editOpenedWith = text
        gestures.editKey = key
    }
    /// What the box opened holding — a worktree's folder name, which is no rename to send back (git knows the worktree
    /// by its path, `editId`).
    property string editOpenedWith: ""
    function stopEdit() {
        // A box coming down spends the gesture that opened it (as `GraphPane.stopNaming` does), or the next click on
        // the same row reopens it a window later — a blink.
        if (gestures.editKey !== "")
            gestures.forgetClicks()
        gestures.editKey = ""
        gestures.editText = ""
        gestures.editMode = ""
        gestures.editWaiting = false
    }
    function submitEdit(text) {
        const kind = gestures.editKind
        const id = gestures.editId
        const oid = gestures.editOid
        const mode = gestures.editMode
        // Enter on a box nobody has typed in leaves it standing, as on a refused one (`editUnanswered`).
        if ((mode === "branch" || mode === "tag" || mode === "worktree") && text.trim() === "")
            return
        const was = kind === "worktreeFolder" ? gestures.editOpenedWith
                  : kind === "remote" ? gestures.remoteBranchHalf(id) : id
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
        // A rename keeps its box until git answers (デザイン規約 §答えの要らない報せ): git can still turn it down for what
        // nothing here could know (another git holding a lock, a name taken in the moment since), and a box closed
        // first loses what was typed. The page takes it down on the landing (`RepoPage.absorbWriteResult`).
        gestures.editWaiting = true
        gestures.host.renameSubmitted(kind, id, oid, text.trim())
    }
    /// A rename is out and git has not answered; the answer takes the box down, or leaves it holding the name to try
    /// again — the report says why, and the name was not what git turned down.
    property bool editWaiting: false
    function renameLanded() {
        if (gestures.editWaiting)
            gestures.stopEdit()
    }
    function renameRefused() {
        if (!gestures.editWaiting)
            return
        gestures.editWaiting = false
    }
    /// The panel's hand (`HandWatch`), which the open row's seat reads: a row the rows moved under opens only once
    /// the hand has moved.
    property HandWatch hand: null
    /// The open row and its seat (`OpenRowSeat`), read here by the names the rows and the runs use.
    readonly property OpenRowSeat openRow: OpenRowSeat {
        id: openRowSeat
        hand: gestures.hand
        menuDoor: gestures.menuDoor
        editKey: gestures.editKey
        host: gestures.host
    }
    readonly property alias openKey: openRowSeat.openKey
    readonly property alias openSeat: openRowSeat.seat
    readonly property alias openTipOut: openRowSeat.tipOut
    readonly property alias menuOnOpenRow: openRowSeat.menuOnRow
    function openFacts(key, at, row, now, model, name) {
        openRowSeat.openFacts(key, at, row, now, model, name)
    }
    function rowsRebuilt() {
        openRowSeat.rowsRebuilt()
    }
    function closeFacts(key, now) {
        openRowSeat.closeFacts(key, now)
    }
    // A menu is other input, raised by the keyboard too (the press's own door is `RepoPage.notePress`).
    onMenuOpenChanged: {
        if (gestures.menuOpen)
            gestures.dropWithheld()
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
    function beginRename(kind, id, text, oid) {
        // On the row it names: a worktree's folder is renamed on the WORKTREES row (`NavList.keyOf`).
        const row = kind === "worktreeFolder" ? "worktree" : kind
        gestures.startEdit(kind, row + ":" + id, "rename", id, oid, text)
    }
    /// The new-branch box on any row that names a commit, not just a tag's.
    function beginBranchAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "branch", id, oidHex, "")
    }
    /// The same box for a new tag. One field, four modes, told apart by the placeholder, the frame and what it opens
    /// holding (`NavNameBox`).
    function beginTagAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "tag", id, oidHex, "")
    }
    /// …and for a new branch out in a worktree of its own (`Create worktree here…`).
    function beginWorktreeAt(kind, id, oidHex) {
        gestures.startEdit(kind, kind + ":" + id, "worktree", id, oidHex, "")
    }
}

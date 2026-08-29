import QtQuick
import platitude
import platitude.ui

// The sidebar's row gestures: which row was clicked last, and which one has a name box open in it. Held beside the
// lists rather than inside one — only one row at a time is either, whichever section it sits in, and both have to
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

    /// The two clicks a row answers with one gesture — one for the sidebar, not one per row (`ReclickGesture`): a
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
    /// `origin/` is where the branch lives, not part of its name.
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
    /// A name the remote already carries. Refused here rather than left to git: a plain push to a name that exists
    /// fast-forwards it and reports success, so somebody else's branch would move instead of this one being renamed.
    /// Only ever a rename's rule: the box for a new branch's name opens on a remote row too, and what it makes is a
    /// local branch — a name the remote happens to carry is no answer to that.
    readonly property bool editTaken: gestures.editMode === "rename" && gestures.editRemote !== ""
        && gestures.editText.trim() !== ""
        && gestures.remotesModel.oidOfName(gestures.editRemote + "/" + gestures.editText.trim()) !== ""
    /// A box that opened empty and is still empty: the one for a new branch's name, before a word has been put in it.
    /// **Not a refusal.** The frame answers for what was typed (デザイン規約 §可否・警告の出し場所), and nothing has been —
    /// a box that comes up already turned down is turning down the reader's arrival. A rename rubbed out to nothing is
    /// the other thing: a name was there and has been taken away, which git would refuse.
    readonly property bool editUnanswered:
        (gestures.editMode === "branch" || gestures.editMode === "tag")
        && gestures.editText.trim() === ""
    /// What is typed cannot be accepted. The rules are git's own, asked of core (a stash's label is free text, not a
    /// ref name).
    readonly property bool editRefused: gestures.editKey !== "" && !gestures.editUnanswered
        && (gestures.editTaken
            || gestures.editCaseOnly
            || !(gestures.editKind === "stash"
                 ? GitFacts.validStashMessage(gestures.editText)
                 : GitFacts.validRefName(gestures.editText)))
    /// Only the letters' case differs from the name the row already carries. **git writes a ref as a file**, so on a
    /// case-insensitive disk the new name lands on the old one's and both are gone — core refuses it outright
    /// (`tag::rename`), and asking here is what puts the answer in the box the name was typed into instead of in a log
    /// with nothing in it (デザイン規約 §答えの要らない報せ, 2026-08-29 ユーザー判断).
    ///
    /// **`tag` is the kind here, not the mode** — the mode says which of the box's three questions is being asked
    /// (`rename` / `branch` / `tag`), and this is only about the one that renames something that is already there.
    readonly property bool editCaseOnly:
        gestures.editKind === "tag" && gestures.editMode === "rename"
        && gestures.editText.trim() !== gestures.editId
        && gestures.editText.trim().toLowerCase() === gestures.editId.toLowerCase()
    readonly property string editRefusedWhy: !gestures.editRefused ? ""
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
        // The box opens where the row is — folded, that is the section standing beside the rail, and the list is not
        // put back for it (デザイン規約 §左メニューを畳む: a click in a peek does not undo the fold, which would take the
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
        // the very next click on it opens the box again a window later, which reads as a blink (2026-08-26 ユーザー報告).
        if (gestures.editKey !== "")
            gestures.forgetClicks()
        gestures.editKey = ""
        gestures.editText = ""
        gestures.editMode = ""
    }
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
        gestures.stopEdit()
        if (mode === "branch")
            gestures.host.branchAtRequested(oid, text.trim())
        else if (mode === "tag")
            gestures.host.tagAtRequested(oid, text.trim())
        else if (text.trim() !== was)
            gestures.host.renameSubmitted(kind, id, text.trim())
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
        const id = full !== "" ? full : name
        if (kind === "branch")
            gestures.host.refSwitchRequested("L", id)
        else if (kind === "remote")
            gestures.host.refSwitchRequested("R", id)
        else if (kind === "worktree")
            gestures.host.worktreeActivated(full)
        else if (kind === "tag")
            // A tag is a mark, not somewhere to carry on from: the row offers the one thing that would make it one.
            gestures.startEdit(kind, "tag:" + id, "branch", id, oidHex, "")
        // A stash is not a place to stand, and a folder is not a row.
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

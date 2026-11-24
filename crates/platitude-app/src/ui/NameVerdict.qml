import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// What a name box may accept, and the line that says why not — one verdict for the left panel's box
// (`SidebarRowGestures.editVerdict`) and the graph's (`RepoPage.graphNameVerdict`, drawn by `GraphPane` through
// `namingRefused*`), each handing it what is typed, what it names and in which mode. A rename is refused even rubbed
// out, a new name only once one is typed: a box opened empty would come up turning down the reader's arrival
// (デザイン規約 §可否・警告の出し場所). The rules are git's own, asked of core; a stash's label is free text, a
// worktree's name a folder's.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」).
QtObject {
    id: verdict

    /// The configured remote names a remote branch's name is cut by.
    required property RepoTab repoTab
    /// The names already there: a remote's branches, the worktrees (and where a new one would go), the branches and
    /// the tags.
    required property NavSectionModel remotesModel
    required property NavSectionModel worktreesModel
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel

    /// The box: whether one is open, what it makes — `rename`, or a new `branch` / `tag` / `worktree` — the kind of
    /// the row it stands on (`branch` / `remote` / `tag` / `stash` / `worktreeFolder` …), that row's id (a ref's full
    /// name, a worktree's path, a stash's selector) and what is typed.
    property bool open: false
    property string mode: ""
    property string kind: ""
    property string rowId: ""
    property string text: ""

    readonly property string name: verdict.text.trim()
    readonly property bool renaming: verdict.mode === "rename"
    /// The box names a tag: a new one, or a tag's rename. Every other ref box names a branch — a new one, a branch's
    /// rename, a remote branch's (a branch over there) — a stash's is free text, and a worktree's folder is a folder.
    readonly property bool namesTag: verdict.mode === "tag" || (verdict.renaming && verdict.kind === "tag")

    /// The remote a remote row lives on (`origin`), empty for every other kind. Cut by the configured names, since a
    /// remote's own name may contain `/`; an unconfigured one cuts at the first slash (`GitFacts.remoteBranchOf`).
    readonly property string remote: verdict.kind !== "remote" ? ""
        : GitFacts.remoteBranchOf(verdict.rowId, verdict.repoTab.remoteNames).remote
    /// A rename to a name the remote already carries. Refused here: a plain push to an existing name fast-forwards
    /// it and reports success, moving somebody else's branch. Rename only — the new-branch box on a remote row makes
    /// a local branch.
    readonly property bool taken: verdict.renaming && verdict.remote !== "" && verdict.name !== ""
        && verdict.remotesModel.hasRemoteBranch(verdict.remote, verdict.name)
    /// Where the worktree named in the box would go (`NavSectionModel.newWorktreeFor`), asked per keystroke; undefined
    /// for every other box.
    readonly property var worktreePlace: verdict.mode !== "worktree" ? undefined
        : verdict.worktreesModel.newWorktreeFor(verdict.name)
    /// Where a worktree's folder would go renamed to what is typed (`NavSectionModel.renamedWorktreeFor`), asked per
    /// keystroke; undefined for every other box, and for a name that is not one folder's.
    readonly property var folderPlace: verdict.kind !== "worktreeFolder" ? undefined
        : verdict.worktreesModel.renamedWorktreeFor(verdict.rowId, verdict.name)
    /// A branch being renamed that a rebase or bisect stopped in some worktree holds — this one's too: git renames none
    /// such, whatever the name (`NavSectionModel.heldByOperation`). A branch merely checked out in a worktree is not
    /// turned down: git renames it and moves that worktree's HEAD along, locked or not — a lock stops `worktree remove`
    /// / `move` / `prune`, not this. Undefined for every other box. Headless:
    /// `rename-branch-box fix/rebasing:fix/renamed --preset worktree-places` (`<branch>:<typed>` opens the box on a row
    /// other than HEAD's).
    readonly property var held: verdict.renaming && verdict.kind === "branch"
        ? verdict.worktreesModel.heldByOperation(verdict.rowId) : undefined
    readonly property string heldWhy: verdict.held === undefined ? ""
        : Words.heldByOperation(verdict.held.by, verdict.held.here,
                                verdict.worktreesModel.worktreeCalled(verdict.held.path))

    /// Whether what is typed cannot be accepted, and why — empty for a name the box takes.
    readonly property bool refused: verdict.refusedWhy !== ""
    readonly property string refusedWhy: {
        if (!verdict.open)
            return ""
        // A worktree's folder follows the disk's rules, not git's, and nothing git said goes into its box.
        if (verdict.kind === "worktreeFolder")
            return Words.worktreeRenameRefused(verdict.text, verdict.folderPlace)
        // A new worktree is warned of what stands in its way before the press — that is the box's whole answer.
        if (verdict.mode === "worktree")
            return Words.worktreeNameRefused(verdict.text, verdict.branchesModel.nameClash(verdict.name, ""),
                                             verdict.worktreePlace)
        const name = verdict.name
        const tag = verdict.namesTag
        // A new branch or tag: git's rules, then the names already there.
        if (verdict.mode === "branch" || verdict.mode === "tag") {
            if (name === "")
                return ""
            if (!(tag ? GitFacts.validTagName(name) : GitFacts.validBranchName(name)))
                return Words.notAName
            return Words.nameClashed((tag ? verdict.tagsModel : verdict.branchesModel).nameClash(name, ""), tag)
        }
        if (!verdict.renaming)
            return ""
        // A stash's label is free text on one line, untrimmed (`GitFacts.validStashMessage`).
        if (verdict.kind === "stash")
            return GitFacts.validStashMessage(verdict.text) ? ""
                 : name === "" ? Words.nameNeeded
                 : qsTr("One line, and nothing invisible in it")
        if (name === "")
            return Words.nameNeeded
        if (verdict.heldWhy !== "")
            return verdict.heldWhy
        // A tag renamed to its own name in other case: git writes a ref as a file, so on a case-insensitive disk the
        // new name lands on the old one's and both are gone; core refuses it (`tag::rename`), and asking here puts the
        // answer in the box (デザイン規約 §答えの要らない報せ).
        if (tag && GitFacts.tagRenameCaseOnly(verdict.rowId, name))
            return Words.caseOnlyRename
        if (verdict.taken)
            return Words.remoteHasBranch(verdict.remote)
        if (!(tag ? GitFacts.validTagName(name) : GitFacts.validBranchName(name)))
            return Words.notAName
        // A local branch's or tag's rename asks the names here, its own out of its own way; a remote branch's asks the
        // remote's (`taken`).
        if (verdict.kind !== "branch" && verdict.kind !== "tag")
            return ""
        return Words.nameClashed((tag ? verdict.tagsModel : verdict.branchesModel).nameClash(name, verdict.rowId), tag)
    }
    /// The branch, tag or remote that line names, behind its mark (`Words.refusalMarks`) — asked again of the clash it
    /// was written from.
    readonly property var refusedMarks: {
        // Nothing refused, nothing to seat — and no clash asked of the model for it.
        if (verdict.refusedWhy === "")
            return []
        // A new worktree's box names the branch it makes, refused against the branches.
        const branch = verdict.mode === "branch" || verdict.mode === "worktree"
                       || (verdict.renaming && verdict.kind === "branch")
        const clash = !verdict.namesTag && !branch ? undefined
            : (verdict.namesTag ? verdict.tagsModel : verdict.branchesModel).nameClash(
                  verdict.name, verdict.renaming ? verdict.rowId : "")
        return Words.refusalMarks(verdict.refusedWhy, clash, verdict.namesTag, verdict.taken ? verdict.remote : "")
    }
    /// The worktree's folder in that line, which the box's tip stands the tree mark in front of
    /// (`Words.worktreeNameMark` / `worktreeRenameMark`); empty where the line names none.
    readonly property string refusedMark: verdict.kind === "worktreeFolder"
        ? Words.worktreeRenameMark(verdict.refusedWhy, verdict.folderPlace)
        : verdict.heldWhy !== "" && verdict.refusedWhy === verdict.heldWhy && !verdict.held.here
        ? verdict.worktreesModel.worktreeCalled(verdict.held.path)
        : Words.worktreeNameMark(verdict.refusedWhy, verdict.worktreePlace)
}

import QtQuick
import platitude
import platitude.ui
import platitude.ui.words

// What the graph's name box may accept, and the line that says why not — decided here, for the page to hand to
// `GraphPane` (`namingRefused*`), which only draws it. A rename is refused even rubbed out, a new name only once one
// is typed: a box opened empty would come up turning down the reader's arrival (デザイン規約 §可否・警告の出し場所).
// The left panel's box answers the same rules in `SidebarRowGestures` (`edit*`), apart from a stash's free-text label,
// which only the left panel renames.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」).
QtObject {
    id: verdict

    /// The box: what is typed, what it names and in which mode (`namingOid` / `namingMode` / `namingKind` /
    /// `namingId` / `namingText`).
    required property GraphPane graphPane
    /// The configured remote names a remote branch's name is cut by.
    required property RepoTab repoTab
    /// The names already there: a remote's branches, the worktrees (and where a new one would go), the branches and
    /// the tags.
    required property NavSectionModel remotesModel
    required property NavSectionModel worktreesModel
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel

    readonly property string graphRenameRemote: verdict.graphPane.namingKind !== "remote" ? ""
        : GitFacts.remoteOfRef(verdict.graphPane.namingId, verdict.repoTab.remoteNames)
    readonly property bool graphNameTaken: verdict.graphPane.namingMode === "rename" && verdict.graphRenameRemote !== ""
        && verdict.graphPane.namingText.trim() !== ""
        && verdict.remotesModel.hasRemoteBranch(verdict.graphRenameRemote, verdict.graphPane.namingText.trim())
    /// Where the worktree named in the graph's box would go (`NavSectionModel.newWorktreeFor`), asked per keystroke;
    /// undefined for every other box.
    readonly property var graphWorktreePlace: verdict.graphPane.namingMode === "worktree"
        ? verdict.worktreesModel.newWorktreeFor(verdict.graphPane.namingText.trim()) : undefined
    /// Where a worktree's folder would go renamed to what is typed in the graph's box
    /// (`NavSectionModel.renamedWorktreeFor`); undefined for every other box.
    readonly property var graphFolderPlace:
        verdict.graphPane.namingMode === "rename" && verdict.graphPane.namingKind === "worktreeFolder"
        ? verdict.worktreesModel.renamedWorktreeFor(verdict.graphPane.namingId, verdict.graphPane.namingText.trim())
        : undefined
    /// A local branch being renamed that a rebase or bisect stopped in some worktree holds — this one's too: git
    /// renames none such (`NavSectionModel.heldByOperation`, as `SidebarRowGestures.editHeld`).
    readonly property var graphNameHeld:
        verdict.graphPane.namingMode === "rename" && verdict.graphPane.namingKind === "branch"
        ? verdict.worktreesModel.heldByOperation(verdict.graphPane.namingId) : undefined
    readonly property string graphNameHeldWhy: verdict.graphNameHeld === undefined ? ""
        : Words.heldByOperation(verdict.graphNameHeld.by, verdict.graphNameHeld.here,
                                verdict.worktreesModel.worktreeCalled(verdict.graphNameHeld.path))
    /// The folder the box's tip stands the tree mark in front of, where the refusal names one
    /// (`Words.worktreeNameMark` / `worktreeRenameMark`).
    readonly property string graphNameRefusedMark: verdict.graphPane.namingKind === "worktreeFolder"
        ? Words.worktreeRenameMark(verdict.graphNameRefusedWhy, verdict.graphFolderPlace)
        : verdict.graphNameHeldWhy !== "" && verdict.graphNameRefusedWhy === verdict.graphNameHeldWhy
          && !verdict.graphNameHeld.here
        ? verdict.worktreesModel.worktreeCalled(verdict.graphNameHeld.path)
        : Words.worktreeNameMark(verdict.graphNameRefusedWhy, verdict.graphWorktreePlace)
    /// The branch, tag or remote that refusal names, behind its mark (`Words.refusalMarks`) — asked again of the clash
    /// the line was written from.
    readonly property var graphNameRefusedMarks: {
        // Nothing refused, nothing to seat — and no clash asked of the model for it.
        if (verdict.graphNameRefusedWhy === "")
            return []
        const tag = verdict.graphPane.namingMode === "tag"
                    || (verdict.graphPane.namingMode === "rename" && verdict.graphPane.namingKind === "tag")
        const clash = (tag ? verdict.tagsModel : verdict.branchesModel).nameClash(
            verdict.graphPane.namingText.trim(),
            verdict.graphPane.namingMode === "rename" ? verdict.graphPane.namingId : "")
        return Words.refusalMarks(verdict.graphNameRefusedWhy, clash, tag,
                                  verdict.graphNameTaken ? verdict.graphRenameRemote : "")
    }
    readonly property string graphNameRefusedWhy: {
        if (verdict.graphPane.namingOid === "")
            return ""
        // A worktree's folder follows the disk's rules, not git's, and nothing git said goes into its box
        // (`SidebarRowGestures.editFolderWhy`).
        if (verdict.graphPane.namingMode === "rename" && verdict.graphPane.namingKind === "worktreeFolder")
            return Words.worktreeRenameRefused(verdict.graphPane.namingText, verdict.graphFolderPlace)
        // A new worktree is warned of what stands in its way before the press — that is the box's whole answer.
        if (verdict.graphPane.namingMode === "worktree")
            return Words.worktreeNameRefused(verdict.graphPane.namingText,
                                         verdict.branchesModel.nameClash(verdict.graphPane.namingText.trim(), ""),
                                         verdict.graphWorktreePlace)
        const name = verdict.graphPane.namingText.trim()
        // A new branch or tag: git's rules, then the names already there. Empty is not refused — the box answers for
        // what was typed (デザイン規約 §可否・警告の出し場所).
        if (verdict.graphPane.namingMode === "branch" || verdict.graphPane.namingMode === "tag") {
            const tag = verdict.graphPane.namingMode === "tag"
            if (name === "")
                return ""
            if (!(tag ? GitFacts.validTagName(name) : GitFacts.validBranchName(name)))
                return Words.notAName
            return Words.nameClashed((tag ? verdict.tagsModel : verdict.branchesModel).nameClash(name, ""), tag)
        }
        if (verdict.graphPane.namingMode !== "rename")
            return ""
        if (name === "")
            return Words.nameNeeded
        if (verdict.graphNameHeldWhy !== "")
            return verdict.graphNameHeldWhy
        // As `SidebarRowGestures.editCaseOnly`: a tag renamed to its own name in other case loses both names on a
        // case-insensitive disk.
        const tag = verdict.graphPane.namingKind === "tag"
        if (tag && name !== verdict.graphPane.namingId
                && name.toLowerCase() === verdict.graphPane.namingId.toLowerCase())
            return qsTr("Only the letter case differs — on this disk that deletes both names")
        if (verdict.graphNameTaken)
            return Words.remoteHasBranch(verdict.graphRenameRemote)
        if (!(tag ? GitFacts.validTagName(name) : GitFacts.validBranchName(name)))
            return Words.notAName
        // A remote branch's rename asks the remote's names (`graphNameTaken`); a local ref's, the ones here, its own
        // out of its own way.
        if (verdict.graphPane.namingKind === "remote")
            return ""
        return Words.nameClashed(
            (tag ? verdict.tagsModel : verdict.branchesModel).nameClash(name, verdict.graphPane.namingId), tag)
    }
}

import QtQuick
import platitude.ui

// The toolbar's push for a branch the worktree is not on, in the toolbar's forms (デザイン規約 §リモートへ送る): a
// plain click where git would take it; the leased overwrite, held, where the far side has commits this branch
// lacks; and a first push that asks where it goes. The flag long, as every menu row's (§git 用語のコード表記).
// Out, in the first push's words, where the push would stay here: there is no remote to send it to. Out in its own
// words where the far side has it all: nothing to send (§メニュー「「今できない」行は無効で残す」).
//
// On the first level beside `pull`, standing on the BRANCH card's frozen answers (`RefBranchMenu.state`): both
// entrances carry the row and the card it reads, so the two cannot disagree. The branch the worktree is on gets no row
// (`RefsSnapshot::branch_push` answers nothing for it): its push's one door is the toolbar
// (規約 §メニュー「そこにしか入口が無いもの」). Nor does a remote branch's row: it is a reading of what is over there, with
// nothing to send. A branch another worktree has out keeps the row: git pushes a branch checked out anywhere.
// The forms are read off the branch's own counts and marks — against its upstream, or against the
// tracking ref of where a mark sends it (`remote::standing`). The destination is named as the remote branch
// (`origin/feature/x`); `…` says the question comes first.
//
// The press is answered by the toolbar's owner (`RepoTab.pushAnswer` → `PublishFlow.pushFails`), not by a ref row's
// (`refPushAnswer`): a refusal is about the branch, whichever door sent it, and with one owner only one push is out at
// a time (`PublishFlow.pushDoorsOpen`). A first push asks through `RepoPage.startPublishAskFor`: the question names
// this branch, marks its row, and checks the far name against its history. The table: `tst_branchcard`; headless
// `push-branch-menu` / `-ask` / `-go` (`[chip:]<branch>`, `--preset others-behind`).
AppMenuItem {
    id: pushRow

    required property RefBranchMenu card
    readonly property QtObject facts: pushRow.card.state

    readonly property bool publishing: pushRow.facts.pushState === "publish"
    /// The first push's words: where it goes is asked — or, out, there is nowhere to name.
    readonly property bool firstPushWords: pushRow.publishing || pushRow.facts.pushStaysHere
    readonly property bool overwrites: pushRow.facts.pushState === "behind" || pushRow.facts.pushState === "diverged"
    /// Why the row is out where the push would stay in this repository — the toolbar's words for the same standing
    /// (`BandPushButton`).
    readonly property string blockedStaysHere: qsTr("Cannot push — its upstream is not supported")
    /// …and where the far side already has every commit: the toolbar's words for `clean` (user decision — ask before
    /// changing).
    //: %1 is where the push goes, e.g. origin/feature/login.
    readonly property string blockedClean: qsTr("Nothing to push — %1 is up to date").arg(pushRow.facts.pushTarget)

    // Off the running press's length, so the chip cannot change under a hand already on the row
    // (`AppMenuItem.armedPresses`, デザイン規約 §回数押し).
    code: pushRow.armedPresses <= 0 ? "push" : "push --force"
    // Where it goes, behind its kind's mark; the first push asks it, which the `…` says
    // (デザイン規約 §はじめてリモートへ送る).
    //: Follows the `push` chip: "push to origin/feature".
    refSentence: pushRow.firstPushWords ? "" : qsTr("to %1")
    refName: pushRow.facts.pushTarget
    refTint: Theme.textPrimary
    refMark: "remote"
    //: Follows the `push` chip: "push to…", where the branch goes being asked first.
    text: pushRow.firstPushWords ? qsTr("to…") : pushRow.refSentence.arg(pushRow.refName)
    offered: pushRow.facts.pushState !== "" || pushRow.facts.pushStaysHere
    blockedReason: pushRow.facts.pushStaysHere ? pushRow.blockedStaysHere
                 : pushRow.facts.pushState === "clean" ? pushRow.blockedClean : ""
    holdPresses: pushRow.overwrites ? Metrics.holdPressesWarned : 0
    // Reaching past this machine is the warning tone (デザイン規約 §状態).
    holdTone: Theme.warning
    onTriggered: {
        if (pushRow.publishing)
            pushRow.card.publishRequested(pushRow.facts.refId, pushRow.facts.pushUpstream)
        else
            pushRow.card.pushRequested(pushRow.facts.refId, "", "")
    }
    onHeld: {
        pushRow.closeMenu()
        pushRow.card.pushRequested(pushRow.facts.refId, "lease", pushRow.facts.pushLease)
    }
}

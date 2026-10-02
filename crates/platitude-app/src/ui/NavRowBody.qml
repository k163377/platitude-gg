import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A sidebar row's ink, left to right: the name cell, the seat an open name box takes, and the right-aligned columns
// (a worktree's branch, a branch's ahead/behind, the remote/PR badge, the push mark). The row (`NavItemDelegate`)
// keeps the gestures, washes and box, and is handed in whole rather than copied into bindings paid on every reuse.
//
// Columns past the name are Loaders built only where worn (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」),
// invisible while inactive since an empty loader still takes the spacing. The marks give the layout their size: a
// canvas has no implicit size for a loader to pass on.
RowLayout {
    id: body

    required property Item row
    /// Where the box stands while open. The box is drawn outside this layout (`NavNameBox`), so growing past the
    /// seat does not push the row's columns.
    readonly property alias boxSeat: boxSeat
    /// Where this row draws its name (`CutName`) — what a caller measuring the column reads.
    readonly property Item nameInk: nameCell.nameInk
    /// The name in full, while the row has itself open: the field a drag takes it out of, and how far it hangs
    /// below the row's own line (`NameCell.whole`).
    readonly property Item nameWhole: nameCell.wholeField
    readonly property real nameWholeOver: nameCell.wholeOver
    /// The repository's own working copy (`models::nav::item::MAIN`): named by its branch, told apart by the house.
    readonly property bool homeCopy: body.row.kindHint === "worktree" && body.row.change === "MAIN"
    /// The branch that row is named by. Empty when detached (git lists no `branch` line), and the folder name stands
    /// instead — else the row is a house and a blank.
    readonly property string homeName: body.homeCopy ? body.row.bucket : ""
    /// The colour of "this window stands here": `textLink` as for a branch (the main copy is named by one), and a
    /// linked copy's `textHereTree` (デザイン規約 §ref の種別).
    readonly property color hereTone:
        body.row.kindHint === "worktree" && !body.homeCopy ? Theme.textHereTree : Theme.textLink
    /// Whether this section has a mark seat — not STASHES / TAGS (デザイン規約 §余白「印の立たないセクションは席を取らない」).
    readonly property bool seated: body.row.kindHint !== "stash" && body.row.kindHint !== "tag"

    spacing: Theme.spaceXs
    // The seat and the name (`NameCell`, shared with the file lists). The seat stays open when empty, so names at one
    // depth begin in one column (`seated`).
    NameCell {
        id: nameCell
        // Unseated and editing, the cell is empty but would still take the spacing and push the box right.
        visible: body.seated || !body.row.editing
        // While editing, the box seat takes the slack instead.
        Layout.fillWidth: !body.row.editing
        showName: !body.row.editing
        seated: body.seated
        // Open, the name shows whole in its place and wraps down (`NameCell.whole`) — the name this row draws, so the
        // main copy's branch.
        whole: !body.row.factsOpen ? ""
             : body.homeName !== "" ? body.homeName : body.row.factsName
        folder: body.row.folder
        change: body.row.change
        // A sidebar folder keeps its fold state in the change slot it has no change code for (`models::nav::item`).
        folded: body.row.change === "FOLDED"
        showChange: body.row.kindHint === "wt"
        // A worktree's state rides the change slot (`models::nav::item`): lock, prunable `!`, or the main copy's house
        // (git never locks or prunes the main one, so they never contend). A branch another copy holds wears `tree`,
        // not the padlock, which means `git worktree lock`. Open, `tree` goes (the lines name that copy) but a copy's
        // own state stays, so it does not flicker under the pointer.
        seatMark: body.row.kindHint === "branch"
                    ? (body.row.change === "HELD" && !body.row.factsOpen ? "tree" : "")
                : body.row.kindHint !== "worktree" ? ""
                : body.row.change === "LOCKED" ? "lock"
                : body.row.change === "PRUNABLE" ? "bang"
                : body.row.change === "MAIN" ? "home" : ""
        // Prunable warns; `tree` wears the WORKTREES colour it points at (規約 §ref の種別); a lock stays quiet.
        seatTint: body.row.change === "PRUNABLE" ? Theme.warning
                : nameCell.seatMark === "tree" ? Theme.success
                : Theme.textSecondary
        // The main copy is named by its branch, in the name's place (デザイン規約 §左メニューの所作; `homeName`).
        name: body.homeName !== "" ? body.homeName : body.row.name
        // A staged rename's source, as the commit's file list shows it. Not `orig_path`, which a folder row uses for
        // its own path.
        origPath: body.row.orig_name
        // Where the ref is, as a chip says it: grey when only remote, `hereTone` where the reader stands
        // (デザイン規約 §ref の種別). Open, a tag whose copy here stands apart from the remote its lines are read against
        // wears their warning: the name is that copy's line (デザイン規約 §左メニューの所作 の TAGS の段).
        tone: body.row.is_head ? body.hereTone
            : body.row.factsOpen && body.row.factsHereApart ? Theme.warning
            : body.row.only_remote ? Theme.textSecondary : Theme.textPrimary
        weight: body.row.is_head ? Theme.fontWeightStrong : Font.Normal
        // A pending file's line-ending mark; the row's hover says what, the diff pane in full.
        marked: body.row.kindHint === "wt" && body.row.eol_mark
    }
    Item {
        id: boxSeat
        visible: body.row.editing
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
    // A linked worktree's branch, right-aligned; `CutName` keeps the column's edge. Not while the open lines name it,
    // not on a detached copy (a word here reads as a branch), and not on the main copy, already named by it
    // (デザイン規約 §左メニューの所作).
    Loader {
        id: branchSeat
        active: !body.row.folder && body.row.kindHint === "worktree" && body.row.bucket !== "" && !body.homeCopy
                && !(body.row.factsOpen && body.row.factsBranch !== "")
        visible: branchSeat.active
        Layout.maximumWidth: body.row.listWidth / 2
        sourceComponent: CutName {
            text: body.row.bucket
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
        }
    }
    // Every branch's distance from its upstream (`models::nav` の `Role::Ahead` rides each row); none when level or
    // untracked (デザイン規約 §左メニューの所作). The main copy, named by its branch and opening no line for it, draws
    // that branch's measure here only while open — the counts it read as it opened (`NavFacts.answers`).
    Loader {
        id: trackSeat
        readonly property int ahead: body.homeCopy ? body.row.factsAhead : body.row.ahead
        readonly property int behind: body.homeCopy ? body.row.factsBehind : body.row.behind
        active: !body.row.folder && (trackSeat.ahead > 0 || trackSeat.behind > 0)
                && (body.row.kindHint === "branch" || (body.homeName !== "" && body.row.factsOpen))
        visible: trackSeat.active
        Layout.alignment: Qt.AlignVCenter
        sourceComponent: HeadTrack {
            ahead: trackSeat.ahead
            behind: trackSeat.behind
        }
    }
    // Nothing = local only, cloud = on a remote, PR mark = has a PR (fake until Phase 4: PGG_FAKE_PR). Tags too
    // (`ls-remote --tags`); remote-branch and worktree rows show only the PR. One colour for both
    // (デザイン規約 §ref の種別).
    Loader {
        id: remoteSeat
        /// An upstream that is gone (`models::nav::field` の `Role::Bucket`), kept as a badge: `has_remote` is false
        /// once the far side deleted the ref.
        readonly property bool gone: body.row.kindHint === "branch" && body.row.bucket !== ""
        // Stays while the row is open, unlike the seat mark: the counts beside it would shift (デザイン規約
        // §左メニューの所作).
        active: !body.row.folder
                && (remoteSeat.gone
                    || ((body.row.kindHint === "branch"
                         || body.row.kindHint === "tag")
                        && (body.row.has_remote || body.row.has_pr))
                    || ((body.row.kindHint === "remote" || body.row.kindHint === "worktree") && body.row.has_pr))
        visible: remoteSeat.active
        // The graph chips' size for the same badge (デザイン規約 §寸法).
        Layout.preferredWidth: Theme.iconSm
        Layout.preferredHeight: Theme.iconSm
        sourceComponent: GoneBadge {
            pullRequest: body.row.has_pr
            gone: remoteSeat.gone
        }
    }
    // On the remote pushes go to: the toolbar's push mark in the badge's seat, in `accent` as the one in effect
    // (デザイン規約 §アクセント).
    Loader {
        id: pushSeat
        active: body.row.pushesHere
        visible: pushSeat.active
        Layout.preferredWidth: Theme.iconSm
        Layout.preferredHeight: Theme.iconSm
        sourceComponent: NavIcon {
            kind: "push"
            tint: Theme.accent
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
}

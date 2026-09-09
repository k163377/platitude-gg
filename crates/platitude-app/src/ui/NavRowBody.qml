import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a sidebar row holds, left to right: the shared name cell, the seat an open name box takes its slack from, and
// the right-aligned columns (a worktree's branch, the head's ahead/behind, the remote/PR badge, the push mark).
//
// The row itself (`NavItemDelegate`) keeps the gestures, the washes and the box: this is only the ink. It is handed
// the whole row rather than eighteen mirrored properties — the row is recycled with its delegate, so there is nothing
// here to keep, and a second copy of every binding would be paid on every reuse.
//
// **Every column past the name is built only on the rows that wear it.** A delegate is built per row on screen, and
// a column built and hidden on every row is heap the rows are measured by (rules-refs/app-ui.md, the Loader rule).
// Each of those loaders is invisible while inactive as well: a layout skips an invisible item, and an empty loader
// would still take the spacing. The marks name their size to the layout outright — a canvas has no implicit size for
// a loader to pass on.
RowLayout {
    id: body

    /// The row being drawn (`NavItemDelegate`) — every column reads it.
    required property Item row
    /// Where the box stands, and the slack it takes off the row while it is open. The box itself is drawn outside
    /// this layout (`NavNameBox`) — it is allowed to be wider than the seat, and a seat that grew with it would push
    /// the row's own columns sideways.
    readonly property alias boxSeat: boxSeat

    spacing: Theme.spaceXs
    // The mark and the name, in the part both file lists share (`NameCell`): the slot every row opens with — a
    // folder's fold arrow, a worktree file's change icon, and later the mark for a hidden branch — and the name
    // after it. A ref row has nothing to put in the slot and it stays open all the same, which is what keeps every
    // name at a given depth beginning in one column.
    NameCell {
        // The box below takes the row's slack while it is open, and the slot stays where it is: a name going into a
        // box must not walk the columns beside it sideways.
        Layout.fillWidth: !body.row.editing
        showName: !body.row.editing
        folder: body.row.folder
        change: body.row.change
        // A sidebar folder keeps its fold state in the change slot it has no change code for (`models::nav::item`).
        folded: body.row.change === "FOLDED"
        showChange: body.row.kindHint === "wt"
        // A worktree row has no change code, so the seat carries the state of the checkout instead — the same
        // shared slot a folder keeps its fold state in (`models::nav::item`). A lock is somebody's choice and
        // wears the quiet colour every other row mark does; a folder git can no longer find is a warning.
        //
        // A branch row uses the same slot for the one question it shares with those rows: whether a move can land
        // here. Its mark is the WORKTREES section's own (`tree`) — where the branch actually is — and **not the
        // padlock**, which is spoken for by `git worktree lock`; a mark cannot mean two things in one window.
        seatMark: body.row.kindHint === "branch" ? (body.row.change === "HELD" ? "tree" : "")
                : body.row.kindHint !== "worktree" ? ""
                : body.row.change === "LOCKED" ? "lock"
                : body.row.change === "PRUNABLE" ? "bang" : ""
        seatTint: body.row.change === "PRUNABLE" ? Theme.warning : Theme.textSecondary
        name: body.row.name
        // Where a renamed file came from, said the same way the commit's own file list says it: a rename is two
        // names, and a row that shows only the new one leaves the reader to work out what moved. Empty on
        // everything else — the model fills it for staged files alone, which is the only side git names a source
        // on, and a folder row keeps its own path in the slot beside it (`orig_path`, which this is not).
        origPath: body.row.orig_name
        // The name says where the ref is, the way a chip's does: grey for one this repository does not hold (デザイン規約
        // §ref の種別).
        tone: body.row.is_head ? Theme.textLink : body.row.only_remote ? Theme.textSecondary : Theme.textPrimary
        weight: body.row.is_head ? Font.DemiBold : Font.Normal
        // A pending file whose change says something about its line endings wears the mark on the name's shoulder.
        // What it is about is the row's hover; the sentence in full is the diff pane's.
        marked: body.row.kindHint === "wt" && body.row.eol_mark
    }
    // Where the box stands, and the slack it takes off the row while it is open (see `boxSeat` above).
    Item {
        id: boxSeat
        visible: body.row.editing
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
    // Worktree rows: checked-out branch on the right. Cut in the part that keeps the column's edges (`CutName`) — this
    // one is the row's right-aligned column, so a cut that stopped short of the gutter left the branch names hanging
    // a different distance from the edge on every row.
    Loader {
        id: branchSeat
        active: !body.row.folder && body.row.kindHint === "worktree"
        visible: branchSeat.active
        Layout.maximumWidth: body.row.listWidth / 2
        sourceComponent: CutName {
            text: body.row.bucket !== "" ? body.row.bucket : qsTr("detached")
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
        }
    }
    // How far this branch stands from its upstream, left of the state icon. **Every branch that has something to say
    // says it, not only the one HEAD is on** — the counts ride each row out of the listing (`models::nav` の
    // `Role::Ahead`), so a branch the remote moved past is legible without switching to it.
    //
    // The seat is empty wherever there is nothing to count: level with the upstream, or no upstream to measure
    // against. Neither is a zero worth a column (デザイン規約 §左メニューの所作).
    Loader {
        id: trackSeat
        active: !body.row.folder && body.row.kindHint === "branch" && (body.row.ahead > 0 || body.row.behind > 0)
        visible: trackSeat.active
        Layout.alignment: Qt.AlignVCenter
        sourceComponent: HeadTrack {
            ahead: body.row.ahead
            behind: body.row.behind
        }
    }
    // Branch remote state: nothing = local only, remote icon = has a remote, PR icon = has a PR (real data in Phase
    // 4; PGG_FAKE_PR previews the look). Remote-branch and worktree rows show the PR state too. A tag reads the same
    // way — the badge answers "is this only here?" whatever it is on, and the fetch carries the bit for it
    // (`ls-remote --tags`). **Both marks wear one colour**: the mark itself is the answer, and a colour on top of it
    // would be a second one (デザイン規約 §ref の種別).
    Loader {
        id: remoteSeat
        active: !body.row.folder
                && (((body.row.kindHint === "branch"
                      || body.row.kindHint === "tag")
                     && (body.row.has_remote || body.row.has_pr))
                    || ((body.row.kindHint === "remote" || body.row.kindHint === "worktree") && body.row.has_pr))
        visible: remoteSeat.active
        // The size the graph's chips wear the same badge at: one question, one mark, one size (デザイン規約 §寸法).
        Layout.preferredWidth: Theme.iconSm
        Layout.preferredHeight: Theme.iconSm
        sourceComponent: NavIcon {
            kind: body.row.has_pr ? "pr" : "remote"
            tint: Theme.textSecondary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
    // The remote this repository sends pushes to. The toolbar's own push mark, in the seat the badge above holds
    // on every other row — one question, one mark, one size. `accent` because what it answers is which of the rows
    // is the one in effect (デザイン規約 §色 アクセント: 選択インジケータ), not what kind of ref the row is.
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

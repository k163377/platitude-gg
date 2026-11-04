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
    /// The repository's own worktree (`models::nav::item::MAIN`): named by its branch, told apart by the house. Not by
    /// its folder: that is the tab's name and the window's title already.
    readonly property bool homeWorktree: body.row.kindHint === "worktree" && body.row.change === "MAIN"
    /// The branch that row is named by. Empty when detached (git lists no `branch` line), and the folder name stands
    /// instead — else the row is a house and a blank. Both sides are held by `tst_navfacts`'
    /// `test_the_repositorys_own_worktree_is_named_by_the_branch_it_has_out`, on rows built by hand.
    readonly property string homeName: body.homeWorktree ? body.row.bucket : ""
    /// The name this row draws: the main worktree's branch, else the row's own (`homeName`).
    readonly property string shownName: body.homeName !== "" ? body.homeName : body.row.name
    /// The colour of "this window stands here": `textLink` as for a branch (the main worktree is named by one), and a
    /// linked worktree's `textHereTree` (デザイン規約 §ref の種別).
    readonly property color hereTone:
        body.row.kindHint === "worktree" && !body.homeWorktree ? Theme.textHereTree : Theme.textLink
    /// A branch the repository's own worktree keeps: it wears the house, in the green the tree would
    /// (デザイン規約 §ref の種別「名前の印」; `NavSectionModel.homeBranch`).
    readonly property bool heldHome: body.row.kindHint === "branch" && body.row.change === "HELD"
                                     && body.row.worktreesModel !== null
                                     && body.row.fullName === body.row.worktreesModel.homeBranch
    /// Whether this section has a mark seat — not STASHES / TAGS (user decision — ask before changing): flat lists of
    /// names, with nothing that folds and no worktree state, so the seat would always be empty ground and read as a
    /// margin nobody wrote. Their names start at the row's head. One answer per section; a TAGS row's opened lines
    /// keep the seat for their cloud (`NavFactLine`).
    readonly property bool seated: body.row.kindHint !== "stash" && body.row.kindHint !== "tag"
    /// The row-end columns' seat for a state after the name — the badge, the push mark: the seat of the name's letters
    /// (`MarkSeat`, デザイン規約 §アイコンの席 の名指し), one width down the rows.
    readonly property real stateSeat: MarkSeat.cell(nameCell.nameInk.pixelSize)

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
        // main worktree's branch.
        whole: !body.row.factsOpen ? ""
             : body.homeName !== "" ? body.homeName : body.row.factsName
        folder: body.row.folder
        change: body.row.change
        // A sidebar folder keeps its fold state in the change slot it has no change code for (`models::nav::item`).
        folded: body.row.change === "FOLDED"
        showChange: body.row.kindHint === "file"
        // A worktree's state rides the change slot (`models::nav::item`): lock, prunable `!`, or the main worktree's
        // house (git never locks or prunes the main one, so they never contend). A branch another worktree holds wears
        // `tree`, not the padlock, which means `git worktree lock` (user decision — ask before changing); its name
        // keeps its colour, since a click still jumps and merge / rebase still work — `textMuted` would lie. Open,
        // `tree` goes (the lines name that worktree) but a worktree's own state stays, so it does not flicker under the
        // pointer. Do not give a WORKTREES row the `tree` mark (user decision — ask before changing): its seat is the
        // lock's, the `!`'s and the house's, and one seat holds one mark.
        seatMark: body.row.kindHint === "branch"
                    ? (body.row.change === "HELD" && !body.row.factsOpen ? (body.heldHome ? "home" : "tree") : "")
                : body.row.kindHint !== "worktree" ? ""
                : body.row.change === "LOCKED" ? "lock"
                : body.row.change === "PRUNABLE" ? "bang"
                : body.row.change === "MAIN" ? "home" : ""
        // Prunable warns; `tree` wears the WORKTREES colour it points at (規約 §ref の種別); a lock stays quiet.
        seatTint: body.row.change === "PRUNABLE" ? Theme.warning
                : nameCell.seatMark === "tree" || body.heldHome ? Theme.success
                : Theme.textSecondary
        // The main worktree is named by its branch, in the name's place (`homeName`), so the name's face and colour
        // come with it. Not right-aligned in the far column: every name in this panel starts here, and a lone word at
        // the opposite edge reads as a row with no name, only a note. Nor the name emptied and the
        // branch put there — with `showName: false` this cell does not stretch, the slack is shared by preferred
        // widths, and the branch stands a third of the way along.
        name: body.shownName
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
        // A pending file's line-ending mark (デザイン規約 §改行コードの警告): a `!` on the name's right shoulder, not at
        // the row's right end — that is the seat of the stage `+` the hover puts out, and a mark there would hide the
        // moment the hand reached for it. The row's hover opens the line-ending card and nothing else (`EolHoverCard`,
        // after the usual rest), which adds what the mark does not say; the diff pane says it in full.
        marked: body.row.kindHint === "file" && body.row.eol_mark
    }
    Item {
        id: boxSeat
        visible: body.row.editing
        Layout.fillWidth: true
        Layout.fillHeight: true
    }
    // A linked worktree's branch, right-aligned; `CutName` keeps the column's edge. Not while the open lines name it,
    // not on a detached worktree (a word here reads as a branch — "no branch" is said by the empty column), and not on
    // the main worktree, already named by it: there it would say the row's one fact twice.
    Loader {
        id: branchSeat
        active: !body.row.folder && body.row.kindHint === "worktree" && body.row.bucket !== "" && !body.homeWorktree
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
    // untracked (デザイン規約 §左メニューの所作). The main worktree, named by its branch and opening no line for it (the
    // same word would stand right under the name; its upstream line still opens), draws that branch's measure here
    // only while open (user decision — ask before changing), as a BRANCHES row does — the counts it read as
    // it opened (`NavFacts.answers`; its own `ahead` role is 0). Held by `tst_navfacts`'
    // `test_the_repositorys_own_worktree_draws_its_branchs_measure_only_while_open`.
    Loader {
        id: trackSeat
        readonly property int ahead: body.homeWorktree ? body.row.factsAhead : body.row.ahead
        readonly property int behind: body.homeWorktree ? body.row.factsBehind : body.row.behind
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
        /// once the far side deleted the ref. A role, not a slot (`upstreamGoneOf`): a slot read in a binding freezes,
        /// and the badge would not follow a fetch.
        readonly property bool gone: body.row.kindHint === "branch" && body.row.bucket !== ""
        // Stays while the row is open, unlike the seat mark: the counts beside it measure from this cloud and would
        // shift (user decision — ask before changing; デザイン規約 §左メニューの所作).
        active: !body.row.folder
                && (remoteSeat.gone
                    || ((body.row.kindHint === "branch"
                         || body.row.kindHint === "tag")
                        && (body.row.has_remote || body.row.has_pr))
                    || ((body.row.kindHint === "remote" || body.row.kindHint === "worktree") && body.row.has_pr))
        visible: remoteSeat.active
        // The row-end column's seat (`stateSeat`), the badge drawn to the name's letters in its middle.
        Layout.preferredWidth: body.stateSeat
        Layout.preferredHeight: Theme.iconSm
        sourceComponent: GoneBadge {
            pullRequest: body.row.has_pr
            gone: remoteSeat.gone
            size: nameCell.nameInk.pixelSize
            wordBase: nameCell.y + nameCell.nameBase - remoteSeat.y
        }
    }
    // On the remote pushes go to: the toolbar's push mark in the badge's seat, in `accent` as the one in effect
    // (デザイン規約 §アクセント).
    Loader {
        id: pushSeat
        active: body.row.pushesHere
        visible: pushSeat.active
        Layout.preferredWidth: body.stateSeat
        // Taller than the badge's box, past the baseline: the mark's foot hangs a share of a pixel under it
        // (`NavIcon.footed`).
        Layout.preferredHeight: Theme.iconMd
        // A state after the name, as the badge is (デザイン規約 §アイコンの席「名前の後の状態」).
        sourceComponent: NavIcon {
            kind: "push"
            set: "name"
            size: nameCell.nameInk.pixelSize
            wordBase: nameCell.y + nameCell.nameBase - pushSeat.y
            tint: Theme.accent
            width: body.stateSeat
            height: Theme.iconMd
        }
    }
}

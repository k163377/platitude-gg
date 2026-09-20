pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The mark and the name a list row opens with, in the one place both file lists read it from: a commit's changed files
// (`FileRowDelegate`) and the working tree's (`NavItemDelegate`, which draws the ref rows with the same part). What a
// click means, and whatever else a row puts at its right edge, stays with the row — this is the half the two lists were
// saying twice, down to the seat the names start after.
RowLayout {
    id: nameCell

    /// A directory row: the seat holds a fold arrow, and the name speaks in the quieter colour — it is the shape of
    /// the names under it.
    property bool folder: false
    /// The change code git reports for the file (`M`, `??`, `UU`, …). A folder row has none, and the sidebar's rows
    /// keep their fold state in this same field to spare a `NavItem` a second one (`models::nav::FOLDED`).
    property string change: ""
    /// Whether a folder row is shut — which way its arrow points. **Asked as its own question**, because the two lists
    /// do not keep the answer in the same place: the sidebar packs it into `change` (above), a commit's changed files
    /// have a field of their own (`models::details::FileItem.collapsed`). Reading only the sidebar's is what left the
    /// commit list's arrow lying open in both states.
    property bool folded: false
    /// Whether a change mark belongs in the seat at all — a ref row has no change to report. **The seat is held open
    /// either way**: letting it collapse is what put a leaf's name to the *left* of the folder it sits under (layouts
    /// drop invisible children entirely), and at a given depth every name has to begin in the same column.
    property bool showChange: true
    property string name: ""
    /// Where a renamed file came from, drawn ahead of the new name with the way between them. Empty on everything else.
    /// **Already written the way the row writes names** — the models cut it back exactly as far as they cut the new one
    /// (`encode::rename_source`), so nothing here takes a path apart.
    property string origPath: ""
    /// The colour a named row wears. A folder is not asked: it is the quieter one wherever it appears, in both lists.
    property color tone: Theme.textPrimary
    property int weight: Font.Normal
    /// A mark on the name's shoulder, the way a button in trouble wears one (`ActionButton.alert`) — **outside the
    /// layout**, so no row moves and whatever sits at the right edge keeps its own seat.
    property bool marked: false
    property color markTint: Theme.warning
    /// A mark for the seat that is not a change code: a locked working copy's padlock, the bang on one whose folder
    /// has gone, and the WORKTREES mark on a branch another copy has out. `NavIcon.kind`, empty where the seat's
    /// usual tenants have it. **Only one of the three ever draws** — a row is a folder, a file with a change, or one
    /// of these.
    property string seatMark: ""
    property color seatTint: Theme.textSecondary
    /// Whether the name is in the row at all. The seat stays either way: a box opening over the name
    /// (`NavItemDelegate`'s rename) leaves the row's other columns where they are. The owner takes the slack back
    /// the same way it gives it — `Layout.fillWidth` follows this.
    property bool showName: true
    /// The same name in full, drawn **in the name's own place** in place of the cut one — what a row shows while it
    /// is open (デザイン規約 §左メニューの所作). Empty leaves the cut name where it is.
    ///
    /// **The row's arrangement does not move when a reader rests on it**: this stands over the cut name in the same
    /// column, so the seat, the counts and the badge stay where they were, and a name too long for one line wraps
    /// **downward** out of the row — which the row then grows by (`NavItemDelegate.nameOverflow`).
    property string whole: ""
    /// That field, and how far it hangs below the one line a name is drawn on — the row reads the first to drive
    /// a drag into and the second to grow by. **The overhang is counted off the field's own line**, not off the
    /// label it stands over: the two line boxes differ by a hair, and taking the difference against the label
    /// would open that hair as a gap under every name that fits.
    readonly property Item wholeField: wholeSeat.item
    readonly property real wholeOver:
        wholeSeat.item ? Math.max(0, wholeSeat.item.implicitHeight - wholeSeat.item.lineHeight) : 0
    /// Automation: the turn the fold arrow is drawn at, -1 on a row that has none. Read off the icon itself, so a
    /// run cannot go green with the arrow unwired (verify-ui — the same reading as
    /// `FileRowDelegate.litKey`).
    readonly property real foldTurn: foldSeat.item ? foldSeat.item.rotation : -1
    /// How wide the slot every row opens with is. **The seat is taken on the ink** — the same reading `NavHeader`'s
    /// own fold seat makes, where an `iconMd` chevron stands in an `iconSm` seat and overflows it (デザイン規約
    /// §余白「印が自分で持っている余白は、隣の詰めに数える」). A change code fills its box, so a list that shows one takes the box
    /// outright (`iconMd`); the marks that stand here where no change can — the fold arrow and the sidebar's state
    /// marks — are drawn on the `iconSm` grid and none of them reaches its edge, so that seat comes down a step
    /// further and the air stops being paid for twice (by design: the widest of them, the house a repository's own
    /// working copy wears, spans 11 of the 16 and is filled rather than stroked, which comes to the same ink as
    /// the padlock's 9 and the line it carries — so the seat holds both at one width, and that width is the
    /// ceiling a mark drawn for this seat is designed against).
    ///
    /// **One answer per list.** `showChange` is a section's kind, so every row's name begins in the same column as
    /// its neighbours' — the invariant the seat is held open for in the first place. It is the same reason the seat
    /// is one answer for the list: the padlock and the fold arrow hold different amounts of air, and a seat that
    /// measured them one row at a time would step the names in and out.
    readonly property int seatSize: nameCell.showChange ? Theme.iconMd : Theme.iconXs
    /// Where a mark stands inside that seat: hard against its left edge, which for the trimmed seat above means the
    /// box it is drawn in hangs a pixel out on the right. **The air is taken off the right alone** (observed) — the
    /// left of the mark is what the fold's own step reads as nesting, and centring the mark in a seat
    /// narrower than its box would walk it back towards the frame by half of whatever the right gave up. Zero where
    /// the change mark fills its seat: there the box *is* the ink, and the arrow it shares the slot with is centred in
    /// it the way it always was.
    readonly property real seatNudge: nameCell.showChange ? 0 : (Theme.iconSm - nameCell.seatSize) / 2
    /// Where the name itself is drawn — what the lines a row opens under itself read their voice off, and the one
    /// that knows whether the name had to be cut (`NavRowFacts`, app-ui.md §測って決める値は押し出す).
    readonly property Item nameInk: newName

    spacing: Theme.spaceXs

    // The seat every row opens with. **Each mark in it is built only on the rows that wear it**: a delegate is built
    // per row on screen and every mark is a canvas, so a mark built and hidden on the rows it is not for is heap the
    // rows are measured by (rules-refs/app-ui.md, the Loader rule).
    Item {
        Layout.preferredWidth: nameCell.seatSize
        Layout.preferredHeight: nameCell.seatSize
        Layout.alignment: Qt.AlignVCenter
        Loader {
            id: foldSeat
            active: nameCell.folder
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
                kind: "chevron"
                rotation: nameCell.folded ? 0 : 90
                tint: Theme.textSecondary
            }
        }
        Loader {
            active: !nameCell.folder && nameCell.showChange
            anchors.fill: parent
            sourceComponent: ChangeIcon {
                change: nameCell.change
            }
        }
        // The fold arrow's step. These stand in the sidebar alone, beside the arrows of the rows they are
        // nested among, and a mark drawn on the wider grid carried a heavier line than the folds it sits
        // between — weight is what the eye reads as size (`NavIcon.stroke`; by design). The line is
        // levelled *with* the arrow, so both wear the family's own
        // (デザイン規約 §寸法「畳みの山形は 2 階級」).
        //
        // **Placed in the seat.** A mark given the seat's own size takes the seat's top-left corner, and on a row
        // whose seat is centred in it that reads as a mark riding a step high (observed — the slip the
        // first pass at this made). So it is centred down the seat and set against its left edge (`seatNudge`).
        Loader {
            active: nameCell.seatMark !== ""
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: nameCell.seatNudge
            sourceComponent: NavIcon {
                width: Theme.iconSm
                height: Theme.iconSm
                kind: nameCell.seatMark
                tint: nameCell.seatTint
            }
        }
    }
    // A rename is two names with the way between them drawn: U+2192 is East Asian Ambiguous, so the CJK families
    // this app names hold it in a full-width cell and each draws its own arrow inside it (規約
    // §寸法「印は描いて出す」). Both names give ground when the row is narrow. The old name is capped at its own
    // width so it shrinks without growing (app-ui.md §`Layout.fillWidth` を書いていない子は縮まない床); **the new one takes
    // the slack** — a row where every child refuses it hands the leftover to the engine, which centres what it
    // cannot fill (measured: plain rows drifted to the middle of the pane).
    RowLayout {
        visible: nameCell.showName
        Layout.fillWidth: true
        spacing: 0
        // **The old name and its arrow are built only on a rename** — a label pair and a canvas that one row in a
        // thousand wears (the seat's rule). Invisible while inactive as well: a layout skips an invisible item, and an
        // empty loader would still take the spacing.
        Loader {
            id: origSeat
            active: !nameCell.folder && nameCell.origPath !== ""
            visible: origSeat.active
            Layout.fillWidth: true
            // The ceiling is the name's own width, and `CutName` reports that already rounded up — the layout hands an
            // item the whole pixel below a fractional ceiling, and a ceiling one hair under the name's own width cuts
            // it (app-ui.md §自然幅の上限は切り上げる). The loader's implicit width is the name's.
            Layout.maximumWidth: origSeat.implicitWidth
            sourceComponent: CutName {
                text: nameCell.origPath
                pixelSize: Theme.fontMd
                // The name the file has now is the subject; where it came from is context, and wears the colour the
                // rest of the app gives context (§テキスト: author / 日時 / 短縮ハッシュ). `textMuted` is the disabled
                // signal (§無効), and a second meaning for it cannot be read apart.
                color: Theme.textSecondary
            }
        }
        Loader {
            id: renameSeat
            active: origSeat.active
            visible: origSeat.active
            Layout.preferredWidth: renameSeat.item ? renameSeat.item.inkWidth + Theme.spaceXs * 2 : 0
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
            sourceComponent: Item {
                readonly property real inkWidth: renameMark.inkWidth
                NavIcon {
                    id: renameMark
                    anchors.centerIn: parent
                    kind: "arrow"
                    // Beside a word, so a step under the row's own mark, with the line taken down by the same ratio
                    // (app-ui.md §語の隣に立つ印).
                    width: Theme.iconSm
                    height: Theme.iconSm
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    tint: Theme.textSecondary
                }
            }
        }
        CutName {
            id: newName
            Layout.fillWidth: true
            text: nameCell.name
            pixelSize: Theme.fontMd
            weight: nameCell.weight
            color: nameCell.folder ? Theme.textSecondary : nameCell.tone
            // The pieces go while the whole name stands over them; the cut goes on being worked out, so this column
            // keeps the width it had and nothing beside it moves.
            inked: nameCell.whole === ""
            // The name in full, in this very place. **Anchored rather than laid out**: a child of the cut name is
            // outside the row's layout, so a name that wraps hangs below the row instead of growing it sideways,
            // and the row takes that overhang on itself (`NavItemDelegate.nameOverflow`). Built only while it is
            // asked for — a field per row on screen is the heap the rows are measured by (the seat's rule).
            Loader {
                id: wholeSeat
                active: nameCell.whole !== ""
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: wholeSeat.item ? wholeSeat.item.implicitHeight : 0
                sourceComponent: CardText {
                    text: nameCell.whole
                    pixelSize: Theme.fontMd
                    weight: nameCell.weight
                    color: nameCell.folder ? Theme.textSecondary : nameCell.tone
                }
            }
            // Built only on a marked row (the seat's rule).
            Loader {
                active: nameCell.marked
                // Clamped: a cut name ends at the column's own right edge, and the mark belongs to the name that is on
                // screen. Half a gap back into the name's own trailing bearing, so it reads as part of the
                // word.
                x: Math.min(newName.inkWidth - Theme.spaceXs / 2,
                            newName.width - Theme.iconSm)
                y: 0
                sourceComponent: NavIcon {
                    kind: "bang"
                    tint: nameCell.markTint
                    width: Theme.iconSm
                    height: Theme.iconSm
                }
            }
        }
    }
}

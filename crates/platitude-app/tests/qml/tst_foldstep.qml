import QtQuick
// For the attached ToolTip alone.
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// How far a fold steps a row in, and how far a name stands from its mark, read off the marks themselves in every
// list that folds (デザイン規約 §余白): one depth down, a chevron's box starts under the middle of the chevron above
// it; a change mark's box ends where a chevron's does; and a name starts as far past its mark's box in the file
// lists as in the sidebar. The lists draw their marks on different grids, so a step or a placement written per list
// would agree with itself in one and drift in another.
Item {
    id: root
    width: 400
    height: 300
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    // A commit's changed files (`DetailsPane.fileList`): a folder, a file inside it, a file beside it.
    ListModel {
        id: changed
        ListElement { path: "src"; name: "src"; change: ""; folder: true; collapsed: false; depth: 0 }
        ListElement { path: "src/a.rs"; name: "a.rs"; change: "M"; folder: false; collapsed: false; depth: 1 }
        ListElement { path: "b.rs"; name: "b.rs"; change: "M"; folder: false; collapsed: false; depth: 0 }
    }
    AppListView {
        id: changedList
        width: 200
        height: 100
        model: changed
        verticalBar: PaneScrollBar {}
        delegate: FileRowDelegate {
            listWidth: changedList.width
        }
    }

    // The working tree's rows (`WipBucketPane`), which take the delegate's own step.
    Column {
        y: 100
        width: 200
        NavItemDelegate {
            id: wipFolder
            index: 0
            name: "src"
            full: "unstaged:src"
            oid_hex: ""
            change: ""
            bucket: "unstaged"
            orig_path: "src"
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: true
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: 200
            height: Theme.rowHeight
            kindHint: "wt"
        }
        NavItemDelegate {
            id: wipInside
            index: 1
            name: "a.rs"
            full: "src/a.rs"
            oid_hex: ""
            change: "M"
            bucket: "unstaged"
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 1
            folder: false
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: 200
            height: Theme.rowHeight
            kindHint: "wt"
        }
        NavItemDelegate {
            id: wipBeside
            index: 2
            name: "b.rs"
            full: "b.rs"
            oid_hex: ""
            change: "M"
            bucket: "unstaged"
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: false
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: 200
            height: Theme.rowHeight
            kindHint: "wt"
        }
    }

    // The sidebar's rows: a narrower seat and no change marks, so what stands one depth down is another chevron.
    Column {
        y: 200
        width: 200
        NavItemDelegate {
            id: refFolder
            index: 0
            name: "team"
            full: "team"
            oid_hex: ""
            change: ""
            bucket: ""
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            folder: true
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: 200
            height: Theme.rowHeight
        }
        NavItemDelegate {
            id: refInside
            index: 1
            name: "api"
            full: "team/api"
            oid_hex: ""
            change: ""
            bucket: ""
            orig_path: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 1
            folder: true
            eol_mark: false
            ahead: 0
            behind: 0
            listWidth: 200
            height: Theme.rowHeight
        }
    }

    TestCase {
        name: "FoldStep"
        when: windowShown

        /// The mark of that kind a row is showing — found by walking the row, so the answer is the icon on screen and
        /// not a number the row was handed.
        function markOf(item, kind) {
            if (item.kind === kind && item.visible && item.width > 0)
                return item
            for (let i = 0; i < item.children.length; ++i) {
                const hit = markOf(item.children[i], kind)
                if (hit !== null)
                    return hit
            }
            return null
        }
        /// The name a row draws, found the way the marks are: the first item down the row holding that text.
        function nameOf(item, text) {
            if (item.text === text && item.visible && item.width > 0)
                return item
            for (let i = 0; i < item.children.length; ++i) {
                const hit = nameOf(item.children[i], text)
                if (hit !== null)
                    return hit
            }
            return null
        }
        /// Where a mark's box stands across the scene: its middle, and its right edge taken off the middle — a
        /// chevron is drawn turned, and a turned item's own origin is no longer its left.
        function middleOf(mark) {
            return mark.mapToItem(root, mark.width / 2, mark.height / 2).x
        }
        function rightOf(mark) {
            return middleOf(mark) + mark.width / 2
        }
        /// How far a row's name starts past its mark's box.
        function nameGap(row, kind, text) {
            const mark = markOf(row, kind)
            verify(mark !== null, "the row shows its mark")
            const name = nameOf(row, text)
            verify(name !== null, "the row shows its name")
            return name.mapToItem(root, 0, 0).x - rightOf(mark)
        }

        function test_one_fold_down_starts_under_the_chevrons_middle_data() {
            return [
                { tag: "a commit's changed files", folder: () => changedList.itemAtIndex(0),
                  inside: () => changedList.itemAtIndex(1), beside: () => changedList.itemAtIndex(2), mark: "pen" },
                { tag: "the working tree's files", folder: () => wipFolder, inside: () => wipInside,
                  beside: () => wipBeside, mark: "pen" },
                { tag: "the sidebar's refs", folder: () => refFolder, inside: () => refInside, beside: null,
                  mark: "chevron" },
            ]
        }
        function test_one_fold_down_starts_under_the_chevrons_middle(row) {
            tryVerify(() => row.folder() !== null && row.inside() !== null, undefined, "the rows are built")
            const fold = markOf(row.folder(), "chevron")
            verify(fold !== null, "the folder row shows its chevron")
            const inside = markOf(row.inside(), row.mark)
            verify(inside !== null, "the row inside it shows its mark")
            // Whatever grid the mark is drawn on, its box ends where a chevron's one fold down would: a box as wide
            // as the chevron's, starting under the middle of the chevron above.
            compare(rightOf(inside) - fold.width, middleOf(fold),
                    "a chevron's box ending where the mark one fold down ends starts under the chevron's middle")
            if (row.beside === null)
                return
            const beside = markOf(row.beside(), row.mark)
            verify(beside !== null, "the row beside the folder shows its mark")
            compare(rightOf(beside), rightOf(fold), "at one depth, a change mark's box ends where a chevron's does")
        }

        // Each grid leaves the same air on the right of its box, so one distance from the box to the name is one
        // distance from the ink to the name.
        function test_a_name_starts_as_far_past_its_mark_in_every_list() {
            tryVerify(() => changedList.itemAtIndex(2) !== null, undefined, "the rows are built")
            const sidebar = nameGap(refFolder, "chevron", "team")
            compare(nameGap(changedList.itemAtIndex(2), "pen", "b.rs"), sidebar,
                    "a changed file's name starts as far past its mark's box as a sidebar name does")
            compare(nameGap(wipBeside, "pen", "b.rs"), sidebar,
                    "and so does a working tree file's")
        }
    }
}

import QtQuick
import QtTest
import platitude.ui

// What a worktree's rename box says of the folder name typed into it (`Words.worktreeRenameRefused`), from the place
// `NavSectionModel.renamedWorktreeFor` answered: what Windows keeps out of a name names the part it keeps, and a name
// with nothing in its way says nothing (デザイン規約 §worktree の名前を変える).
Item {
    TestCase {
        name: "FolderWords"

        function place(taken, kept) {
            return { path: "C:/work/x", name: "x", taken: taken, kept: kept === undefined ? "" : kept }
        }

        function test_what_windows_keeps_out_names_the_part_it_keeps() {
            compare(Words.worktreeRenameRefused("why?", place("character", "?")),
                    "Not a folder name on Windows — ? is not allowed")
            compare(Words.worktreeRenameRefused("con.txt", place("device", "con")),
                    "Not a folder name on Windows — con is kept for a device")
            compare(Words.worktreeRenameRefused("dot.", place("trailing", ".")),
                    "Not a folder name on Windows — it cannot end in a dot")
        }

        /// Where the second click opens no box: what cannot be done, then the folder (デザイン規約 §worktree の名前を変える
        /// 「箱を開かないもの」).
        function test_a_worktree_the_click_cannot_rename_says_why() {
            compare(Words.worktreeRenameWithheld("locked", "vaulted"), "Cannot rename — vaulted is locked")
            compare(Words.worktreeRenameWithheld("gone", "gone"), "Cannot rename — gone is missing")
            compare(Words.worktreeRenameWithheld("box", "detached"), "", "a box opens: nothing to say")
        }

        function test_the_tree_mark_stands_only_on_a_folder_in_the_way() {
            for (const taken of ["character", "device", "trailing"]) {
                const p = place(taken, "?")
                compare(Words.worktreeRenameMark(Words.worktreeRenameRefused("x", p), p), "", taken)
            }
            const there = place("folder")
            compare(Words.worktreeRenameMark(Words.worktreeRenameRefused("x", there), there), "x")
        }

        function test_a_free_name_says_nothing() {
            compare(Words.worktreeRenameRefused("loose", place("")), "")
            compare(Words.worktreeRenameRefused("a/b", undefined), "Not a folder name")
            compare(Words.worktreeRenameRefused(" ", place("")), "A name is needed")
        }
    }
}

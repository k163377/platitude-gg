import QtQuick
import QtTest
import platitude.ui

// What the discard log says of an entry (`RecoverEntries`, 破棄記録仕様.md §3–§4): what the operation did, what each
// part took, and what the restore makes of it — and the time, the moment first.
Item {
    id: root

    RecoverEntries {
        id: shown
    }

    TestCase {
        name: "RecoverWords"

        /// A test that names worktrees through `calledOf` hands the default back however it ends.
        function cleanup() {
            shown.calledOf = path => null
        }

        function part(restore, fields) {
            const made = { restore: restore, name: "", with: "", look: "commit", tip: "1a2b3c4d", remote: "",
                           lost: 0, files: 0, notCopied: 0 }
            for (const key in fields)
                made[key] = fields[key]
            return made
        }
        function row(kind, name, parts, remote) {
            return { kind: kind, name: name, remote: remote === undefined ? "" : remote, worktree: "", at: 0, lost: 0,
                     parts: parts }
        }

        function test_the_title_says_what_was_done() {
            const one = [part("branch", { name: "x", lost: 2 })]
            compare(shown.titleOf(row("reset", "main", one)), "Reset main")
            compare(shown.titleOf(row("amend", "main", one)), "Amended main")
            compare(shown.titleOf(row("rebase", "topic, base", one)), "Rebased topic, base")
            compare(shown.titleOf(row("moved", "spike", one)), "Moved spike")
            compare(shown.titleOf(row("left-detached", "c", one)), "Left a detached HEAD")
            compare(shown.titleOf(row("reset", "", one)), "Reset a detached HEAD", "moved on no branch")
            compare(shown.titleOf(row("amend", "", one)), "Amended a detached HEAD")
            compare(shown.titleOf(row("rebase", "", one)), "Rebased a detached HEAD")
            compare(shown.titleOf(row("deleted", "3.x", one)), "Deleted 3.x")
            compare(shown.titleOf(row("deleted", "3.x", one, "origin")), "Deleted 3.x here and on origin")
            compare(shown.titleOf(row("deleted-remote-branch", "feature", one, "origin")), "Deleted origin/feature")
            compare(shown.titleOf(row("force-pushed", "main", one, "origin")), "Force-pushed origin/main")
            // The kind is the mark's to say: a tag and a worktree with no word for what they are.
            compare(shown.titleOf(row("deleted-tag", "v1", one)), "Deleted v1")
            compare(shown.titleOf(row("deleted-tag", "v1", one, "origin")), "Deleted v1 here and on origin")
            compare(shown.titleOf(row("deleted-remote-tag", "v1", one, "origin")), "Deleted v1 on origin")
            compare(shown.titleOf(row("force-pushed-tag", "v1", one, "origin")), "Force-pushed v1 to origin")
            compare(shown.titleOf(row("removed-worktree", "side", one)), "Removed side")
            compare(shown.titleOf(row("dropped-stash", "On main: x", one)), "Dropped a stash")
            compare(shown.titleOf(row("popped-stash", "On main: x", one)), "Popped a stash")
        }

        /// The left menu's row opens on the kind's mark, so it names every thing by its own name — a stash by its
        /// message — and says the rest as the sentence does.
        function test_a_row_of_the_list_names_the_thing_itself() {
            const one = [part("branch", { name: "x", lost: 2 })]
            compare(shown.listTitleOf(row("dropped-stash", "On main: x", one)), "Dropped On main: x")
            compare(shown.listTitleOf(row("popped-stash", "On main: x", one)), "Popped On main: x")
            for (const kind of ["reset", "deleted-tag", "removed-worktree", "left-detached"])
                compare(shown.listTitleOf(row(kind, "side", one)), shown.titleOf(row(kind, "side", one)), kind)
        }

        /// The row's names are code with no mark of their own — the row's mark says the kind.
        function test_a_row_of_the_list_sets_its_names_as_code_with_no_mark() {
            const one = [part("branch", { name: "x", lost: 2 })]
            const stash = shown.listMarksOf(row("dropped-stash", "On main: x", one))
            compare(stash.map(mark => [mark[0], mark[1], mark[3]]), [[8, "", 10]],
                    "the stash's message after `Dropped `")
            const both = shown.listMarksOf(row("deleted-tag", "v1", one, "origin"))
            compare(both.map(mark => [mark[0], mark[1], mark[3]]), [[8, "", 2], [23, "", 6]], "the tag and the remote")
            compare(shown.listMarksOf(row("left-detached", "c", one)), [], "a detached HEAD has no name")
        }

        function test_thrown_away_work_is_counted_by_its_paths_and_goes_back_where_it_was() {
            const work = part("changes", { name: "repo", look: "uncommitted", files: 2 })
            const discarded = row("discarded", "main", [work])
            compare(shown.titleOf(discarded), "Discarded 2 files")
            const entry = shown.entryOf(discarded)
            compare(entry.mark, "minus", "the deleted file's mark")
            compare(entry.parts[0].text, "2 uncommitted")
            compare(entry.parts[0].restore, "back into repo")
            work.files = 1
            compare(shown.titleOf(row("deleted-untracked", "main", [work])), "Deleted 1 untracked file")
            work.notCopied = 1
            compare(shown.takenOf(row("discarded", "main", [work]), work), "1 uncommitted, 1 not copied")
            compare(shown.titleOf(row("discarded", "main", [work])), "Discarded 2 files", "uncopied files count")
            work.files = 0
            compare(shown.takenOf(row("discarded", "main", [work]), work), "1 not copied", "nothing copied")
        }

        /// A worktree is named as everything else names it (`calledOf`): the repository's own by the branch it has
        /// out, behind the house; another by its folder, behind the tree.
        function test_a_worktree_is_named_as_everything_else_names_it() {
            shown.calledOf = path => path === "C:/work/repo" ? { name: "main", home: true }
                                                             : { name: "side", home: false }
            const into = part("changes", { name: "repo", look: "uncommitted", files: 2, worktreePath: "C:/work/repo" })
            const work = shown.entryOf(row("discarded", "main", [into]))
            compare(work.parts[0].restore, "back into main", "the repository's own, by its branch")
            compare(work.parts[0].restoreMarks.map(mark => mark[1]), ["home"])

            const ownRow = row("reset", "main", [part("branch", { name: "main", lost: 1 })])
            ownRow.worktree = "repo"
            ownRow.worktreePath = "C:/work/repo"
            const own = shown.entryOf(ownRow)
            compare(own.worktree, "main", "read from a linked worktree: in the repository's own")
            compare(own.worktreeMark, "home")
            const sideRow = row("reset", "topic", [part("branch", { name: "topic", lost: 1 })])
            sideRow.worktree = "side"
            sideRow.worktreePath = "C:/work/side"
            compare(shown.entryOf(sideRow).worktreeMark, "tree")
            compare(shown.entryOf(row("reset", "main", [part("branch", { name: "main", lost: 1 })])).worktree, "",
                    "this worktree's own is not said")
        }

        /// The title's names each wear their kind's mark: the branch, the tag, the folder; a remote's branch is one
        /// name behind the one cloud; a rebase's names each behind their own. A restore's line names its branch and
        /// what it tracks behind theirs.
        function test_the_title_s_names_wear_their_kinds_marks() {
            const one = [part("branch", { name: "x", lost: 1 })]
            const kindsOf = r => shown.titleMarksOf(r).map(mark => mark[1])
            compare(kindsOf(row("reset", "main", one)), ["branch"])
            compare(kindsOf(row("deleted-tag", "v1.0", one)), ["tag"])
            compare(kindsOf(row("removed-worktree", "side", one)), ["tree"])
            compare(kindsOf(row("rebase", "topic, base", one)), ["branch", "branch"], "each behind its own")
            for (const kind of ["force-pushed", "deleted-remote-branch"]) {
                const pushed = row(kind, "topic", one, "origin")
                const marks = shown.titleMarksOf(pushed)
                compare(marks.map(mark => mark[1]), ["remote"], kind + ": one cloud")
                compare(shown.titleOf(pushed).substr(marks[0][0], marks[0][3]), "origin/topic", kind + ": one name")
            }
            const own = part("branch", { name: "spike", with: "origin/spike", lost: 2 })
            const restore = shown.entryOf(row("deleted", "spike", [own], "origin")).parts[0]
            compare(restore.restoreMarks.map(mark => mark[1]), ["branch", "remote"])
            compare(restore.restoreMarks.map(mark => restore.restore.substr(mark[0], mark[3])),
                    ["spike", "origin/spike"])
        }

        function test_each_part_says_what_it_took_and_what_comes_back() {
            const own = part("branch", { name: "spike", with: "origin/spike", lost: 2 })
            const theirs = part("branch", { name: "spike-pgg-restored", remote: "origin", lost: 1 })
            const entry = shown.entryOf(row("deleted", "spike", [own, theirs], "origin"))
            compare(entry.parts[0].text, "2 commits")
            compare(entry.parts[0].restore, "as spike, tracking origin/spike")
            compare(entry.parts[0].mark, "branch")
            compare(entry.parts[1].text, "1 commit")
            compare(entry.parts[1].restore, "as spike-pgg-restored")
            compare(entry.parts[1].mark, "remote", "a remote's branch wears the remote's mark")

            const merged = part("branch", { name: "done", lost: 0 })
            compare(shown.takenOf(row("deleted", "done", [merged]), merged), "No commits of its own")
            const tag = part("tag", { name: "v1" })
            compare(shown.entryOf(row("deleted-tag", "v1", [tag])).parts[0].text, "At 1a2b3c4d")
            compare(shown.restoredAs(tag), "as v1")
            const stash = part("stash", { name: "On main: try", look: "stash", files: 1 })
            compare(shown.restoredAs(stash), "back to the stashes")
            const worktree = part("worktree", { name: "C:/work/side", with: "side-work" })
            compare(shown.restoredAs(worktree), "as a worktree there, on side-work")
        }

        // Where it was taken, in the mark (P3-確認事項 §破棄記録と復元): a remote's tag alone wears the cloud on the
        // shoulder a step down; here and on a remote at once is the pair; a remote's branch is the cloud itself.
        function test_the_mark_says_where_it_was_taken() {
            const tag = part("tag", { name: "v1" })
            const theirTag = part("tag", { name: "v1", remote: "origin" })
            const branch = part("branch", { name: "x", lost: 1 })
            const cases = [
                [row("deleted-tag", "v1", [tag]), "tag", false, false, Theme.refTag],
                [row("deleted-remote-tag", "v1", [theirTag], "origin"), "tag", true, false, Theme.refTagDim],
                [row("force-pushed-tag", "v1", [theirTag], "origin"), "tag", true, false, Theme.refTagDim],
                [row("deleted-tag", "v1", [tag], "origin"), "tag", false, true, Theme.refTag],
                [row("deleted", "x", [branch], "origin"), "branch", false, true, Theme.accent],
                [row("deleted", "x", [branch]), "branch", false, false, Theme.accent],
                [row("deleted-remote-branch", "x", [branch], "origin"), "remote", false, false, Theme.textSecondary],
                [row("force-pushed", "x", [branch], "origin"), "remote", false, false, Theme.textSecondary]
            ]
            for (const [given, mark, badged, paired, tint] of cases) {
                const entry = shown.entryOf(given)
                compare(entry.mark, mark, given.kind + " " + given.remote)
                compare(entry.badged, badged, given.kind + " " + given.remote + " badged")
                compare(entry.paired, paired, given.kind + " " + given.remote + " paired")
                compare(Qt.colorEqual(entry.tint, tint), true, given.kind + " " + given.remote + " tint")
            }
            // A part is one thing: the remote's tag of a delete that reached two objects wears the cloud, the one
            // here does not.
            const both = shown.entryOf(row("deleted-tag", "v1", [tag, theirTag], "origin"))
            compare(both.parts[0].badged, false)
            compare(both.parts[1].badged, true)
            compare(Qt.colorEqual(both.parts[1].tint, Theme.refTagDim), true)
        }

        function test_the_moment_comes_first_then_how_long_ago() {
            const now = 1790000000
            compare(shown.ago(now - 30, now), "Just now")
            compare(shown.ago(now - 120, now), "2 min ago")
            compare(shown.whenWords(now - 120, now), Words.stamp(now - 120) + " (2 min ago)")
        }

        function test_the_rows_are_worded_whole() {
            shown.rows = [row("reset", "main", [part("branch", { name: "main-pgg-restored", lost: 2 })])]
            compare(shown.entries.length, 1)
            compare(shown.entries[0].title, "Reset main")
            compare(shown.entries[0].parts[0].text, "2 commits")
            shown.rows = []
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What a row of the left panel says about the ref it names: **the table** (`NavFacts`) asked through the real row
// that asks it (`NavItemDelegate.gatherFacts`), and **the ink** — the measure the row draws on its own line and the
// lines it opens underneath — read off the items the row actually built.
//
// **The two halves are separate on purpose.** `factsLines` is what the table came to; the drawn lines are what the
// row handed to the part that draws them. A row that gathered its answers and never passed them on has the first
// and not the second, and a test that stopped at the first would not know.
//
// **The models are the only thing standing in.** A row's answers come off its section's model, the branches' and the
// working copies' (`NavFacts.answers`), and those are the application's own Rust-backed types, which these runs are
// not given (`xtask::qmltest`). The row already takes all three as plain slots, so an object written here answers
// the lookups and everything above them is the product's: which lookup each section makes, what it does with the
// answer, which lines come out and in what order. **Three separate stand-ins**, so a row asking the wrong model is a
// wrong answer here rather than the same one twice.
//
// **The one answer that cannot be here is `heldBy`.** The folder a working copy stands in is the leaf of the path
// git prints, and that cut is the application singleton's (`GitFacts.pathLeaf`) — the table asks for it only where
// a copy is holding the row's branch, which is what lets everything else be read here. That one stays with the
// headless run that has the real singleton (`nav-open remote:0:topic-a --preset tracked-elsewhere`).
Item {
    id: root
    width: 320
    height: 240

    /// What each stand-in answers, written per case. Each model answers with a value of its own, so a lookup put to
    /// the wrong one of the three comes back wrong rather than right by coincidence.
    property string sectionUpstream: ""
    property string sectionTracked: ""
    property var sectionCarried: []
    property string branchUpstream: ""
    property string branchGone: ""
    property int branchAhead: 0
    property int branchBehind: 0
    property string copyHolding: ""

    /// The row's own section — BRANCHES asks it for the reading, REMOTES for the branch that reads the row, TAGS for
    /// the remotes carrying the name.
    QtObject {
        id: sections
        function upstreamOf(name) { return root.sectionUpstream }
        function trackedBy(full) { return root.sectionTracked }
        function tagRemotes(name, against) { return root.sectionCarried }
    }
    /// The branches' section, which a working copy's row asks about the branch it holds — and nobody else asks at
    /// all.
    QtObject {
        id: branches
        function upstreamOf(name) { return root.branchUpstream }
        function upstreamGoneOf(name) { return root.branchGone }
        function aheadOf(name) { return root.branchAhead }
        function behindOf(name) { return root.branchBehind }
    }
    /// The working copies', the one section holding the list of them.
    QtObject {
        id: copies
        function worktreeHolding(name) { return root.copyHolding }
    }

    /// A row of whichever section a case is about. Built per case rather than reconfigured, because `gatherFacts`
    /// freezes what it read and a row carrying the last case's answers would pass on them.
    Component {
        id: rowMaker
        NavItemDelegate {
            index: 0
            oid_hex: ""
            orig_name: ""
            is_head: false
            has_remote: false
            only_remote: false
            has_pr: false
            depth: 0
            eol_mark: false
            listWidth: 260
            opensFacts: true
            sectionModel: sections
            branchesModel: branches
            worktreesModel: copies
        }
    }

    /// One row, off the fields a case names and the empty ones every row has. **Closed** — a key of its own and
    /// nobody's key open — so a case that wants the lines drawn says so ([`openRow`]); the row's own line and the
    /// lines under it both draw a measure, and a walk over an open row would find two.
    function rowOf(fields) {
        const all = { "name": "", "full": "", "bucket": "", "change": "", "orig_path": "",
                      "folder": false, "ahead": 0, "behind": 0, "pushRemote": "" }
        for (const key in fields)
            all[key] = fields[key]
        const row = rowMaker.createObject(root, all)
        row.rowKey = all.kindHint + ":" + all.full
        return row
    }

    /// That row with its lines out, the way the panel opens one: the row gathers them itself off the key moving
    /// (`NavItemDelegate.onFactsOpenChanged`), so nothing here calls the table by hand.
    function openRow(row) {
        row.openKey = row.rowKey
        return row
    }

    /// Every item the row built, itself included, that answers to `tell` — **found by walking what is there** rather
    /// than through a handle the product would carry for this file alone. A name it stops finding is a test that
    /// fails on the count, which is what the walk is held to.
    function drawn(item, tell) {
        let found = item[tell] === undefined ? [] : [item]
        const kids = item.children
        if (kids !== undefined)
            for (let i = 0; i < kids.length; i++)
                found = found.concat(root.drawn(kids[i], tell))
        return found
    }

    /// The measures standing anywhere under `item` (`HeadTrack`). **Its own legs carry the `hue` too** — they are
    /// handed it so that one expression wears one colour — so the legs are what the second field tells apart.
    function measuresUnder(item) {
        return root.drawn(item, "hue").filter(found => found.turn === undefined)
    }

    /// The lines a row opened (`NavFactLine` — the only part with a `markTint`; the card holding them has a `noted`
    /// of its own, so that field would name it too).
    function linesUnder(item) {
        return root.drawn(item, "markTint")
    }

    TestCase {
        name: "NavFacts"
        when: windowShown

        // ---- BRANCHES ------------------------------------------------

        /// The everyday branch row: the reading it is measured against, drawn from the section's own lookup, with the
        /// counts the row already carries as roles — a fetch moves those, so they are never asked for.
        function test_a_branch_names_the_reading_it_is_measured_against() {
            root.sectionUpstream = "origin/feature/topic-a"
            const row = root.rowOf({ "kindHint": "branch", "name": "feature/topic-a",
                                     "full": "feature/topic-a", "ahead": 2, "behind": 3 })
            verify(row.gatherFacts(), "the row has a name to open on")
            compare(row.factsName, "feature/topic-a")
            compare(row.factsUpstream, "origin/feature/topic-a")
            compare(row.factsGone, false)
            compare(row.factsAhead, 2, "the row's own role, not the branches' lookup")
            compare(row.factsBehind, 3)
            compare(row.factsLines.length, 1)
            compare(row.factsLines[0].mark, "remote")
            compare(row.factsLines[0].text, "origin/feature/topic-a")
            compare(row.factsLines[0].tone, Theme.textSecondary)
            row.destroy()
        }

        /// A reading git can no longer reach. **The state is a role and not a lookup** — `upstreamOf` answers with
        /// refs that are there, so a row reading only that would lose the `[gone]` branch entirely — and the line
        /// wears the warning whole.
        function test_a_branch_whose_reading_is_gone_wears_the_state_whole() {
            root.sectionUpstream = ""
            const row = root.rowOf({ "kindHint": "branch", "name": "release-1.2",
                                     "full": "release-1.2", "bucket": "origin/release-1.2" })
            verify(row.gatherFacts())
            compare(row.factsGone, true)
            compare(row.factsUpstream, "origin/release-1.2")
            compare(row.factsLines.length, 1)
            compare(row.factsLines[0].text, "origin/release-1.2 is gone")
            compare(row.factsLines[0].tone, Theme.warning)
            compare(row.factsLines[0].markTint, Theme.warning)
            row.destroy()
        }

        /// The line a branch opens with while somebody else has it out. **Asked of the table itself**, which is the
        /// one case in this file that cannot go through a row: what the row would answer with is the leaf of the
        /// path git prints, and that cut is the application singleton's (the note at the top). What the call pins is
        /// the colour — the mark names a row of WORKTREES, so it wears that section's own colour the way the line
        /// naming a branch wears BRANCHES' (規約 §ref の種別), and no picture is judged on it.
        function test_the_copy_holding_a_branch_is_named_in_the_worktrees_colour() {
            const lines = NavFacts.lines("branch", { "heldBy": "topic", "upstream": "", "gone": false,
                                                     "ahead": 0, "behind": 0 })
            compare(lines.length, 1, "the copy, and no reading to measure against")
            compare(lines[0].mark, "tree")
            compare(lines[0].markTint, Theme.success, "the WORKTREES section's own colour")
            compare(lines[0].text, "topic")
            compare(lines[0].tone, Theme.textSecondary, "the name is the quiet half of the line")
        }

        /// A branch with nothing beside it opens on its name alone: a line nobody filled is left out rather than
        /// drawn empty.
        function test_a_branch_with_nothing_beside_it_opens_on_its_name() {
            root.sectionUpstream = ""
            root.copyHolding = ""
            const row = root.rowOf({ "kindHint": "branch", "name": "solo", "full": "solo" })
            verify(row.gatherFacts())
            compare(row.factsName, "solo")
            compare(row.factsLines.length, 0)
            row.destroy()
        }

        // ---- REMOTES -------------------------------------------------

        /// A remote-tracking row leads with the branch that reads it — the section's own reverse lookup — and the
        /// measure that branch's row draws is the one already on this row, read from the far side.
        function test_a_remote_row_leads_with_the_branch_that_reads_it() {
            root.sectionTracked = "main"
            root.copyHolding = ""
            const row = root.rowOf({ "kindHint": "remote", "name": "origin/main",
                                     "full": "origin/main", "ahead": 1 })
            verify(row.gatherFacts())
            compare(row.factsLocal, "main")
            compare(row.factsLines.length, 1)
            compare(row.factsLines[0].mark, "branch")
            compare(row.factsLines[0].text, "main")
            compare(row.factsLines[0].ahead, 1)
            row.destroy()
        }

        /// The remote's own row is the one leaf of that section which does not open: what it has to say is the role
        /// it holds, which is a sentence rather than a name.
        function test_the_remotes_own_row_does_not_open() {
            const row = root.rowOf({ "kindHint": "remote", "name": "origin", "full": "origin",
                                     "folder": true })
            row.remoteNames = ["origin"]
            verify(!row.expands, "the remote itself has no lines")
            row.destroy()
        }

        // ---- WORKTREES -----------------------------------------------

        /// A working copy says of the branch it holds what that branch's own row would — **asked of the branches'
        /// section**, because a copy's own list carries no such roles, so its row's `ahead` / `behind` are not the
        /// answer. It goes on showing the folder it is named by, and the path it stands at is a field of its own.
        function test_a_working_copy_says_of_its_branch_what_that_branchs_row_would() {
            root.sectionUpstream = "origin/not-this-one"
            root.branchUpstream = "origin/feature/topic-a"
            root.branchGone = ""
            root.branchAhead = 4
            root.branchBehind = 5
            const row = root.rowOf({ "kindHint": "worktree", "name": "topic",
                                     "full": "C:/copies/topic", "bucket": "feature/topic-a",
                                     "ahead": 9, "behind": 9 })
            verify(row.gatherFacts())
            compare(row.factsName, "topic", "the folder it is named by")
            compare(row.factsPath, "C:/copies/topic")
            compare(row.factsBranch, "feature/topic-a")
            compare(row.factsUpstream, "origin/feature/topic-a", "the branches' answer, not the section's")
            compare(row.factsAhead, 4, "and its counts, not the row's own roles")
            compare(row.factsBehind, 5)
            compare(row.factsLines.length, 2)
            compare(row.factsLines[0].mark, "branch")
            compare(row.factsLines[0].text, "feature/topic-a")
            compare(row.factsLines[0].markTint, Theme.accent, "the BRANCHES section's own colour")
            compare(row.factsLines[1].text, "origin/feature/topic-a")
            row.destroy()
        }

        /// A folder git can no longer find draws a line naming the warning; a lock draws none — the padlock on the
        /// row is the whole of what a lock has to say here, and the words somebody gave `git worktree lock` are
        /// theirs.
        function test_a_folder_that_is_gone_draws_a_line_and_a_lock_draws_none() {
            root.branchUpstream = ""
            root.branchGone = ""
            const gone = root.rowOf({ "kindHint": "worktree", "name": "gone",
                                      "full": "C:/copies/gone", "bucket": "gone/branch",
                                      "change": "PRUNABLE", "orig_path": "C:/copies/gone" })
            verify(gone.gatherFacts())
            compare(gone.factsState, "PRUNABLE")
            compare(gone.factsLines.length, 2, "the branch it holds, then the warning")
            compare(gone.factsLines[1].mark, "", "a warning names nothing, so it draws no mark")
            compare(gone.factsLines[1].text, "Folder is gone")
            compare(gone.factsLines[1].tone, Theme.warning)
            gone.destroy()

            const locked = root.rowOf({ "kindHint": "worktree", "name": "hotfix",
                                        "full": "C:/copies/hotfix", "bucket": "hotfix/urgent",
                                        "change": "LOCKED", "orig_path": "somebody's reason" })
            verify(locked.gatherFacts())
            compare(locked.factsState, "LOCKED")
            compare(locked.factsWhy, "somebody's reason")
            compare(locked.factsLines.length, 1, "the branch it holds, and nothing about the lock")
            locked.destroy()
        }

        /// The repository's own copy is **named by the branch it has out** (`NavRowBody.homeCopy`), so the column
        /// at the far end of the row stays empty and the line that would name that branch a second time is left
        /// out of what the row opens. **On no branch it falls back to its folder** — the main checkout can sit on
        /// a detached HEAD like any other, and it is the one row with nothing else to draw.
        function test_the_repositorys_own_copy_is_named_by_the_branch_it_has_out() {
            root.branchUpstream = ""
            root.branchGone = ""
            const home = root.rowOf({ "kindHint": "worktree", "name": "repo", "full": "C:/work/repo",
                                      "bucket": "main", "change": "MAIN" })
            const named = root.drawn(home, "cutAt")
            compare(named.length, 1, "the name, and no column at the far end of the row")
            compare(named[0].text, "main", "the branch it has out")
            verify(home.gatherFacts())
            compare(home.factsLines.length, 0, "and no line under it saying that branch again")
            home.destroy()

            const loose = root.rowOf({ "kindHint": "worktree", "name": "repo", "full": "C:/work/repo",
                                       "bucket": "", "change": "MAIN" })
            const fallen = root.drawn(loose, "cutAt")
            compare(fallen.length, 1)
            compare(fallen[0].text, "repo", "its folder, where it holds no branch to be named by")
            loose.destroy()

            const linked = root.rowOf({ "kindHint": "worktree", "name": "topic", "full": "C:/copies/topic",
                                        "bucket": "feature/topic-a" })
            const both = root.drawn(linked, "cutAt")
            compare(both.length, 2, "every other copy is named by its folder and shows its branch beside it")
            compare(both[0].text, "topic")
            compare(both[1].text, "feature/topic-a")
            linked.destroy()
        }

        // ---- TAGS ----------------------------------------------------

        /// A tag has no namespace, so one row stands for every side of the name and what it folds is the list of who
        /// out there has it: one line to a remote, in the order the section answered. **Which of them stands apart is
        /// a colour and a supplement**, and the reading it is apart *from* is named in the words — a line saying
        /// only "another commit" would leave the reader to guess another commit than what.
        function test_a_tag_opens_one_line_per_remote_carrying_its_name() {
            root.sectionCarried = [{ "remote": "origin", "apart": false }, { "remote": "fork", "apart": true }]
            const row = root.rowOf({ "kindHint": "tag", "name": "v1.5", "full": "v1.5",
                                     "pushRemote": "origin" })
            verify(row.gatherFacts())
            compare(row.factsRemotes, "origin,fork")
            compare(row.factsApart, "fork")
            compare(row.factsAgainst, "origin")
            compare(row.factsLines.length, 2)
            compare(row.factsLines[0].text, "origin")
            compare(row.factsLines[0].tone, Theme.textSecondary)
            compare(row.factsLines[0].note, "", "the one standing where the reading is says nothing extra")
            compare(row.factsLines[1].text, "fork")
            compare(row.factsLines[1].tone, Theme.warning)
            compare(row.factsLines[1].note, "On another commit than origin")
            row.destroy()
        }

        /// A tag nobody else carries opens on nothing, as a branch with no reading does — the name in full is reason
        /// enough to open.
        function test_a_tag_nobody_else_carries_opens_on_nothing() {
            root.sectionCarried = []
            const row = root.rowOf({ "kindHint": "tag", "name": "v2.0-local", "full": "v2.0-local",
                                     "pushRemote": "origin" })
            verify(row.gatherFacts())
            compare(row.factsRemotes, "")
            compare(row.factsLines.length, 0)
            row.destroy()
        }

        // ---- what every section shares -------------------------------

        /// A fold parent is the shape of the names below it and not a ref, so it has no line to open: it says its
        /// own name in the shared tooltip instead.
        function test_a_fold_parent_opens_nothing() {
            root.sectionUpstream = "origin/anything"
            const row = root.rowOf({ "kindHint": "branch", "name": "backend",
                                     "full": "team/backend", "folder": true })
            verify(!row.expands, "a folder has nothing to open")
            row.destroy()
        }

        // ---- the ink ------------------------------------------------

        /// The ⇅ a branch row draws on its own line, **through the row that draws it**: the seat is
        /// `NavRowBody`'s, the part in it is the product's `HeadTrack`, and what is read back is what a reader
        /// sees — which legs are drawn, the number under each and the colour the pair wears.
        ///
        /// **Not the numbers the row was handed.** The seat and the part are two bindings apart from the row's
        /// roles, and either of them reading the wrong one draws a measure that is not the row's while every value
        /// on the row stays right.
        function test_the_measure_a_branch_row_draws_on_its_own_line_data() {
            return [
                { tag: "level with it", ahead: 0, behind: 0, legs: [], hue: Theme.textSecondary },
                { tag: "ahead only", ahead: 2, behind: 0, legs: ["2"], hue: Theme.textSecondary },
                { tag: "behind only", ahead: 0, behind: 3, legs: ["3"], hue: Theme.warning },
                { tag: "both", ahead: 2, behind: 3, legs: ["2", "3"], hue: Theme.warning },
            ]
        }

        function test_the_measure_a_branch_row_draws_on_its_own_line(data) {
            const row = root.rowOf({ "kindHint": "branch", "name": "feature/topic-a",
                                     "full": "feature/topic-a",
                                     "ahead": data.ahead, "behind": data.behind })
            const measures = root.measuresUnder(row)
            if (data.legs.length === 0) {
                // A branch level with its reading, or with none to measure against, has nothing to count — and an
                // empty seat is a column the other rows are indented past.
                compare(measures.length, 0, "no measure is drawn at all")
                row.destroy()
                return
            }
            compare(measures.length, 1, "one measure on the row's line")
            const track = measures[0]
            verify(track.parent.visible, "the seat the row gave it is showing")
            compare(track.ahead, data.ahead, "counting what the row carries")
            compare(track.behind, data.behind)
            compare(track.hue, data.hue, "the pair is one expression, so one colour covers it")
            // The legs themselves, and the number drawn under each: a leg with nothing to say is not drawn, and
            // what is drawn is never a zero.
            const legs = root.drawn(track, "turn").filter(leg => leg.visible)
            compare(legs.length, data.legs.length, "the legs a reader can see")
            for (let i = 0; i < legs.length; i++) {
                const ink = root.drawn(legs[i], "text")
                compare(ink.length, 1, "one number to a leg")
                compare(ink[0].text, data.legs[i], "the number as it is drawn")
                compare(ink[0].color, data.hue, "in the colour the pair wears")
            }
            row.destroy()
        }

        /// And the lines the row opens are the ones it draws. **The table is read above; this is the wiring**:
        /// between them stand the row's own seat, the card it loads and the repeater in it, and a row that gathered
        /// its answers and handed on nothing draws an empty card while `factsLines` goes on saying two.
        ///
        /// The measure on the first of them is the same part the row's own line uses, handed the branch's counts —
        /// which is the one place in this panel where a line counts something.
        function test_the_lines_a_row_opens_are_the_ones_it_draws() {
            root.branchUpstream = "origin/feature/topic-a"
            root.branchGone = ""
            root.branchAhead = 4
            root.branchBehind = 5
            const row = root.openRow(root.rowOf({
                "kindHint": "worktree", "name": "topic", "full": "C:/copies/topic",
                "bucket": "feature/topic-a" }))
            verify(row.factsOpen, "the row has itself open")
            verify(row.factsItem !== null, "and the card is built under it")
            compare(row.factsLines.length, 2, "what the table came to")

            const lines = root.linesUnder(row.factsItem)
            compare(lines.length, 2, "and what the card drew")
            compare(lines[0].text, "feature/topic-a", "the branch it holds leads")
            compare(lines[0].markTint, Theme.accent, "in the BRANCHES section's own colour")
            compare(lines[1].text, "origin/feature/topic-a", "then the reading that branch is measured against")

            const measures = root.measuresUnder(lines[0])
            compare(measures.length, 1, "the branch's own counts ride the line naming it")
            compare(measures[0].ahead, 4)
            compare(measures[0].behind, 5)
            compare(measures[0].hue, Theme.warning)
            compare(root.measuresUnder(lines[1]).length, 0, "and a reading counts nothing")
            row.destroy()
        }

        /// A line whose words the table left empty is not drawn at all — the card is measured over the lines it was
        /// handed, so one drawn blank would be a row of nothing the reader is asked to look past.
        function test_a_row_with_nothing_to_add_draws_no_lines() {
            root.sectionUpstream = ""
            root.copyHolding = ""
            const row = root.openRow(root.rowOf({ "kindHint": "branch", "name": "solo", "full": "solo" }))
            verify(row.factsOpen)
            verify(row.factsItem !== null)
            compare(row.factsLines.length, 0)
            compare(root.linesUnder(row.factsItem).length, 0)
            row.destroy()
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What a row of the left panel says about the ref it names: the table (`NavFacts`) asked through the real row
// (`NavItemDelegate.gatherFacts`), and the ink — the measure on the row's own line and the lines it opens — read off
// the items the row built. The two are checked apart: a row that gathered its answers and never passed them on has
// `factsLines` right and draws nothing.
//
// Only the models stand in: the Rust-backed section models are not given to these runs (`xtask::qmltest`), and the
// row takes all three as plain slots (`NavFacts.answers`). Three separate stand-ins, so a row asking the wrong model
// gets a wrong answer rather than the same one.
//
// `heldBy` cannot be read here: the folder name is cut by the application singleton (`GitFacts.pathLeaf`). It stays
// with the headless run that has it (`nav-open remote:1:topic-a --preset tracked-elsewhere`).
Item {
    id: root
    width: 320
    height: 240

    property string sectionUpstream: ""
    property string sectionTracked: ""
    property var sectionCarried: []
    property bool sectionHereApart: false
    /// Who the right reading belongs to (`NavSectionModel.tagWeighedAgainst`); empty where the remotes disagree.
    property string sectionBy: "origin"
    /// What the last `tagApartAt` was asked, so a case can say which commit and which reference were weighed.
    property var apartAsked: null
    property string branchUpstream: ""
    property string branchGone: ""
    property int branchAhead: 0
    property int branchBehind: 0
    property string holdingWorktree: ""
    property string sectionReadingAt: "a1"
    property string branchAt: "b2"
    property string branchReadingAt: "c3"

    /// The row's own section — BRANCHES asks it for the reading, REMOTES for the branch that reads the row, TAGS for
    /// the remotes carrying the name.
    QtObject {
        id: sections
        function upstreamOf(name) { return root.sectionUpstream }
        function upstreamOidOf(name) { return root.sectionReadingAt }
        function trackedBy(full) { return root.sectionTracked }
        function tagRemotes(name, against) { return root.sectionCarried }
        function tagWeighedAgainst(name, against) { return root.sectionBy }
        function tagApartAt(name, oid, against) {
            root.apartAsked = { "name": name, "oid": oid, "against": against }
            return root.sectionHereApart
        }
    }
    /// A worktree's row asks it about the branch it holds; a remote-tracking row, where the branch reading it
    /// stands.
    QtObject {
        id: branches
        function upstreamOf(name) { return root.branchUpstream }
        function upstreamOidOf(name) { return root.branchReadingAt }
        function upstreamGoneOf(name) { return root.branchGone }
        function aheadOf(name) { return root.branchAhead }
        function behindOf(name) { return root.branchBehind }
        function oidOfName(name) { return root.branchAt }
    }
    QtObject {
        id: worktrees
        function worktreeHolding(name) { return root.holdingWorktree }
        function headOfWorktree(path) { return "" }
    }

    /// Built per case rather than reconfigured: `gatherFacts` freezes what it read, so a reused row would pass on the
    /// last case's answers.
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
            worktreesModel: worktrees
        }
    }

    /// One row, off the fields a case names over empty defaults. Closed, so a case that wants the lines drawn says so
    /// (`openRow`): the row's own line and its lines both draw a measure, and a walk over an open row finds two.
    function rowOf(fields) {
        const all = { "name": "", "full": "", "bucket": "", "change": "", "orig_path": "",
                      "folder": false, "ahead": 0, "behind": 0, "pushRemote": "" }
        for (const key in fields)
            all[key] = fields[key]
        const row = rowMaker.createObject(root, all)
        row.rowKey = all.kindHint + ":" + all.full
        return row
    }

    /// Opens the row the way the panel does, by moving the key; the row gathers its lines itself
    /// (`NavItemDelegate.onFactsOpenChanged`).
    function openRow(row) {
        row.openKey = row.rowKey
        return row
    }

    /// Every item under `item`, itself included, that has the property `tell` — found by walking the tree rather than
    /// through a handle the product would carry for this file alone. A property renamed away fails on the count.
    function drawn(item, tell) {
        let found = item[tell] === undefined ? [] : [item]
        const kids = item.children
        if (kids !== undefined)
            for (let i = 0; i < kids.length; i++)
                found = found.concat(root.drawn(kids[i], tell))
        return found
    }

    /// The `HeadTrack`s under `item`. Its legs carry `hue` too, so `turn` (legs only) filters them out.
    function measuresUnder(item) {
        return root.drawn(item, "hue").filter(found => found.turn === undefined)
    }

    /// The `NavFactLine`s under `item` — the only part with a `markTint` (`noted` would match the card too).
    function linesUnder(item) {
        return root.drawn(item, "markTint")
    }

    TestCase {
        name: "NavFacts"
        when: windowShown

        // ---- BRANCHES ------------------------------------------------

        /// The reading comes from the section's own lookup; the counts are the row's own roles, never looked up.
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
            compare(row.factsLines[0].to.key, "remote:origin/feature/topic-a")
            compare(row.factsLines[0].to.oid, "a1", "the row's own section's answer")
            row.destroy()
        }

        /// The line then clicks as part of the row.
        function test_a_reading_with_no_commit_goes_nowhere() {
            root.sectionUpstream = "origin/feature/topic-a"
            root.sectionReadingAt = ""
            const row = root.rowOf({ "kindHint": "branch", "name": "feature/topic-a", "full": "feature/topic-a" })
            verify(row.gatherFacts())
            compare(row.factsLines.length, 1)
            compare(row.factsLines[0].to, null)
            root.sectionReadingAt = "a1"
            row.destroy()
        }

        /// The gone state is a role, not a lookup: `upstreamOf` answers only refs that exist, so a row reading only
        /// that would lose the `[gone]` reading entirely.
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
            compare(row.factsLines[0].to, null, "a reading git cannot reach is not anywhere to go")
            row.destroy()
        }

        /// Asked of the table itself: a row here cannot reach `heldBy` (the note at the top). What it pins is the
        /// colour — the mark names a WORKTREES row, so it wears that section's colour (デザイン規約 §ref の種別), and no
        /// picture judges it.
        function test_the_worktree_holding_a_branch_is_named_in_the_worktrees_colour() {
            const held = NavFacts.place("worktree", "C:/worktrees/topic", "d4")
            const lines = NavFacts.lines("branch", { "heldBy": "topic", "heldTo": held, "upstream": "",
                                                     "gone": false, "ahead": 0, "behind": 0 })
            compare(lines.length, 1, "the worktree, and no reading to measure against")
            compare(lines[0].mark, "tree")
            compare(lines[0].markTint, Theme.success, "the WORKTREES section's own colour")
            compare(lines[0].text, "topic")
            compare(lines[0].tone, Theme.textSecondary, "the name is the quiet half of the line")
            // Keyed by the path (`NavList.keyOf`): two worktrees can share a leaf.
            compare(lines[0].to.key, "worktree:C:/worktrees/topic")
            compare(lines[0].to.oid, "d4")
        }

        /// A line nobody filled is left out rather than drawn empty.
        function test_a_branch_with_nothing_beside_it_opens_on_its_name() {
            root.sectionUpstream = ""
            root.holdingWorktree = ""
            const row = root.rowOf({ "kindHint": "branch", "name": "solo", "full": "solo" })
            verify(row.gatherFacts())
            compare(row.factsName, "solo")
            compare(row.factsLines.length, 0)
            row.destroy()
        }

        // ---- REMOTES -------------------------------------------------

        /// The branch comes from the section's reverse lookup; its measure is this row's own, read from the far side.
        function test_a_remote_row_leads_with_the_branch_that_reads_it() {
            root.sectionTracked = "main"
            root.holdingWorktree = ""
            const row = root.rowOf({ "kindHint": "remote", "name": "origin/main",
                                     "full": "origin/main", "ahead": 1 })
            verify(row.gatherFacts())
            compare(row.factsLocal, "main")
            compare(row.factsLines.length, 1)
            compare(row.factsLines[0].mark, "branch")
            compare(row.factsLines[0].text, "main")
            compare(row.factsLines[0].ahead, 1)
            compare(row.factsLines[0].to.key, "branch:main")
            compare(row.factsLines[0].to.oid, "b2", "asked of the branches' section")
            row.destroy()
        }

        /// What the remote's own row has to say is its role — a sentence, not a name — so it opens no lines.
        function test_the_remotes_own_row_does_not_open() {
            const row = root.rowOf({ "kindHint": "remote", "name": "origin", "full": "origin",
                                     "folder": true })
            row.remoteNames = ["origin"]
            verify(!row.expands, "the remote itself has no lines")
            row.destroy()
        }

        // ---- WORKTREES -----------------------------------------------

        /// Asked of the branches' section: a worktree's own list carries no upstream roles, so its row's `ahead` /
        /// `behind` are not the answer.
        function test_a_worktree_says_of_its_branch_what_that_branchs_row_would() {
            root.sectionUpstream = "origin/not-this-one"
            root.branchUpstream = "origin/feature/topic-a"
            root.branchGone = ""
            root.branchAhead = 4
            root.branchBehind = 5
            const row = root.rowOf({ "kindHint": "worktree", "name": "topic",
                                     "full": "C:/worktrees/topic", "bucket": "feature/topic-a",
                                     "ahead": 9, "behind": 9 })
            verify(row.gatherFacts())
            compare(row.factsName, "topic", "the folder it is named by")
            compare(row.factsPath, "C:/worktrees/topic")
            compare(row.factsBranch, "feature/topic-a")
            compare(row.factsUpstream, "origin/feature/topic-a", "the branches' answer, not the section's")
            compare(row.factsAhead, 4, "and its counts, not the row's own roles")
            compare(row.factsBehind, 5)
            compare(row.factsLines.length, 2)
            compare(row.factsLines[0].mark, "branch")
            compare(row.factsLines[0].text, "feature/topic-a")
            compare(row.factsLines[0].markTint, Theme.accent, "the BRANCHES section's own colour")
            compare(row.factsLines[1].text, "origin/feature/topic-a")
            compare(row.factsLines[0].to.key, "branch:feature/topic-a")
            compare(row.factsLines[0].to.oid, "b2")
            compare(row.factsLines[1].to.key, "remote:origin/feature/topic-a")
            compare(row.factsLines[1].to.oid, "c3", "the branches' answer, not the section's")
            row.destroy()
        }

        /// A lock draws no line: the padlock on the row says it, and the reason given to `git worktree lock` is not
        /// shown.
        function test_a_folder_that_is_gone_draws_a_line_and_a_lock_draws_none() {
            root.branchUpstream = ""
            root.branchGone = ""
            const gone = root.rowOf({ "kindHint": "worktree", "name": "gone",
                                      "full": "C:/worktrees/gone", "bucket": "gone/branch",
                                      "change": "PRUNABLE", "orig_path": "C:/worktrees/gone" })
            verify(gone.gatherFacts())
            compare(gone.factsState, "PRUNABLE")
            compare(gone.factsLines.length, 2, "the branch it holds, then the warning")
            compare(gone.factsLines[1].mark, "", "a warning names nothing, so it draws no mark")
            compare(gone.factsLines[1].text, "Folder is gone")
            compare(gone.factsLines[1].tone, Theme.warning)
            gone.destroy()

            const locked = root.rowOf({ "kindHint": "worktree", "name": "hotfix",
                                        "full": "C:/worktrees/hotfix", "bucket": "hotfix/urgent",
                                        "change": "LOCKED", "orig_path": "somebody's reason" })
            verify(locked.gatherFacts())
            compare(locked.factsState, "LOCKED")
            compare(locked.factsWhy, "somebody's reason")
            compare(locked.factsLines.length, 1, "the branch it holds, and nothing about the lock")
            locked.destroy()
        }

        /// The main checkout is named by its branch (`NavRowBody.homeWorktree`), so the far column stays empty and no
        /// line repeats the branch. On a detached HEAD it falls back to its folder.
        function test_the_repositorys_own_worktree_is_named_by_the_branch_it_has_out() {
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

            const linked = root.rowOf({ "kindHint": "worktree", "name": "topic", "full": "C:/worktrees/topic",
                                        "bucket": "feature/topic-a" })
            const both = root.drawn(linked, "cutAt")
            compare(both.length, 2, "every other worktree is named by its folder and shows its branch beside it")
            compare(both[0].text, "topic")
            compare(both[1].text, "feature/topic-a")
            linked.destroy()
        }

        // ---- TAGS ----------------------------------------------------

        /// One line per remote carrying the name, in the section's order. The one standing apart gets a colour and a
        /// note naming what it is apart from — "another commit" alone leaves the reader to guess than what.
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
            compare(row.factsLines[1].note, "Differs from origin")
            // A remote is not a row of this panel, so neither line goes anywhere.
            compare(row.factsLines[0].to, null)
            compare(row.factsLines[1].to, null)
            row.destroy()
        }

        /// The copy here is weighed by the same test as the remotes, with its own commit against the same reference,
        /// and its line is the row's own name: that name wears the warning while open, and a rest on it says why in the
        /// carriers' words. Closed, the row says where the tag is and nothing more.
        function test_a_tag_whose_copy_here_stands_apart_wears_the_warning_on_its_own_name() {
            root.sectionCarried = [{ "remote": "fork", "apart": true }, { "remote": "origin", "apart": false }]
            root.sectionHereApart = true
            root.apartAsked = null
            const row = root.rowOf({ "kindHint": "tag", "name": "v1.5", "full": "v1.5",
                                     "pushRemote": "origin", "oid_hex": "e5" })
            const name = root.drawn(row, "cutAt")
            compare(name.length, 1)
            compare(name[0].color, Theme.textPrimary, "closed: a tag held here")

            root.openRow(row)
            verify(row.factsOpen)
            compare(root.apartAsked.name, "v1.5")
            compare(root.apartAsked.oid, "e5", "the copy here, by the commit it stands on")
            compare(root.apartAsked.against, "origin", "against the remote the lines are read against")
            compare(row.factsHereApart, true)
            compare(row.factsNameNote, "Differs from origin")
            compare(row.factsLines[0].note, row.factsNameNote, "one sentence for every holder apart")
            compare(name[0].color, Theme.warning, "open: the name is that copy's line")

            row.openKey = ""
            compare(name[0].color, Theme.textPrimary, "and plain again once the row closes")
            row.destroy()
            root.sectionHereApart = false
        }

        /// The note names whoever the right reading belongs to: every remote agreeing on it where the one this window
        /// acts on is silent, and nobody where the remotes disagree — then every holder is apart and says so.
        function test_a_tags_note_names_who_decides_or_that_nobody_does_data() {
            return [
                { tag: "the reference", by: "origin", note: "Differs from origin" },
                { tag: "remotes that agree", by: "fork, mirror", note: "Differs from fork, mirror" },
                { tag: "remotes that disagree", by: "", note: "Remotes disagree" },
            ]
        }
        function test_a_tags_note_names_who_decides_or_that_nobody_does(data) {
            root.sectionCarried = [{ "remote": "fork", "apart": true }, { "remote": "mirror", "apart": true }]
            root.sectionBy = data.by
            root.sectionHereApart = true
            const row = root.rowOf({ "kindHint": "tag", "name": "v3", "full": "v3", "pushRemote": "origin",
                                     "oid_hex": "e5" })
            verify(row.gatherFacts())
            compare(row.factsLines[0].note, data.note)
            compare(row.factsLines[1].note, data.note)
            compare(row.factsNameNote, data.note, "the copy here says the same")
            compare(row.factsBy, data.by.split(", ").join(","), "and the report lists them as the others")
            row.destroy()
            root.sectionBy = "origin"
            root.sectionHereApart = false
        }

        /// Each carrier's line names its remote on its own, so a menu asked for on it acts on that remote
        /// (`NavRowFacts.lineMenu`). What the row's own section passes up is read off the signal the row raises.
        function test_a_menu_asked_on_a_carriers_line_names_that_remote() {
            root.sectionCarried = [{ "remote": "fork", "apart": false }, { "remote": "mirror", "apart": false }]
            const row = root.openRow(root.rowOf({ "kindHint": "tag", "name": "v3", "full": "v3",
                                                  "pushRemote": "origin", "oid_hex": "e5" }))
            verify(row.factsItem !== null)
            compare(row.factsLines[1].aim, "mirror")
            let aimed = null
            row.refMenuRequested.connect((name, full, oidHex, aim) => aimed = aim)
            row.factsItem.lineMenu(1)
            compare(aimed, "mirror", "the line's remote")
            row.factsItem.lineMenu(-1)
            compare(aimed, "", "and none off the lines")
            row.destroy()
        }

        /// The same through the hand: the line is the one the right press went down on — not the last left press's,
        /// and not the first line for a block no left press has touched.
        function test_a_right_click_names_the_line_it_went_down_on() {
            root.sectionCarried = [{ "remote": "fork", "apart": false }, { "remote": "mirror", "apart": false }]
            const row = root.openRow(root.rowOf({ "kindHint": "tag", "name": "v3", "full": "v3",
                                                  "pushRemote": "origin", "oid_hex": "e5" }))
            const facts = row.factsItem
            verify(facts !== null)
            let aimed = null
            row.refMenuRequested.connect((name, full, oidHex, aim) => aimed = aim)
            const onMirror = facts.lineWordsMiddle(1)
            facts.handPressed(Qt.RightButton, onMirror.x, onMirror.y)
            facts.handClicked(Qt.RightButton, Qt.NoModifier)
            compare(aimed, "mirror", "the line under the right press")

            const onFork = facts.lineWordsMiddle(0)
            facts.handPressed(Qt.LeftButton, onFork.x, onFork.y)
            facts.handReleased()
            facts.handPressed(Qt.RightButton, onMirror.x, onMirror.y)
            facts.handClicked(Qt.RightButton, Qt.NoModifier)
            compare(aimed, "mirror", "a left press on another line before it changes nothing")
            row.destroy()
        }

        /// A menu raised on the open row is one about it, which keeps it open (`factsMenuAsked`) — from the row's own
        /// line under a real right-click as from its lines; a closed row's menu is any other menu. Said once per menu
        /// (the lines go through the same press), and before the ask.
        function test_a_menu_raised_on_the_open_row_keeps_it_open() {
            root.sectionUpstream = ""
            root.holdingWorktree = ""
            const row = root.openRow(root.rowOf({ "kindHint": "branch", "name": "solo", "full": "solo",
                                                  "oid_hex": "e5" }))
            verify(row.factsItem !== null)
            let menus = 0
            let kept = 0
            // What had been said when the ask went up: the menu's opening is what reads the note
            // (`SidebarRowGestures.onMenuOpenChanged`), so a note after the ask comes too late.
            let keptAtAsk = -1
            row.refMenuRequested.connect(() => {
                menus++
                keptAtAsk = kept
            })
            row.factsMenuAsked.connect(() => kept++)
            mouseClick(row, row.width / 2, row.lineHeight / 2, Qt.RightButton)
            compare(menus, 1, "the menu, off the row's own line")
            compare(kept, 1, "raised on the open row")
            compare(keptAtAsk, 1, "said before the ask")
            row.factsItem.lineMenu(-1)
            compare(menus, 2, "the menu, off its lines")
            compare(kept, 2, "raised on the open row, once")
            row.openKey = ""
            mouseClick(row, row.width / 2, row.lineHeight / 2, Qt.RightButton)
            compare(menus, 3, "a closed row's menu")
            compare(kept, 2, "keeps nothing open")
            row.destroy()
        }

        /// A name only a remote has has no copy here to weigh, so nothing is asked and its grey stays.
        function test_a_tag_only_a_remote_has_weighs_no_copy_here() {
            root.sectionCarried = [{ "remote": "fork", "apart": false }]
            root.sectionHereApart = true
            root.apartAsked = null
            const row = root.openRow(root.rowOf({ "kindHint": "tag", "name": "v0.9-theirs", "full": "v0.9-theirs",
                                                  "pushRemote": "origin", "only_remote": true, "oid_hex": "f6" }))
            verify(row.factsOpen)
            compare(root.apartAsked, null, "the remote's reading is not a copy here")
            compare(row.factsHereApart, false)
            compare(row.factsNameNote, "")
            compare(root.drawn(row, "cutAt")[0].color, Theme.textSecondary)
            row.destroy()
            root.sectionHereApart = false
        }

        /// The name in full is reason enough to open, as for a branch with no reading.
        function test_a_tag_nobody_else_carries_opens_on_nothing() {
            root.sectionCarried = []
            const row = root.rowOf({ "kindHint": "tag", "name": "v2.0-local", "full": "v2.0-local",
                                     "pushRemote": "origin" })
            verify(row.gatherFacts())
            compare(row.factsRemotes, "")
            compare(row.factsLines.length, 0)
            row.destroy()
        }

        // ---- where a press goes -------------------------------------

        /// Two ways to one commit are one target (`NavFacts.joined`): a line going where the row stands is the row's
        /// own, and one going where the line above goes shares its band. Asked of the table, since a row here cannot
        /// carry the worktree's line (the note at the top).
        function test_lines_going_to_one_place_are_one_target() {
            const own = NavFacts.joined([{ "to": { "key": "worktree:C:/c", "oid": "r0" } },
                                         { "to": { "key": "remote:origin/x", "oid": "u1" } }], "r0")
            compare(own[0].to, null, "the worktree standing where the row does is the row's own")
            compare(own[1].to.oid, "u1", "the reading elsewhere goes there")
            compare(own[1].withAbove, false, "and has a band of its own")

            const pair = NavFacts.joined([{ "to": { "key": "branch:x", "oid": "c2" } },
                                          { "to": { "key": "worktree:C:/c", "oid": "c2" } }], "r1")
            compare(pair[0].withAbove, false)
            compare(pair[1].withAbove, true, "the worktree holding that branch shares its band")

            const apart = NavFacts.joined([{ "to": null }, { "to": { "key": "remote:origin/x", "oid": "u1" } }], "r1")
            compare(apart[1].withAbove, false, "a line going nowhere joins nothing to it")
        }

        /// The same rule through real rows: a worktree, and a remote-tracking row level with its branch.
        function test_a_line_standing_where_its_row_does_is_the_rows_own() {
            root.branchUpstream = "origin/feature/topic-a"
            root.branchGone = ""
            const worktree = root.rowOf({ "kindHint": "worktree", "name": "topic", "full": "C:/worktrees/topic",
                                          "bucket": "feature/topic-a", "oid_hex": root.branchAt })
            verify(worktree.gatherFacts())
            compare(worktree.factsLines[0].text, "feature/topic-a")
            compare(worktree.factsLines[0].to, null, "the branch the worktree holds is where the worktree stands")
            compare(worktree.factsLines[1].to.oid, "c3", "the reading goes elsewhere")
            worktree.destroy()

            root.sectionTracked = "main"
            root.holdingWorktree = ""
            const level = root.rowOf({ "kindHint": "remote", "name": "origin/main", "full": "origin/main",
                                       "oid_hex": root.branchAt })
            verify(level.gatherFacts())
            compare(level.factsLines[0].to, null, "a branch level with its reading stands where the row does")
            level.destroy()
        }

        // ---- what every section shares -------------------------------

        /// A fold parent is not a ref, so it has no lines; its name goes to the shared tooltip instead.
        function test_a_fold_parent_opens_nothing() {
            root.sectionUpstream = "origin/anything"
            const row = root.rowOf({ "kindHint": "branch", "name": "backend",
                                     "full": "team/backend", "folder": true })
            verify(!row.expands, "a folder has nothing to open")
            row.destroy()
        }

        // ---- the ink ------------------------------------------------

        /// The ⇅ on a branch row's own line, read off what is drawn (`NavRowBody`'s seat, `HeadTrack`) and not off the
        /// row's roles: the two are two bindings apart, and either could read the wrong one.
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
                // An empty seat would be a column the other rows are indented past.
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
            // A leg with nothing to count is not drawn, so no drawn number is a zero.
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

        /// The wiring from `factsLines` to the ink: the row's seat, the card it loads and the repeater in it. The
        /// branch line carries the same `HeadTrack` as the row's own line, with the branch's counts.
        function test_the_lines_a_row_opens_are_the_ones_it_draws() {
            root.branchUpstream = "origin/feature/topic-a"
            root.branchGone = ""
            root.branchAhead = 4
            root.branchBehind = 5
            const row = root.openRow(root.rowOf({
                "kindHint": "worktree", "name": "topic", "full": "C:/worktrees/topic",
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

        /// The main worktree opens no line for its branch, so the branch's measure rides the row's own line — and only
        /// while open: closed, the row says what it is and nothing more.
        function test_the_repositorys_own_worktree_draws_its_branchs_measure_only_while_open() {
            root.branchUpstream = "origin/main"
            root.branchGone = ""
            root.branchAhead = 6
            root.branchBehind = 0
            const home = root.rowOf({ "kindHint": "worktree", "name": "repo", "full": "C:/work/repo",
                                      "bucket": "main", "change": "MAIN" })
            compare(root.measuresUnder(home).length, 0, "closed, no measure")

            root.openRow(home)
            verify(home.factsOpen)
            const measures = root.measuresUnder(home)
            compare(measures.length, 1, "open, one measure")
            verify(measures[0].parent.visible, "on the row's own line")
            compare(measures[0].ahead, 6, "counting the branch the worktree has out")
            compare(measures[0].behind, 0)

            home.openKey = ""
            verify(!home.factsOpen)
            compare(root.measuresUnder(home).length, 0, "and gone again once the row closes")
            home.destroy()

            // Detached, the worktree is named by its folder and has no branch to measure.
            const loose = root.openRow(root.rowOf({ "kindHint": "worktree", "name": "repo", "full": "C:/work/repo",
                                                    "bucket": "", "change": "MAIN" }))
            compare(root.measuresUnder(loose).length, 0)
            loose.destroy()
        }

        /// The card is sized over the lines it was handed, so a blank one would be an empty row to look past.
        function test_a_row_with_nothing_to_add_draws_no_lines() {
            root.sectionUpstream = ""
            root.holdingWorktree = ""
            const row = root.openRow(root.rowOf({ "kindHint": "branch", "name": "solo", "full": "solo" }))
            verify(row.factsOpen)
            verify(row.factsItem !== null)
            compare(row.factsLines.length, 0)
            compare(root.linesUnder(row.factsItem).length, 0)
            row.destroy()
        }
    }
}

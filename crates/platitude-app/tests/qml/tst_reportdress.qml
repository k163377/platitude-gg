import QtQuick
import QtTest
import platitude.ui

// The dress a report wears: the three answers `Words` gives for one kind, and what a real `NoticeBar` does with them
// (デザイン規約 §答えの要らない報せ). Which report arrives is the page's and the headless verbs prove it; the switch
// over the kind is asked here, every arm in one process (rules-refs/app-ui.md「その条件表は QML 部品テストへ移し」).
Item {
    id: root
    width: 420
    height: 200

    /// A forge's answer, wider than the bar, with the rule it enforced at the end — the part a cut bar drops.
    readonly property string longReason: "GH006: Protected branch update failed for refs/heads/main."
                                         + " Cannot force-push to a protected branch."
                                         + " Review this repository's branch protection rules."

    NoticeBar {
        id: bar
        width: root.width
    }
    SignalSpy {
        id: acknowledged
        target: bar
        signalName: "acknowledged"
    }
    /// Somewhere else for the focus to go, since the bar hands the pill the focus as it opens.
    Item { id: elsewhere }

    TestCase {
        name: "ReportDress"
        when: windowShown

        function init() {
            bar.open = false
            bar.yieldsEscape = false
            acknowledged.clear()
            elsewhere.forceActiveFocus()
            // Fully shut first: a case reading the height mid-animation would read the previous one.
            tryVerify(() => bar.shut)
        }

        /// A copy of the page's door (`RepoPage.showReport`): heading, our second line where nobody outside spoke,
        /// tone. It proves nothing about the real door — one dropping an answer leaves every case here green; that
        /// claim is carried by `report-tone half-rename`.
        function dress(kind, remote, name, reason) {
            bar.label = Words.writeReported(kind, remote, name)
            bar.detail = reason !== "" ? reason : Words.writeReportedWhy(kind, remote, name)
            bar.tone = Words.reportTone(kind)
            bar.markWord = Words.reportNamesCopy(kind) ? name : ""
            bar.open = true
        }

        /// A working copy the removal left standing: the gesture's colour, and the copy's name in the heading behind
        /// the tree mark every copy's name wears (デザイン規約 §ref の種別「名前の印」). Both kinds, and a report naming
        /// no copy, since a bar that marks every heading passes the first half alone.
        function test_a_copy_git_kept_is_named_behind_the_tree_mark() {
            compare(Words.reportTone("worktree-kept"), "warning")
            compare(Words.reportTone("worktree-half"), "warning")
            compare(Words.writeReported("worktree-kept", "", "topic"), "topic was not removed")
            compare(Words.writeReported("worktree-half", "", "topic"), "topic was not removed completely")
            compare(Words.writeReportedWhy("worktree-kept"), "It has uncommitted changes.")
            verify(!Words.reportNamesCopy("rename"), "a branch's name wears no tree")

            // One space of room, where this font needs two: pinned, or rich text drops it at the block's start.
            compare(Words.roomInSentence("topic was not removed", 0, 1), "&nbsp;topic was not removed")

            dress("worktree-kept", "", "topic", "")
            tryVerify(() => bar.settled && bar.markShown && bar.markInk > 0)
            // The name opens the sentence, where rich text drops a lone leading space: the room must still be there,
            // as wide as the mark's ink, or the mark is drawn over the name's first letter.
            verify(bar.markSeat.x >= bar.markInk, "the name was pushed by the whole mark: " + bar.markSeat.x
                   + " < " + bar.markInk)

            dress("update", "origin", "main", "")
            tryVerify(() => bar.settled)
            verify(!bar.markShown, "no copy named, no mark")
        }

        /// The mark goes in front of the name where the sentence says it — last in a tip, first in a heading — and
        /// never into a word that only holds the same letters (`work` and `tree` in `worktree`).
        function test_the_tree_mark_stands_before_the_name_and_nowhere_else() {
            compare(Words.markSeatIn("Checked out in another worktree — tree", "tree", true), 34)
            compare(Words.markSeatIn("Checked out in another worktree — work", "work", true), 34)
            compare(Words.markSeatIn("topic was not removed", "topic", false), 0)
            compare(Words.markSeatIn("removed was not removed", "removed", false), 0, "a heading's name is its first")
            compare(Words.markSeatIn("Checked out in another worktree — rework", "work", true), -1,
                    "the letters, not the word")
            compare(Words.markSeatIn("Den Ordner topic gibt es", "topic", true), -1, "a translation that moved it: none")
            compare(Words.markSeatIn("anything", "", true), -1)
            compare(Words.roomInSentence("anything", -1, 2), "", "no seat, no room: drawn as it came")
        }

        /// A copy that was not made: the same yellow, the same mark, in the heading the folder would have been
        /// (デザイン規約 §作業コピーを作る). git's words come as the reason; where nothing was asked of
        /// git, the second line is the screen's.
        function test_a_copy_not_made_is_named_behind_the_tree_mark_in_yellow() {
            for (const kind of ["worktree-not-added", "worktree-folder-taken"]) {
                compare(Words.reportTone(kind), "warning", kind)
                verify(Words.reportNamesCopy(kind), kind)
                compare(Words.writeReported(kind, "", "hotfix-patch"), "hotfix-patch was not created", kind)
            }
            compare(Words.writeReportedWhy("worktree-folder-taken"), "The folder is not empty.")
            dress("worktree-not-added", "", "hotfix-patch", "fatal: a branch named 'hotfix/patch' already exists")
            tryVerify(() => bar.settled && bar.markShown)
            compare(bar.detail, "fatal: a branch named 'hotfix/patch' already exists")
        }

        /// Both arms, because either alone passes against a bar painted one colour.
        function test_the_gesture_that_is_still_going_wears_the_other_colour() {
            compare(Words.reportTone("rename"), "warning")
            compare(Words.reportTone("half-rename"), "warning")
            compare(Words.reportTone("update"), "danger")
            compare(Words.reportTone("delete"), "danger")
            compare(Words.reportTone("outdated"), "danger")
            compare(Words.reportTone("stale-stage"), "danger")
            compare(Words.reportTone("conflicted-part"), "danger")
            compare(Words.reportTone("tip-moved"), "danger")
            compare(Words.reportTone("op-standing"), "danger")
            compare(Words.reportTone(""), "danger", "a kind nobody spelled still says so in colour")
        }

        /// The subject is whoever decided: the far side for the two it decided, the thing that did not move for the
        /// rest.
        function test_the_heading_names_what_did_not_happen() {
            compare(Words.writeReported("delete", "origin", "v1.0"), "origin would not delete v1.0")
            compare(Words.writeReported("update", "origin", "main"), "origin would not update main")
            compare(Words.writeReported("outdated", "origin", "main"), "main was not sent to origin")
            compare(Words.writeReported("rename", "origin", "main"), "main was not renamed")
            compare(Words.writeReported("half-rename", "origin", "main"), "The rename did not finish")
            compare(Words.writeReported("stale-stage", "", ""), "Nothing was staged")
            compare(Words.writeReported("stale-unstage", "", ""), "Nothing was unstaged")
            compare(Words.writeReported("stale-discard", "", ""), "Nothing was discarded")
            compare(Words.writeReported("conflicted-part", "", ""), "Nothing was taken from this file")
            compare(Words.writeReported("", "", ""), "The commit was not made")
        }

        /// One heading over all seven refusals (five the plan's own, two the world moving after the press); the line
        /// underneath says which.
        function test_every_refused_rewrite_says_the_same_thing_did_not_happen() {
            const kinds = ["across-merge", "off-branch", "fold-first", "unfetched-base", "drop-all",
                           "tip-moved", "op-standing"]
            for (const kind of kinds)
                compare(Words.writeReported(kind, "origin", "main"), "The history was not rewritten", kind)
        }

        /// An empty second line is an answer: the page puts what came across there instead.
        function test_the_second_line_is_ours_only_where_nobody_outside_spoke() {
            const owed = ["stale-stage", "stale-unstage", "stale-discard", "conflicted-part", "half-rename",
                          "across-merge", "off-branch", "fold-first", "unfetched-base", "drop-all",
                          "tip-moved", "op-standing",
                          // Refused by git before the far side heard it: nobody to quote, and git's own text is
                          // terminal advice.
                          "outdated", "moved", "moved-delete", "tag-elsewhere"]
            for (const kind of owed)
                verify(Words.writeReportedWhy(kind, "origin", "main").length > 0, kind)
            compare(Words.writeReportedWhy("delete"), "")
            compare(Words.writeReportedWhy("update"), "")
            compare(Words.writeReportedWhy("rename"), "", "the box that is still holding the name answers instead")
            compare(Words.writeReportedWhy(""), "")
        }

        /// The refusals a read answers, told apart by what the remote turned out to be: holding commits the push would
        /// drop, changed since the last fetch (sending or deleting, a branch or a tag alike), or holding the tag on
        /// another commit. Source text, as below.
        function test_a_refusal_a_read_answers_says_what_the_remote_turned_out_to_be() {
            const said = {
                "outdated": ["origin", "main", "main was not sent to origin",
                             "origin has commits that this push would drop."],
                "moved": ["origin", "main", "main was not sent to origin",
                          "origin has changed since the last fetch."],
                "moved-delete": ["origin", "next", "next was not deleted from origin",
                                 "origin has changed since the last fetch."],
                "tag-elsewhere": ["origin", "v6.3", "v6.3 was not sent to origin",
                                  "origin already has v6.3 on another commit."]
            }
            for (const kind in said) {
                const [remote, name, heading, why] = said[kind]
                compare(Words.writeReported(kind, remote, name), heading, kind)
                compare(Words.writeReportedWhy(kind, remote, name), why, kind)
                compare(Words.reportTone(kind), "danger", kind)
            }
        }

        /// The three the plan turns down reach the row menu's door with the same line: the list is delegated whole
        /// (`Words.rewriteRefusedWhy`).
        function test_the_rewrites_say_the_same_line_at_both_doors() {
            for (const kind of ["across-merge", "off-branch", "unfetched-base"])
                compare(Words.writeReportedWhy(kind), Words.rewriteRefusedWhy(kind), kind)
        }

        /// Which line belongs to which refusal: the cases above pass under any mapping, e.g. off-branch and
        /// unfetched-base swapped. The expectation is the source text — `qsTr` returns it with no translator
        /// installed, and `qmltestrunner` installs none.
        function test_each_refused_rewrite_says_its_own_reason() {
            const owed = {
                "across-merge": "A merge sits in the history this would replay, and a replay drops merges."
                                + " What came back would be flattened.",
                "off-branch": "This commit is not in the current branch's history."
                              + " Switch to a branch that has it first.",
                "fold-first": "This is the first commit, so there is nothing before it to fold into.",
                "unfetched-base": "The commit below this one is not in this clone. Replaying from here would cut"
                                  + " the branch off from the rest of its history.",
                "drop-all": "This is the last commit, and a branch cannot be left with no history at all.",
                "tip-moved": "The branch moved after this was worked out.",
                "op-standing": "An operation is in progress here. Finish it or put it down first."
            }
            for (const kind in owed) {
                compare(Words.rewriteRefusedWhy(kind), owed[kind], kind)
                compare(Words.writeReportedWhy(kind), owed[kind], kind)
            }
            compare(Words.rewriteRefusedWhy("no-such-kind"), "", "a kind nobody spelled is owed no line")
        }

        /// The plan's own door names the history in its heading (`RepoPage.onRefusedPlan`, `Words.planRefused`):
        /// unlike the row menu, it has no pressed row on screen to say which. Source text, as above.
        function test_each_refused_plan_says_which_history_stopped_it() {
            compare(Words.planRefused("across-merge"), "A merge is in the way")
            compare(Words.planRefused("off-branch"), "Not on this branch")
            compare(Words.planRefused("unfetched-base"), "The history stops here")
        }

        /// The two doors part on the heading and meet on the line — one case, because either half alone passes a
        /// wrong implementation.
        function test_the_plan_door_parts_from_the_row_menu_on_the_heading_only() {
            for (const kind of ["across-merge", "off-branch", "unfetched-base"]) {
                verify(Words.planRefused(kind) !== Words.writeReported(kind, "origin", "main"),
                       kind + ": the range's own heading, not the pressed row's")
                compare(Words.rewriteRefusedWhy(kind), Words.writeReportedWhy(kind),
                        kind + ": and one line under both")
            }
            // Unreachable (`PlanRefusal` is exhaustive on both sides of the bridge), but a general heading beats an
            // empty one over a lowered bar.
            compare(Words.planRefused("no-such-kind"), "The history was not rewritten")
        }

        /// The hairline carries the whole of the colour; the words stay in their own.
        function test_the_hairline_takes_the_tone_and_the_words_do_not() {
            dress("half-rename", "origin", "main", "")
            // All the way down, which is where the hairline is the width of the bar.
            tryVerify(() => bar.settled)
            compare(bar.label, "The rename did not finish")
            compare(bar.detail, Words.writeReportedWhy("half-rename"))
            compare(bar.tone, "warning")
            const rule = findRule()
            compare(rule.color, Theme.warning)

            dress("update", "origin", "main", "")
            compare(bar.tone, "danger")
            compare(rule.color, Theme.danger)
            compare(bar.detail, "", "the page puts the far side's own words there")

            bar.tone = ""
            compare(rule.color, Theme.borderSubtle, "a report in no state at all still closes with an edge")
        }

        function test_the_far_sides_own_words_are_the_ones_quoted() {
            dress("update", "origin", "main", "GH006: protected branch hook declined")
            compare(bar.detail, "GH006: protected branch hook declined")
        }

        /// Quoted whole (デザイン規約 §答えの要らない報せ). Both halves are the claim: `wordsCut` for the words, and the
        /// height, since a bar wrapping inside a height it never grew shows as much as one that elided.
        function test_a_long_quote_wraps_and_the_bar_grows_by_it() {
            dress("update", "origin", "main", "GH006: protected branch hook declined")
            tryVerify(() => bar.settled)
            const oneLine = bar.openHeight
            verify(!bar.wordsCut, "a quote that fits is not cut either")

            dress("update", "origin", "main", root.longReason)
            // Both: the words ask for their room a frame before the bar gets it, and the bar animates there.
            tryVerify(() => bar.openHeight > oneLine && bar.settled)
            verify(!bar.wordsCut, "and not one word of it was cut")
            compare(bar.detail, root.longReason, "with the whole of it still in the field")
        }

        /// `OK` keeps the middle of the bar (デザイン規約 §答えの要らない報せ). Asked on a bar the words made taller than the
        /// pill: on one line, every rule about the pill's place answers the same.
        function test_the_pill_keeps_the_middle_however_many_lines_the_words_take() {
            dress("update", "origin", "main", "GH006: protected branch hook declined")
            tryVerify(() => bar.settled)
            fuzzyCompare(pillMiddle(), bar.height / 2, 1, "centred while the quote is one line")
            const oneLine = bar.openHeight

            dress("update", "origin", "main", root.longReason)
            tryVerify(() => bar.openHeight > oneLine && bar.settled)
            verify(bar.height > bar.pill.height + 2 * Theme.spaceMd,
                   "the words, not the pill, are what the bar is now as tall as: " + bar.height)
            fuzzyCompare(pillMiddle(), bar.height / 2, 1, "and it is still the middle it keeps")
        }

        function pillMiddle() {
            return bar.pill.mapToItem(bar, 0, bar.pill.height / 2).y
        }

        /// The pill and the key walk the same body (`dismiss`), which is what a headless run presses — and neither
        /// lowers the bar: that is the page's.
        function test_the_pill_and_the_key_both_answer_for_the_reader() {
            dress("update", "origin", "main", "")
            // The pill is pressed where it is standing, so the bar has to be all the way down first.
            tryVerify(() => bar.settled)
            verify(bar.escapes, "the report takes Escape while nothing stands over it")

            mouseClick(bar, bar.width - 40, bar.height / 2)
            compare(acknowledged.count, 1)
            verify(bar.open, "the bar is the page's to lower")

            bar.dismiss()
            compare(acknowledged.count, 2, "the key walks the pill's own body")
        }

        /// Two enabled `StandardKey.Cancel` shortcuts in one window fire neither (`tst_escape.qml`), so the bar gives
        /// way under a question, keeping its words.
        function test_a_question_standing_over_it_takes_the_key() {
            dress("update", "origin", "main", "")
            bar.yieldsEscape = true
            verify(!bar.escapes)
            compare(bar.label, "origin would not update main", "giving way is not going away")
            bar.yieldsEscape = false
            verify(bar.escapes)
        }

        /// The bar's closing hairline, found by shape rather than name: the child at the border's own thickness.
        function findRule() {
            for (let i = 0; i < bar.children.length; i++) {
                const child = bar.children[i]
                if (child.height === Theme.borderWidth && child.color !== undefined)
                    return child
            }
            fail("the bar closes with no hairline")
            return null
        }
    }
}

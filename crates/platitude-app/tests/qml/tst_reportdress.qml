import QtQuick
import QtTest
import platitude.ui

// The dress a report wears: the three answers `Words` gives for one kind, and what a real `NoticeBar` does with them
// (デザイン規約 §答えの要らない報せ).
//
// **A condition table and a component, and nothing between them.** Which report arrived is the page's to decide and
// the headless verbs prove it; what is left once the kind is in hand is a switch over that kind and a bar that wears
// its answers — both of them here, where every arm can be asked for the price of one process. The verb that used to
// walk the arms raised a whole window per kind and read the same three strings back off one hairline.
Item {
    id: root
    width: 420
    height: 200

    // Raised the way `RepoPage.showReport` raises it: the three answers `Words` gives, and nothing else.
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
            // All the way back up before the next case dresses it: the bar spends 200ms going, and a case that read
            // the height on the way would read the one before it.
            tryVerify(() => bar.shut)
        }

        /// What did not happen, why where nobody outside said, and the state it is in — the three answers the page's
        /// door hands the bar (`RepoPage.showReport`).
        ///
        /// **A copy of that door, and it proves nothing about it.** A door that stopped passing one of the three
        /// would leave every case below green, so what carries that claim is the run that enters the real one
        /// (`report-tone half-rename`). What is asked here is the two ends it joins: the answers `Words` gives, and
        /// what the bar does wearing them.
        function dress(kind, remote, name, reason) {
            bar.label = Words.writeReported(kind, remote, name)
            bar.detail = reason !== "" ? reason : Words.writeReportedWhy(kind)
            bar.tone = Words.reportTone(kind)
            bar.open = true
        }

        /// **Both arms, because either alone passes against a bar painted one colour.** A run that asked only about
        /// the rename would pass on an implementation that painted everything `warning`, and one that asked only
        /// about the remote's refusal would pass on the opposite.
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

        /// The heading each kind wears. **The subject is whoever decided**: the far side for the two it decided,
        /// the thing that did not move for the rest.
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

        /// **One heading over the seven a rewrite is turned down for**, so a reader who knows what they pressed reads
        /// which of them it was on the line underneath. Five are the plan's own and two are the outside world moving
        /// after the press — the same sentence for all seven is the claim.
        function test_every_refused_rewrite_says_the_same_thing_did_not_happen() {
            const kinds = ["across-merge", "off-branch", "fold-first", "unfetched-base", "drop-all",
                           "tip-moved", "op-standing"]
            for (const kind of kinds)
                compare(Words.writeReported(kind, "origin", "main"), "The history was not rewritten", kind)
        }

        /// The second line is ours only where nobody outside wrote one, and every kind that owes one has one.
        /// **An empty second line is an answer** — the page puts what came across there instead.
        function test_the_second_line_is_ours_only_where_nobody_outside_spoke() {
            const owed = ["stale-stage", "stale-unstage", "stale-discard", "conflicted-part", "half-rename",
                          "across-merge", "off-branch", "fold-first", "unfetched-base", "drop-all",
                          "tip-moved", "op-standing"]
            for (const kind of owed)
                verify(Words.writeReportedWhy(kind).length > 0, kind)
            compare(Words.writeReportedWhy("delete"), "")
            compare(Words.writeReportedWhy("update"), "")
            compare(Words.writeReportedWhy("outdated"), "")
            compare(Words.writeReportedWhy("rename"), "", "the box that is still holding the name answers instead")
            compare(Words.writeReportedWhy(""), "")
        }

        /// The three the plan turns down before it opens reach the second door with the same line, which is why the
        /// list is delegated whole rather than copied (`Words.rewriteRefusedWhy`).
        function test_the_rewrites_say_the_same_line_at_both_doors() {
            for (const kind of ["across-merge", "off-branch", "unfetched-base"])
                compare(Words.writeReportedWhy(kind), Words.rewriteRefusedWhy(kind), kind)
        }

        /// **Which line belongs to which refusal**, one by one.
        ///
        /// The two cases above are satisfied by any mapping at all: swap the off-branch line with the
        /// unfetched-base one and both stay green, while a reader short of history is told to switch branches.
        /// The only thing that ever said which line was due was the picture each refusal's own run took — one
        /// window per reason — and this is where that reading goes instead.
        ///
        /// **The expectation is the source text.** `qsTr` hands it back wherever no translator is installed and
        /// `qmltestrunner` installs none, which is the footing the headings above are already compared on.
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
                // And through the door the row menu takes, which delegates the whole list rather than
                // keeping a second copy — so the same table answers for both.
                compare(Words.writeReportedWhy(kind), owed[kind], kind)
            }
            compare(Words.rewriteRefusedWhy("no-such-kind"), "", "a kind nobody spelled is owed no line")
        }

        /// **The heading the plan's own door writes, one by one.** Where the row menu says the same thing over all
        /// seven — the reader is looking at the row they pressed — the plan has no row on screen to say which
        /// history it is, so the heading carries it (`RepoPage.onRefusedPlan`, `Words.planRefused`).
        ///
        /// The three ran as three headless windows until this took the reading over: one press each, one picture
        /// each, and the only thing the three pictures said differently was these three lines.
        ///
        /// **The expectation is the source text**, on the same footing as the headings and reasons above.
        function test_each_refused_plan_says_which_history_stopped_it() {
            compare(Words.planRefused("across-merge"), "A merge is in the way")
            compare(Words.planRefused("off-branch"), "Not on this branch")
            compare(Words.planRefused("unfetched-base"), "The history stops here")
        }

        /// **The two doors part on the heading and meet on the line.** Said as one case because either half alone is
        /// satisfied by the wrong implementation: a plan door that fell through to the row menu's heading would pass
        /// the line half, and one that wrote its own second line would pass the heading half — and a reader short of
        /// history would be told to switch branches either way.
        function test_the_plan_door_parts_from_the_row_menu_on_the_heading_only() {
            for (const kind of ["across-merge", "off-branch", "unfetched-base"]) {
                verify(Words.planRefused(kind) !== Words.writeReported(kind, "origin", "main"),
                       kind + ": the range's own heading, not the pressed row's")
                compare(Words.rewriteRefusedWhy(kind), Words.writeReportedWhy(kind),
                        kind + ": and one line under both")
            }
            // What is true of every refused rewrite, for a shape nobody here has words of its own for. Nothing
            // reaches it — `PlanRefusal` is exhaustive on both sides of the bridge — and an empty heading over a
            // bar that has come down is the one answer worse than a general one.
            compare(Words.planRefused("no-such-kind"), "The history was not rewritten")
        }

        /// The bar wearing them. **The hairline is the whole of the colour** — the words stay in their own.
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

        /// What came across is quoted as it came; what nobody said is ours.
        function test_the_far_sides_own_words_are_the_ones_quoted() {
            dress("update", "origin", "main", "GH006: protected branch hook declined")
            compare(bar.detail, "GH006: protected branch hook declined")
        }

        /// The one control, and the key that says the same thing. **Both walk the same body** (`dismiss`), which is
        /// what a headless run presses — and neither takes the bar down: the write it is about is the page's to
        /// finish with.
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

        /// **Two enabled `StandardKey.Cancel` shortcuts in one window fire neither** (`tst_escape.qml`), so the bar
        /// gives way where a question stands over it — and the words it is showing do not move.
        function test_a_question_standing_over_it_takes_the_key() {
            dress("update", "origin", "main", "")
            bar.yieldsEscape = true
            verify(!bar.escapes)
            compare(bar.label, "origin would not update main", "giving way is not going away")
            bar.yieldsEscape = false
            verify(bar.escapes)
        }

        /// The hairline the bar closes with, found by what it is rather than by name: the one child drawn along the
        /// bottom edge at the border's own thickness.
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

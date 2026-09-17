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

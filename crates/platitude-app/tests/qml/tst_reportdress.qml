import QtQuick
import QtTest
import platitude.ui

// The dress a report wears: the three answers `Words` gives for one kind, and what a real `NoticeBar` does with
// them (デザイン規約 §答えの要らない報せ). Which report arrives is the page's and the headless verbs prove it; the
// switch over the kind is asked here, every arm in one process
// (rules-refs/app-ui.md「その条件表は QML 部品テストへ移し」). Every expected line is the source text — `qsTr`
// returns it with no translator installed, and `qmltestrunner` installs none — never another answer of `Words`; the
// log's own title and link (`Words.commandsTitle` / `commandsHref`) are names the lines point at, read as they are.
Item {
    id: root
    width: 420
    height: 200

    /// A lock's reason as its owner wrote it, wider than the bar — the one thing of somebody else's a line carries.
    readonly property string longReason: "kept on the release laptop until the 2.0 build is signed off"
                                         + " by the whole team, then moved to the archive disk"

    NoticeBar {
        id: bar
        width: root.width
    }
    SignalSpy {
        id: acknowledged
        target: bar
        signalName: "acknowledged"
    }
    SignalSpy {
        id: linked
        target: bar
        signalName: "linkAsked"
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

        /// A copy of the page's door (`RepoPage.showReport`): heading, our second line, tone. It proves nothing about
        /// the real door — one dropping an answer leaves every case here green; that claim is carried by
        /// `report-tone half-rename`.
        function dress(kind, remote, name, fact) {
            bar.label = Words.writeReported(kind, remote, name)
            bar.detail = Words.writeReportedWhy(kind, remote, name, fact)
            bar.tone = Words.reportTone(kind)
            bar.markWord = Words.reportNamesWorktree(kind) ? name : ""
            bar.markWordAt = Words.reportNamesWorktree(kind) ? Words.reportNameSeat(kind, remote) : -1
            bar.detailPlace = Words.reportPlace(kind)
            bar.detailHref = bar.detailPlace === "" ? "" : Words.commandsHref
            bar.open = true
        }

        /// A worktree the removal left standing: the gesture's colour, and the worktree's name in the heading behind
        /// the tree mark every worktree's name wears (デザイン規約 §ref の種別「名前の印」). Both kinds, and a
        /// report naming no worktree, since a bar that marks every heading passes the first half alone.
        function test_a_worktree_git_kept_is_named_behind_the_tree_mark() {
            compare(Words.reportTone("worktree-kept"), "warning")
            compare(Words.reportTone("worktree-half"), "warning")
            compare(Words.writeReported("worktree-kept", "", "topic"), "topic was not removed")
            compare(Words.writeReported("worktree-half", "", "topic"), "topic was not removed completely")
            compare(Words.writeReportedWhy("worktree-kept"), "It has uncommitted changes.")
            verify(!Words.reportNamesWorktree("rename"), "a branch's name wears no tree")

            // One space of room, where this font needs two: pinned, or rich text drops it at the block's start.
            compare(Words.roomInSentence("topic was not removed", 0, 1), "&nbsp;topic was not removed")

            dress("worktree-kept", "", "topic", "")
            tryVerify(() => bar.settled && bar.markShown && bar.markInk > 0)
            // The name opens the sentence, where rich text drops a lone leading space: the room must still be there,
            // as wide as the mark's ink, or the mark is drawn over the name's first letter.
            verify(bar.markSeat.x >= bar.markInk, "the name was pushed by the whole mark: " + bar.markSeat.x
                   + " < " + bar.markInk)

            dress("update-rule", "origin", "main", "")
            tryVerify(() => bar.settled)
            verify(!bar.markShown, "no worktree named, no mark")
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
            compare(Words.markSeatIn("Den Ordner topic gibt es", "topic", true), -1,
                    "a translation that moved it: none")
            compare(Words.markSeatIn("Cannot rename — vaulted is locked", "vaulted", true), 16,
                    "a tip may open what follows its conclusion with the name")
            compare(Words.markSeatIn("Cannot rename — revaulted is locked", "vaulted", true), -1,
                    "the letters, not the word")
            compare(Words.markSeatIn("vaulted — vaulted", "vaulted", false), 0, "a heading's seat is only its first")
            compare(Words.markSeatIn("anything", "", true), -1)
            compare(Words.roomInSentence("anything", -1, 2), "", "no seat, no room: drawn as it came")
            // A heading's: the seat its own sentence gives the name, first or within.
            compare(Words.reportNameSeat("worktree-kept", ""), 0)
            compare(Words.reportNameSeat("carried-in-the-way", ""), 13)
            compare(Words.reportNameSeat("rename", "origin"), 0)
        }

        /// Another worktree's changes not applied here: the worktree's yellow, its name behind the tree mark inside
        /// the heading's sentence, and our own line under it — git's says which files, the log keeps it.
        function test_changes_not_applied_name_their_worktree_inside_the_heading() {
            for (const kind of ["carried-in-the-way", "carried-file-in-the-way", "carried-unread", "carried-refused"]) {
                compare(Words.reportTone(kind), "warning", kind)
                verify(Words.reportNamesWorktree(kind), kind)
                compare(Words.writeReported(kind, "", "topic"), "Changes from topic were not applied", kind)
            }
            compare(Words.writeReportedWhy("carried-in-the-way"),
                    "Uncommitted changes in this worktree would be overwritten.")
            compare(Words.writeReportedWhy("carried-file-in-the-way"), "Files in this worktree would be overwritten.")
            compare(Words.writeReportedWhy("carried-unread"), "Not all of its changes could be read.")
            compare(Words.writeReportedWhy("carried-refused"), "git did not apply them — see GIT COMMANDS")
            dress("carried-in-the-way", "", "topic", "")
            tryVerify(() => bar.settled && bar.markShown && bar.markInk > 0)
        }

        /// A report whose line sends the reader to the command log names it as a place to go, pressed through to the
        /// page with the log's href; a line of its own holds nothing to press.
        function test_a_line_sending_the_reader_to_the_log_names_it_as_a_place_to_go() {
            linked.clear()
            dress("carried-refused", "", "topic", "")
            tryVerify(() => bar.settled)
            verify(bar.pressDetailLink(), "the place word is there to press")
            compare(linked.count, 1)
            compare(linked.signalArguments[0][0], Words.commandsHref)

            dress("carried-in-the-way", "", "topic", "")
            tryVerify(() => bar.settled)
            verify(!bar.pressDetailLink(), "a line naming no place has nothing to press")
            compare(linked.count, 1)
        }

        /// The mark stands where the heading's sentence puts the name, not on the first word spelled like it: a
        /// worktree called `from` is marked where `zzzz`, a name no word of the sentence spells, is.
        function test_a_worktree_named_like_a_word_of_the_heading_is_marked_at_its_own_seat() {
            dress("carried-in-the-way", "", "zzzz", "")
            tryVerify(() => bar.settled && bar.markShown)
            const seat = bar.markSeat.x
            dress("carried-in-the-way", "", "from", "")
            tryVerify(() => bar.settled && bar.markShown)
            compare(bar.markSeat.x, seat)
        }

        /// A worktree that was not made — or made with its hook failed: the same yellow, the same mark, in the
        /// heading the folder would have been (デザイン規約 §worktree を作る). The second line is the screen's, a
        /// branch git left behind named in it.
        function test_a_worktree_not_made_is_named_behind_the_tree_mark_in_yellow() {
            for (const kind of ["worktree-not-added", "worktree-folder-taken", "worktree-place-taken",
                                "worktree-branch-left"]) {
                compare(Words.reportTone(kind), "warning", kind)
                verify(Words.reportNamesWorktree(kind), kind)
                compare(Words.writeReported(kind, "", "hotfix-patch"), "hotfix-patch was not created", kind)
            }
            compare(Words.writeReportedWhy("worktree-folder-taken", "", "", ""), "The folder is not empty.")
            compare(Words.writeReportedWhy("worktree-not-added", "", "", ""), "git turned it down — see GIT COMMANDS")
            compare(Words.writeReportedWhy("worktree-place-taken", "", "", ""), "It is taken by another worktree.")
            compare(Words.writeReportedWhy("worktree-branch-left", "", "hotfix-patch", "hotfix/patch"),
                    "The branch hotfix/patch was made anyway — see GIT COMMANDS")
            compare(Words.reportTone("worktree-hook"), "warning")
            compare(Words.writeReported("worktree-hook", "", "hotfix-patch"),
                    "hotfix-patch was made, but its post-checkout hook failed")
            compare(Words.writeReportedWhy("worktree-hook", "", "", ""), "See GIT COMMANDS for what the hook said.")
            compare(Words.reportNameSeat("worktree-hook", ""), 0)
            dress("worktree-branch-left", "", "hotfix-patch", "hotfix/patch")
            tryVerify(() => bar.settled && bar.markShown)
            compare(bar.detail, "The branch hotfix/patch was made anyway — see GIT COMMANDS")
        }

        /// A worktree git would not remove, by what kept it: changes, a lock — with its owner's reason, or none —
        /// anything else, and a removal that stopped part way. Each line the screen's own.
        function test_a_worktree_kept_says_what_kept_it() {
            for (const kind of ["worktree-kept", "worktree-locked", "worktree-refused"]) {
                compare(Words.writeReported(kind, "", "topic"), "topic was not removed", kind)
                compare(Words.reportTone(kind), "warning", kind)
            }
            compare(Words.writeReportedWhy("worktree-locked", "", "topic", "on a stick"), "It is locked: on a stick.")
            compare(Words.writeReportedWhy("worktree-locked", "", "topic", ""), "It is locked.")
            compare(Words.writeReportedWhy("worktree-refused", "", "topic", ""),
                    "git turned it down — see GIT COMMANDS")
            compare(Words.writeReportedWhy("worktree-half", "", "topic", ""),
                    "git could not delete everything — see GIT COMMANDS")
        }

        /// A worktree whose folder kept its name: the same yellow and mark, the folder as it still is. git's turning down
        /// sends the reader to the log; what stood at the new path is said here, nobody to quote
        /// (デザイン規約 §worktree の名前を変える).
        function test_a_worktree_not_renamed_is_named_behind_the_tree_mark_and_says_where_to_look() {
            for (const kind of ["worktree-not-renamed", "worktree-rename-taken"]) {
                compare(Words.reportTone(kind), "warning", kind)
                verify(Words.reportNamesWorktree(kind), kind)
                compare(Words.writeReported(kind, "", "loose"), "loose was not renamed", kind)
            }
            compare(Words.writeReportedWhy("worktree-not-renamed"), "git turned it down — see GIT COMMANDS")
            compare(Words.reportPlace("worktree-not-renamed"), Words.commandsTitle)
            compare(Words.writeReportedWhy("worktree-rename-taken"), "Something is already there.")
            compare(Words.reportPlace("worktree-rename-taken"), "", "a line of its own sends nobody anywhere")
            dress("worktree-not-renamed", "", "loose", "")
            tryVerify(() => bar.settled && bar.markShown)
            compare(bar.detail, "git turned it down — see GIT COMMANDS")
            compare(bar.detailPlace, Words.commandsTitle)
        }

        /// Both arms, because either alone passes against a bar painted one colour.
        function test_the_gesture_that_is_still_going_wears_the_other_colour() {
            for (const kind of ["rename", "rename-locked", "half-rename", "half-rename-refused", "half-rename-locked",
                                "rename-unfinished", "stash-rename", "stash-rename-locked", "carried-locked"])
                compare(Words.reportTone(kind), "warning", kind)
            for (const kind of ["update-rule", "delete-rule", "outdated", "pre-push", "pre-push-delete",
                                "push-unsigned", "push-unsigned-delete", "signed-push-untaken",
                                "signed-push-untaken-delete", "commit", "commit-hook", "commit-unsigned",
                                "commit-locked", "stash-gone", "stale-stage", "stage-locked", "unstage-locked",
                                "discard-locked", "conflicted-part", "tip-moved", "op-standing", "pre-rebase",
                                "rewrite-unsigned", "rewrite-commit-hook", "rewrite-locked", "rewrite-untracked"])
                compare(Words.reportTone(kind), "danger", kind)
            compare(Words.reportTone(""), "danger", "a kind nobody spelled still says so in colour")
        }

        /// A refused rename keeps its box open holding the name: nothing moved and the row is there (デザイン規約
        /// §答えの要らない報せ「改名の箱は書き込みが着いてから閉じる」). A stash that left the list has no row to
        /// stand on.
        function test_a_rename_git_turned_down_keeps_its_box() {
            for (const kind of ["rename", "rename-locked", "stash-rename", "stash-rename-locked"])
                verify(Words.renameStillOpen(kind), kind)
            for (const kind of ["half-rename", "half-rename-refused", "half-rename-locked", "rename-unfinished",
                                "stash-gone", "commit", ""])
                verify(!Words.renameStillOpen(kind), kind)
        }

        /// The subject is whoever decided: the far side for what it decided, the thing that did not move for the
        /// rest. The far side's refusals are two words, and the heading turns on the first whatever the second.
        function test_the_heading_names_what_did_not_happen() {
            compare(Words.writeReported("delete-rule", "origin", "v1.0"), "origin would not delete v1.0")
            compare(Words.writeReported("delete-current", "origin", "main"), "origin would not delete main")
            compare(Words.writeReported("update-rule", "origin", "main"), "origin would not update main")
            compare(Words.writeReported("update-too-large", "origin", "main"), "origin would not update main")
            for (const kind of ["outdated", "pre-push", "push-unsigned", "signed-push-untaken"])
                compare(Words.writeReported(kind, "origin", "main"), "main was not sent to origin", kind)
            for (const kind of ["pre-push-delete", "push-unsigned-delete", "signed-push-untaken-delete"])
                compare(Words.writeReported(kind, "origin", "main"), "main was not deleted from origin", kind)
            compare(Words.writeReported("rename", "origin", "main"), "main was not renamed")
            compare(Words.writeReported("rename-locked", "origin", "main"), "main was not renamed")
            for (const kind of ["half-rename", "half-rename-refused", "half-rename-locked"])
                compare(Words.writeReported(kind, "origin", "main"), "The rename did not finish", kind)
            compare(Words.writeReported("rename-unfinished", "", "renamed"), "The rename did not finish")
            for (const kind of ["stash-rename", "stash-rename-locked", "stash-gone"])
                compare(Words.writeReported(kind, "", ""), "The stash was not renamed", kind)
            for (const kind of ["stale-stage", "stage-locked"])
                compare(Words.writeReported(kind, "", ""), "Nothing was staged", kind)
            for (const kind of ["stale-unstage", "unstage-locked"])
                compare(Words.writeReported(kind, "", ""), "Nothing was unstaged", kind)
            for (const kind of ["stale-discard", "discard-locked"])
                compare(Words.writeReported(kind, "", ""), "Nothing was discarded", kind)
            compare(Words.writeReported("conflicted-part", "", ""), "Nothing was taken from this file")
            compare(Words.writeReported("carried-locked", "", "topic"), "Changes from topic were not applied")
            for (const kind of ["commit", "commit-hook", "commit-unsigned", "commit-locked", ""])
                compare(Words.writeReported(kind, "", ""), "The commit was not made", kind)
        }

        /// One heading over every refused rewrite (five the plan's own, two the world as the press found it, five
        /// something after git started); the line underneath says which.
        function test_every_refused_rewrite_says_the_same_thing_did_not_happen() {
            const kinds = ["across-merge", "off-branch", "fold-first", "unfetched-base", "drop-all",
                           "tip-moved", "op-standing", "pre-rebase", "rewrite-unsigned",
                           "rewrite-commit-hook", "rewrite-locked", "rewrite-untracked"]
            for (const kind of kinds)
                compare(Words.writeReported(kind, "origin", "main"), "The history was not rewritten", kind)
        }

        /// Every second line is ours, never anybody's message (デザイン規約 §答えの要らない報せ). Where git ran and
        /// something said no, the line says who and sends the reader to the log, where its words are — and the one
        /// word that goes there is the log's title, read off the line itself (`reportPlace`).
        function test_who_said_no_is_said_and_the_log_is_where_their_words_are() {
            const owed = {
                "pre-push": "The pre-push hook turned it down — see GIT COMMANDS",
                "pre-push-delete": "The pre-push hook turned it down — see GIT COMMANDS",
                "push-unsigned": "The push could not be signed — see GIT COMMANDS",
                "push-unsigned-delete": "The push could not be signed — see GIT COMMANDS",
                "signed-push-untaken": "origin does not take signed pushes — see GIT COMMANDS",
                "signed-push-untaken-delete": "origin does not take signed pushes — see GIT COMMANDS",
                "update-rule": "A rule on origin turned it down — see GIT COMMANDS",
                "delete-rule": "A rule on origin turned it down — see GIT COMMANDS",
                "update-rewrite": "origin does not take pushes that rewrite its history — see GIT COMMANDS",
                "delete-no-deletes": "origin does not let branches be deleted — see GIT COMMANDS",
                "update-checked-out": "origin has main checked out in a working tree of its own — see GIT COMMANDS",
                "delete-current": "main is the current branch of origin — see GIT COMMANDS",
                "update-too-large": "The push is larger than origin accepts — see GIT COMMANDS",
                "update-shallow": "origin does not take pushes from a shallow clone — see GIT COMMANDS",
                "update-locked": "Another git process seems to be running on origin — see GIT COMMANDS",
                "delete-refused": "origin turned it down — see GIT COMMANDS",
                "update-refused": "origin turned it down — see GIT COMMANDS",
                "commit": "git turned it down — see GIT COMMANDS",
                "commit-hook": "A commit hook turned it down — see GIT COMMANDS",
                "commit-unsigned": "The commit could not be signed — see GIT COMMANDS",
                "commit-locked": "Another git process seems to be running — see GIT COMMANDS",
                "pre-rebase": "The pre-rebase hook turned it down — see GIT COMMANDS",
                "rewrite-unsigned": "The commit could not be signed — see GIT COMMANDS",
                "rewrite-commit-hook": "A commit hook turned it down — see GIT COMMANDS",
                "rewrite-locked": "Another git process seems to be running — see GIT COMMANDS",
                "rewrite-untracked": "Untracked files would be overwritten — see GIT COMMANDS",
                "rename": "git turned it down — see GIT COMMANDS",
                "rename-locked": "Another git process seems to be running — see GIT COMMANDS",
                "rename-unfinished": "The branch has its new name, but git stopped before the rest moved"
                                     + " — see GIT COMMANDS",
                "half-rename": "The new name was made; the old one is still there.",
                "half-rename-refused": "The new name was made, but git did not remove the old one"
                                       + " — see GIT COMMANDS",
                "half-rename-locked": "The new name was made, but another git process seems to be running"
                                      + " — see GIT COMMANDS",
                "stash-rename": "git turned it down — see GIT COMMANDS",
                "stash-rename-locked": "Another git process seems to be running — see GIT COMMANDS",
                "stash-gone": "It is no longer in the stash list.",
                "stage-locked": "Another git process seems to be running — see GIT COMMANDS",
                "unstage-locked": "Another git process seems to be running — see GIT COMMANDS",
                "discard-locked": "Another git process seems to be running — see GIT COMMANDS",
                "carried-refused": "git did not apply them — see GIT COMMANDS",
                "carried-locked": "Another git process seems to be running — see GIT COMMANDS",
                "worktree-hook": "See GIT COMMANDS for what the hook said.",
                "outdated": "origin has commits that this push would drop.",
                "worktree-kept": "It has uncommitted changes."
            }
            for (const kind in owed) {
                compare(Words.writeReportedWhy(kind, "origin", "main", ""), owed[kind], kind)
                compare(Words.reportPlace(kind), owed[kind].indexOf("GIT COMMANDS") >= 0 ? "GIT COMMANDS" : "",
                        kind + ": the place word is the line's own")
            }
            compare(Words.writeReportedWhy("", "", "", ""), "", "a kind nobody spelled is owed no line")
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
        /// (`Words.rewriteRefusedWhy`). Asked of both doors against the source text, so a door that spells its own
        /// copy differently fails here.
        function test_the_rewrites_say_the_same_line_at_both_doors() {
            const owed = {
                "across-merge": "A merge sits in the history this would replay, and a replay drops merges."
                                + " What came back would be flattened.",
                "off-branch": "This commit is not in the current branch's history."
                              + " Switch to a branch that has it first.",
                "unfetched-base": "The commit below this one is not in this clone. Replaying from here would cut"
                                  + " the branch off from the rest of its history."
            }
            for (const kind in owed) {
                compare(Words.writeReportedWhy(kind), owed[kind], kind + ": the row menu's door")
                compare(Words.rewriteRefusedWhy(kind), owed[kind], kind + ": the plan's door")
            }
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
                "op-standing": "An operation is in progress. Finish it or put it down first."
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
            compare(bar.detail, "The new name was made; the old one is still there.")
            compare(bar.tone, "warning")
            const rule = findRule()
            compare(rule.color, Theme.warning)

            dress("update-rule", "origin", "main", "")
            compare(bar.tone, "danger")
            compare(rule.color, Theme.danger)
            compare(bar.detail, "A rule on origin turned it down — see GIT COMMANDS", "our own line, never theirs")

            bar.tone = ""
            compare(rule.color, Theme.borderSubtle, "a report in no state at all still closes with an edge")
        }

        /// The one thing of somebody else's a line carries — a lock's reason — is plain text: a `<` in it is a
        /// character, not markup that takes the rest of the line away.
        function test_a_line_without_a_place_is_drawn_as_plain_text() {
            dress("worktree-locked", "", "topic", "held <b>until</b> Friday")
            tryVerify(() => bar.settled)
            compare(bar.detail, "It is locked: held <b>until</b> Friday.")
            const field = findDetail()
            compare(field.textFormat, Text.PlainText)
            compare(field.text, bar.detail)
        }

        /// Quoted whole (デザイン規約 §答えの要らない報せ). Both halves are the claim: `wordsCut` for the words, and
        /// the height, since a bar wrapping inside a height it never grew shows as much as one that elided.
        function test_a_long_line_wraps_and_the_bar_grows_by_it() {
            dress("worktree-locked", "", "topic", "on a stick")
            tryVerify(() => bar.settled)
            const oneLine = bar.openHeight
            verify(!bar.wordsCut, "a line that fits is not cut either")

            dress("worktree-locked", "", "topic", root.longReason)
            // Both: the words ask for their room a frame before the bar gets it, and the bar animates there.
            tryVerify(() => bar.openHeight > oneLine && bar.settled)
            verify(!bar.wordsCut, "and not one word of it was cut")
            compare(bar.detail, "It is locked: " + root.longReason + ".", "with the whole of it still in the field")
        }

        /// `OK` keeps the middle of the bar (デザイン規約 §答えの要らない報せ). Asked on a bar the words made taller
        /// than the pill: on one line, every rule about the pill's place answers the same.
        function test_the_pill_keeps_the_middle_however_many_lines_the_words_take() {
            dress("worktree-locked", "", "topic", "on a stick")
            tryVerify(() => bar.settled)
            fuzzyCompare(pillMiddle(), bar.height / 2, 1, "centred while the line is one line")
            const oneLine = bar.openHeight

            dress("worktree-locked", "", "topic", root.longReason)
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
            dress("update-rule", "origin", "main", "")
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
            dress("update-rule", "origin", "main", "")
            bar.yieldsEscape = true
            verify(!bar.escapes)
            compare(bar.label, "origin would not update main", "giving way is not going away")
            bar.yieldsEscape = false
            verify(bar.escapes)
        }

        /// The second line's field when it names no place, found by what it shows: the one visible text item holding
        /// the line.
        function findDetail() {
            const stack = [bar]
            while (stack.length > 0) {
                const item = stack.pop()
                if (item !== bar && item.visible && item.text === bar.detail && item.textFormat !== undefined)
                    return item
                for (let i = 0; i < item.children.length; i++)
                    stack.push(item.children[i])
            }
            fail("no field shows the second line")
            return null
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

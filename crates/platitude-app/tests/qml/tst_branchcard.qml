import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import "painted.js" as Painted

// The real branch card (`RefBranchMenu`) standing on the answers a menu would hand it (`RefMenuFacts.stand`), and
// the four rows it lends the level above — the push beside `pull` and the delete table (`BranchPushRow` /
// `BranchDeleteRow` / `BranchRemoteDeleteRow` / `BranchBothDeleteRow`), laid out as both entrances lay them: which rows
// each keeps, what each says, the gesture and colour each takes, and what a press asks for. Which rows a state may
// offer is `offers::ref_menu`'s and tested there.
//
// The greyed rows' sentences are judged here: a wrong reason frames like a right one.
Item {
    id: root
    // Wide enough for a card that bid a long name whole: the window would cut it otherwise.
    width: 640
    height: 480

    /// What a press asked for, last one wins: the card and its rows run nothing themselves.
    property string asked: ""

    /// The rows on offer, named in words — data, not a rule —, turned into the record `GitFacts.refMenu` answers by
    /// `menuOf`.
    readonly property var free: ["switch", "branch-here", "integrate", "delete", "delete-remote", "set-upstream",
                                 "unset-upstream"]
    /// The branch the worktree is on: no move, no delete, and the rows say so.
    readonly property var current: ["branch-here", "integrate", "set-upstream", "unset-upstream", "current"]
    /// One another worktree holds: `switch` stays and leads there without asking, the delete goes.
    readonly property var held: ["switch", "branch-here", "integrate", "set-upstream", "unset-upstream"]
    /// A reading standing on another commit: the local delete stays, the one that reaches over there goes.
    readonly property var drifted: ["switch", "branch-here", "integrate", "delete", "set-upstream", "unset-upstream"]

    /// The level above the card, holding the rows the card lends it and the card itself, as `RefRowMenu` and
    /// `CommitRowMenu` hold them.
    AppMenu {
        id: level
        BranchPushRow {
            id: pushRow
            card: branchCard
        }
        BranchDeleteRow {
            id: deleteRow
            card: branchCard
        }
        BranchRemoteDeleteRow {
            id: remoteDeleteRow
            card: branchCard
        }
        BranchBothDeleteRow {
            id: bothDeleteRow
            card: branchCard
        }
        RefBranchMenu {
            id: branchCard
            onDeleteRequested: (kind, id, name, oidHex) => root.asked = "delete " + kind + " " + id
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                root.asked = "upstream " + branch + " " + counterpart + " " + unsettable
            onUnsetUpstreamRequested: branch => root.asked = "unset " + branch
            onCheckDeleteRequested: branch => root.asked = "check " + branch
            onForceDeleteRequested: branch => root.asked = "force " + branch
            onDeleteRemoteRequested: (remoteRef, expect) => root.asked = "delete-remote " + remoteRef + " " + expect
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                root.asked = "delete-both " + branch + " " + remoteRef + " " + forced + " " + expect
            onPushRequested: (branch, force, lease) => root.asked = "push " + branch + " " + force + " " + lease
            onPublishRequested: (branch, upstream) => root.asked = "publish " + branch + " " + upstream
        }
    }

    /// The record `GitFacts.refMenu` answers (`encode::RefMenu`) where `words` are on offer and the upstream stands
    /// `upstreamWhere`: the part of it the BRANCH card reads.
    function menuOf(words, upstreamWhere) {
        return { "delete": words.includes("delete"), "deleteRemote": words.includes("delete-remote"),
                 "setUpstream": words.includes("set-upstream"), "unsetUpstream": words.includes("unset-upstream"),
                 "current": words.includes("current"), "upstreamWhere": upstreamWhere }
    }
    /// `merged` is what the drawn rows already say (`yes` / `no`, empty when only git can answer). The row stands on
    /// `abc123` and its reading on `def456`, so a press says which commit it leased. `push` is the push row's answer
    /// (`NavSectionModel.branchPush`), none by default. `again`: why the menu stands again with the card up
    /// (`RefBranchMenu.standOn`'s — `follow` / `wait`), none as it is raised; the row on `oid` (`abc123` unless named).
    function standOn(fields, again, oid) {
        const all = { "kind": "branch", "name": "feature/topic-a", "full": "feature/topic-a",
                      "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "origin/feature/topic-a",
                      "upstream": "origin/feature/topic-a", "upstreamWhere": "remote",
                      "remoteCounterpartOid": "def456", "open": true, "merged": "yes",
                      "offers": root.free, "push": undefined }
        for (const key in fields)
            all[key] = fields[key]
        root.asked = ""
        branchCard.standOn(all.kind, all.name, all.full, oid === undefined ? "abc123" : oid, {
            "lead": { "path": all.heldByWorktree, "called": all.holderLeaf, "home": false },
            "card": { "remoteCounterpart": all.remoteCounterpart, "remoteCounterpartOid": all.remoteCounterpartOid,
                      "upstream": all.upstream },
            "menu": root.menuOf(all.offers, all.upstreamWhere),
            "open": all.open,
            "merged": all.merged,
            "push": all.push
        }, again === undefined ? "" : again)
    }
    /// What a press does in each standing and whether the row keeps a seat, as core answers them — data, the
    /// answers `remote::standing::tests::each_standing_takes_a_plain_press_the_overwrite_or_none` pins.
    readonly property var takes: ({
        "closed": { "sendsPlain": false, "asks": false, "overwrites": false, "offered": false },
        "publish": { "sendsPlain": true, "asks": true, "overwrites": false, "offered": true },
        "stays-here": { "sendsPlain": false, "asks": false, "overwrites": false, "offered": true },
        "elsewhere": { "sendsPlain": true, "asks": false, "overwrites": false, "offered": true },
        "ready": { "sendsPlain": true, "asks": false, "overwrites": false, "offered": true },
        "clean": { "sendsPlain": false, "asks": false, "overwrites": false, "offered": true },
        "behind": { "sendsPlain": false, "asks": false, "overwrites": true, "offered": true },
        "diverged": { "sendsPlain": false, "asks": false, "overwrites": true, "offered": true }
    })
    /// A push row's answer in `state` (`encode::PushRecord`), going to `origin/feature/topic-a` and leased to
    /// `fed789` where it counts.
    function pushing(state) {
        return Object.assign({ "state": state, "target": "origin/feature/topic-a", "lease": "fed789",
                               "upstream": "origin/feature/topic-a", "ahead": 0, "behind": 0 }, root.takes[state])
    }
    /// …and where the push would stay in this repository (`stays-here` — measured against another branch here and
    /// marked to no remote, or marked to `.`): nowhere named, nothing leased.
    readonly property var pushingHere: Object.assign(root.pushing("stays-here"),
                                                     { "target": "", "lease": "", "upstream": "" })
    /// git turns the plain delete of `name` down, handed to the card as the carrying menu hands it: cleared as the
    /// delete is asked, then the name (`RepoTab.branchDeleteRefused`), so each refusal is an edge of its own.
    function refuse(name) {
        branchCard.refusedDelete = ""
        branchCard.refusedDelete = name
    }

    /// Every write of the push row's answer (`test_standing_again_writes_the_push_answer_once`).
    SignalSpy {
        id: pushSeen
        target: branchCard
        signalName: "pushStateChanged"
    }

    TestCase {
        name: "BranchCard"
        when: windowShown

        /// A menu a test opened goes whatever the test said, so the next one finds its rows at rest.
        function cleanup() {
            level.releaseShape()
            branchCard.releaseShape()
            if (!level.visible)
                return
            level.dismiss()
            tryVerify(() => !level.visible)
        }

        /// A branch's three delete forms are a fixed table whose rows stay and grey (`RefBranchMenu`).
        function test_why_each_row_of_a_branchs_table_is_out_data() {
            return [
                {
                    tag: "the branch the tree is on",
                    fields: { offers: root.current },
                    reason: "Switch away first — this is the branch you are on",
                },
                {
                    tag: "held by another worktree",
                    fields: { offers: root.held, heldByWorktree: "C:/worktrees/topic", holderLeaf: "topic" },
                    reason: "Checked out in another worktree — topic",
                    // The one reason naming something outside this repository, so the tip gets a word for the tree
                    // mark (デザイン規約 §ref の種別).
                    marks: "topic",
                },
            ]
        }

        function test_why_each_row_of_a_branchs_table_is_out(data) {
            root.standOn(data.fields)
            verify(deleteRow.offered, "the table keeps its seats")
            compare(deleteRow.blockedReason, data.reason)
            compare(bothDeleteRow.blockedReason, data.reason, "and the pair names the half that runs first")
            const marks = data.marks === undefined ? "" : data.marks
            compare(deleteRow.tipMarkWord, marks, "the word the tip stands a mark against")
            compare(bothDeleteRow.tipMarkWord, marks)
        }

        function test_a_drifted_reading_is_out_while_the_local_delete_is_not() {
            root.standOn({ offers: root.drifted })
            compare(deleteRow.blockedReason, "", "the branch here goes")
            compare(remoteDeleteRow.blockedReason, "Differs from origin/feature/topic-a")
            compare(bothDeleteRow.blockedReason, "Differs from origin/feature/topic-a")
        }

        /// The colour a row's tag is painted in.
        function tagInk(row) {
            return Painted.inkOf(row, row.note)
        }

        /// A tag is one of the row's words: on the row out of reach it greys with them, and stays the warning on the
        /// one that can still be pressed (デザイン規約 §無効).
        function test_a_row_out_of_reach_greys_its_tag() {
            root.standOn({ offers: root.drifted, merged: "no" })
            verify(bothDeleteRow.blocked)
            compare(bothDeleteRow.note, "not merged")
            compare(tagInk(bothDeleteRow), Theme.textMuted)
            verify(!deleteRow.blocked)
            compare(deleteRow.note, "not merged")
            compare(tagInk(deleteRow), Theme.warning)
        }

        /// Under the presses the tag lifts with the words over the fill: in the warning it would sink into it
        /// (デザイン規約 §回数押しの色).
        function test_a_row_being_pressed_lifts_its_tag() {
            root.standOn({ merged: "no" })
            compare(deleteRow.note, "not merged")
            verify(level.offerHere(), "the menu opens")
            tryVerify(() => level.opened)
            // The menu lays its rows out after it stands: press once they are, not on the bare frame.
            verify(waitForItemPolished(level.contentItem), "the rows are laid out")
            mouseClick(deleteRow, 30, deleteRow.height / 2)
            tryVerify(() => deleteRow.holding)
            compare(tagInk(deleteRow), Theme.textOnAccent)
        }

        function test_a_branch_nothing_holds_back_offers_all_three() {
            root.standOn({})
            for (const row of [deleteRow, remoteDeleteRow, bothDeleteRow]) {
                verify(row.offered)
                compare(row.blockedReason, "")
            }
            compare(deleteRow.code, "branch --delete")
            compare(remoteDeleteRow.code, "push --delete")
            compare(deleteRow.holdPresses, Metrics.holdPressesPlain,
                    "the everyday delete is the plain delete's two presses; git is what may still refuse it")
            compare(deleteRow.holdTone, Theme.textPrimary, "in the plain ink")
            verify(remoteDeleteRow.holdPresses > 0, "reaching past this machine is held")
            compare(remoteDeleteRow.holdTone, Theme.warning)
            compare(bothDeleteRow.holdTone, Theme.warning, "while the local half is not a force delete")
        }

        /// It writes configuration, which the current branch and a held one still take (offers::ref_menu).
        function test_the_upstream_row_follows_its_own_word() {
            root.standOn({})
            verify(branchCard.upstreamItem.offered)
            root.standOn({ offers: root.current })
            verify(branchCard.upstreamItem.offered, "the branch the tree is on writes settings like any other")
        }

        /// Taking the upstream off (`RefBranchMenu`'s unset row): seated by core's word, in words that pair
        /// with `Set upstream…`, naming what goes, the plain delete's two presses in the plain ink.
        function test_the_unset_row_names_the_upstream_it_takes_off() {
            root.standOn({})
            const row = branchCard.unsetUpstreamItem
            verify(row.offered)
            compare(row.code, "", "words, no chip")
            compare(row.text, "Unset upstream origin/feature/topic-a")
            compare(row.refName, "origin/feature/topic-a")
            compare(row.holdPresses, Metrics.holdPressesPlain)
            compare(row.holdTone, Theme.textPrimary)
            // A name is plain, a remote's as any other's: the code ground sets it apart (§ref の種別「文の中の名前」).
            verify(Qt.colorEqual(row.refColor, Theme.textPrimary), "a remote branch: " + row.refColor)
            root.standOn({ remoteCounterpart: "", upstream: "release", upstreamWhere: "local" })
            verify(Qt.colorEqual(row.refColor, Theme.textPrimary), "a branch here: " + row.refColor)
            // …but for the branch stood on, in its own colour wherever it is named.
            root.standOn({ remoteCounterpart: "", upstream: "main", upstreamWhere: "current" })
            verify(Qt.colorEqual(row.refColor, Theme.textLink), "the branch the tree is on: " + row.refColor)

            // A gone upstream names no reading, and is taken off all the same.
            root.standOn({ remoteCounterpart: "", remoteCounterpartOid: "", upstream: "origin/gone" })
            verify(row.offered)
            compare(row.refName, "origin/gone")

            root.standOn({ offers: root.current })
            verify(row.offered, "the branch the tree is on takes it like any other setting")

            root.standOn({ remoteCounterpart: "", upstream: "", offers: ["switch", "delete", "set-upstream"] })
            verify(!row.offered, "nothing to take off")
        }

        /// The card holds the upstream rows alone, so nothing else widens it: the words round the name bid whole and
        /// the name at most `labelColW` (`AppMenuItem.growsForText`) — a short one stands whole, a long one gives way
        /// into the hover.
        function test_a_card_of_upstream_rows_alone_cuts_only_a_long_name_data() {
            return [
                { tag: "a short upstream", upstream: "origin/topic", cut: false },
                { tag: "one past the names' bid", upstream: "origin/" + "a".repeat(40), cut: true },
            ]
        }

        function test_a_card_of_upstream_rows_alone_cuts_only_a_long_name(data) {
            root.standOn({ remoteCounterpart: data.upstream, upstream: data.upstream })
            verify(level.offerHere(), "the menu opens")
            tryVerify(() => level.opened)
            verify(level.openSub(branchCard), "the card opens")
            tryVerify(() => branchCard.opened)
            verify(waitForItemPolished(branchCard.contentItem), "the rows are laid out")
            const row = branchCard.unsetUpstreamItem
            compare(row.refName, data.upstream)
            compare(row.refCut, data.cut, "in a card " + branchCard.width + " wide")
        }

        /// The question an emptied name turns into is only offered where there is something to take off.
        function test_the_upstream_question_is_told_whether_an_emptied_name_unsets() {
            root.standOn({ remoteCounterpart: "", remoteCounterpartOid: "", upstream: "origin/gone" })
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a  true", "opens at the default, and the gone one can go")

            root.standOn({ remoteCounterpart: "", upstream: "", offers: ["switch", "delete", "set-upstream"] })
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a  false", "nothing for it to take off")
        }

        /// A remote-tracking row seats only the rows it can name; its own delete is the held one reaching past this
        /// machine.
        function test_a_remote_row_keeps_only_the_rows_it_can_name() {
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: ["switch", "branch-here", "integrate", "delete"] })
            verify(deleteRow.offered)
            compare(deleteRow.code, "push --delete")
            verify(deleteRow.holdPresses > 0)
            compare(deleteRow.holdTone, Theme.warning)
            verify(!remoteDeleteRow.offered, "a reading has no reading of its own")
            verify(!bothDeleteRow.offered)
            verify(!branchCard.upstreamItem.offered, "and no setting of its own either")
            verify(!branchCard.unsetUpstreamItem.offered)
            verify(!branchCard.applies, "so the card, aimed at it, takes its title row away")
        }

        /// The toolbar's forms for a branch the worktree is not on (デザイン規約 §リモートへ送る): a click where git takes
        /// it, the held leased overwrite where only that reaches, a question where nothing says where it goes.
        function test_the_push_row_takes_the_toolbars_form_data() {
            return [
                { tag: "ready", state: "ready", code: "push", presses: 0, words: "to origin/feature/topic-a" },
                { tag: "elsewhere", state: "elsewhere", code: "push", presses: 0,
                  words: "to origin/feature/topic-a" },
                { tag: "behind", state: "behind", code: "push --force", presses: Metrics.holdPressesWarned,
                  words: "to origin/feature/topic-a" },
                { tag: "diverged", state: "diverged", code: "push --force", presses: Metrics.holdPressesWarned,
                  words: "to origin/feature/topic-a" },
                { tag: "never sent", state: "publish", code: "push", presses: 0, words: "to…" },
            ]
        }

        function test_the_push_row_takes_the_toolbars_form(data) {
            root.standOn({ push: root.pushing(data.state) })
            verify(pushRow.offered)
            verify(!pushRow.blocked, "a press reaches in every one of them")
            compare(pushRow.code, data.code)
            compare(pushRow.holdPresses, data.presses)
            compare(pushRow.text, data.words)
            compare(pushRow.holdTone, Theme.warning, "an overwrite reaches past this machine")
        }

        /// Nothing to name, or a door of its own: no seat (デザイン規約 §メニュー). Where is core's answer
        /// (`remote::PushStanding::offered`): the branch the tree is on — the toolbar's button —, and a tab that is not
        /// open, stand closed (`RefsSnapshot::branch_push`).
        function test_the_push_row_has_no_seat_where_it_could_do_nothing_here_data() {
            return [
                { tag: "no answer read", fields: {} },
                { tag: "core gives it none", fields: { push: root.pushing("closed") } },
                { tag: "a remote-tracking row sends nothing",
                  fields: { kind: "remote", name: "origin/main", full: "origin/main", remoteCounterpart: "",
                            offers: ["delete"], push: root.pushing("ready") } },
            ]
        }

        function test_the_push_row_has_no_seat_where_it_could_do_nothing_here(data) {
            root.standOn(data.fields)
            verify(!pushRow.offered)
        }

        /// No remote to send to, so the row stands out in the first push's words and says why — the toolbar's words
        /// for the same standing (デザイン規約 §リモートへ送る).
        function test_the_push_row_stands_out_where_the_push_would_stay_here() {
            root.standOn({ push: root.pushingHere })
            verify(pushRow.offered)
            verify(pushRow.blocked)
            compare(pushRow.code, "push")
            compare(pushRow.text, "to…")
            compare(pushRow.holdPresses, 0)
            compare(pushRow.blockedReason, "Cannot push — its upstream is not supported")

            // The next card on a branch that can be sent drops it.
            root.standOn({ push: root.pushing("ready") })
            verify(!pushRow.blocked)
        }

        /// The far side has it all: the row keeps its seat, named where it goes, and says there is nothing to send —
        /// the toolbar's words for the same standing (デザイン規約 §メニュー「「今できない」行は無効で残す」).
        function test_the_push_row_stands_out_where_there_is_nothing_to_send() {
            root.standOn({ push: root.pushing("clean") })
            verify(pushRow.offered)
            verify(pushRow.blocked)
            compare(pushRow.code, "push")
            compare(pushRow.text, "to origin/feature/topic-a")
            compare(pushRow.holdPresses, 0)
            compare(pushRow.blockedReason, "Nothing to push — origin/feature/topic-a is up to date")
        }

        /// Whether the row has a seat and what a press does are core's answers, the toolbar's
        /// (`remote::PushStanding::offered` / `sends_plain` / `asks`), not a table of the words here: the word picks
        /// only what the row says.
        function test_the_push_row_presses_where_core_says_a_press_reaches_data() {
            return [
                { tag: "ready, core giving it no seat", state: "ready", core: { "offered": false }, offered: false },
                { tag: "clean, core sending it", state: "clean", core: { "sendsPlain": true }, offered: true,
                  asked: "push feature/topic-a  " },
                { tag: "publish, core sending it without asking", state: "publish", core: { "asks": false },
                  offered: true, asked: "push feature/topic-a  " },
            ]
        }

        function test_the_push_row_presses_where_core_says_a_press_reaches(data) {
            root.standOn({ push: Object.assign(root.pushing(data.state), data.core) })
            compare(pushRow.offered, data.offered)
            compare(pushRow.blockedReason, "")
            if (!data.offered)
                return
            pushRow.triggered()
            compare(root.asked, data.asked, "a plain push, nothing leased")
        }

        function test_the_push_row_asks_for_the_push_its_form_says() {
            root.standOn({ push: root.pushing("ready") })
            pushRow.triggered()
            compare(root.asked, "push feature/topic-a  ", "a plain push, nothing leased")

            // Leased to the commit the card showed the far side on.
            root.standOn({ push: root.pushing("diverged") })
            pushRow.held()
            compare(root.asked, "push feature/topic-a lease fed789")

            // The question opens where the branch already points.
            root.standOn({ push: root.pushing("publish") })
            pushRow.triggered()
            compare(root.asked, "publish feature/topic-a origin/feature/topic-a")
        }

        function test_a_row_that_names_no_branch_has_no_card() {
            root.standOn({ kind: "tag", name: "v1.0", full: "v1.0", offers: [] })
            verify(!branchCard.applies)
        }

        /// git's refusal lands while the menu stands: the row turns into the held `-D` under the hand
        /// (`BranchDeleteRow.onHeld`), and the pair follows. Only a refusal of this card's branch.
        function test_a_refusal_turns_the_row_into_the_held_force_delete() {
            root.standOn({})
            compare(deleteRow.code, "branch --delete")
            compare(deleteRow.note, "")
            compare(bothDeleteRow.note, "")

            root.refuse("feature/topic-b")
            compare(branchCard.forceDeleteBranch, "", "another branch's refusal is not this card's")
            compare(deleteRow.holdPresses, Metrics.holdPressesPlain)
            compare(deleteRow.code, "branch --delete")

            root.refuse("feature/topic-a")
            compare(branchCard.forceDeleteBranch, "feature/topic-a", "the card holds the refusal")
            compare(deleteRow.holdPresses, Metrics.holdPressesWarned, "the plain two are no longer enough")
            // Commits nothing else reaches, but the discard record brings them back with the name
            // (デザイン規約 §回数押し の色の表): the warning, not the danger.
            compare(deleteRow.holdTone, Theme.warning, "what goes comes back from the record")
            compare(bothDeleteRow.note, "not merged")
            compare(bothDeleteRow.holdTone, Theme.warning, "so the pair stays the warning too")
            // With no press under way, the chip and the note already say what the row now runs.
            compare(deleteRow.code, "branch -D")
            compare(deleteRow.note, "not merged")
        }

        /// The menu standing again on what moved while it stood (`again`: `follow`), on the same branch at the same
        /// commit: git's refusal and its answer to the plain delete hold — asked again, the row would blank between the
        /// ask and the answer —, while a menu raised afresh drops the one and asks the other again.
        function test_standing_again_keeps_gits_answers_on_the_same_commit() {
            root.standOn({ merged: "" })
            compare(root.asked, "check feature/topic-a", "only git can answer")
            root.refuse("feature/topic-a")
            compare(deleteRow.code, "branch -D")

            root.standOn({ merged: "" }, "follow")
            compare(root.asked, "", "not asked again")
            compare(branchCard.forceDeleteBranch, "feature/topic-a", "the refusal holds")
            compare(deleteRow.code, "branch -D")

            root.standOn({ merged: "" }, "follow", "abc999")
            compare(root.asked, "check feature/topic-a", "the branch on another commit is asked again")
            compare(branchCard.forceDeleteBranch, "", "and its refusal was about the commit it left")

            root.refuse("feature/topic-a")
            root.standOn({ merged: "" })
            compare(branchCard.forceDeleteBranch, "", "a menu raised afresh asks again")
            compare(root.asked, "check feature/topic-a")
        }

        /// Standing again, the remote rows stay leased to the commit the menu went up on; that remote branch moved
        /// since, they grey where they stand asking for the menu to be raised again — a press would take commits it
        /// never showed (デザイン規約 §メニュー「表示も押下も開いた時」).
        function test_standing_again_holds_the_remote_lease_and_greys_a_moved_reading() {
            root.standOn({})
            level.holdShape()
            root.standOn({}, "follow")
            verify(!remoteDeleteRow.withdrawn, "the reading where it was")
            verify(!bothDeleteRow.withdrawn)

            root.standOn({ remoteCounterpartOid: "999fed" }, "follow")
            compare(branchCard.state.remoteCounterpartOid, "def456", "the lease stays the commit the menu showed")
            verify(remoteDeleteRow.withdrawn, "the remote delete greys where it stands")
            compare(remoteDeleteRow.blockedWhy, remoteDeleteRow.withdrawnWhy)
            verify(bothDeleteRow.withdrawn, "and so does the pair, whose remote half is the same lease")
            verify(!deleteRow.withdrawn, "the branch here is untouched")
            root.standOn({ remoteCounterpartOid: "def456" }, "follow")
            verify(remoteDeleteRow.withdrawn, "moved once is moved for the menu that showed it")

            level.releaseShape()
            root.standOn({ remoteCounterpartOid: "999fed" })
            compare(branchCard.state.remoteCounterpartOid, "999fed", "a menu raised afresh leases where it is now")
            verify(remoteDeleteRow.offered)
        }

        /// Standing again writes each answer once: a push row read part-way with no answer would grey for a moment and
        /// blank the presses under way (`AppMenuItem.onBlockedChanged`).
        function test_standing_again_writes_the_push_answer_once() {
            root.standOn({ push: root.pushing("diverged") })
            verify(pushRow.offered)
            pushSeen.clear()
            root.standOn({ push: root.pushing("diverged") }, "follow")
            compare(pushSeen.count, 0, "an answer that did not move is not written again")
            verify(pushRow.offered)
        }

        /// The push stays leased to the far side as the menu went up (`BranchPushRow`): that moved since — a fetch
        /// brought commits over there — the row keeps what it showed and greys asking for the menu to be raised again,
        /// since a press would overwrite commits it never showed (デザイン規約 §メニュー「表示も押下も開いた時」). The
        /// branch here moving is not the far side: the row stands on it.
        function test_standing_again_holds_the_push_lease_and_greys_a_moved_far_side() {
            const moved = root.pushing("diverged")
            moved.lease = "fed999"
            root.standOn({ push: root.pushing("diverged") })
            level.holdShape()
            root.standOn({ push: moved }, "follow")
            compare(branchCard.state.push.lease, "fed789", "the lease stays the commit the menu showed")
            verify(pushRow.withdrawn, "the push greys where it stands")
            compare(pushRow.blockedWhy, pushRow.withdrawnWhy)
            root.standOn({ push: root.pushing("diverged") }, "follow")
            verify(pushRow.withdrawn, "moved once is moved for the menu that showed it")

            level.releaseShape()
            root.standOn({ push: root.pushing("ready") })
            level.holdShape()
            root.standOn({ push: moved }, "follow")
            verify(pushRow.withdrawn, "a plain push that would now overwrite is the far side moving too")
            compare(branchCard.pushState, "ready", "and the row keeps what it showed")

            level.releaseShape()
            root.standOn({ push: root.pushing("clean") })
            level.holdShape()
            root.standOn({ push: root.pushing("ready") }, "follow", "abc999")
            verify(!pushRow.withdrawn, "a commit made here leaves the far side where it was")
            compare(branchCard.pushState, "ready")

            level.releaseShape()
            root.standOn({ push: moved })
            compare(branchCard.state.push.lease, "fed999", "a menu raised afresh leases where it is now")
            verify(pushRow.offered)
        }

        /// A push that would stay here keeps what it showed too: standing again where the row has no seat — the tab
        /// closed, or the tree on that branch — under a hand on the menu, it greys in its own words rather than
        /// blanking them (デザイン規約 §メニュー「表示も押下も開いた時」), as a row leased to a far side does.
        function test_standing_again_keeps_a_push_that_would_stay_here_in_its_words() {
            root.standOn({ push: root.pushingHere })
            level.holdShape()
            root.standOn({ push: root.pushing("closed") }, "follow")
            verify(pushRow.withdrawn)
            compare(pushRow.text, "to…")
            compare(pushRow.blockedWhy, pushRow.withdrawnWhy)
        }

        /// The remote rows stay on the reading the menu went up with: the branch's upstream pointed elsewhere outside
        /// (`git branch -u`, a remote renamed), they keep that name and lease and grey — a press would reach a remote
        /// branch the rows never named. The upstream rows read the tree as it is.
        function test_standing_again_holds_the_remote_rows_when_the_upstream_changes() {
            root.standOn({})
            level.holdShape()
            root.standOn({ remoteCounterpart: "origin/feature/other", remoteCounterpartOid: "999fed",
                           upstream: "origin/feature/other" }, "follow")
            compare(branchCard.state.remoteCounterpart, "origin/feature/topic-a", "the name stays the one shown")
            compare(branchCard.state.remoteCounterpartOid, "def456", "and so does the lease")
            compare(remoteDeleteRow.codeName, "origin/feature/topic-a")
            verify(remoteDeleteRow.withdrawn, "the remote delete greys where it stands")
            verify(bothDeleteRow.withdrawn, "and so does the pair")
            compare(branchCard.state.upstream, "origin/feature/other", "the upstream is the tree's")
            root.asked = ""
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a origin/feature/other true",
                    "and the upstream question starts from the reading as it is")
        }

        /// A remote branch's own delete is leased to the commit the row stood on as the menu went up: moved since, the
        /// menu stays up (デザイン規約 §hover「外乱で閉じない」) and the row keeps that commit and greys.
        function test_a_remote_branch_that_moved_keeps_its_delete_leased_and_greys_it() {
            const remote = { kind: "remote", name: "origin/main", full: "origin/main",
                             remoteCounterpart: "", remoteCounterpartOid: "", offers: ["delete"] }
            root.standOn(remote)
            level.holdShape()
            root.standOn(remote, "follow", "abc999")
            compare(branchCard.state.refOid, "abc123", "the lease stays the commit the row stood on")
            verify(deleteRow.withdrawn, "the delete greys where it stands")
            compare(deleteRow.blockedWhy, deleteRow.withdrawnWhy)
            root.standOn(remote, "follow")
            verify(deleteRow.withdrawn, "moved once is moved for the menu that showed it")

            level.releaseShape()
            root.standOn(remote, "", "abc999")
            compare(branchCard.state.refOid, "abc999", "a menu raised afresh leases where it is now")
            verify(deleteRow.offered)
        }

        /// The branch renamed with the menu up (`renamed`): the card goes on standing on it under the new name as the
        /// same branch — the lease over there kept, and git's answers about the plain delete holding, the commits being
        /// the same: the early check's under the name it was asked by, not asked again, so the row keeps its `-D`.
        function test_a_branch_renamed_with_the_menu_up_keeps_its_leases_under_the_new_name() {
            root.standOn({ merged: "" })
            branchCard.checkedBranch = "feature/topic-a"
            branchCard.checkedMerged = "no"
            compare(deleteRow.code, "branch -D", "git said the plain delete would be refused")
            level.holdShape()
            branchCard.renamed("feature/topic-a", "feature/topic-b")
            root.standOn({ name: "feature/topic-b", full: "feature/topic-b", merged: "",
                           remoteCounterpartOid: "999fed" }, "follow")
            compare(branchCard.refId, "feature/topic-b")
            compare(branchCard.state.remoteCounterpartOid, "def456", "the lease over there is kept")
            verify(remoteDeleteRow.withdrawn, "and that reading moved since, so the remote delete greys")
            compare(root.asked, "", "the merged check is not asked again")
            compare(deleteRow.code, "branch -D", "and its answer still stands under the new name")

            level.releaseShape()
            root.standOn({ merged: "" })
            root.refuse("feature/topic-a")
            branchCard.renamed("feature/topic-a", "feature/topic-b")
            compare(branchCard.forceDeleteBranch, "feature/topic-b", "git's refusal goes with the branch")
        }

        /// A rename names the branch the card stands on, or does nothing.
        function test_a_rename_of_another_branch_leaves_the_card_alone() {
            root.standOn({})
            branchCard.renamed("feature/other", "feature/else")
            compare(branchCard.refId, "feature/topic-a")
        }

        /// A command the menu waited for ends with what the rows reach over there moved — a fetch brought the remote
        /// branch, the reading or the far side somewhere else: the rows keep the leases the menu went up with and grey,
        /// as when that moves outside (デザイン規約 §メニュー「表示も押下も開いた時」). git's answer to the plain delete
        /// is asked again: the command may have merged the branch.
        function test_a_wait_that_ends_holds_the_leases_and_asks_git_again() {
            const remote = { kind: "remote", name: "origin/main", full: "origin/main",
                             remoteCounterpart: "", remoteCounterpartOid: "", offers: ["delete"] }
            root.standOn(remote)
            level.holdShape()
            root.standOn(remote, "wait", "abc999")
            compare(branchCard.state.refOid, "abc123", "the remote branch's own delete keeps its lease")
            verify(deleteRow.withdrawn, "and greys where it stands")

            level.releaseShape()
            root.standOn({ push: root.pushing("diverged") })
            level.holdShape()
            const moved = root.pushing("diverged")
            moved.lease = "fed999"
            root.standOn({ remoteCounterpartOid: "999fed", push: moved }, "wait")
            compare(branchCard.state.remoteCounterpartOid, "def456", "the reading's lease holds")
            verify(remoteDeleteRow.withdrawn, "the remote delete greys")
            compare(branchCard.state.push.lease, "fed789", "the push's lease holds")
            verify(pushRow.withdrawn, "the push greys")

            level.releaseShape()
            root.standOn({ merged: "" })
            root.refuse("feature/topic-a")
            level.holdShape()
            root.standOn({ merged: "" }, "wait")
            compare(root.asked, "check feature/topic-a", "git is asked again")
            compare(branchCard.forceDeleteBranch, "", "and its refusal was about the tree before the command")
        }

        /// A row with no chip whose name has gone with the menu up — the upstream unset outside — keeps its words
        /// without the name and its mark, greyed: the sentence goes with the name (`AppMenuItem.sentence`), and a row
        /// with no chip would be left blank.
        function test_a_row_with_no_chip_keeps_its_words_once_its_name_has_gone() {
            root.standOn({})
            branchCard.holdShape()
            root.standOn({ remoteCounterpart: "", remoteCounterpartOid: "", upstream: "", upstreamWhere: "",
                           offers: ["switch", "delete", "set-upstream"] }, "follow")
            const unset = branchCard.unsetUpstreamItem
            verify(unset.withdrawn, "greyed where it stands")
            compare(unset.sentence, "", "the sentence names nothing now")
            compare(unset.text, "Unset upstream", "the words stay")
            branchCard.releaseShape()
        }

        /// Presses under way are aimed at what the row acts on (`AppMenuItem.premise`): the menu standing again part
        /// way with that moved — the branch here went to another commit, so the pair would take commits the presses
        /// were not made for — the presses so far count for nothing, and the ones left over fire nothing.
        function test_presses_under_way_count_for_nothing_once_what_they_act_on_moves() {
            root.standOn({})
            level.holdShape()
            bothDeleteRow.holdDrive.begin()
            bothDeleteRow.holdDrive.letUp()
            root.standOn({}, "follow", "abc999")
            verify(bothDeleteRow.holdDrive.stale, "the presses were made for the branch where it was")
            for (let i = 1; i < Metrics.holdPressesWarned; i++) {
                bothDeleteRow.holdDrive.begin()
                bothDeleteRow.holdDrive.letUp()
            }
            compare(root.asked, "", "so the presses left over fire nothing")
        }

        /// Where the drawn rows cannot answer, git is asked — by request, like any other.
        function test_the_early_answer_comes_off_the_rows_or_is_asked_of_git() {
            root.standOn({ merged: "no" })
            verify(branchCard.deleteAnswered)
            verify(!branchCard.deleteMerged)
            verify(branchCard.deleteAsked)
            compare(root.asked, "", "the rows answered, so nobody was asked")
            compare(deleteRow.holdPresses, Metrics.holdPressesWarned,
                    "and the row is already the warning's three")

            root.standOn({ merged: "" })
            verify(!branchCard.deleteAnswered)
            verify(branchCard.deleteAsked)
            compare(root.asked, "check feature/topic-a")

            // git's late answer dresses the row the same way, by name.
            branchCard.checkedBranch = "feature/topic-a"
            branchCard.checkedMerged = "no"
            verify(deleteRow.holdPresses > 0)
            branchCard.checkedMerged = "unknown"
            compare(deleteRow.holdPresses, Metrics.holdPressesPlain, "a read that fell over is not a refusal")

            // And a card over a row nobody can delete asks nothing at all.
            root.standOn({ offers: root.current })
            verify(!branchCard.deleteAsked)
            compare(root.asked, "")
        }

        function test_each_row_asks_for_the_write_in_the_words_it_takes() {
            root.standOn({})
            // Pressed through as the plain delete: the menu stays up for git's answer (`BranchDeleteRow`'s `onHeld`).
            deleteRow.held()
            compare(root.asked, "delete branch feature/topic-a")

            root.standOn({})
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a origin/feature/topic-a true")

            root.standOn({})
            branchCard.unsetUpstreamItem.held()
            compare(root.asked, "unset feature/topic-a")

            // What reaches the remote is leased to the reading's own commit.
            root.standOn({})
            remoteDeleteRow.held()
            compare(root.asked, "delete-remote origin/feature/topic-a def456")

            root.standOn({})
            bothDeleteRow.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a false def456")

            // Once git has turned the local half down, the pair runs the forced form.
            root.standOn({})
            root.refuse("feature/topic-a")
            bothDeleteRow.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a true def456")

            // A remote row's own delete goes to the reading it names, leased to the commit the row stands on.
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", remoteCounterpartOid: "", offers: ["delete"] })
            deleteRow.held()
            compare(root.asked, "delete-remote origin/main abc123")
        }
    }
}

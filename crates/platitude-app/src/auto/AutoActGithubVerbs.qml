pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The links to GitHub's pages (デザイン規約 §GitHub のページへのリンク): the GITHUB card on a graph row's menu
/// (`github-card`), on the menu of a choice of two commits (`github-choice`), on a commit diff's selected lines
/// (`diff-github`), on the left menu's ref rows (`github-ref`) and REMOTES rows (`github-remote`), and on a commit's
/// changed file (`github-file`). Each stops with the card standing — and, given `others`, the card behind OTHER
/// REMOTES — and says which remote leads each link and which wait behind (`github_card leads= others=`): a row missing
/// for a remote that has the thing, and one standing for a remote that has not, draw alike.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a circle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var diffPane: driver.diffPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitRowMenu: driver.commitRowMenu
    readonly property var diffRowMenu: driver.diffRowMenu
    readonly property var refRowMenu: driver.refRowMenu
    readonly property var remoteRowMenu: driver.remoteRowMenu
    readonly property var commitFileMenu: driver.commitFileMenu
    readonly property var detailsPane: driver.detailsPane

    /// Runs `act` if it is one of this family's, and says whether it was (`AutoActDriver` asks each family in turn).
    function run(act, arg) {
        // `others` anywhere in the argument opens the second card too; the rest is the verb's own.
        const parts = arg === "" ? [] : arg.split(":")
        const others = parts.indexOf("others") >= 0
        const named = parts.filter(p => p !== "others")
        if (act === "github-card") {
            // `<row>`: the graph row the menu is raised on, its first chip the name the cards are about — as a
            // right-click anywhere along the row aims it (`RepoPage.openRowMenu`). On a tag's row, a fetch of its own
            // first (`fetchForTag`).
            cardTimer.row = named.length > 0 ? Number(named[0]) : 0
            cardTimer.others = others
            const chip = page.rowChipAt(graphModel.oidAt(cardTimer.row))
            cardTimer.fetched = acts.fetchForTag(chip !== null && chip !== undefined
                                                 && GitFacts.namedKind(chip.kind) === "tag")
            cardTimer.start()
        } else if (act === "github-ref") {
            // `<kind>:<name>`: the left menu raised on that row (`RepoPage.openRefMenu`) — `branch`, `remote` (its
            // name whole, `upstream/main`) or `tag` — with a tag's fetch first as `github-card` has.
            refTimer.kind = named.length > 0 ? named[0] : "branch"
            refTimer.name = named.slice(1).join(":")
            refTimer.others = others
            refTimer.fetched = acts.fetchForTag(refTimer.kind === "tag")
            refTimer.start()
        } else if (act === "github-remote") {
            // `<remote>`: the menu its REMOTES row raises (`RepoPage.openRemoteMenu`).
            remoteTimer.remote = named.length > 0 ? named[0] : "origin"
            remoteTimer.others = others
            remoteTimer.start()
        } else if (act === "github-file") {
            // `<row>:<path>`: the commit on that graph row, and a right-click on its changed file's row.
            fileTimer.begin(named.length > 0 ? Number(named[0]) : 0, named.slice(1).join(":"), others)
        } else if (act === "github-choice") {
            // `<row>:<row>`: a plain press on the first and a Ctrl press on the second, then a right-click on the
            // second through the row's own door (`GraphRowDelegate.askMenu`).
            choiceTimer.begin(named.length >= 2 ? [Number(named[0]), Number(named[1])] : [1, 4], others)
        } else if (act === "diff-github") {
            // `<row>:<path>`: the commit on that graph row, its file's diff, and a drag over its first lines.
            diffTimer.begin(named.length > 0 ? Number(named[0]) : 0, named.slice(1).join(":"), others)
        } else {
            return false
        }
        return true
    }

    /// Opens `card` off `menu` — and the card behind OTHER REMOTES where `others` asks — and says what stands.
    function stand(name, menu, card, others) {
        menu.openSub(card)
        if (others)
            card.openSub(card.othersCard)
        Harness.report(name + " opened=" + card.opened
                          + " leads=" + acts.spelled(card.leads)
                          + " others=" + acts.spelled(card.others))
        // The links themselves, to read: ids differ per machine, so nothing judges this line.
        Harness.report(name + "_urls " + card.leads.concat(card.others).map(row => row.url).join(" "))
    }
    /// Bracketed, so a line that wants these rows cannot pass on the prefix of one that has more.
    function spelled(rows) {
        return "[" + rows.map(row => row.kind + "@" + row.remote).join(",") + "]"
    }
    /// Whether the run's own fetch is through where one went out, and nothing is out that would raise a menu greyed
    /// (`AutoActDriver.menusLive`), named as `verb`'s stage where not.
    function menuLive(verb, fetched) {
        return Awaited.all(verb, { "fetch": !fetched || driver.wroteAndSettled(), "live": driver.menusLive() })
    }
    /// Which remotes carry a tag is read off every remote's listing, which only a fetch brings, and the opening's may
    /// not be through yet (`tagMenuTimer`'s `read`): on a tag, a fetch of its own goes out. Says whether one did.
    function fetchForTag(onTag) {
        if (onTag)
            driver.pressWrite("fetch", () => {
                driver.repoTab.fetch("")
                return true
            })
        return onTag
    }

    SampleTimer {
        id: cardTimer
        property int row: 0
        property bool others: false
        /// Whether a fetch went out for this run, to wait out.
        property bool fetched: false
        onTriggered: {
            if (!acts.menuLive("github_card", cardTimer.fetched))
                return
            cardTimer.stop()
            page.openRowMenu(graphModel.oidAt(cardTimer.row))
            acts.stand("github_card", commitRowMenu.menu, commitRowMenu.githubCard, cardTimer.others)
            driver.complete()
        }
    }

    // The same waits as `cardTimer`'s, and the name on its own section's rows.
    SampleTimer {
        id: refTimer
        property string kind: ""
        property string name: ""
        property bool others: false
        property bool fetched: false
        onTriggered: {
            const section = refTimer.kind === "tag" ? driver.tagsModel
                          : refTimer.kind === "remote" ? driver.remotesModel : driver.branchesModel
            const oid = section.oidOfName(refTimer.name)
            if (!acts.menuLive("github_ref", refTimer.fetched) || !Awaited.all("github_ref", { "row": oid !== "" }))
                return
            refTimer.stop()
            // The left menu's own door, as its row's right-click takes it.
            page.openRefMenu(refTimer.kind, refTimer.name, refTimer.name, oid, true)
            acts.stand("github_ref", refRowMenu.menu, refRowMenu.githubCard, refTimer.others)
            driver.complete()
        }
    }

    SampleTimer {
        id: remoteTimer
        property string remote: ""
        property bool others: false
        onTriggered: {
            if (!acts.menuLive("github_remote", false))
                return
            remoteTimer.stop()
            page.openRemoteMenu(remoteTimer.remote)
            acts.stand("github_remote", remoteRowMenu.menu, remoteRowMenu.githubCard, remoteTimer.others)
            driver.complete()
        }
    }

    // The row pressed and its card read, then the right-click on the file's row once the list has built it — the row's
    // own door (`FileRowDelegate.askMenu`), which reads the file first — and the card once that read has landed.
    SampleTimer {
        id: fileTimer
        property int row: 0
        property string path: ""
        property bool others: false
        property int step: 0
        function begin(row, path, others) {
            fileTimer.row = row
            fileTimer.path = path
            fileTimer.others = others
            fileTimer.step = 0
            fileTimer.start()
        }
        onTriggered: {
            if (fileTimer.step === 0) {
                Awaited.at("github_file", "row")
                const item = graphPane.view.itemAtIndex(fileTimer.row)
                if (!item) {
                    graphPane.view.positionViewAtIndex(fileTimer.row, ListView.Contain)
                    return
                }
                item.leftClick(Qt.NoModifier)
                fileTimer.step = 1
                return
            }
            if (fileTimer.step === 1) {
                const file = detailsPane.filesWalk.rowFor("", fileTimer.path)
                if (!Awaited.all("github_file", {
                    "card": driver.cardSettled && detailsModel.shaHex === graphModel.oidAt(fileTimer.row),
                    "file": file !== null
                }))
                    return
                file.askMenu()
                fileTimer.step = 2
                return
            }
            // The menu's read is a git subprocess away (`RepoPage.openCommitFileMenu`): ended before it lands, the run
            // is photographed and recorded over whichever of the diff's rows the scheduler let in. With no menu standing
            // there is no card to wait for: the line below says so, and the run is judged on it.
            if (!Awaited.all("github_file", {
                "diff": !commitFileMenu.showing || (page.diffPath === fileTimer.path && diffPane.diffSettled())
            }))
                return
            fileTimer.stop()
            Harness.report("github_file_menu open=" + commitFileMenu.showing + " read=" + page.diffPath)
            acts.stand("github_file", commitFileMenu.menu, commitFileMenu.githubCard, fileTimer.others)
            driver.complete()
        }
    }

    // The two presses go in one per tick, as `graph-choose` puts them; the right-click waits for the right pane to
    // compare the two — the menu takes them in its order (`DetailsModel.compareFrom` / `compareTo`).
    SampleTimer {
        id: choiceTimer
        property var rows: []
        property int step: 0
        property bool others: false
        function begin(rows, others) {
            choiceTimer.rows = rows
            choiceTimer.step = 0
            choiceTimer.others = others
            choiceTimer.start()
        }
        onTriggered: {
            const want = choiceTimer.rows[Math.min(choiceTimer.step, choiceTimer.rows.length - 1)]
            const item = graphPane.view.itemAtIndex(want)
            // A row far down has no delegate until the view is over it: sent there, and pressed on a later tick.
            if (!item) {
                Awaited.at("github_choice", "row " + want)
                graphPane.view.positionViewAtIndex(want, ListView.Contain)
                return
            }
            if (choiceTimer.step < choiceTimer.rows.length) {
                Awaited.at("github_choice", "press " + choiceTimer.step)
                item.leftClick(choiceTimer.step === 0 ? Qt.NoModifier : Qt.ControlModifier)
                choiceTimer.step++
                return
            }
            if (!Awaited.all("github_choice", { "chosen": page.chosenCount === 2,
                                                 "comparing": detailsModel.comparing }))
                return
            choiceTimer.stop()
            item.askMenu()
            Harness.report("github_choice_menu choice=" + commitRowMenu.choiceShowing + " chosen=" + page.chosenCount)
            acts.stand("github_choice", commitRowMenu.choiceLevel, commitRowMenu.choiceGithubCard, choiceTimer.others)
            driver.complete()
        }
    }

    // The row pressed, its card read, the file asked for, and its rows laid out before the drag — each from the one
    // that answers it.
    SampleTimer {
        id: diffTimer
        property int row: 0
        property string path: ""
        property bool others: false
        property int step: 0
        function begin(row, path, others) {
            diffTimer.row = row
            diffTimer.path = path
            diffTimer.others = others
            diffTimer.step = 0
            diffTimer.start()
        }
        onTriggered: {
            if (diffTimer.step === 0) {
                Awaited.at("diff_github", "row")
                const item = graphPane.view.itemAtIndex(diffTimer.row)
                if (!item) {
                    graphPane.view.positionViewAtIndex(diffTimer.row, ListView.Contain)
                    return
                }
                item.leftClick(Qt.NoModifier)
                diffTimer.step = 1
                return
            }
            if (diffTimer.step === 1) {
                // The card of the row pressed, not of a row before it: a root commit's has no parent to wait for.
                if (!Awaited.all("diff_github", {
                    "card": driver.cardSettled && detailsModel.shaHex === graphModel.oidAt(diffTimer.row)
                }))
                    return
                page.openDiff("commit", diffTimer.path, "")
                diffTimer.step = 2
                return
            }
            // The rows are in once a changed line is.
            if (!Awaited.all("diff_github", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                return
            diffTimer.stop()
            // From the head of the first line, under the heading, to the end of the last: the lines a link spans.
            const last = diffPane.view.count - 1
            diffPane.pickText(0, 1, 0, last, driver.pastLineEnd)
            diffPane.askCodeMenu(0, last, 0)
            const span = diffPane.diffModel.selectionLines()
            const lines = span === undefined ? "none" : (span.newer ? "new " : "old ") + span.first + "-" + span.last
            Harness.report("diff_github_menu open=" + diffRowMenu.showing + " lines=" + lines)
            acts.stand("diff_github", diffRowMenu.menu, diffRowMenu.githubCard, diffTimer.others)
            driver.complete()
        }
    }
}

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
            refTimer.refName = named.slice(1).join(":")
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

    /// Opens `card` off `menu` — and the card behind OTHER REMOTES where `others` asks.
    function stand(menu, card, others) {
        menu.openSub(card)
        if (others)
            card.openSub(card.othersCard)
    }
    /// What stands on `card`, as `name`'s lines: which remote leads each link and which wait behind, then the links
    /// themselves — ids differ per machine, so nothing judges that second line.
    function standLines(name, card) {
        return [name + " opened=" + card.opened
                + " leads=" + acts.spelled(card.leads)
                + " others=" + acts.spelled(card.others),
                name + "_urls " + card.leads.concat(card.others).map(row => row.url).join(" ")]
    }
    /// Bracketed, so a line that wants these rows cannot pass on the prefix of one that has more.
    function spelled(rows) {
        return "[" + rows.map(row => row.kind + "@" + row.remote).join(",") + "]"
    }
    /// Whether the run's own fetch is through where one went out, and nothing is out that would raise a menu greyed
    /// (`AutoActDriver.menusLive`), named as `verb`'s observation where not.
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

    PageStage {
        id: cardTimer
        name: "github_card"
        order: ["opened"]
        property int row: 0
        property bool others: false
        /// Whether a fetch went out for this run, to wait out.
        property bool fetched: false
        onTriggered: {
            if (!Awaited.all("github_card_ready", { "ready": acts.menuLive("github_card", cardTimer.fetched) }))
                return
            cardTimer.input("opened", () => {
                page.openRowMenu(graphModel.oidAt(cardTimer.row))
                acts.stand(commitRowMenu.menu, commitRowMenu.githubCard, cardTimer.others)
            })
            cardTimer.complete(() => acts.standLines("github_card", commitRowMenu.githubCard))
        }
    }

    // The same waits as `cardTimer`'s, and the name on its own section's rows.
    PageStage {
        id: refTimer
        name: "github_ref"
        order: ["opened"]
        property string kind: ""
        /// The ref's name, whole.
        property string refName: ""
        property bool others: false
        property bool fetched: false
        onTriggered: {
            const section = refTimer.kind === "tag" ? driver.tagsModel
                          : refTimer.kind === "remote" ? driver.remotesModel : driver.branchesModel
            const oid = section.oidOfName(refTimer.refName)
            if (!acts.menuLive("github_ref", refTimer.fetched) || !Awaited.all("github_ref", { "row": oid !== "" }))
                return
            refTimer.input("opened", () => {
                // The left menu's own door, as its row's right-click takes it (`area`: the panel's rows).
                page.openRefMenu(refTimer.kind, refTimer.refName, refTimer.refName, oid, "sidebar")
                acts.stand(refRowMenu.menu, refRowMenu.githubCard, refTimer.others)
            })
            refTimer.complete(() => acts.standLines("github_ref", refRowMenu.githubCard))
        }
    }

    PageStage {
        id: remoteTimer
        name: "github_remote"
        order: ["opened"]
        property string remote: ""
        property bool others: false
        onTriggered: {
            if (!Awaited.all("github_remote_ready", { "ready": acts.menuLive("github_remote", false) }))
                return
            remoteTimer.input("opened", () => {
                page.openRemoteMenu(remoteTimer.remote)
                acts.stand(remoteRowMenu.menu, remoteRowMenu.githubCard, remoteTimer.others)
            })
            remoteTimer.complete(() => acts.standLines("github_remote", remoteRowMenu.githubCard))
        }
    }

    // The row pressed and its card read, then the right-click on the file's row once the list has built it — the row's
    // own door (`FileRowDelegate.askMenu`), which reads the file first — and the card once that read has landed.
    PageStage {
        id: fileTimer
        name: "github_file"
        order: ["row_clicked", "menu_asked", "opened"]
        property int row: 0
        property string path: ""
        property bool others: false
        function begin(row, path, others) {
            fileTimer.row = row
            fileTimer.path = path
            fileTimer.others = others
            fileTimer.start()
        }
        onTriggered: {
            const op = fileTimer
            if (op.at("start")) {
                const item = graphPane.view.itemAtIndex(op.row)
                if (!Awaited.all("github_file_row", { "row": item !== null })) {
                    graphPane.view.positionViewAtIndex(op.row, ListView.Contain)
                    return
                }
                op.input("row_clicked", () => {
                    item.leftClick(Qt.NoModifier)
                })
                return
            }
            if (op.at("row_clicked")) {
                const file = detailsPane.filesWalk.rowFor("", op.path)
                if (!Awaited.all("github_file", {
                    "card": driver.cardSettled && detailsModel.shaHex === graphModel.oidAt(op.row),
                    "file": file !== null
                }))
                    return
                op.input("menu_asked", () => {
                    file.askMenu()
                })
                return
            }
            // The menu's read is a git subprocess away (`RepoPage.openCommitFileMenu`): ended before it lands, the run
            // is photographed and recorded over whichever of the diff's rows the scheduler let in. With no menu
            // standing there is no card to wait for: the line below says so, and the run is judged on it.
            if (!Awaited.all("github_file", {
                "diff": !commitFileMenu.showing || (page.diffPath === op.path && diffPane.diffSettled()),
                "page": PageSettled.settled(page)
            }))
                return
            op.input("opened", () => {
                acts.stand(commitFileMenu.menu, commitFileMenu.githubCard, op.others)
            })
            op.complete(() => ["github_file_menu open=" + commitFileMenu.showing + " read=" + page.diffPath]
                              .concat(acts.standLines("github_file", commitFileMenu.githubCard)))
        }
    }

    // The two presses go in one per tick, as `graph-choose` puts them; the right-click waits for the right pane to
    // compare the two — the menu takes them in its order (`DetailsModel.compareFrom` / `compareTo`).
    PageStage {
        id: choiceTimer
        name: "github_choice"
        order: ["first_press", "second_press", "opened"]
        /// The two rows: pressed plain, then with Ctrl; the menu is raised on the second.
        property var rows: []
        property bool others: false
        function begin(rows, others) {
            choiceTimer.rows = rows
            choiceTimer.others = others
            choiceTimer.start()
        }
        onTriggered: {
            const op = choiceTimer
            const pressing = op.at("start") ? 0 : op.at("first_press") ? 1 : -1
            const want = op.rows[pressing < 0 ? op.rows.length - 1 : pressing]
            const item = graphPane.view.itemAtIndex(want)
            if (pressing >= 0) {
                // A row far down has no delegate until the view is over it: sent there, and pressed on a later tick.
                // The row built is what the press stands on, and the observation is grounded on it by name.
                if (!Awaited.all("github_choice_row_" + want, { "row": item !== null })) {
                    graphPane.view.positionViewAtIndex(want, ListView.Contain)
                    return
                }
                op.input(pressing === 0 ? "first_press" : "second_press", () => {
                    item.leftClick(pressing === 0 ? Qt.NoModifier : Qt.ControlModifier)
                })
                return
            }
            if (!Awaited.all("github_choice", { "row": item !== null,
                                                 "chosen": page.chosenCount === 2,
                                                 "comparing": detailsModel.comparing,
                                                 "page": PageSettled.settled(page) }))
                return
            op.input("opened", () => {
                item.askMenu()
                acts.stand(commitRowMenu.choiceLevel, commitRowMenu.choiceGithubCard, op.others)
            })
            op.complete(() => ["github_choice_menu choice=" + commitRowMenu.choiceShowing
                               + " chosen=" + page.chosenCount]
                              .concat(acts.standLines("github_choice", commitRowMenu.choiceGithubCard)))
        }
    }

    // The row pressed, its card read, the file asked for, and its rows laid out before the drag — each from the one
    // that answers it.
    PageStage {
        id: diffTimer
        name: "diff_github"
        order: ["row_clicked", "diff_opened", "picked"]
        property int row: 0
        property string path: ""
        property bool others: false
        function begin(row, path, others) {
            diffTimer.row = row
            diffTimer.path = path
            diffTimer.others = others
            diffTimer.start()
        }
        onTriggered: {
            const op = diffTimer
            if (op.at("start")) {
                const item = graphPane.view.itemAtIndex(op.row)
                if (!Awaited.all("diff_github_row", { "row": item !== null })) {
                    graphPane.view.positionViewAtIndex(op.row, ListView.Contain)
                    return
                }
                op.input("row_clicked", () => {
                    item.leftClick(Qt.NoModifier)
                })
                return
            }
            if (op.at("row_clicked")) {
                // The card of the row pressed, not of a row before it: a root commit's has no parent to wait for.
                if (!Awaited.all("diff_github", {
                    "card": driver.cardSettled && detailsModel.shaHex === graphModel.oidAt(op.row)
                }))
                    return
                op.input("diff_opened", () => {
                    page.openDiff("commit", op.path, "")
                })
                return
            }
            // The rows are in once a changed line is.
            if (!Awaited.all("diff_github", { "rows": diffPane.firstChangedLine(0) >= 0 }))
                return
            op.input("picked", () => {
                // From the head of the first line, under the heading, to the end of the last: the lines a link spans.
                const last = diffPane.view.count - 1
                diffPane.pickText(0, 1, 0, last, driver.pastLineEnd)
                diffPane.askCodeMenu(0, last, 0)
                acts.stand(diffRowMenu.menu, diffRowMenu.githubCard, op.others)
            })
            op.complete(() => {
                const span = diffPane.diffModel.selectionLines()
                const lines = span === undefined ? "none"
                                                 : (span.newer ? "new " : "old ") + span.first + "-" + span.last
                return ["diff_github_menu open=" + diffRowMenu.showing + " lines=" + lines]
                       .concat(acts.standLines("diff_github", diffRowMenu.githubCard))
            })
        }
    }
}

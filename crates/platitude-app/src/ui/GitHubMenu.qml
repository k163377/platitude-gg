pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// GitHub's pages for what the carrying menu stands on, one row a link put on the clipboard (デザイン規約 §GitHub の
// ページへのリンク). One card at every entrance: the graph row's menu, the menu on a choice of two commits, the diff's
// own text, the left menu's ref and remote rows, and a commit's changed files. Each link leads with one remote — the
// window's own where it has the thing, else the first by name (`GitFacts.linkLead`) — and the other remotes that have
// it wait behind OTHER REMOTES.
//
// It reads no model: the carrying menu works out which remotes have what (`GitHubLinks`) and hands the answer to
// `standOn` as the menu opens, so the rows are fixed before the card shows (デザイン規約 §メニュー).
AppMenu {
    id: githubMenu

    /// The rows, made by `standOn`: `{kind, sentence, name, mark, remote, url}` each, in the order the links were
    /// handed.
    property var leads: []
    property var others: []
    /// Automation: the card behind OTHER REMOTES, for `AppMenu.openSub`.
    readonly property alias othersCard: othersMenu

    /// A link on its way to the clipboard; the page owns the clipboard.
    signal copyRequested(string text)

    /// Stands the card on `links` — `{kind, holders, preferred, lines}` each, `holders` the remotes that have it as
    /// `{remote, url, name}` in name order, `name` what the link points at as that remote calls it, `lines` the lines'
    /// numbers of a `lines` link — leading each with its `preferred` where that is among them. A link no remote has
    /// has no row.
    function standOn(links) {
        const leads = []
        const others = []
        for (let i = 0; i < links.length; i++) {
            const link = links[i]
            const lead = GitFacts.linkLead(link.preferred, link.holders.map(h => h.remote))
            for (let j = 0; j < link.holders.length; j++) {
                const holder = link.holders[j]
                const row = {
                    "kind": link.kind,
                    "sentence": githubMenu.sentenceFor(link.kind, link.lines),
                    "name": holder.name,
                    // A ref's name wears its kind's mark, a repository its remote's; a commit's id and a pair of
                    // them wear none (デザイン規約 §ref の種別「ref の名前の印」).
                    "mark": link.kind === "branch" || link.kind === "tag" ? link.kind
                          : link.kind === "repo" ? "remote" : "",
                    "remote": holder.remote,
                    "url": holder.url
                }
                if (holder.remote === lead)
                    leads.push(row)
                else
                    others.push(row)
            }
        }
        githubMenu.leads = leads
        githubMenu.others = others
    }

    /// What a row says it puts on the clipboard, the thing's name in `%1` and the remote in `%2` — both as code, each
    /// behind its mark (`AppMenuItem.refName` / `refName2`): "Copy link to 1c57829b on origin". The name says what it
    /// is by its own dress — a commit's id, a ref's mark, a remote's cloud — so the kind takes no word, but for a
    /// pair, for lines and for a file, which a commit's id alone does not say.
    function sentenceFor(kind, lines) {
        switch (kind) {
        //: "Copy link to 1c57829b on ☁origin" — a commit, a branch or a tag, then the remote.
        case "commit":
        case "branch":
        case "tag": return qsTr("Copy link to %1 on %2")
        //: The two commits as git's range: "Copy link to comparison c0b4c03b..5f7336af on ☁origin".
        case "compare": return qsTr("Copy link to comparison %1 on %2")
        //: The lines' numbers (%3, one or a run), the commit and the remote: "Copy link to lines 1–2 at 5f7336af on
        //: ☁origin".
        case "lines":
            return (lines.indexOf("–") >= 0 ? qsTr("Copy link to lines %3 at %1 on %2")
                                            : qsTr("Copy link to line %3 at %1 on %2")).replace("%3", lines)
        //: A changed file as it stands in a commit: "Copy link to file at 5f7336af on ☁origin".
        case "file": return qsTr("Copy link to file at %1 on %2")
        //: A remote's repository, the remote alone: "Copy link to ☁origin".
        case "repo": return qsTr("Copy link to %1")
        }
        return ""
    }

    // The host, in capitals as the other cards' titles name what they act on (`RefBranchMenu`'s BRANCH), behind
    // GitHub's own mark in the light ink GitHub's logo terms allow.
    title: qsTr("GITHUB")
    titleKind: "github"
    titleTint: Theme.textPrimary
    applies: githubMenu.leads.length > 0
    // No row here is held, so the card fits its rows (`AppMenu.widthFloor`).
    widthFloor: 0

    // Names in no place of the reader's own, so the plain white (`refTint`). Reading what is on screen only, so a
    // running command holds none of them.
    Instantiator {
        model: githubMenu.leads
        delegate: AppMenuItem {
            id: leadRow
            required property var modelData
            refSentence: leadRow.modelData.sentence
            refName: leadRow.modelData.name
            refMark: leadRow.modelData.mark
            refTint: Theme.textPrimary
            refName2: leadRow.modelData.remote
            refMark2: "remote"
            startsCommand: false
            onTriggered: githubMenu.copyRequested(leadRow.modelData.url)
        }
        onObjectAdded: (at, object) => githubMenu.insertItem(at, object)
        onObjectRemoved: (at, object) => githubMenu.removeItem(object)
    }
    AppMenuSeparator {}
    AppMenu {
        id: othersMenu
        title: qsTr("OTHER REMOTES")
        titleKind: "remote"
        applies: githubMenu.others.length > 0
        widthFloor: 0
        Instantiator {
            model: githubMenu.others
            delegate: AppMenuItem {
                id: otherRow
                required property var modelData
                refSentence: otherRow.modelData.sentence
                refName: otherRow.modelData.name
                refMark: otherRow.modelData.mark
                refTint: Theme.textPrimary
                refName2: otherRow.modelData.remote
                refMark2: "remote"
                startsCommand: false
                onTriggered: githubMenu.copyRequested(otherRow.modelData.url)
            }
            onObjectAdded: (at, object) => othersMenu.insertItem(at, object)
            onObjectRemoved: (at, object) => othersMenu.removeItem(object)
        }
    }
}

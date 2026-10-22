import QtQuick
import platitude
import platitude.ui

// ---- links to GitHub's pages (デザイン規約 §GitHub のページへのリンク) ----
// Which GitHub remotes have what a menu stands on, and the address of its page on each — worked out as the menu opens
// and handed to its GITHUB card (`GitHubMenu.standOn`), so every entrance offers a thing on the same remotes in the
// same order. Each answer is a list of links, `{kind, preferred, holders[, lines]}`: `holders` are the remotes that
// have the thing as `{remote, url, name}` in name order, `name` what the link points at as that remote calls it.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」).
QtObject {
    id: links

    required property RepoTab repoTab
    /// Which remotes have a commit is read off the rows on screen (`GraphModel.remotesHolding`).
    required property GraphModel graphModel
    /// Where each remote's branches stand, and which a remote has by name.
    required property NavSectionModel remotesModel
    /// A local branch's upstream, the name it has on the remote it tracks.
    required property NavSectionModel branchesModel
    /// Which remotes carry a tag, and whether their reading agrees.
    required property NavSectionModel tagsModel

    /// The remotes whose URL names a repository on GitHub, in name order, each with that repository's page
    /// (`GitFacts.githubRepo`).
    function remotes() {
        const names = links.repoTab.remoteNames
        const hubs = []
        for (let i = 0; i < names.length; i++) {
            const repo = GitFacts.githubRepo(links.repoTab.remoteUrl(names[i]))
            if (repo !== "")
                hubs.push({ "remote": names[i], "repo": repo })
        }
        return hubs
    }
    /// Which of `hubs` already have commit `oidHex`, read off the rows on screen (`GraphModel.remotesHolding`).
    function holding(hubs, oidHex) {
        // The row's own mark first: a commit no remote has asks for no tips and no walk.
        if (!links.graphModel.publishedAt(oidHex))
            return []
        const tips = []
        for (let i = 0; i < hubs.length; i++) {
            const on = links.remotesModel.tipsOn(hubs[i].remote, links.repoTab.remoteNames)
            for (let j = 0; j < on.length; j++)
                tips.push({ "oid": on[j], "owner": i })
        }
        return links.graphModel.remotesHolding(oidHex, tips).map(at => hubs[at])
    }
    /// A commit as the rows name it (`GraphRowItem.short_sha`).
    function shortId(oidHex) {
        return oidHex.substring(0, 8)
    }

    /// What a commit and the ref named on it offer — a graph row and the chip it draws, or a left menu's ref row and
    /// the commit it points at: the commit's page on each GitHub remote that has it, and the ref's page on each that
    /// has that name — a branch under the name it has there (`GitFacts.githubBranchName`: a local branch's upstream
    /// may be called otherwise), a tag where that remote's reading agrees. `kind` is `branch` / `remote` / `tag`, or
    /// anything else for no ref. Every link but a remote branch's leads with the window's own remote
    /// (`RepoTab.defaultRemote`); a remote branch's with its own remote.
    function ofRef(oidHex, kind, name) {
        const hubs = links.remotes()
        if (hubs.length === 0)
            return []
        const own = links.repoTab.defaultRemote
        const names = links.repoTab.remoteNames
        const found = [{
            "kind": "commit",
            "preferred": own,
            "holders": links.holding(hubs, oidHex).map(h => ({
                "remote": h.remote,
                "url": GitFacts.githubCommitLink(h.repo, oidHex),
                "name": links.shortId(oidHex)
            }))
        }]
        if (kind === "branch" || kind === "remote") {
            const upstream = kind === "branch" ? links.branchesModel.upstreamOf(name) : ""
            const holders = []
            for (let i = 0; i < hubs.length; i++) {
                const there = GitFacts.githubBranchName(kind, name, upstream, hubs[i].remote, names)
                if (there !== "" && links.remotesModel.oidOfName(hubs[i].remote + "/" + there) !== "")
                    holders.push({ "remote": hubs[i].remote, "url": GitFacts.githubBranchLink(hubs[i].repo, there),
                                   "name": there })
            }
            found.push({ "kind": "branch", "preferred": kind === "remote" ? GitFacts.remoteOfRef(name, names) : own,
                         "holders": holders })
        } else if (kind === "tag") {
            const carriers = links.tagsModel.tagRemotes(name, own)
            const holders = []
            for (let i = 0; i < hubs.length; i++) {
                for (let j = 0; j < carriers.length; j++) {
                    if (carriers[j].remote === hubs[i].remote && !carriers[j].apart)
                        holders.push({ "remote": hubs[i].remote, "url": GitFacts.githubTagLink(hubs[i].repo, name),
                                       "name": name })
                }
            }
            found.push({ "kind": "tag", "preferred": own, "holders": holders })
        }
        return found
    }
    /// What a choice of two commits offers: their comparison as `git diff <from> <to>` takes it, on each GitHub
    /// remote that has both.
    function ofPair(fromHex, toHex) {
        const hubs = links.remotes()
        if (hubs.length === 0)
            return []
        const to = links.holding(hubs, toHex).map(h => h.remote)
        return [{
            "kind": "compare",
            "preferred": links.repoTab.defaultRemote,
            "holders": links.holding(hubs, fromHex).filter(h => to.includes(h.remote)).map(h => ({
                "remote": h.remote,
                "url": GitFacts.githubCompareLink(h.repo, fromHex, toHex),
                // git's own spelling of the pair, oldest first.
                "name": links.shortId(fromHex) + ".." + links.shortId(toHex)
            }))
        }]
    }
    /// What selected lines of a commit's diff offer: lines `first`–`last` of `path` in commit `atHex`, on each GitHub
    /// remote that has commit `heldHex` — the commit the diff is of, which brings its parents along.
    function ofLines(heldHex, atHex, path, first, last) {
        const hubs = links.remotes()
        if (hubs.length === 0)
            return []
        return [{
            "kind": "lines",
            "preferred": links.repoTab.defaultRemote,
            // The lines' numbers, said in the sentence; the name is the commit whose file they are
            // (`GitHubMenu.sentenceFor`).
            "lines": last > first ? first + "–" + last : String(first),
            "holders": links.holding(hubs, heldHex).map(h => ({
                "remote": h.remote,
                "url": GitFacts.githubLinesLink(h.repo, atHex, path, first, last),
                "name": links.shortId(atHex)
            }))
        }]
    }
    /// What a row of a commit's changed files offers: `path` as it stands in commit `atHex`, on each GitHub remote
    /// that has commit `heldHex`, as `ofLines` takes them.
    function ofFile(heldHex, atHex, path) {
        const hubs = links.remotes()
        if (hubs.length === 0)
            return []
        return [{
            "kind": "file",
            "preferred": links.repoTab.defaultRemote,
            "holders": links.holding(hubs, heldHex).map(h => ({
                "remote": h.remote,
                "url": GitFacts.githubFileLink(h.repo, atHex, path),
                "name": links.shortId(atHex)
            }))
        }]
    }
    /// What a REMOTES row offers: that remote's repository page, where its URL names one on GitHub.
    function ofRemote(remote) {
        const hub = links.remotes().find(h => h.remote === remote)
        if (hub === undefined)
            return []
        return [{
            "kind": "repo",
            "preferred": remote,
            "holders": [{ "remote": remote, "url": hub.repo, "name": remote }]
        }]
    }
}

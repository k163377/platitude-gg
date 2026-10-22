pragma Singleton

import QtQuick
import platitude

// Wording that more than one place has to say the same way — a second copy of a sentence drifts from the first.
QtObject {
    /// The application's name, as the window title and both empty states write it.
    readonly property string appName: qsTr("Platitude GG")

    /// How many presses run a confirmed control, as the sentence saying so opens ("Press three times to …"). Every such
    /// sentence takes the count from here, handed `Metrics`' own number. The words are for the two counts `Metrics`
    /// holds (`holdPressesPlain` / `holdPressesWarned`); a count other than those needs its words here first.
    function pressCount(presses) {
        return presses === 2 ? qsTr("Press twice") : qsTr("Press three times")
    }

    /// A confirmed control's answer to a screen reader, wherever a HoldDriver drives one: how many presses run it.
    function pressToActivate(presses) {
        return qsTr("%1 to activate").arg(Words.pressCount(presses))
    }

    /// The menu-row tag on anything that replays commits a remote already has — the amend-family rows and both
    /// rebase entries (デザイン規約 §回数押し).
    readonly property string rewritesPushed: qsTr("rewrites pushed commits")

    /// Starting a branch where the row stands, from the graph row's menu and the sidebar's ref menu
    /// (デザイン規約 §メニュー「入口が違っても同じ操作は同じ文」); the TAG card's `Create tag here…` (`RefTagMenu`) is
    /// worded to match.
    readonly property string createBranchHere: qsTr("Create branch here…")
    /// The same box's other road: a new branch on this commit, out in a worktree of its own — from both menus
    /// (デザイン規約 §worktree を作る).
    readonly property string createWorktreeHere: qsTr("Create worktree here…")

    /// A name box's refusal of a name git's rules turn down — a branch's (`GitFacts.validBranchName`) or a tag's
    /// (`GitFacts.validTagName`), in the graph's box, the left menu's and the new worktree's alike.
    readonly property string notAName: qsTr("git will not take this as a name")
    /// A rename box rubbed out — every rename box's refusal of nothing, the left menu's and the graph's alike.
    readonly property string nameNeeded: qsTr("A name is needed")

    /// Why a name cannot stand beside the branches or tags already there (`NavSectionModel.nameClash`, undefined where
    /// nothing is in the way): one of that name, one under it or above it — one sentence each way, naming which of
    /// the two would have to be the folder — or one that differs only in letter case. `tag` picks the noun. Said
    /// before the press by every box that makes or renames a branch or a tag (デザイン規約 §左メニューの所作「名前の衝突」).
    function nameClashed(clash, tag) {
        if (clash === undefined)
            return ""
        switch (clash.kind) {
        case "taken": return tag ? qsTr("A tag called that already exists")
                                 : qsTr("A branch called that already exists")
        //: %1 is the branch or tag under the typed name: `a` typed beside `a/b`.
        case "holds": return qsTr("%1 already uses this name as a folder").arg(clash.with)
        //: %1 is the branch or tag the typed name would go inside: `a/b` typed beside `a`.
        case "under": return tag ? qsTr("%1 is already a tag and cannot also be a folder").arg(clash.with)
                                 : qsTr("%1 is already a branch and cannot also be a folder").arg(clash.with)
        //: %1 is the branch or tag whose name differs from the typed one only in letter case.
        case "case": return qsTr("%1 differs only in letter case — some disks take it for the same name")
                                .arg(clash.with)
        default: return ""
        }
    }

    /// The branch or tag that sentence names, behind its mark (デザイン規約 §ref の種別「名前の印」, `marksOf`).
    function clashMarks(clash, tag) {
        if (clash === undefined || clash.with === undefined || clash.with === "")
            return []
        return Words.marksOf(name => Words.nameClashed({ kind: clash.kind, with: name }, tag),
                             [[clash.with, tag ? "tag" : "branch"]])
    }
    /// A remote branch renamed to a name its remote already carries — both rename boxes' refusal.
    function remoteHasBranch(remote) {
        return qsTr("%1 already has a branch called that").arg(remote)
    }
    /// The names a rename or new-name box's refusal `why` says, behind their marks: the clash it was written from
    /// (`nameClashed`, `tag` its noun), or the remote a remote branch's rename asks (`remoteHasBranch`). None for a
    /// refusal that is neither.
    function refusalMarks(why, clash, tag, remote) {
        if (why === "")
            return []
        if (remote !== "" && why === Words.remoteHasBranch(remote))
            return Words.marksOf(Words.remoteHasBranch, [[remote, "remote"]])
        return why === Words.nameClashed(clash, tag) ? Words.clashMarks(clash, tag) : []
    }

    /// Why the name typed into the new-worktree box cannot be taken — the box's tip in the graph and in the left
    /// menu alike, everything git would refuse that this end can see, said before the press. `clash` is what
    /// `NavSectionModel.nameClash` answered for it among the branches; `place` what `NavSectionModel.newWorktreeFor`
    /// answered. Empty while nothing is typed: a box opened empty would turn the reader's arrival down
    /// (デザイン規約 §可否・警告の出し場所).
    function worktreeNameRefused(typed, clash, place) {
        const name = typed.trim()
        if (name === "")
            return ""
        if (!GitFacts.validBranchName(name))
            return Words.notAName
        const clashed = Words.nameClashed(clash, false)
        if (clashed !== "")
            return clashed
        return place === undefined ? "" : Words.worktreePlaceTaken(place.taken, place.name)
    }

    /// Why a new worktree cannot go where it would, said by the `worktree add` row's hover and the name box's
    /// tip alike (`NavSectionModel.newWorktreeFor`'s `taken`): what is in the way, then the worktree's folder behind a
    /// dash, where the tip stands the tree mark (`worktreeNameMark`, デザイン規約 §ref の種別「名前の印」).
    function worktreePlaceTaken(taken, name) {
        switch (taken) {
        //: %1 is the folder the new worktree would be made in, named as the worktree git still lists there.
        case "listed": return qsTr("Taken by another worktree — %1").arg(name)
        //: %1 is the folder the new worktree would be made in.
        case "folder": return qsTr("The folder is not empty — %1").arg(name)
        default: return ""
        }
    }
    /// The word a worktree-making tip stands the tree mark in front of: the folder, where `why` is the sentence
    /// `worktreePlaceTaken` wrote about `place` — asked of the sentence shown, as `RefBranchMenu`'s delete rows ask.
    function worktreeNameMark(why, place) {
        return place !== undefined && why !== "" && why === Words.worktreePlaceTaken(place.taken, place.name)
               ? place.name : ""
    }

    /// Why a worktree's folder cannot take the name typed into its rename box — the graph's box and the left menu's
    /// alike, said before the press. `place` is what `NavSectionModel.renamedWorktreeFor` answered for it (undefined
    /// for a name that is not one folder's). A rename rubbed out is refused, as every rename box's is.
    function worktreeRenameRefused(typed, place) {
        if (typed.trim() === "")
            return Words.nameNeeded
        if (place === undefined)
            return qsTr("Not a folder name")
        switch (place.taken) {
        // What Windows keeps out of a name (`worktrees::Disk`): git would turn it down, or — a device's name — leave
        // the worktree in a folder nothing reaches.
        //: %1 is the character Windows does not allow, such as ?.
        case "character": return qsTr("Not a folder name on Windows — %1 is not allowed").arg(place.kept)
        //: %1 is the device name Windows keeps, such as CON.
        case "device": return qsTr("Not a folder name on Windows — %1 is kept for a device").arg(place.kept)
        // A dot only: the boxes ask with the spaces at the ends cut off, so a trailing space never reaches here.
        case "trailing": return qsTr("Not a folder name on Windows — it cannot end in a dot")
        case "case": return qsTr("Only the letter case differs — on this disk that is the same folder")
        case "listed": return Words.worktreePlaceTaken("listed", place.name)
        //: %1 is the folder the worktree would be renamed to. Anything there is in the way, an empty folder too.
        case "folder": return qsTr("Something is already there — %1").arg(place.name)
        default: return ""
        }
    }
    /// The word that tip stands the tree mark in front of: the folder, where the sentence names one.
    function worktreeRenameMark(why, place) {
        return place !== undefined && why !== "" && (place.taken === "listed" || place.taken === "folder")
               && why === Words.worktreeRenameRefused(place.name, place) ? place.name : ""
    }
    /// Why the spaced second click on a worktree opened no box (`GitFacts.worktreeRename`): what cannot be done —
    /// nothing was sent, so not "not renamed", which is the word for a press git turned down — then what the worktree
    /// `name` is — locked, or missing (git's word for a worktree whose folder is gone). The name
    /// wears the tree mark (`SharedToolTip.tipMarkWord`); a lock's own reason is not quoted — it is whatever the person
    /// who locked it typed, and says nothing of the rename.
    function worktreeRenameWithheld(answer, name) {
        switch (answer) {
        //: %1 is the worktree's folder.
        case "locked": return qsTr("Cannot rename — %1 is locked").arg(name)
        //: %1 is the worktree's folder, which is gone from the disk.
        case "gone": return qsTr("Cannot rename — %1 is missing").arg(name)
        default: return ""
        }
    }
    /// Why an amend cannot be pressed when what is staged takes back all the last commit changed — git refuses that
    /// amend, so the commit button and the details pane's save say so first (`RepoTab.amendEmpties`).
    readonly property string amendEmpties: qsTr("Amending would leave the last commit empty — what is staged undoes it")

    /// Opening a repository, as the graph's empty state, the tab strip's `+` menu and its accessible name offer it.
    readonly property string openRepository: qsTr("Open repository…")

    /// Adding a remote, as the sidebar header, the collapsed rail and the publish flow offer it.
    readonly property string addRemote: qsTr("Add remote…")

    /// The command log's own name — its heading (`CommandsToggle`) and the word every tip that sends a reader there
    /// points at.
    readonly property string commandsTitle: qsTr("GIT COMMANDS")
    /// The name the command log answers to inside a tooltip's markup (`CardText.linkAsked` →
    /// `RepoPage.tipLinkAsked`). Not translated: the tip's writer and the page that reads it back must spell it the
    /// same in every language. The scheme is this application's own, so nothing else takes it for an address to open.
    readonly property string commandsHref: "pgg:commands"

    /// The way out that writes nothing, at the three dialogs' foot. Dialogs only: the plan has no box to dismiss, so
    /// its way out names what goes (`RebasePlanPane` — `Discard`).
    readonly property string cancel: qsTr("Cancel")

    /// Why a menu row is out while a git command runs (`AppMenuItem.blockedWhy`, デザイン規約 §無効). Not "git": the
    /// row under the pointer wears a git command of its own on its chip, and `running` is what tells the two apart.
    readonly property string otherCommandRunning: qsTr("Wait for the running command")

    /// Why every holder of a tag wears the warning when the remotes carry it on different commits and the remote this
    /// window acts on is not among them: no reading is the right one (`differsFrom` has nobody to name). Said where
    /// that one is — a tag's holders on the left panel and in a chip's card (`NavFacts.apartNote`).
    readonly property string remotesDisagree: qsTr("Remotes disagree")

    /// The sentence after the `merge` and `pull` chips, naming the branch that takes the commits in — the current one
    /// for a merge, the one that moves for a pull (`AppMenuItem.refSentence`; デザイン規約 §履歴を合流させる).
    //: Follows the `merge` or `pull` chip: "merge into main", "pull into release". %1 is a branch name.
    readonly property string intoBranch: qsTr("into %1")

    /// Why the `pull` row is greyed while both sides have moved, from both entrances (`RefRowMenu` / `CommitRowMenu`;
    /// デザイン規約 §取り込んで合流させる).
    readonly property string pullDiverged: qsTr(
        "The branches have diverged, so pick the remote branch and rebase onto it")
    /// …and while a branch the worktree is not on has commits its upstream lacks: a fast-forward is all a pull does
    /// from here. What cannot happen, then why, the way other clients put it (lazygit, GitLens); `upstream` is what it
    /// tracks.
    function pullOwnCommits(upstream) {
        //: %1 is the remote branch this branch tracks, e.g. origin/feature/login.
        return qsTr("Cannot fast-forward — it has commits not on %1").arg(upstream)
    }
    /// Why the `pull` row is greyed, by the word `GitFacts.pullRow` answered after `out-`: the tree diverged, the row's
    /// branch has commits its `upstream` lacks, or another worktree has it out — `leaf` is that worktree's folder.
    /// Empty where the row is live.
    function pullOut(out, leaf, upstream) {
        switch (out) {
        case "diverged": return Words.pullDiverged
        case "own": return Words.pullOwnCommits(upstream)
        case "held": return Words.checkedOutElsewhere(leaf)
        default: return ""
        }
    }

    /// Why a rename box turns its branch down: a rebase or bisect stopped in some worktree holds it — this window's own
    /// too — and git renames none such (`NavSectionModel.heldByOperation`). The state and where, as a greyed row says
    /// it (デザイン規約 §hover のツールチップ); another worktree by the name it goes by, behind a dash where the tip stands
    /// the tree mark.
    function heldByOperation(by, here, leaf) {
        if (by === "bisect") {
            if (here)
                return qsTr("Being bisected in this worktree")
            //: %1 is the name of the other worktree the bisect stopped in.
            return qsTr("Being bisected in another worktree — %1").arg(leaf)
        }
        if (here)
            return qsTr("Being rebased in this worktree")
        //: %1 is the name of the other worktree the rebase stopped in.
        return qsTr("Being rebased in another worktree — %1").arg(leaf)
    }

    /// Why a row acting on a branch another worktree has checked out is greyed — the branch card's delete rows and the
    /// `pull` row. The state and where, not why (デザイン規約 §hover のツールチップ); the name the worktree goes by — its
    /// folder, or the branch the repository's own has out — not the whole path git prints: nobody reads a tooltip
    /// that wide.
    function checkedOutElsewhere(leaf) {
        //: %1 is the name of the other worktree that has this branch checked out: its folder, or for the repository's
        //: own worktree the branch it has out.
        return qsTr("Checked out in another worktree — %1").arg(leaf)
    }

    /// The window band's badge words (BandStateGroup measures and draws them, BandStateCard titles them);
    /// the WIP pane's conflicts bucket header shares the first.
    readonly property string badgeConflicts: qsTr("CONFLICTS")
    readonly property string badgeSetIdentity: qsTr("SET IDENTITY")
    readonly property string badgeOldGit: qsTr("OLD GIT")
    /// What is drawn is out of date: the walk gave up part-way, or the rebuild that would refresh the rows did not
    /// land. The badge is the only place this state is said (デザイン規約 §ウィンドウの縁).
    readonly property string badgeStaleGraph: qsTr("STALE GRAPH")
    /// Pending files need Git LFS and git cannot run it here (デザイン規約 §ウィンドウの縁).
    readonly property string badgeNoLfs: qsTr("NO LFS")

    /// The one way a commit's moment is written, from the epoch seconds rows carry. The commands panel's `HH:mm:ss`
    /// clock stays its own.
    function stamp(epochSeconds) {
        return Qt.formatDateTime(new Date(epochSeconds * 1000), "yyyy-MM-dd HH:mm")
    }
    /// The day that stamp opens with: what a line short of room cuts it back to (`CommitAuthorRow.dateDayRoom`). The
    /// time goes as one — a stamp cut through its figures reads as another time.
    function stampDay(epochSeconds) {
        return Qt.formatDateTime(new Date(epochSeconds * 1000), "yyyy-MM-dd")
    }

    /// Why a copy of a name wears the warning, or why a row cannot reach the other copy: it stands on another commit
    /// than `reference` has it on. One sentence wherever two copies of one name disagree — a tag's holders on the left
    /// panel and in a chip's card (`NavFacts.apartNote`), and the delete rows that reach a remote on the branch card
    /// and the tag card, named there by what the copy the menu is about is read against (デザイン規約 §左メニューの所作
    /// の削除の表). Says the state only: a line pointing at the row that deletes the reading would outgrow the menu it
    /// explains.
    function differsFrom(reference) {
        return qsTr("Differs from %1").arg(reference)
    }
    /// Why a tag menu's rows that delete over there stand greyed when several remotes carry the name and none of them
    /// is the one this window acts on: no one of them can be picked unasked, and the menu names who they are — each is
    /// reached from its own line (デザイン規約 §左メニューの所作 の削除の表). `carriers` as a sentence lists them.
    function severalRemotesHave(carriers) {
        return qsTr("Several remotes have it: %1").arg(carriers)
    }

    /// What to call a side git left nothing to name (デザイン規約 §conflict の ours / theirs). The names swap over
    /// during a rebase, so they are handed in from `WorktreeModel`.
    function ourSide(name) {
        return name !== "" ? name : qsTr("this branch")
    }
    function theirSide(name) {
        return name !== "" ? name : qsTr("the incoming side")
    }

    /// Why a folder would not open, from the kind core answered with (`plain` / `bare` / `untrusted` / `config` /
    /// `other`), for both the picker's dialog and a tab's failed screen (デザイン規約 §リポジトリを開く). `untrusted`
    /// is a folder another account owns (`safe.directory`), whose heading is this application's and whose way out —
    /// the `git config --global --add safe.directory` line — is in git's own words underneath. `config` is a
    /// configuration file git could not open, named in git's words underneath. `other` says only that much: what
    /// happened is git's to say, quoted underneath. `plain`'s heading is git's own words for it (setup.c `not a git
    /// repository`, as 2.43.0 to 2.56.0 write them).
    function openFailure(kind) {
        switch (kind) {
        case "bare": return qsTr("A bare repository has nothing to show")
        case "untrusted": return qsTr("git does not trust this folder's owner")
        case "config": return qsTr("git could not read its configuration")
        case "other": return qsTr("Could not open this folder")
        default: return qsTr("Not a git repository")
        }
    }

    /// What to do before the folder can open, where the kind has a step of the reader's own; empty for the rest. It
    /// stands between the path and git's words, which name the file.
    function openFailureNext(kind) {
        if (kind !== "config")
            return ""
        return qsTr("Fix what keeps git from reading the file named below, then open the folder again")
    }

    /// Whether the folder's failure comes with git's own words to show under the heading (`openFailure`).
    function openFailureQuotesGit(kind) {
        return kind === "other" || kind === "untrusted" || kind === "config"
    }

    /// What kept a remote out of reach, from the word core answered with (`remote::Unreached`) — the command log's
    /// header line in this application's words, where git's own would name a URL and a terminal's prompt. Every
    /// other failure's header line is git's first line (デザイン規約 §git が言ったことを読む場所).
    function unreachedWhy(kind) {
        switch (kind) {
        case "credentials":
            return qsTr("The remote asked for credentials, which this window cannot type — sign in once from a terminal")
        case "ssh-key":
            return qsTr("The remote did not accept this machine's SSH key")
        case "host-key":
            return qsTr("SSH has not met this remote's host key yet — connect once from a terminal")
        default: return ""
        }
    }

    /// Why a picture was not filed against an address, from the kind core answered with (`avatar::AvatarRefusal`).
    /// `facts` carries the numbers the sentence takes, in order; `said` is the operating system's own line for
    /// `read` / `write`, carried untouched under a frame written here, and empty for the rest
    /// (rules-refs/app-ui.md「Rust に文言を置かない」).
    ///
    /// The pixel refusal names the dimensions: the ceiling a reader was told about is the file size, so a small file
    /// turned down has no explanation without them.
    function avatarFailure(kind, facts, said) {
        switch (kind) {
        case "too-large":
            return qsTr("This file is larger than %1 MB.").arg(facts[0])
        case "unreadable":
            return qsTr("This file is not a PNG or a JPEG that can be read.")
        case "too-many-pixels":
            return qsTr("This file is %1, past the %2 megapixels this can take.")
                     .arg(facts[0]).arg(facts[1])
        case "unstorable":
            return qsTr("The picture could not be stored.")
        case "no-store":
            return qsTr("There is nowhere to keep avatars on this machine.")
        case "read":
            return qsTr("This file could not be read.") + (said === "" ? "" : "\n" + said)
        case "write":
            return qsTr("The picture could not be written.") + (said === "" ? "" : "\n" + said)
        default: return ""
        }
    }

    /// The heading for a write that did not happen, from the kind core answered with (`RepoTab.writeReportKind`).
    /// Whoever decided is the subject, and a branch and a tag share one sentence (デザイン規約 §答えの要らない報せ).
    function writeReported(kind, remote, name) {
        // The far side's refusals are two words, what was asked and what for (`drain::report_word`): the heading turns
        // on the first.
        if (kind.startsWith("delete-"))
            return qsTr("%1 would not delete %2").arg(remote).arg(name)
        if (kind.startsWith("update-"))
            return qsTr("%1 would not update %2").arg(remote).arg(name)
        switch (kind) {
        // A branch and a tag alike, whatever stopped them; the line underneath says what did.
        //: %1 is a branch or a tag, %2 a remote.
        case "pre-push":
        case "outdated":
        case "moved":
        case "tag-elsewhere": return qsTr("%1 was not sent to %2").arg(name).arg(remote)
        //: %1 is a branch or a tag, %2 a remote.
        case "pre-push-delete":
        case "moved-delete": return qsTr("%1 was not deleted from %2").arg(name).arg(remote)
        case "stale-stage":
        case "stage-locked": return qsTr("Nothing was staged")
        case "stale-unstage":
        case "unstage-locked": return qsTr("Nothing was unstaged")
        case "stale-discard":
        case "discard-locked": return qsTr("Nothing was discarded")
        case "conflicted-part": return qsTr("Nothing was taken from this file")
        case "rename":
        case "rename-locked": return qsTr("%1 was not renamed").arg(name)
        case "half-rename":
        case "rename-unfinished": return qsTr("The rename did not finish")
        // `stash@{n}` is no name the screen shows, and the message the row shows can be long.
        case "stash-rename":
        case "stash-rename-locked":
        case "stash-gone": return qsTr("The stash was not renamed")
        case "worktree-kept":
        case "worktree-locked":
        case "worktree-refused": return qsTr("%1 was not removed").arg(name)
        case "worktree-half": return qsTr("%1 was not removed completely").arg(name)
        case "worktree-not-added":
        case "worktree-folder-taken":
        case "worktree-place-taken":
        case "worktree-branch-left": return qsTr("%1 was not created").arg(name)
        // git kept the worktree and the tab stands in it, as for any made one — opened says made — and the hook is what
        // failed.
        case "worktree-hook": return qsTr("%1 was opened in this tab, but its post-checkout hook failed").arg(name)
        // Another worktree's changes, applied here from its uncommitted row: `name` is the name that worktree goes by —
        // its folder, or the branch the repository's own has out (`RepoPage.showReport`).
        case "carried-in-the-way":
        case "carried-file-in-the-way":
        case "carried-unread":
        case "carried-refused":
        case "carried-locked": return qsTr("Changes from %1 were not applied").arg(name)
        case "worktree-not-renamed":
        case "worktree-rename-taken": return qsTr("%1 was not renamed").arg(name)
        // Every rewrite refusal: which one it was is the line underneath.
        case "across-merge":
        case "off-branch":
        case "fold-first":
        case "unfetched-base":
        case "drop-all":
        case "tip-moved":
        case "op-standing":
        case "pre-rebase":
        case "rewrite-unsigned":
        case "rewrite-commit-hook":
        case "rewrite-locked":
        case "rewrite-untracked": return Words.historyNotRewritten
        default: return qsTr("The commit was not made")
        }
    }

    /// The row menu's heading for every rewrite refusal, and `planRefused`'s fallback.
    readonly property string historyNotRewritten: qsTr("The history was not rewritten")

    /// The heading at the plan's door, where a preview turns the range down before it opens
    /// (`RepoPage.onRefusedPlan`). Unlike the row menu's one heading, each shape is written out: no pressed row is on
    /// screen to name it (デザイン規約 §答えの要らない報せ).
    function planRefused(kind) {
        switch (kind) {
        case "across-merge":
            return qsTr("A merge is in the way")
        case "off-branch":
            return qsTr("Not on this branch")
        case "unfetched-base":
            return qsTr("The history stops here")
        // Unreachable: `rebase_plan::drain::refusal_kind` matches `PlanRefusal` exhaustively, so a fourth shape stops
        // the build there.
        default: return Words.historyNotRewritten
        }
    }

    /// The second line of every report: always this application's own words, never anybody's message
    /// (デザイン規約 §答えの要らない報せ). Where git ran and something said no — a hook, a signing key, another git, a rule
    /// over there — the line says who, and sends the reader to the log, where that row has what they said
    /// (`reportPlace`). `remote` and `name` are the heading's; `fact` the one thing core hands over besides
    /// (`WriteReport::fact`). The refusals a read answers say what the remote turned out to be, and no more: the graph
    /// shows the rest once the read that follows has landed.
    function writeReportedWhy(kind, remote, name, fact) {
        if (kind.startsWith("delete-") || kind.startsWith("update-"))
            return Words.remoteRefusedWhy(kind, remote, name)
        switch (kind) {
        // Not git's own advice under the refusal: it ends in `git pull`, and the session is fetching already.
        //: %1 is a remote.
        case "outdated":
            return qsTr("%1 has commits that this push would drop.").arg(remote)
        // A lease the remote has left — the ref moved, or went.
        //: %1 is a remote.
        case "moved":
        case "moved-delete":
            return qsTr("%1 has changed since the last fetch.").arg(remote)
        //: %1 is a remote, %2 a tag.
        case "tag-elsewhere":
            return qsTr("%1 already has %2 on another commit.").arg(remote).arg(name)
        // Run before git talked to the remote, which heard nothing.
        //: %1 is the command log's own title.
        case "pre-push":
        case "pre-push-delete":
            return qsTr("The pre-push hook turned it down — see %1").arg(Words.commandsTitle)
        // `pre-commit`, `commit-msg`, `prepare-commit-msg` alike — and a reword's or a replayed commit's, in a
        // rewrite put back where it began.
        //: %1 is the command log's own title.
        case "commit-hook":
        case "rewrite-commit-hook":
            return qsTr("A commit hook turned it down — see %1").arg(Words.commandsTitle)
        // A key gone, a signing program that failed: what the program said is the log's.
        //: %1 is the command log's own title.
        case "commit-unsigned":
        case "rewrite-unsigned":
            return qsTr("The commit could not be signed — see %1").arg(Words.commandsTitle)
        //: %1 is the command log's own title.
        case "pre-rebase":
            return qsTr("The pre-rebase hook turned it down — see %1").arg(Words.commandsTitle)
        // git's first checkout would write over files it does not track; its words list them. Said as the other tools
        // say it, by what would be lost — and in git's words for it (unpack-trees.c `untracked working tree files would
        // be overwritten by checkout`, as 2.43.0 to 2.56.0 write them).
        //: %1 is the command log's own title.
        case "rewrite-untracked":
            return qsTr("Untracked files would be overwritten — see %1").arg(Words.commandsTitle)
        case "stale-stage":
        case "stale-unstage":
        case "stale-discard":
            return qsTr("The file changed on disk since these rows were read. It has been read again — make the selection on the rows that are there now.")
        case "conflicted-part":
            return qsTr("This file is still conflicted. It has to be resolved before parts of it can be taken.")
        // Both names stand. What stopped the second half — git turning the old name's removal down or a lock it named
        // — is the step's row in the log; a stash list that moved leaves only a `rev-parse` answering another entry.
        //: %1 is the command log's own title.
        case "half-rename":
            return qsTr("The new name was made, but the old one is still there — see %1").arg(Words.commandsTitle)
        // Dropped or popped elsewhere since it was chosen.
        case "stash-gone":
            return qsTr("It is no longer in the stash list.")
        // A lock file another git held at that moment, or one a crashed git left behind — git's own words leave both
        // open, and so does this line: trying again is the answer to the first, and the log's line names the file for
        // the second. The words are git's (lockfile.c `unable_to_lock_message`, as 2.43.0 to 2.56.0 write them).
        //: %1 is the command log's own title.
        case "rename-locked":
        case "commit-locked":
        case "stash-rename-locked":
        case "rewrite-locked":
        case "carried-locked":
        case "stage-locked":
        case "unstage-locked":
        case "discard-locked":
            return qsTr("Another git process seems to be running — see %1").arg(Words.commandsTitle)
        // What git said in words of its own that none of the above name. A rename's name was answered before the
        // press, so git turned down something else — a name taken in the moment since, a branch being rebased or
        // bisected.
        //: %1 is the command log's own title.
        case "rename":
        case "commit":
        case "stash-rename":
        case "worktree-refused":
        case "worktree-not-added":
            return qsTr("git turned it down — see %1").arg(Words.commandsTitle)
        // Locked after the menu opened. The reason is the lock's owner's own words (`worktree lock --reason`), as the
        // row's tip says it before the press.
        //: %1 is the reason the worktree was locked with.
        case "worktree-locked":
            return fact === "" ? qsTr("It is locked.") : qsTr("It is locked: %1.").arg(fact)
        // git goes on past what it could not delete — in the worktree's folder (a file held open on Windows, a path
        // too long) or its own record of it — and names the folder, not the file.
        //: %1 is the command log's own title.
        case "worktree-half":
            return qsTr("git could not delete everything — see %1").arg(Words.commandsTitle)
        // git still lists this place for a worktree whose folder went (`worktree prune` would clear it).
        case "worktree-place-taken":
            return qsTr("It is taken by another worktree.")
        // git made the `-b` branch before it turned the rest down, and does not take it back.
        //: %1 is a branch, %2 the command log's own title.
        case "worktree-branch-left":
            return qsTr("The branch %1 was made anyway — see %2").arg(fact).arg(Words.commandsTitle)
        //: %1 is the command log's own title.
        case "worktree-hook":
            return qsTr("See %1 for what the hook said.").arg(Words.commandsTitle)
        // git moves the ref first: the branch has its new name, and what follows it — its settings, another
        // worktree's HEAD — stayed behind.
        //: %1 is the command log's own title.
        case "rename-unfinished":
            return qsTr("The branch has its new name, but git stopped before the rest moved — see %1")
                .arg(Words.commandsTitle)
        // Only when git said it was the changes: its own words end in advice to force it, which nothing here offers.
        case "worktree-kept":
            return qsTr("It has uncommitted changes.")
        // Turned down here before git ran (`worktrees::add`): nobody to quote. The heading names the folder; this
        // says what was in the way, as the row's tip did before the press (`worktreePlaceTaken`).
        case "worktree-folder-taken":
            return qsTr("The folder is not empty.")
        // git's own words list the files and end in advice to the terminal; the log keeps them. What stood in the way
        // is this worktree's own work: a file changed here that they change too, or one standing where they put a
        // file — said as the other tools say it, by what would have been lost, in git's `would be overwritten`
        // (unpack-trees.c, as 2.43.0 to 2.56.0 write it).
        case "carried-in-the-way":
            return qsTr("Uncommitted changes in this worktree would be overwritten.")
        // Turned down before git: a file here — committed or not — stands where one of their untracked files goes.
        case "carried-file-in-the-way":
            return qsTr("Files in this worktree would be overwritten.")
        // There: a conflict standing, a file git would not take, or the worktree gone — found before anything was
        // applied.
        case "carried-unread":
            return qsTr("Not all of its changes could be read.")
        // Anything else git turned down (a conflict standing here, untracked files it would remove): its words say why,
        // so the line names where they are, as the fetch's tip does (デザイン規約 §リモートから取り込む).
        //: %1 is the command log's own title.
        case "carried-refused":
            return qsTr("git did not apply them — see %1").arg(Words.commandsTitle)
        // The boxes answered what they know before the press (one folder's name the system allows, free, not another
        // worktree's), so git turned down the rest — submodules, a lock taken since, a program holding the folder. Its
        // words are the log's (デザイン規約 §答えの要らない報せ).
        //: %1 is the command log's own title.
        case "worktree-not-renamed":
            return qsTr("git turned it down — see %1").arg(Words.commandsTitle)
        // Turned down here before git ran (`worktrees::move_to`): git moves a worktree into a folder already there.
        case "worktree-rename-taken":
            return qsTr("Something is already there.")
        // The rewrite refusals live in `rewriteRefusedWhy`, which the plan's door reads too. Delegated whole: a
        // second copy of that list gets a new refusal on one side only.
        default: return Words.rewriteRefusedWhy(kind)
        }
    }

    /// The far side's refusals, by what for — the second word of `delete-…` / `update-…` (`drain::report_word`): what
    /// its rules or its own settings are, in this application's words. Its own words are the log's: a forge's are
    /// several sentences, git's own end in advice to whoever runs the server.
    function remoteRefusedWhy(kind, remote, name) {
        switch (kind.substring(kind.indexOf("-") + 1)) {
        // A hook over there — `pre-receive`, `update`, a forge's protected branch.
        //: %1 is a remote, %2 the command log's own title.
        case "rule":
            return qsTr("A rule on %1 turned it down — see %2").arg(remote).arg(Words.commandsTitle)
        // `receive.denyNonFastForwards`: forced or not, nothing it holds is dropped.
        //: %1 is a remote, %2 the command log's own title.
        case "rewrite":
            return qsTr("%1 does not take pushes that rewrite its history — see %2").arg(remote)
                .arg(Words.commandsTitle)
        //: %1 is a remote, %2 the command log's own title.
        case "no-deletes":
            return qsTr("%1 does not let branches be deleted — see %2").arg(remote).arg(Words.commandsTitle)
        // A repository with a working tree of its own: a clone on the same disk, a shared folder.
        //: %1 is a remote, %2 a branch, %3 the command log's own title.
        case "checked-out":
            return qsTr("%1 has %2 checked out in a working tree of its own — see %3").arg(remote).arg(name)
                .arg(Words.commandsTitle)
        // The branch its HEAD names.
        //: %1 is a branch, %2 a remote, %3 the command log's own title.
        case "current":
            return qsTr("%1 is the current branch of %2 — see %3").arg(name).arg(remote).arg(Words.commandsTitle)
        //: %1 is a remote, %2 the command log's own title.
        case "too-large":
            return qsTr("The push is larger than %1 accepts — see %2").arg(remote).arg(Words.commandsTitle)
        //: %1 is a remote, %2 the command log's own title.
        case "shallow":
            return qsTr("%1 does not take pushes from a shallow clone — see %2").arg(remote).arg(Words.commandsTitle)
        // The far side's git said its lock was taken: the words of `rename-locked` and the rest above.
        //: %1 is a remote, %2 the command log's own title.
        case "locked":
            return qsTr("Another git process seems to be running on %1 — see %2").arg(remote)
                .arg(Words.commandsTitle)
        //: %1 is a remote, %2 the command log's own title.
        default:
            return qsTr("%1 turned it down — see %2").arg(remote).arg(Words.commandsTitle)
        }
    }

    /// Why a rewrite was turned down — the one line both doors share: the row menu (`writeReportedWhy`) and the plan
    /// (`RepoPage.onRefusedPlan`), which reaches three of the five (デザイン規約 §答えの要らない報せ).
    ///
    /// Written in the words of what is on screen — never `rebase`, since nothing was run (デザイン規約 §用語(UI 文言)).
    function rewriteRefusedWhy(kind) {
        switch (kind) {
        case "across-merge":
            return qsTr("A merge sits in the history this would replay, and a replay drops merges. What came back would be flattened.")
        case "off-branch":
            return qsTr("This commit is not in the current branch's history. Switch to a branch that has it first.")
        case "fold-first":
            return qsTr("This is the first commit, so there is nothing before it to fold into.")
        case "unfetched-base":
            return qsTr("The commit below this one is not in this clone. Replaying from here would cut the branch off from the rest of its history.")
        case "drop-all":
            return qsTr("This is the last commit, and a branch cannot be left with no history at all.")
        // The two about right now: something outside this window moved between the press and git. One sentence of
        // cause each — a longer line wraps and pushes history rows off the screen (デザイン規約 §答えの要らない報せ).
        case "tip-moved":
            return qsTr("The branch moved after this was worked out.")
        case "op-standing":
            return qsTr("An operation is in progress. Finish it or put it down first.")
        default: return ""
        }
    }

    /// The word of a report's second line that is a place to go — the command log, for the reports whose line sends
    /// the reader there (デザイン規約 §答えの要らない報せ) — and `""` for the rest. Its href is `commandsHref`. Read off
    /// the line itself, so a line that names the log is never left without the link.
    function reportPlace(kind) {
        return Words.writeReportedWhy(kind, "", "", "").indexOf(Words.commandsTitle) >= 0 ? Words.commandsTitle : ""
    }

    /// Whether a report's heading names a worktree — the name then wears the tree mark (`reportMarks`,
    /// デザイン規約 §ref の種別「名前の印」).
    function reportNamesWorktree(kind) {
        return kind.startsWith("worktree-") || kind.startsWith("carried-")
    }

    /// Whether a rename git turned down leaves its box open holding the name, to try again: nothing moved, and the
    /// row the box stands on is still there (デザイン規約 §答えの要らない報せ「改名の箱は書き込みが着いてから閉じる」).
    function renameStillOpen(kind) {
        return kind === "rename" || kind === "rename-locked"
               || kind === "stash-rename" || kind === "stash-rename-locked"
    }

    /// The colour a report's own hairline wears: `warning` while the gesture is still going (the rename box is open,
    /// or a half-done rename is left standing), `danger` once it is over (デザイン規約 §答えの要らない報せ). A worktree
    /// git would not remove is `warning` too: the worktree still stands, with what kept it there. So is one that was
    /// not made, or made with its hook failed: what stood in its way was there before the press, which the menus warn
    /// of (デザイン規約 §答えの要らない報せ). So are another worktree's changes not applied: nothing was written, and they
    /// are still there to try again.
    function reportTone(kind) {
        return Words.renameStillOpen(kind) || kind === "half-rename" || kind === "rename-unfinished"
               || Words.reportNamesWorktree(kind) ? "warning" : "danger"
    }

    /// What a branch loses, said of the branch: the line under the question that moves it onto another
    /// (`Move <branch> here?`) and under the first push that overwrites it (デザイン規約 §長さ「失うもの 1 行」).
    /// A sentence for each count — `commit(s)` would reach the reader as written.
    function losesCommits(name, count) {
        return count === 1 ? qsTr("%1 loses %n commit.", "", count).arg(name)
                           : qsTr("%1 loses %n commits.", "", count).arg(name)
    }

    /// What the two sides each did to a conflicted file, from git's two stage letters (デザイン規約 §conflict の種別).
    function conflict(change, ours, theirs) {
        const us = Words.ourSide(ours)
        const them = Words.theirSide(theirs)
        switch (change) {
        case "UU": return qsTr("Both changed it")
        case "AA": return qsTr("Both added it")
        case "DD": return qsTr("Both deleted it")
        case "DU": return qsTr("Deleted on %1, changed on %2").arg(us).arg(them)
        case "UD": return qsTr("Changed on %1, deleted on %2").arg(us).arg(them)
        case "AU": return qsTr("Added on %1 only").arg(us)
        case "UA": return qsTr("Added on %1 only").arg(them)
        default: return qsTr("Conflicted")
        }
    }

    /// What a change did to a file's line endings, from the pieces `DiffModel` took the notice apart into; `""` when
    /// there is nothing to say. Each branch is a whole sentence, never stitched from fragments
    /// (デザイン規約 §改行コードの警告).
    function lineEndings(kind, from, to, lines, scope, ext) {
        switch (kind) {
        case "flipped":
            return qsTr("Line endings change · %1 → %2").arg(from).arg(to)
        case "mixed":
            // One line is not "1 added lines", and a translator cannot fix that from the outside.
            return lines === 1
                ? qsTr("Mixed line endings · 1 added line uses %1, this file uses %2").arg(from).arg(to)
                : qsTr("Mixed line endings · %1 added lines use %2, this file uses %3").arg(lines).arg(from).arg(to)
        case "new":
            switch (scope) {
            case "here":
                return qsTr("New file uses %1 · other .%2 files here look like %3").arg(from).arg(ext).arg(to)
            case "ext":
                return qsTr("New file uses %1 · other .%2 files look like %3").arg(from).arg(ext).arg(to)
            default:
                return qsTr("New file uses %1 · other files in this repo look like %2").arg(from).arg(to)
            }
        case "first":
            switch (scope) {
            case "here":
                return qsTr("First line ending in this file · %1 · other .%2 files here look like %3")
                    .arg(from).arg(ext).arg(to)
            case "ext":
                return qsTr("First line ending in this file · %1 · other .%2 files look like %3")
                    .arg(from).arg(ext).arg(to)
            default:
                return qsTr("First line ending in this file · %1 · other files in this repo look like %2")
                    .arg(from).arg(to)
            }
        default:
            return ""
        }
    }

    /// The search's note when nothing answers and the whole history is loaded (デザイン規約 §コミットを探す).
    readonly property string findNone: qsTr("No results in the whole history")
    /// The same when older commits are not loaded: the press that loads the next `step` (`findLoadFurther`, the words a
    /// press acts on — the graph's tail says the same, and the count it was cut at).
    function findNoneWithMore(step) {
        return qsTr("No results · %1").arg(Words.findLoadFurther(step))
    }
    function findLoadFurther(step) {
        return qsTr("Load %L1 more commits").arg(step)
    }

    /// A file's mode change as its one line (`DiffModel.modeFrom` / `modeTo`), each side as `modeSide` says it
    /// (デザイン規約 §mode の変更).
    function modeLine(from, to) {
        return qsTr("File mode changed · %1 → %2").arg(Words.modeSide(from)).arg(Words.modeSide(to))
    }
    /// The same off a row of a reading of several commits, which carries the two modes as git prints them, a colon
    /// apart (`encode::flatten_patches`).
    function modeLineOf(pair) {
        const sides = pair.split(":")
        return sides.length === 2 ? Words.modeLine(sides[0], sides[1]) : pair
    }
    /// One side of a mode change (`DiffModel.modeFrom` / `modeTo`): the octal as git prints it, and what it means for
    /// the one permission git records — whether the file runs (デザイン規約 §mode の変更).
    function modeSide(mode) {
        switch (mode) {
        case "100755": return qsTr("%1 (executable)").arg(mode)
        case "100644": return qsTr("%1 (not executable)").arg(mode)
        default: return mode
        }
    }

    /// The line saying why a diff offers its whole file only (`DiffModel.wholeWhy`); `""` when it offers parts
    /// (デザイン規約 §diff の中のステージ).
    function wholeOnlyLine(why) {
        switch (why) {
        case "gitlink": return qsTr("Whole file only · a submodule's commit")
        case "lfs": return qsTr("Whole file only · stored by Git LFS")
        default: return ""
        }
    }

    /// The colour a name takes in words drawn in `prose` (デザイン規約 §ref の種別「文の中の名前」), `own` the name's
    /// own where it has one: plain, whatever the words' colour — the name stands apart on its code ground, not by a
    /// colour — but a step down in a line that is itself a step down (`textSecondary`, a heading's second line), in
    /// its own where that says where the reader is (the current branch's `textLink`, the worktree stood in's
    /// `textHereTree`), and in the words' on a line that cannot be pressed or under a fill, which take everything on
    /// them.
    function nameInk(prose, own) {
        if (Qt.colorEqual(prose, Theme.textMuted) || Qt.colorEqual(prose, Theme.textOnAccent))
            return prose
        if (own !== undefined && (Qt.colorEqual(own, Theme.textLink) || Qt.colorEqual(own, Theme.textHereTree)))
            return own
        return Qt.colorEqual(prose, Theme.textSecondary) ? Theme.textSecondary : Theme.textPrimary
    }
    /// A room `cell` pixels wide out of no-break spaces `space` wide and narrow no-break spaces `thin` wide: the mix
    /// whose width comes nearest the cell, so the gap before a mark is the sentence's own space and no more
    /// (デザイン規約 §ref の種別「名前の印」). Whole spaces alone would leave up to a space's worth in front of it, a
    /// different amount for each kind. Both hold the line: neither is a place to break it.
    function roomFill(cell, space, thin) {
        if (cell <= 0 || space <= 0)
            return ""
        const most = Math.ceil(cell / space)
        let best = "\u00a0".repeat(most)
        let off = most * space - cell
        for (let n = 0; thin > 0 && n <= most; n++) {
            for (let m = 0; n * space + m * thin <= cell + thin; m++) {
                const miss = Math.abs(n * space + m * thin - cell)
                if (n + m > 0 && miss < off - 0.01) {
                    best = "\u00a0".repeat(n) + "\u202f".repeat(m)
                    off = miss
                }
            }
        }
        return best
    }
    /// `name` as code in markup (デザイン規約 §ref の種別「文の中の名前」): the mono face, in `ink`. Its ground is drawn
    /// under the words by whoever lays them out (`CardText`, `AppMenuItem`), measured off the ink.
    function codeMarkup(name, ink) {
        return "<span style=\"font-family:'" + Theme.monoFamily + "'; color:" + ink + "\">" + Words.inked(name)
             + "</span>"
    }
    /// A sentence as rich-text markup with a link on its one word a press can reach (`CardText.markup`), found inside
    /// the already-spelled sentence that is measured and read back (`HoverToolButton.tip`). Every piece is escaped. The
    /// tint rides inside the anchor: a `TextEdit` takes its link colour from the markup only (`CardText`).
    function placeInSentence(sentence, place, href, tint) {
        const seat = sentence.indexOf(place)
        if (place === "" || seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat))
             + "<a href=\"" + href + "\"><font color=\"" + tint + "\">" + Words.inked(place) + "</font></a>"
             + Words.inked(sentence.substring(seat + place.length))
    }
    /// Where a tip's sentence puts `word`, the name a drawn mark goes in front of (`SharedToolTip.tipMarkWord`): a
    /// whole word at its end — a tip names it after its conclusion, `<conclusion> — <name>`, and a cut row's line ends
    /// in it — or opening what follows the conclusion, `<conclusion> — <name> is locked`. -1 anywhere else — a
    /// translation that moved it, or the same letters inside another word (`old` in `folder`) — so the sentence is
    /// drawn as it came, with no mark on the wrong word.
    function tipNameSeat(sentence, word) {
        if (word === "")
            return -1
        const seat = sentence.length - word.length
        if (seat >= 0 && sentence.substring(seat) === word) {
            const before = sentence.charAt(seat - 1)
            if (before === "" || before === " ")
                return seat
        }
        const after = sentence.indexOf(" — " + word + " ")
        return after < 0 ? -1 : after + 3
    }
    /// Where each name of a `, `-joined list stands in a sentence that ends in it (`severalRemotesHave`,
    /// `differsFrom`, `pullOwnCommits`): `[at, kind, , length]` each, for the marks the names wear and the colour they
    /// take (`CardText.marks`). None where the sentence does not end in the list — a translation that moved it is drawn
    /// as it came.
    function listMarks(sentence, list, kind) {
        if (list === "" || !sentence.endsWith(list))
            return []
        let at = sentence.length - list.length
        const marks = []
        for (const name of list.split(", ")) {
            marks.push([at, kind, undefined, name.length])
            at += name.length + 2
        }
        return marks
    }
    /// A character no name holds, written into a sentence in a name's place to find where the sentence puts it
    /// (`standSeats`).
    readonly property string stand: "\uE000"
    /// Where a sentence written with `stand` in one name's place puts that name — every time it says it — as seats in
    /// the sentence with `word` written there instead: each counts the earlier ones at the word's own length. A
    /// translation that moves the name moves the seat (`RepoPage.reportMarks`).
    function standSeats(sentence, word) {
        const seats = []
        let at = sentence.indexOf(Words.stand)
        while (at >= 0) {
            seats.push(at + seats.length * (word.length - Words.stand.length))
            at = sentence.indexOf(Words.stand, at + 1)
        }
        return seats
    }
    /// Where each name a sentence says stands in it, and the mark it wears (デザイン規約 §ref の種別「名前の印」):
    /// `[at, kind, , length]` each, in the sentence's order (`CardText.marks`). `sentence` is the function that writes
    /// it, called once per name with `stand` in that name's place; `names` is `[[name, kind], …]` in the order it takes
    /// them — `[name, kind, own colour]` where the name says where the reader is. A name that is empty or wears no mark
    /// is left out.
    function marksOf(sentence, names) {
        const real = names.map(name => name[0])
        const marks = []
        names.forEach((name, i) => {
            if (name[0] === "" || name[1] === "")
                return
            const args = real.slice()
            args[i] = Words.stand
            Words.standSeats(sentence.apply(null, args), name[0])
                 .forEach(at => marks.push([at, name[1], name[2], name[0].length]))
        })
        return marks.sort((a, b) => a[0] - b[0])
    }
    /// The names a report's heading says, each behind its mark (`NoticeBar.marks`): the remote's cloud, and `name`
    /// behind `nameMark` (empty for none). Seated by the heading's own sentence, so a translation moves the marks with
    /// the names (`marksOf`).
    function reportMarks(kind, remote, name, nameMark) {
        return Words.marksOf((r, n) => Words.writeReported(kind, r, n), [[remote, "remote"], [name, nameMark]])
    }
    /// The same for the line under it (`NoticeBar.detailMarks`), which may also name the branch `fact` is.
    function reportWhyMarks(kind, remote, name, fact, nameMark) {
        return Words.marksOf((r, n, f) => Words.writeReportedWhy(kind, r, n, f),
                             [[remote, "remote"], [name, nameMark],
                              [fact, kind === "worktree-branch-left" ? "branch" : ""]])
    }
    /// Whether a report's heading names a branch or a tag (`writeReported`): the far side's refusals, the pushes, the
    /// renames.
    function reportNamesRef(kind) {
        return kind.startsWith("delete-") || kind.startsWith("update-")
               || ["pre-push", "outdated", "moved", "tag-elsewhere", "pre-push-delete", "moved-delete", "rename",
                   "rename-locked"].indexOf(kind) >= 0
    }
    /// Words as markup reads them. Rich text folds runs of spaces and newlines the way HTML does, so all but the last
    /// space of a run is pinned and a newline becomes `<br>`. Pin only those: `NoticeLine` wraps, and a sentence
    /// pinned at every space breaks through a word (`tst_refwords`). A mark's room (`roomFill`) goes through as
    /// written: its no-break spaces as such, and its narrow ones kept from the fold, which takes them as plain space.
    function inked(words) {
        return words.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
                    .replace(/ (?= )/g, "&nbsp;")
                    .replace(/\u00a0/g, "&nbsp;")
                    .replace(/\u202f/g, "<span style=\"white-space:pre\">&#8239;</span>")
                    .replace(/\n/g, "<br>")
    }
}

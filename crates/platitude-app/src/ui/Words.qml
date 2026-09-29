pragma Singleton

import QtQuick

// Wording that more than one place has to say the same way — a second copy of a sentence drifts from the first.
QtObject {
    /// The application's name, as the window title and both empty states write it.
    readonly property string appName: qsTr("Platitude GG")

    /// A held control's answer to a screen reader, wherever a HoldDriver drives one.
    readonly property string holdToActivate: qsTr("Hold to activate")

    /// The menu-row tag on anything that replays commits a remote already has — the amend-family rows and both
    /// rebase entries (デザイン規約 §長押し).
    readonly property string rewritesPushed: qsTr("rewrites pushed commits")

    /// Starting a branch where the row stands, from the graph row's menu and the sidebar's ref menu
    /// (デザイン規約 §メニュー「入口が違っても同じ操作は同じ文」); the TAG card's `Create tag here…` (`RefTagMenu`) is
    /// worded to match.
    readonly property string createBranchHere: qsTr("Create branch here…")

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

    /// Why the `pull` row is greyed while both sides have moved, from both entrances (`RefRowMenu` / `CommitRowMenu`;
    /// デザイン規約 §取り込んで合流させる).
    readonly property string pullDiverged: qsTr(
        "The branches have diverged, so pick the remote branch and rebase onto it")

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
    /// during a rebase, so they are handed in from `WorkTreeModel`.
    function ourSide(name) {
        return name !== "" ? name : qsTr("this branch")
    }
    function theirSide(name) {
        return name !== "" ? name : qsTr("the incoming side")
    }

    /// Why a folder would not open, from the kind core answered with (`plain` / `bare` / `other`), for both the
    /// picker's dialog and a tab's failed screen (デザイン規約 §リポジトリを開く). `other` says only that much: what
    /// happened is git's to say, quoted underneath.
    function openFailure(kind) {
        switch (kind) {
        case "bare": return qsTr("A bare repository has nothing to show")
        case "other": return qsTr("Could not open this folder")
        default: return qsTr("Not a git repository")
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
        switch (kind) {
        case "delete": return qsTr("%1 would not delete %2").arg(remote).arg(name)
        case "update": return qsTr("%1 would not update %2").arg(remote).arg(name)
        // A branch and a tag alike, whatever stopped them; the line underneath says what did.
        //: %1 is a branch or a tag, %2 a remote.
        case "outdated":
        case "moved":
        case "tag-elsewhere": return qsTr("%1 was not sent to %2").arg(name).arg(remote)
        //: %1 is a branch or a tag, %2 a remote.
        case "moved-delete": return qsTr("%1 was not deleted from %2").arg(name).arg(remote)
        case "stale-stage": return qsTr("Nothing was staged")
        case "stale-unstage": return qsTr("Nothing was unstaged")
        case "stale-discard": return qsTr("Nothing was discarded")
        case "conflicted-part": return qsTr("Nothing was taken from this file")
        case "rename": return qsTr("%1 was not renamed").arg(name)
        case "half-rename": return qsTr("The rename did not finish")
        case "worktree-kept": return qsTr("%1 was not removed").arg(name)
        case "worktree-half": return qsTr("%1 was not removed completely").arg(name)
        // All seven rewrite refusals: which one it was is the line underneath.
        case "across-merge":
        case "off-branch":
        case "fold-first":
        case "unfetched-base":
        case "drop-all":
        case "tip-moved":
        case "op-standing": return Words.historyNotRewritten
        default: return qsTr("The commit was not made")
        }
    }

    /// The row menu's heading for all seven rewrite refusals, and `planRefused`'s fallback.
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

    /// The second line for the reports nobody outside answered, so there is nobody to quote. Empty for the reports
    /// that quote somebody else; the page uses what came across (デザイン規約 §答えの要らない報せ). `remote` and `name`
    /// are the heading's. The refusals a read answers say what the remote turned out to be, and no more: the graph
    /// shows the rest once the read that follows has landed.
    function writeReportedWhy(kind, remote, name) {
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
        case "stale-stage":
        case "stale-unstage":
        case "stale-discard":
            return qsTr("The file changed on disk since these rows were read. It has been read again — make the selection on the rows that are there now.")
        case "conflicted-part":
            return qsTr("This file is still conflicted. It has to be resolved before parts of it can be taken.")
        case "half-rename":
            return qsTr("The new name was made; the old one is still there.")
        // Only when git said it was the changes: its own words end in advice to force it, which nothing here offers.
        case "worktree-kept":
            return qsTr("It has uncommitted changes.")
        // The rewrite refusals live in `rewriteRefusedWhy`, which the plan's door reads too. Delegated whole: a
        // second copy of that list gets a new refusal on one side only.
        default: return Words.rewriteRefusedWhy(kind)
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
            return qsTr("An operation is in progress here. Finish it or put it down first.")
        default: return ""
        }
    }

    /// Whether a report's heading names a working copy — the name then wears the tree mark (`NoticeBar.markWord`,
    /// デザイン規約 §ref の種別「名前の印」).
    function reportNamesCopy(kind) {
        return kind === "worktree-kept" || kind === "worktree-half"
    }

    /// The colour a report's own hairline wears: `warning` while the gesture is still going (the rename box is open,
    /// or a half-done rename is left standing), `danger` once it is over (デザイン規約 §答えの要らない報せ). A working
    /// copy git would not remove is `warning` too: the copy still stands, with what kept it there.
    function reportTone(kind) {
        return kind === "rename" || kind === "half-rename" || kind === "worktree-kept" || kind === "worktree-half"
            ? "warning" : "danger"
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

    /// A ref name inside a sentence, as rich-text markup: the name in `tint` (デザイン規約 §ref の種別
    /// 「名前が出る場所すべてで」), the rest in the line's own colour. The name goes at the `%1` seat rather than being
    /// searched for — a branch called `it` would be found in `into it`. `""` when the sentence has no seat; the caller
    /// then uses its plain words. Every piece is escaped: a branch called `<b>` has to read as its name.
    function nameInSentence(sentence, name, tint) {
        const seat = sentence.indexOf("%1")
        if (seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat))
             + "<font color=\"" + tint + "\">" + Words.inked(name) + "</font>"
             + Words.inked(sentence.substring(seat + 2))
    }
    /// The same shape with a link where the colour was: the one word a press can reach (`CardText.markup`), found
    /// inside the already-spelled sentence that is measured and read back (`HoverToolButton.tip`). The tint rides
    /// inside the anchor: a `TextEdit` takes its link colour from the markup only (`CardText`).
    function placeInSentence(sentence, place, href, tint) {
        const seat = sentence.indexOf(place)
        if (place === "" || seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat))
             + "<a href=\"" + href + "\"><font color=\"" + tint + "\">" + Words.inked(place) + "</font></a>"
             + Words.inked(sentence.substring(seat + place.length))
    }
    /// The same sentence with `spaces` of room opened in front of `word`, for a drawn mark to stand in
    /// (`SharedToolTip`; where the gap lands is `CardText.charRect`'s to answer). `""` where the word is not in the
    /// sentence — a translation moved it — so the caller draws the sentence as it came, with no mark on the wrong word.
    /// The room is pinned whole: rich text drops a lone space opening a block (a name that starts the sentence,
    /// `NoticeBar`), and a breakable one would let a line end between the mark and its name.
    function roomInSentence(sentence, word, spaces) {
        const seat = sentence.indexOf(word)
        if (word === "" || spaces <= 0 || seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat)) + "&nbsp;".repeat(spaces) + Words.inked(sentence.substring(seat))
    }
    /// Words as markup reads them. Rich text folds runs of spaces and newlines the way HTML does, so all but the last
    /// space of a run is pinned and a newline becomes `<br>`. Pin only those: `NoticeLine` wraps, and a sentence
    /// pinned at every space breaks through a word (`tst_refwords`).
    function inked(words) {
        return words.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
                    .replace(/ (?= )/g, "&nbsp;")
                    .replace(/\n/g, "<br>")
    }
}

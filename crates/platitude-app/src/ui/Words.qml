pragma Singleton

import QtQuick

// Wording that more than one place has to say the same way. A second copy of a sentence is a second answer, and the two
// drift.
QtObject {
    /// The application's name, as the window title and both empty states write it.
    readonly property string appName: qsTr("Platitude GG")

    /// A held control's answer to a screen reader, wherever a HoldDriver drives one.
    readonly property string holdToActivate: qsTr("Hold to activate")

    /// The menu-row tag on anything that replays commits a remote already has — the amend-family rows and both
    /// rebase entries say it with one voice (デザイン規約 §可否・警告の出し場所).
    readonly property string rewritesPushed: qsTr("rewrites pushed commits")

    /// A branch of one's own started where the row stands, as the graph row's menu and the sidebar's ref menu both
    /// offer it — one seat apart, one sentence (デザイン規約 §メニュー: 入口が違っても同じ操作は同じ文). The box a tag
    /// is typed into says it the same way, from inside the TAG card that both menus carry (`RefTagMenu`).
    readonly property string createBranchHere: qsTr("Create branch here…")

    /// Opening a repository, as the graph's empty state, the tab strip's `+` menu and its accessible name offer it.
    readonly property string openRepository: qsTr("Open repository…")

    /// Adding a remote, as the sidebar header, the collapsed rail and the publish flow offer it.
    readonly property string addRemote: qsTr("Add remote…")

    /// The command log's own name — its heading (`CommandsToggle`), and the word every tip that sends a reader there
    /// points at. Said here because the three have to be the one word in every language.
    readonly property string commandsTitle: qsTr("GIT COMMANDS")
    /// The name the command log answers to inside a tooltip's markup (`CardText.linkAsked` → `RepoPage.tipLinkAsked`).
    ///
    /// **A fixed spelling.** It never reaches a reader — it is the one word the writer of a tip and the
    /// page that reads it back have to spell the same, and a name that moved with the language would pair them only
    /// in the language it was written in. The scheme is this application's own so that nothing here can be mistaken
    /// for an address something else in the machine would open.
    readonly property string commandsHref: "pgg:commands"

    /// The way out that writes nothing, as the three dialogs' foot says it. **Dialogs only**: what a dialog's way out
    /// puts away is the box itself, and the reader can see its edges. The plan takes the graph's whole seat with no
    /// box to dismiss, so its own way out names what goes (`RebasePlanPane` — `Discard`).
    readonly property string cancel: qsTr("Cancel")

    /// Why a menu row is out while it cannot be chosen (`AppMenuItem.blockedWhy` — デザイン規約 §無効: the line is the
    /// whole reason the row stays). One state and one line: git is out and the answer is to wait — the line the
    /// delete table has always worn, and the whole of what the held doors say while a rewrite replays behind the
    /// screen. Said here because the ref menu, the branch card, the tag card and the remote menu all have to say it
    /// the same way.
    ///
    /// **What is running.** Every command this window runs is a git one and the panel that holds
    /// them is spelled `GIT COMMANDS`, so naming git here buys nothing — and it would not settle the one ambiguity
    /// there is, since the row the pointer is on wears a git command of its own on its chip. `running` is what
    /// settles it. Short enough that the tip stands inside the menu it explains.
    readonly property string otherCommandRunning: qsTr("Wait for the running command")

    /// Why the delete rows that reach a remote are out on a name whose two sides have drifted — the branch card's
    /// pair and the tag card's say it with one voice (デザイン規約 §左メニューの所作 の削除の表). **The state**:
    /// what the reader would do about it is delete the reading from the row where it does stand, and that
    /// row is on the screen already — a line that sent them there would be longer than the menu it explains.
    readonly property string remoteOnAnotherCommit: qsTr("The remote is on another commit")

    /// Why the `pull` row is greyed: both sides have moved, and bringing two lines together that way is a choice
    /// nobody made — so the row says where the choice is, which is the remote branch's own rows
    /// (デザイン規約 §取り込んで合流させる). **The second half is the way on**, because the row is a dead end
    /// otherwise: greyed, with nothing on screen saying what to do instead. Said with one voice from both entrances
    /// (`RefRowMenu` / `CommitRowMenu`).
    ///
    /// **No dash in it**: the run that reads this line back matches ASCII only (verify-ui §Windows の罠 — a `—`
    /// reaches the harness as broken bytes), so the sentence is punctuated with a comma and the whole of it can be
    /// claimed.
    readonly property string pullDiverged: qsTr(
        "The branches have diverged, so pick the remote branch and rebase onto it")

    /// The window band's badge words (BandStateGroup measures and draws them, BandStateCard titles them);
    /// the WIP pane's conflicts bucket header shares the first.
    readonly property string badgeConflicts: qsTr("CONFLICTS")
    readonly property string badgeSetIdentity: qsTr("SET IDENTITY")
    readonly property string badgeOldGit: qsTr("OLD GIT")
    /// **What is drawn is out of date**, either way it came to be so: the walk gave up part-way
    /// through (some of it, or none), or every row landed and the rebuild that would have refreshed them did not. One
    /// word for the two, because a reader's position is the same in both — nothing on screen can be acted on as if it
    /// were current — and the card's line says which it was. Unlike the window cut nothing says how much is missing,
    /// because neither state reached the end of a walk. **The only place this state is said**: the graph column
    /// carries none of it, and the words behind it are in the card.
    readonly property string badgeStaleGraph: qsTr("STALE GRAPH")

    /// The one way a commit's moment is written down: the rows carry epoch seconds, and the display side makes the
    /// `yyyy-MM-dd HH:mm` out of them (デザイン規約 — 行が持つのは epoch 秒). The commands panel's `HH:mm:ss` clock
    /// is a different thing and stays its own.
    function stamp(epochSeconds) {
        return Qt.formatDateTime(new Date(epochSeconds * 1000), "yyyy-MM-dd HH:mm")
    }

    /// What to call a side git left nothing to name. Reached from a cherry-pick of a commit no branch can see, among
    /// others. The names swap over during a rebase, so they are handed in from
    /// `WorkTreeModel`.
    function ourSide(name) {
        return name !== "" ? name : qsTr("this branch")
    }
    function theirSide(name) {
        return name !== "" ? name : qsTr("the incoming side")
    }

    /// Why a folder would not open, from the kind core answered with (`plain` / `bare` / `other`). Said in two places —
    /// the dialog the picker's answer raises, and the screen a tab that could not open shows — so the six they make
    /// between them are three sentences.
    ///
    /// `other` is git having trouble of its own, and its line says only that
    /// much: what happened is git's to say, and the screen quotes it underneath.
    function openFailure(kind) {
        switch (kind) {
        case "bare": return qsTr("A bare repository has nothing to show")
        case "other": return qsTr("Could not open this folder")
        default: return qsTr("Not a git repository")
        }
    }

    /// Why a picture was not filed against an address, from the kind core answered with
    /// (`avatar::AvatarRefusal`). `facts` carries the numbers the sentence takes, in the order it takes them;
    /// `said` is the operating system's own line, for the two failures it made and empty for the rest.
    ///
    /// **Five of the seven are this application's own refusals**, and their words belong here, like every other
    /// word on screen (app-ui.md「Rust に文言を置かない」). The other two are the reverse
    /// case: what the operating system said is carried across untouched, under a frame written here — the same
    /// division a report makes between the heading and whoever wrote the line under it (デザイン規約 §長さ).
    ///
    /// The size is named on the one about pixels because the ceiling a reader was told about is the *file* size:
    /// without the dimensions, a two-megabyte file being refused has no explanation at all.
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

    /// What did not happen, for a write that did not happen and has something to say for itself — from the kind core
    /// answered with (`delete` / `update` / `outdated` / `commit` — `RepoTab.writeReportKind`). Why is whoever said no
    /// to say, and the bar quotes it underneath (デザイン規約 §答えの要らない報せ).
    ///
    /// **Whoever decided is the subject.** For the two the far side decided that is the remote: nothing here failed,
    /// and a sentence in this application's own voice ("could not delete…") would read as one that did. For the other
    /// two nobody over there decided anything — git worked the refusal out from what this end holds, or a hook here
    /// said no — so the subject is the thing that did not move.
    ///
    /// A name is all these take. Which of them is a tag and which a branch is not said: the row that was pressed is
    /// still on screen saying so, and the far side's own answer names neither.
    function writeReported(kind, remote, name) {
        switch (kind) {
        case "delete": return qsTr("%1 would not delete %2").arg(remote).arg(name)
        case "update": return qsTr("%1 would not update %2").arg(remote).arg(name)
        case "outdated": return qsTr("%1 was not sent to %2").arg(name).arg(remote)
        case "stale-stage": return qsTr("Nothing was staged")
        case "stale-unstage": return qsTr("Nothing was unstaged")
        case "stale-discard": return qsTr("Nothing was discarded")
        case "conflicted-part": return qsTr("Nothing was taken from this file")
        case "rename": return qsTr("%1 was not renamed").arg(name)
        case "half-rename": return qsTr("The rename did not finish")
        // The five a rewrite is turned down for. **One heading between them**: what did not happen is the same
        // thing each time, and which of the five it was is the reason underneath — where a reader who already
        // knows what they pressed will look for it.
        case "across-merge":
        case "off-branch":
        case "fold-first":
        case "unfetched-base":
        case "drop-all":
        // The two the outside world makes after the press. Same heading, for the same reason: what did not happen
        // is the same thing, and which of them it was is the line underneath.
        case "tip-moved":
        case "op-standing": return Words.historyNotRewritten
        default: return qsTr("The commit was not made")
        }
    }

    /// What did not happen, for a rewrite that did not — the row menu's heading over all seven of them, and what the
    /// plan's door falls back to for a refusal nobody here has words of its own for (`planRefused`).
    readonly property string historyNotRewritten: qsTr("The history was not rewritten")

    /// What did not happen at the **plan's** door, where a preview turns the range down before it opens
    /// (`RepoPage.onRefusedPlan`). **This is the one thing the two doors say differently**: the row menu's heading is
    /// about the one commit that was pressed, so it is the same sentence for every shape
    /// (`historyNotRewritten`), while here the shape *is* what the reader is being told about the range — there is no
    /// pressed row on screen to say it instead. The line underneath is the same either way (`rewriteRefusedWhy`).
    ///
    /// **Each of the three is written out.** A shape left to the fallback would take a heading about some other
    /// history and read as if it were its own, which is the reading a picture of one bar cannot rule out.
    function planRefused(kind) {
        switch (kind) {
        case "across-merge":
            return qsTr("A merge is in the way")
        case "off-branch":
            return qsTr("Not on this branch")
        case "unfetched-base":
            return qsTr("The history stops here")
        // Nothing reaches this: what raises the bar is `PlanRefusal`, which core spells and
        // `rebase_plan::drain::refusal_kind` names in one exhaustive arm each. A fourth shape stops the build
        // there — and if one ever arrived here anyway, what is true of every one of them is still true of it.
        default: return Words.historyNotRewritten
        }
    }

    /// The second line for the reports **nobody outside answered** — the ones this end refused itself before git was
    /// asked, and the push git turned down here without the far side ever seeing it. Either way there is nobody to
    /// quote, so the words are ours and belong here (app-ui.md「Rust に文言を置かない」).
    /// Empty for the reports that quote somebody else; the page uses what came across.
    function writeReportedWhy(kind) {
        switch (kind) {
        // **The far side never saw this push.** git turned it down here, reading what this clone holds, and the
        // advice it writes under that is for somebody at a terminal — three sentences ending in `git pull`, which
        // is the one thing this reader has no use for: the session is fetching already
        // (デザイン規約 §答えの要らない報せ). So the line says the cause and stops there.
        case "outdated":
            return qsTr("The remote has commits this clone has not fetched yet.")
        case "stale-stage":
        case "stale-unstage":
        case "stale-discard":
            return qsTr("The file changed on disk since these rows were read. It has been read again — make the selection on the rows that are there now.")
        case "conflicted-part":
            return qsTr("This file is still conflicted. It has to be resolved before parts of it can be taken.")
        case "half-rename":
            return qsTr("The new name was made; the old one is still there.")
        // The rewrites a fold or a drop is turned down for are said one door further in, because a second door
        // reaches three of them. **Delegated whole** — a second copy of that list is one a sixth
        // refusal is added to on one side only, and the row menu would lose its second line without a word.
        // Everything else quoted somebody outside, and `rewriteRefusedWhy` answers those with the same empty
        // line this has always given them.
        default: return Words.rewriteRefusedWhy(kind)
        }
    }

    /// Why a rewrite was turned down, said the same way at both doors that turn one down: the row menu, where core
    /// withheld the write (`writeReportedWhy`), and the plan, which is refused before it opens
    /// (`RepoPage.onRefusedPlan`). **Three of the five reach that second door** — the other two are about one
    /// row's own fold or drop, which a preview never asks.
    ///
    /// **Only the line is shared.** The heading is the surface's own — one turns down the commit that was pressed
    /// and the other the range it would replay — and so is the hairline's colour, which follows whether the
    /// gesture is over (`reportTone`, デザイン規約 §答えの要らない報せ).
    ///
    /// Each line says the one thing about the history that stopped it, in the words of what is on screen: rows,
    /// branches and the commit under the pointer. The words are a fold, a drop or a plan — never `rebase`, since
    /// nothing was run (デザイン規約 §用語表).
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
        // The two that are about right now: something outside this window moved
        // between the press and git. **One sentence each, like the five above** — a report is written to one
        // (`NoticeBar`, デザイン規約 §長さ). A longer one is not cut, it is wrapped, and what it costs is the rows of
        // history the bar pushes off the screen.
        //
        // **The heading has already said what did not happen**, so these owe the reader the cause and nothing else:
        // spelling out what a replay would have dropped is the application explaining its own reasoning, which is
        // the half a reader in the middle of a gesture does not read.
        case "tip-moved":
            return qsTr("The branch moved after this was worked out.")
        case "op-standing":
            return qsTr("An operation is in progress here. Finish it or put it down first.")
        default: return ""
        }
    }

    /// The colour a report's own hairline wears (デザイン規約 §答えの要らない報せ / §状態). **One axis, and the name boxes are
    /// on the other end of it** (`SlimField.refused`, which is `warning`):
    ///
    ///  - **`warning` — the gesture is still going.** A box is open and one keystroke fixes what is in it (a rename
    ///    git would not make: nothing moved, and the box is still holding the name), or something was left standing
    ///    that has to be dealt with (a rename that fell between its halves).
    ///  - **`danger` — the gesture is over and what was asked for did not happen.** Every other report: by the time a
    ///    bar is down the press has been spent and the screen has moved on (the fetch landed, the diff was read again,
    ///    the row came back), so pressing again is a new gesture.
    ///
    /// **Always a colour**: a press turned down says so in colour, whoever turned it down.
    function reportTone(kind) {
        return kind === "rename" || kind === "half-rename" ? "warning" : "danger"
    }

    /// What the two sides each did to a conflicted file, from the two stage letters git reports (デザイン規約 §conflict の種別).
    /// Shown by the diff pane on the conflicts git prints no patch for — the one place the sentence appears; the
    /// headless `conflict_kind` report reads it through `NavItemDelegate.conflictWords`.
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

    /// What a change did to a file's line endings, from the pieces `DiffModel` took the notice apart into. `""` when
    /// there is nothing to say, which is most of the time.
    ///
    /// Every branch is a **whole sentence**: the two estimated cases can only claim as far as the sample reached, so
    /// the range is part of what is being said. Stitching fragments would also leave a
    /// translator with half a sentence and no way to reorder it.
    ///
    /// `LF` and `CRLF` are written plainly — the chip shape is lowercase monospace and an all-caps
    /// abbreviation does not sit in it (デザイン規約 §git 用語のコード表記).
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

    /// **A ref name inside a sentence**, spelled as the markup rich text reads: the name in the colour it wears
    /// everywhere else it is met, the rest of the line in whatever the line is drawn in (デザイン規約 §ref の種別
    /// 「現在のブランチは、名前が出る場所すべてで textLink」). One field, so the line elides, measures and
    /// is dragged over the way it did before a colour went into it.
    ///
    /// **The seat is cut out of the sentence**: a branch
    /// called `it` would otherwise be found in the first word of `into it`. A sentence with no seat in it gets no
    /// markup at all, and the caller falls back to its plain words.
    ///
    /// **Every piece is escaped on the way in.** The sentence is this application's, but the name is not — a branch
    /// called `<b>` has to read as its name.
    function nameInSentence(sentence, name, tint) {
        const seat = sentence.indexOf("%1")
        if (seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat))
             + "<font color=\"" + tint + "\">" + Words.inked(name) + "</font>"
             + Words.inked(sentence.substring(seat + 2))
    }
    /// The same shape with a place to go where the colour was: the one word in the sentence a press can reach
    /// (`CardText.markup`). **The whole sentence comes in already said** — the plain spelling is the one that is
    /// measured and read back (`HoverToolButton.tip`), so this finds the place inside
    /// it.
    ///
    /// The tint rides inside the anchor because an anchor's ink is the document's own: neither `linkColor` nor
    /// `palette.link` reaches a `TextEdit` (both measured, `CardText`). The href is read by nobody but the page that
    /// answers it.
    function placeInSentence(sentence, place, href, tint) {
        const seat = sentence.indexOf(place)
        if (place === "" || seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat))
             + "<a href=\"" + href + "\"><font color=\"" + tint + "\">" + Words.inked(place) + "</font></a>"
             + Words.inked(sentence.substring(seat + place.length))
    }
    /// The same sentence with room opened in front of one of its words, for a **drawn** mark to stand in
    /// (`SharedToolTip`). A tooltip is one string and a mark is not a character (規約 §ref の種別), so what the markup
    /// can give it is a gap of its own width; where that gap is, is the field's to answer (`CardText.charRect`).
    ///
    /// **Empty where the word is not in the sentence** — which is what a translation that moved or reworded it looks
    /// like. The caller then draws the sentence as it came, with no mark and no gap, rather than a mark standing on
    /// the wrong word.
    function roomInSentence(sentence, word, spaces) {
        const seat = sentence.indexOf(word)
        if (word === "" || spaces <= 0 || seat < 0)
            return ""
        return Words.inked(sentence.substring(0, seat) + " ".repeat(spaces) + sentence.substring(seat))
    }
    /// Words as markup reads them. **Rich text folds a run of spaces the way HTML does** (`encode::markup`), so all
    /// but the last space of a run is pinned — and **only those**: one of the two fields this is drawn in wraps
    /// (`NoticeLine`), and a sentence pinned at every space has nowhere left to break, so it breaks
    /// through the middle of a word instead (measured, qmltestrunner `tst_refwords`).
    ///
    /// **A line break is spelled out as well.** Rich text folds a newline the way it folds spaces, and the one field
    /// that is handed a sentence of more than one line is the tooltip (規約 §hover のツールチップ — 2 行目は
    /// 「なぜ今その見た目か」): left as it came, the two lines come out as one (measured).
    function inked(words) {
        return words.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
                    .replace(/ (?= )/g, "&nbsp;")
                    .replace(/\n/g, "<br>")
    }
}

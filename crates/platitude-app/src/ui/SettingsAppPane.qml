import QtQuick
import QtQuick.Dialogs
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Application` category: what Platitude GG keeps in its own settings file (規約 §設定の画面).
// One file per category, since a category is one place values are stored.
ColumnLayout {
    id: pane

    /// The page the avatar candidates are read off (its repository's authors).
    property var curPage: null

    /// The author an avatar's badge was pressed on, handed down by the screen for the entry.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Enter in one of the number fields. The way out belongs to the screen, so it is asked for.
    signal accepted()

    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs (finished by `WindowAutoActDriver`) wait on — the list's output side.
    readonly property int autoAvatarRows: avatarRepeater.count
    readonly property bool autoAvatarComboOpen: avatarWho.popup.opened
    /// Whether this category is showing, for the verb that presses the rail — reading back the screen's `category`
    /// would report the run's own press.
    readonly property bool autoAppShown: fetchField.visible
    /// The version chip can be grabbed (`CodeChip.grabbable`), which a picture cannot show.
    readonly property bool autoVersionGrabbed: gitVersionChip.grabbed
    function autoAvatarRowLit(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.lit
    }
    function autoAvatarRowPainted(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.pictureReady()
    }
    /// Through the press's own door (`AppCombo.pressField`), including its turn of the loop before the list.
    function autoAvatarOfferCombo() {
        avatarWho.pressField()
    }
    /// Typing into that box and Enter, which the offscreen platform does not deliver: the text goes where typing
    /// writes it (`AppCombo.wanted`, as `focusPrefill` does), and Enter is the box's own `accepted` (`tst_appcombo`).
    function autoTypeAvatarWho(who) {
        avatarWho.wanted = who
        avatarWho.editText = who
    }
    function autoEnterAvatarWho() {
        avatarWho.accepted()
    }
    /// `visible`, not `opened`: a `FileDialog` is a platform window, not a `Popup` — and it is in neither PNG.
    readonly property bool autoAvatarPickerOpen: avatarPicker.visible
    /// Runs the row's hold to its end. False where the list has no such row yet, or its button takes no press yet,
    /// so the caller waits for it.
    function autoAvatarHoldRemove(at) {
        const row = avatarRepeater.itemAt(at)
        return row !== null && row.holdRemove()
    }

    /// Assignments as the settings file holds them (`AppBackend.avatars` — `{email, name, url}`). A picture filed
    /// under no name is shown by its address.
    readonly property var assigned: AppBackend.avatars.map(assignment => ({
        email: assignment.email,
        name: assignment.name !== "" ? assignment.name : assignment.email,
        url: assignment.url
    }))

    /// Who the entry offers, read once as the screen opens — bound, it would reorder under an open popup as the
    /// graph moves.
    property var authorChoices: []
    function readAuthorChoices() {
        // `GraphModel.author_choices` dedupes, sorts and formats, with the prefill so its address shows once.
        pane.authorChoices = pane.curPage
                           ? pane.curPage.pageGraph.authorChoices(pane.prefillName, pane.prefillEmail)
                           : pane.prefillEmail !== ""
                             ? [pane.prefillName + " <" + pane.prefillEmail + ">"] : []
    }

    /// The address and name the picker files under, split out of the entry: `Name <address>`, or either half typed
    /// alone (`platitude_core::trailers::split_identity`).
    readonly property string chosenEmail: GitFacts.identityEmailOf(avatarWho.wanted)
    readonly property string chosenName: GitFacts.identityNameOf(avatarWho.wanted)
    /// The one body the button and the box's Enter share, guarded as the button is: a picker opened over nobody
    /// would file against an empty address. Headless `avatar-enter`.
    function openAvatarPicker() {
        if (pane.chosenEmail === "")
            return
        avatarPicker.open()
    }

    /// The version git printed, in git's own spelling — the chip at the head of the line. Empty where git printed
    /// none.
    readonly property string gitPathChip:
        AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old"
        ? "git " + AppBackend.gitPathVersion : ""
    /// What the git at the path said, as one sentence — empty where the chip says it all. The words live here
    /// (rules-refs/app-ui.md「Rust に文言を置かない」); `version::Probe` sends which answer it was. The two `missing`
    /// sentences differ: an empty box means the git on PATH, a written path is a file that is not there — mixing them
    /// sends the reader to fix the wrong thing. A git that ran and is not git is given git's or the OS's own words.
    readonly property string gitPathWord: {
        // The warning stands only while a restart is offered (user decision — ask before changing): standing always,
        // it would warn about the git running now. The version chip is its subject (user decision): a chip floating
        // alone over a paragraph reads as an unfinished lead-in. Two whole wordings rather than a joined-on clause,
        // each saying only how things stand and what to do — with the exit shut, a press is what the reader needs.
        // It is `Application`'s one warning-coloured sentence and does not compete with `GLOBAL`'s
        // (`SettingsGitPane`): that one rewrites things outside this window, this one has the window go away once.
        // Not dropped a step (`fontMd`): a warning is no footnote.
        if (AppBackend.gitPathOffersRestart) {
            return AppBackend.gitPathState === "old"
                 ? qsTr("is older than the %1 this app is built for — restart to use it anyway.")
                   .arg(AppBackend.minimumGit)
                 : qsTr("is not running yet — restart to switch to it.")
        }
        switch (AppBackend.gitPathState) {
        case "checking":
            return qsTr("Asking for the version…")
        case "ok":
            // The chip alone is the whole answer: with one version on the line, a sentence about it says it twice.
            return ""
        case "old":
            // The `OLD GIT` badge's words, less the version the chip carries (規約 §git が無い時・古い時).
            return qsTr("is older than the %1 this app is built for.").arg(AppBackend.minimumGit)
        case "missing":
            return AppBackend.gitPath === "" ? qsTr("git was not found on PATH.")
                                             : qsTr("Nothing to run at that path.")
        case "failed":
            return AppBackend.gitPathError
        }
        return ""
    }
    /// `warning` while a restart is offered or git is old; `danger` where git did not answer; otherwise the help
    /// text's ink — a version that came back is ordinary (規約 §状態).
    readonly property color gitPathInk: {
        if (AppBackend.gitPathOffersRestart)
            return Theme.warning
        switch (AppBackend.gitPathState) {
        case "old":
            return Theme.warning
        case "missing":
        case "failed":
            return Theme.danger
        }
        return Theme.textSecondary
    }

    /// Whether the button stands, frozen for the presses under way: the
    /// offer moves on git's clock and could take the button away between presses; frozen, the presses finish and
    /// `restart_now` decides whether they still mean anything. The `Binding` below keeps it live and lets go while
    /// presses are under way (`RestoreNone` is the freeze).
    property bool gitButtonRestarts: AppBackend.gitPathOffersRestart
    /// `warning` for a git that answers, `danger` where it is below the supported minimum — the button alone goes
    /// red (user decision — ask before changing); the line stays `warning`, being old a fact about the git. `danger`
    /// means stopped or failed, or discarding work at hand (規約 §状態); a press that moves to an unsupported version is
    /// taken as a third case — what it lands on answers, but on a version this app does not vouch for.
    readonly property color gitButtonTone:
        AppBackend.gitPathState === "old" ? Theme.danger : Theme.warning
    /// A press is under way: the pointer's `down`, or the keyboard's climbing fill (`HoldDriver.pressKey`).
    readonly property bool gitButtonPressing: gitButton.down || gitButton.holdProgress > 0
    Binding {
        target: pane
        property: "gitButtonRestarts"
        value: AppBackend.gitPathOffersRestart
        when: !pane.gitButtonPressing
        restoreMode: Binding.RestoreNone
    }

    /// The git box's state in one reading — shown, stored, and what the binary said — none of it readable off a
    /// picture. Automation-only, the same exposure as `GraphPane.view` (rules-refs/app-ui.md).
    function gitPathTally() {
        // The six flags stay together, ahead of `state=`: each verb-table row claims a run of them as one substring
        // (`verify::verbs`). `named` is a placeholder there at all — `native` alone passes an empty one. `native` is a
        // backslash in what the box shows, its value or its placeholder (規約 §パスの区切り). `answered` folds `ok` and
        // `old` so the supported minimum and the newest git read alike.
        return "shown=" + gitPathField.text + " stored=" + AppBackend.gitPath
             + " named=" + (gitPathField.placeholder !== "")
             + " native=" + (gitPathField.text.includes("\\") || gitPathField.placeholder.includes("\\"))
             + " answered=" + (AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old")
             + " offers=" + AppBackend.gitPathOffersRestart
             + " held=" + pane.gitButtonRestarts
             + " restart=" + AppBackend.restartWanted
             + " state=" + AppBackend.gitPathState
             + " version=" + AppBackend.gitPathVersion
             + " inUse=" + AppBackend.gitPathInUse
    }
    /// Opens the chooser beside a git — on a machine with several, the nearest useful start: the box's path, or with
    /// the box empty the git this run spawns.
    function openGitPicker() {
        const near = GitFacts.folderUrlOf(gitPathField.text !== "" ? gitPathField.text
                                                                  : AppBackend.gitPathInUse)
        if (near !== "")
            gitPicker.currentFolder = near
        gitPicker.open()
    }

    /// The chooser's answer, a `file:` URL, put in the box and written — the box's one way in besides the store.
    function takeGitPick(url) {
        gitPathField.text = GitFacts.pickedPath(url)
        pane.applyGitPath()
    }
    /// Answers in the chooser's place, through its door (`url` from `Harness.fileUrl`): the box spells a path only as
    /// the chooser's answer reads — a bare path would photograph a box spelled with `\` on Windows. Writing
    /// `AppBackend.gitPath` straight would pass with the wiring cut (app-ui.md §UI 自動化).
    function autoPickGitPath(url) {
        pane.takeGitPick(url)
    }

    /// The count's box, typed the way a hand leaves a box: the text, then the edit finished. Empty is the default.
    function autoTypeProcesses(concurrency) {
        concurrencyField.text = concurrency
        pane.applyConcurrency()
    }
    /// What the store holds for that chapter: the count and whether it is this machine's default (the number itself
    /// varies by machine).
    function processesTally() {
        return "concurrency=" + AppBackend.gitConcurrency
               + " default=" + (AppBackend.gitConcurrency === AppBackend.gitConcurrencyDefault)
    }
    /// The `REFRESH` chapter worked the way a hand works it: this repository's two boxes, the worktrees' reading
    /// through the chooser's own door (`pickWorktreesReading`), then the boxes that reading shows — the two bounds, or
    /// the one interval. An empty part is left alone.
    function autoTypeRefresh(floor, ceiling, reading, first, second) {
        if (floor !== "" || ceiling !== "") {
            if (floor !== "")
                refreshFloorField.text = floor
            if (ceiling !== "")
                refreshCeilingField.text = ceiling
            pane.applyRefreshBounds()
        }
        if (reading !== "")
            pane.pickWorktreesReading(pane.worktreesValues.indexOf(reading))
        if (AppBackend.worktreesReading === "fixed" && first !== "") {
            worktreesField.text = first
            pane.applyWorktrees()
        } else if (AppBackend.worktreesReading === "auto" && (first !== "" || second !== "")) {
            if (first !== "")
                worktreesFloorField.text = first
            if (second !== "")
                worktreesCeilingField.text = second
            pane.applyWorktreesBounds()
        }
    }
    /// The four bound boxes holding these texts as the way out is taken: typed and never finished, an empty one
    /// emptied — a validator with a floor sends no `editingFinished` for an empty box, so only the way out writes it.
    function autoLeaveBounds(floor, ceiling, first, second) {
        refreshFloorField.text = floor
        refreshCeilingField.text = ceiling
        worktreesFloorField.text = first
        worktreesCeilingField.text = second
    }
    /// What the store holds for that chapter.
    function refreshStore() {
        return "refresh=" + AppBackend.refreshFloorSecs + "-" + AppBackend.refreshCeilingSecs
               + " worktrees=" + AppBackend.worktreesReading
               + " worktrees_bounds=" + AppBackend.worktreesFloorSecs + "-" + AppBackend.worktreesCeilingSecs
               + " worktrees_secs=" + AppBackend.worktreesIntervalSecs
    }
    /// The store, with what the chooser shows and which boxes stand beside it — a box that is not there is an absence
    /// a picture leaves to the eye.
    function refreshTally() {
        const boxes = worktreesFloorField.visible ? "bounds" : worktreesField.visible ? "every" : "none"
        return pane.refreshStore() + " shown=" + worktreesChooser.wanted + " boxes=" + boxes
    }

    /// The readings and their words at matching indexes (`settings::WorktreesReading::word`).
    readonly property var worktreesValues: ["auto", "fixed", "off"]
    readonly property var worktreesWords: [qsTr("Automatic"), qsTr("Fixed"), qsTr("Off")]
    /// Picking the row at `index`; the chooser's handler is one line onto it, so a run takes the same road as a hand.
    /// The box's number goes with it, so a number typed and not yet finished is not lost to the pick.
    function pickWorktreesReading(index) {
        if (index < 0)
            return
        AppBackend.setWorktreesReading(pane.worktreesValues[index],
                                       worktreesField.text === "" ? 0 : Number(worktreesField.text))
        worktreesField.text = String(AppBackend.worktreesIntervalSecs)
    }

    /// Puts the fields back to what the store says, for the screen that just opened.
    function load() {
        fetchField.text = AppBackend.autoFetchMinutes > 0 ? String(AppBackend.autoFetchMinutes) : ""
        concurrencyField.text = String(AppBackend.gitConcurrency)
        pane.showRefreshBounds()
        pane.showWorktreesBounds()
        worktreesField.text = String(AppBackend.worktreesIntervalSecs)
        wholeHistoryBox.checked = AppBackend.initialCommits === 0
        commitsField.text = AppBackend.initialCommits > 0 ? String(AppBackend.initialCommits) : ""
        gitPathField.text = AppBackend.gitPath
        // One `git --version` for the path shown, on every opening: the file on disk can change with nobody retyping.
        // A path that does not answer is said in red under the box, in words the reader can act on — at startup it
        // only fell back to PATH's git (`Hub::resolve_git`), since this screen lies past the startup gate.
        AppBackend.openGitPathScreen()
        pane.readAuthorChoices()
    }
    /// Opened from an avatar, the entry is filled and focus goes to choosing the picture — the only focus this
    /// screen places as it opens (`SettingsDialog.onOpened`).
    function focusPrefill() {
        if (pane.prefillEmail === "") {
            avatarWho.wanted = ""
            avatarWho.editText = ""
            return
        }
        const who = pane.prefillName + " <" + pane.prefillEmail + ">"
        avatarWho.wanted = who
        avatarWho.editText = who
        chooseAvatar.forceActiveFocus()
    }

    // Empty is off.
    function applyFetch() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
    }
    // Empty is the default (zero at once is meaningless). Also written from `applyFields`: a validator with a floor
    // rejects an empty string, so `onEditingFinished` never fires for an emptied field.
    function applyConcurrency() {
        AppBackend.setGitConcurrency(concurrencyField.text === "" ? 0 : Number(concurrencyField.text))
    }
    // The fixed interval, kept whichever reading is chosen. Empty keeps the number held, and the box shows it again.
    function applyWorktrees() {
        AppBackend.setWorktreesReading(AppBackend.worktreesReading,
                                       worktreesField.text === "" ? 0 : Number(worktreesField.text))
        worktreesField.text = String(AppBackend.worktreesIntervalSecs)
    }
    // A pair is written whole: the ceiling is held to the floor by core (`session::pace_bounds_secs`), so its boxes
    // show what came back. Empty is that bound's default. Each pair puts back only its own boxes — the way out writes
    // one pair after the other, and the second's boxes may hold an edit nobody finished.
    function applyRefreshBounds() {
        AppBackend.setRefreshBounds(pane.boxSecs(refreshFloorField), pane.boxSecs(refreshCeilingField))
        pane.showRefreshBounds()
    }
    function applyWorktreesBounds() {
        AppBackend.setWorktreesBounds(pane.boxSecs(worktreesFloorField), pane.boxSecs(worktreesCeilingField))
        pane.showWorktreesBounds()
    }
    function boxSecs(field) {
        return field.text === "" ? 0 : Number(field.text)
    }
    function showRefreshBounds() {
        refreshFloorField.text = String(AppBackend.refreshFloorSecs)
        refreshCeilingField.text = String(AppBackend.refreshCeilingSecs)
    }
    function showWorktreesBounds() {
        worktreesFloorField.text = String(AppBackend.worktreesFloorSecs)
        worktreesCeilingField.text = String(AppBackend.worktreesCeilingSecs)
    }
    // Two controls, one value: the box asks for no window at all (core's `0`); the field answers only while the box
    // is clear, empty being the default. Written from `applyFields` too, as `applyConcurrency` is.
    function applyCommits() {
        AppBackend.setInitialCommits(wholeHistoryBox.checked ? 0
                                     : commitsField.text === "" ? AppBackend.initialCommitsDefault
                                     : Number(commitsField.text))
    }
    // Writing the path also asks the binary. Empty is an answer (the git `PATH` resolves), so nothing is guarded.
    function applyGitPath() {
        AppBackend.setGitPath(gitPathField.text)
    }
    /// Writes every field, for the way out of the screen. The git path is only written — putting it in force is the
    /// button's errand (`AppBackend.applyGitPathNow`): closing a settings screen must not restart the window. The one
    /// writer of an emptied number box whose validator has a floor (`Initial commits`, the fetch's `Interval`, …):
    /// empty is Intermediate there, so Qt sends no `editingFinished` on Enter or on focus out.
    function applyFields() {
        pane.applyFetch()
        pane.applyRefreshBounds()
        pane.applyWorktrees()
        pane.applyWorktreesBounds()
        pane.applyConcurrency()
        pane.applyCommits()
        pane.applyGitPath()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    HelpText {
        text: qsTr("Kept by Platitude GG in its own settings file. Nothing here is written to your git configuration.")
    }

    // Which git this app runs (user decision — ask before changing): on a machine with several, the reader picks the
    // one. One box and, under it, what the binary at that path answered. First, because every other chapter decides
    // how often and how much to ask, and this one whom all of it is asked of. Global only: no repository overrides it
    // yet (P3-確認事項 §app). A run settles its git once, at start (`Hub::resolve_git`). Order (user decision):
    // help → box and button → the version line (the warning, while a restart is offered) → what a restart costs.
    // Headless: `settings-git-path` / `settings-git-leave`.
    SettingsSection {
        caption: qsTr("GIT EXECUTABLE")
        HelpText {
            // Wording follows other GUIs (four looked at): none says "which git to run", so the first sentence opens
            // on the term of art; the heading `GIT EXECUTABLE` matches two of them. The answer always stands under
            // the box, so no `Test` button (IntelliJ has one).
            text: qsTr("Path to the git executable this app runs, in every repository. Empty is the one found on PATH, which the box names.")
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            // The box is the chooser (user decision — ask before changing) — a typed path one letter wrong names a
            // binary that is not there. It keeps a typing box's ground (user decision), the one exception to
            // §選ぶ欄と打つ欄: bare among grounded boxes it reads as disabled.
            // Full width: a path has no fixed length (デザイン規約 §レイアウト初期値).
            FormField {
                id: gitPathField
                Layout.fillWidth: true
                choosing: true
                // What an empty box is read as (the git PATH resolves), given as the resolved path itself: "the git on
                // PATH" is nowhere a reader can look, and this chapter exists because a machine has several, so it
                // must name the one running. From `process::default_program_path` — one step past what the app
                // spawns: behind Git for Windows' launcher on PATH, the git it launches. Spelled with `/` where it
                // comes in (規約 §パスの区切り).
                placeholder: AppBackend.gitPathInUse
                onPicking: pane.openGitPicker()
            }
            // One session runs one git: swapped mid-run, open tabs would be on the old one and new tabs on the new, and
            // the window could not say which answered — so a chosen git takes the window restarting itself (user
            // decision — ask before changing), and only on this press: leaving the screen only writes the path. The
            // one button puts a git in force, standing only while there is one to put — choosing is the box's errand.
            // `warning` frame and words, held: the press costs the window, so even the words wear the warning
            // (§回数押し's "the colour is the frame's and the mark's" is about badges). The hold →
            // `applyGitPathNow` → `restartWanted` → `Main`'s `root.close()` → `main::start_again`. No headless verb
            // can drive the hold — the window closes before a picture; the real window's check is P5-確認事項
            // §実測済み.
            ActionButton {
                id: gitButton
                visible: pane.gitButtonRestarts
                implicitHeight: Theme.controlHeight
                text: qsTr("Restart to apply")
                tone: pane.gitButtonTone
                // Framed beside a framed box (rules-refs/app-ui.md「肯定側のボタンは枠で名乗り」), in the word's
                // colour to say the press costs something.
                frameColor: pane.gitButtonTone
                holdPresses: Metrics.holdPressesWarned
                activeFocusOnTab: true
                onHeld: AppBackend.applyGitPathNow()
            }
        }
        // What the git at that path said (`version::Probe`), below the box: the explanation above, what is true now
        // below (`SettingsSection`) — right under the box, so the short answer about the path comes before the long
        // sentence about what applying costs. The version wears the chip (規約 §git 用語のコード表記) — a bare
        // lowercase `git` opening a line reads as a typo. One line beside the chip: a chip cannot flow inside wrapping
        // text (rich text would drop 同 §'s decision), so a longer sentence would hang its second line off the
        // chip's right edge; anything more goes below.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            visible: pane.gitPathChip !== "" || pane.gitPathWord !== ""
            CodeChip {
                id: gitVersionChip
                visible: pane.gitPathChip !== ""
                word: pane.gitPathChip
                tint: pane.gitPathInk
                // The one chip in this window that is a value, not decoration — a field like every word in the column
                // (`SettingsDialog`): its spelling is what a reader copies into a bug report.
                grabbable: true
                // On the first line — the tail may wrap to two.
                Layout.alignment: Qt.AlignTop
            }
            CardText {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                visible: pane.gitPathWord !== ""
                color: pane.gitPathInk
                pixelSize: Theme.fontMd
                text: pane.gitPathWord
            }
        }
        // What a restart costs (nothing), kept off the chip's one-sentence line: the one thing a reader asked to
        // restart with the exit shut wants to know. Comes and goes with the button.
        CardText {
            Layout.fillWidth: true
            visible: AppBackend.gitPathOffersRestart
            color: Theme.warning
            pixelSize: Theme.fontMd
            text: qsTr("The window comes back with the same repositories open.")
        }

        FileDialog {
            id: gitPicker
            title: qsTr("Choose git")
            // The folder is set on the way in (`openGitPicker`), not bound: a dialog re-homing itself mid-walk would
            // take the reader's place away. No name filter: git has no extension off Windows, a wrapper script is fine,
            // and the version written back says whether the choice was a git.
            onAccepted: pane.takeGitPick(selectedFile.toString())
        }
    }

    // The rest of the category is held while a git waits to be applied (user decision — ask before changing,
    // `SettingsDialog.askingGitPath`): work set now would go with the window being replaced. `enabled` takes the
    // chapter's words to the disabled step with its boxes (§無効).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("AUTOMATIC FETCH")
        HelpText {
            text: qsTr("Runs git fetch --prune on every open repository, at most once per interval. Empty means off; %1 minutes is the longest interval.")
                  .arg(AppBackend.autoFetchMaxMinutes)
        }
        LabeledField {
            caption: qsTr("Interval")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: fetchField
                    implicitWidth: 160
                    placeholder: qsTr("off")
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: 1
                        top: AppBackend.autoFetchMaxMinutes
                    }
                    // On Enter or focus out — per keystroke would pass through "1" on the way to "10".
                    onEditingFinished: pane.applyFetch()
                    onAccepted: pane.accepted()
                }
                // A field one line long, as the value beside it is: a `CardText` files the face's leading under its
                // line, so centred beside the value the unit stands half of it high (3px on macOS).
                LineText {
                    text: qsTr("minutes")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
    }

    // Beside the fetch: the other thing done on its own while a repository is open (`session::pace`). Said once at the
    // head, before both rows (`SettingsSection`). Never "a higher ceiling makes it lighter": where the
    // machine cannot keep up, the range is only a target. A file with no reading key reads `Automatic`, the fixed
    // interval kept for a return to `Fixed`. Headless: `settings-refresh <floor>:<ceiling>:<reading>:<first>:<second>`
    // / `settings-refresh-leave` (each pair puts back only its own boxes).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("REFRESH")
        HelpText {
            text: qsTr("Applies to all repositories. Automatic intervals adapt to how long refreshes take. Shorter intervals update sooner but use more resources.")
        }
        // Both rows are a mode, then its boxes, in the same columns. This worktree is always read automatically, so its
        // mode is a word standing where the other row's chooser stands, its letters where the chooser's letters are —
        // not a disabled chooser, nor an item nobody can pick.
        LabeledField {
            caption: qsTr("Current worktree")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                LineText {
                    Layout.leftMargin: worktreesChooser.leftPadding + worktreesChooser.contentItem.leftPadding
                    Layout.preferredWidth: worktreesChooser.implicitWidth - Layout.leftMargin
                    text: qsTr("Automatic")
                    color: Theme.textPrimary
                }
                FormField {
                    id: refreshFloorField
                    implicitWidth: fetchField.implicitWidth
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.paceMinSecs
                        top: AppBackend.paceMaxSecs
                    }
                    onEditingFinished: pane.applyRefreshBounds()
                    onAccepted: pane.accepted()
                }
                LineText {
                    text: qsTr("to")
                    color: Theme.textSecondary
                }
                FormField {
                    id: refreshCeilingField
                    implicitWidth: fetchField.implicitWidth
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.paceMinSecs
                        top: AppBackend.paceMaxSecs
                    }
                    onEditingFinished: pane.applyRefreshBounds()
                    onAccepted: pane.accepted()
                }
                LineText {
                    text: qsTr("seconds")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
        LabeledField {
            caption: qsTr("Other worktrees")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                // Picking only: these three are the whole set (デザイン規約 §選ぶ欄と打つ欄).
                AppCombo {
                    id: worktreesChooser
                    implicitWidth: fetchField.implicitWidth
                    pickOnly: true
                    model: pane.worktreesWords
                    wanted: pane.worktreesWords[Math.max(0, pane.worktreesValues.indexOf(AppBackend.worktreesReading))]
                    onActivated: index => pane.pickWorktreesReading(index)
                }
                // Each reading shows only the boxes it reads: the two bounds, the one interval, or none — `Off`
                // reads no worktree at all, and a number or a unit beside it would say otherwise.
                FormField {
                    id: worktreesFloorField
                    visible: AppBackend.worktreesReading === "auto"
                    implicitWidth: fetchField.implicitWidth
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.paceMinSecs
                        top: AppBackend.paceMaxSecs
                    }
                    onEditingFinished: pane.applyWorktreesBounds()
                    onAccepted: pane.accepted()
                }
                LineText {
                    visible: worktreesFloorField.visible
                    text: qsTr("to")
                    color: Theme.textSecondary
                }
                FormField {
                    id: worktreesCeilingField
                    visible: worktreesFloorField.visible
                    implicitWidth: fetchField.implicitWidth
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.paceMinSecs
                        top: AppBackend.paceMaxSecs
                    }
                    onEditingFinished: pane.applyWorktreesBounds()
                    onAccepted: pane.accepted()
                }
                FormField {
                    id: worktreesField
                    visible: AppBackend.worktreesReading === "fixed"
                    implicitWidth: fetchField.implicitWidth
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.worktreesIntervalMin
                        top: AppBackend.worktreesIntervalMax
                    }
                    onEditingFinished: pane.applyWorktrees()
                    onAccepted: pane.accepted()
                }
                LineText {
                    visible: worktreesFloorField.visible || worktreesField.visible
                    text: qsTr("seconds")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
    }

    // Beside the fetch interval: the same kind of answer, about this machine (規約 §設定の画面). In force at once
    // (`Hub::set_git_concurrency`); empty is this machine's default (`process::default_concurrency`). Headless:
    // `settings-processes <n>`.
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("GIT PROCESSES")
        HelpText {
            text: qsTr("How many git commands run at once, in every repository. Whatever you are waiting on goes first and always finds room: the reads made in the background, and anything waiting on a network or another program, never take more than a quarter between them. %1 is the most.")
                  .arg(AppBackend.gitConcurrencyMax)
        }
        LabeledField {
            caption: qsTr("At once")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: concurrencyField
                    implicitWidth: 160
                    placeholder: String(AppBackend.gitConcurrencyDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: 1
                        top: AppBackend.gitConcurrencyMax
                    }
                    onEditingFinished: pane.applyConcurrency()
                    onAccepted: pane.accepted()
                }
                LineText {
                    text: qsTr("commands")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
    }

    // The number of commits a graph opens with (`session::DEFAULT_LOG_LIMIT` by default; floor and no ceiling,
    // `session::log_limit`): the `Initial commits` box and, under it, the `Load the whole history` box (user decision
    // — ask before changing). How the rest loads in steps is 規約 §グラフ・インタラクション定数「読み込み窓の切れ目」.
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("COMMIT GRAPH")
        HelpText {
            text: qsTr("How much history a graph opens with. The rest is loaded from the row at the bottom of it, a quarter of this at a time — with the whole history there is no such row. %1 is the smallest window.")
                  .arg(AppBackend.initialCommitsMin)
        }
        LabeledField {
            caption: qsTr("Initial commits")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: commitsField
                    implicitWidth: 160
                    // Disabled, not cleared, under the whole history (§無効): unchecking gives the number back.
                    enabled: !wholeHistoryBox.checked
                    placeholder: String(AppBackend.initialCommitsDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    // No top: the only ceiling is the property's own type (`session::log_limit`).
                    validator: IntValidator {
                        bottom: AppBackend.initialCommitsMin
                    }
                    onEditingFinished: pane.applyCommits()
                    onAccepted: pane.accepted()
                }
                LineText {
                    text: qsTr("commits")
                    color: commitsField.enabled ? Theme.textSecondary : Theme.textMuted
                }
                Item { Layout.fillWidth: true }
            }
        }
        // The whole history is said by a box (user decision — ask before changing): a yes or no, and the number box
        // takes numbers only — empty there already means the default, its placeholder showing it. Checked, the number
        // box is disabled keeping what was typed, and the graph's tail footer does not appear at all.
        AppCheckBox {
            id: wholeHistoryBox
            text: qsTr("Load the whole history")
            // `onToggled`, not `onCheckedChanged`: that would also fire for `load()` putting the box back, and write.
            onToggled: pane.applyCommits()
        }
    }

    // The one place avatars are assigned, replaced and removed (the other way in is a face's pen, `AvatarButton`). A
    // picture is written the moment it is picked: this chapter has no Save. The list is a table — face, name, address,
    // `Remove` — whose name column is shared, so every address starts on one x (`AvatarAssignRow`).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("AVATARS")
        HelpText {
            text: qsTr("An avatar for anyone whose commits you read. The list offers this repository's authors. Nothing is fetched: it comes from one of your own files, copied in beside these settings.")
        }
        Repeater {
            id: avatarRepeater
            /// The widest row's name, so every address starts on the same x (as `AppMenu.codeColW`). Settled by the
            /// rows' own seat changes: a binding re-runs only on `count`, and a same-size reassignment replaces every
            /// row.
            property real nameColW: 0
            function settleNameColW() {
                let widest = 0
                for (let i = 0; i < avatarRepeater.count; i++) {
                    const row = avatarRepeater.itemAt(i)
                    if (row && row.nameSeat !== undefined)
                        widest = Math.max(widest, row.nameSeat)
                }
                nameColW = widest
            }
            onItemRemoved: settleNameColW()
            model: pane.assigned
            delegate: AvatarAssignRow {
                Layout.fillWidth: true
                nameColW: avatarRepeater.nameColW
                pointedAtRow: pane.pointedAtRow
                onNameSeatChanged: avatarRepeater.settleNameColW()
                onRemoveAccepted: (email, url) => leaving.accept(email, url)
                onRemoveLanded: email => leaving.give(email)
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            AppCombo {
                id: avatarWho
                Layout.fillWidth: true
                placeholder: qsTr("name or email")
                model: pane.authorChoices
                // Enter opens the picker, as the button beside it does (user decision — ask before changing): once the
                // name is settled, choosing the picture is all this row can do. Nothing while no name is there, the
                // same condition that lowers the button. The git box's Enter rule (`FormField.answerKey`) for a box
                // that also takes typing.
                onSubmitted: pane.openAvatarPicker()
            }
            ActionButton {
                id: chooseAvatar
                implicitHeight: Theme.controlHeight
                // The feature's one word (デザイン規約 §アバターを与える); a bare noun for a kind about to be chosen
                // (§長さ).
                text: qsTr("Choose avatar…")
                // A plain frame in a form row next to a combo (rules-refs/app-ui.md「肯定側のボタンは枠で名乗り」).
                frameColor: Theme.borderDefault
                activeFocusOnTab: true
                enabled: pane.chosenEmail !== ""
                onActivated: pane.openAvatarPicker()
            }
        }
        CardText {
            Layout.fillWidth: true
            visible: AppBackend.avatarErrorKind !== ""
            color: Theme.danger
            pixelSize: Theme.fontSm
            // Our words off core's kind and facts; an OS failure adds its own line under ours (`Words.avatarFailure`).
            text: Words.avatarFailure(AppBackend.avatarErrorKind,
                                      AppBackend.avatarErrorFacts,
                                      AppBackend.avatarErrorSaid)
        }
    }

    FileDialog {
        id: avatarPicker
        title: qsTr("Choose avatar")
        // Patterns from the store so the two cannot drift; the words are ours (rules-refs/app-ui.md「Rust に文言を置かない」).
        // They name the two formats — a category word would be a second name for the avatar (デザイン規約
        // §アバターを与える) — so these words and `avatar::EXTENSIONS` (with what `picture` decodes) change together.
        nameFilters: [qsTr("PNG and JPEG (%1)").arg(AppBackend.avatarPatterns)]
        onAccepted: AppBackend.assignAvatar(pane.chosenEmail, pane.chosenName, selectedFile.toString())
    }
    // The rows' removals, kept here: a row is rebuilt with every assignment that lands.
    readonly property AvatarRemovals leaving: AvatarRemovals {
        id: leaving
        filed: AppBackend.avatars
        onRemove: email => AppBackend.removeAvatar(email)
    }
}

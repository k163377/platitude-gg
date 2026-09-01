import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Every setting this app can change, on one screen with the categories down the left: what Platitude GG keeps in its
// own file, and what it writes into git's. The two are kept apart because where a value is stored is the one thing
// about it a reader cannot see — so it is the split the screen is built on, and each category says it in a line of
// its own. **How far a git value reaches is a level below that**, inside the git category: `GLOBAL` and
// `REPOSITORY OVERRIDE` are groups of chapters there, not categories of their own (規約 §設定の画面).
//
// The whole window, not a card: a settings screen is read rather than answered, and a card sized to its own content
// grows a scrollbar as soon as one chapter does. The one card left in this family is the identity gate
// (`IdentityDialog`), which is a question.
AppDialog {
    id: settingsDialog

    fills: true

    /// Which category is showing — `"app"` or `"git"`. The entries that name a category are doors into this one
    /// screen, and the door decides which of them the reader lands on.
    property string category: "app"

    /// The categories, and what each is called. **One list**: the rail lays it out and the title reads the current
    /// word out of it, so the two can never come to call the same category by different names.
    readonly property var categories: [{ key: "app", word: qsTr("Application") },
                                       { key: "git", word: qsTr("Git") }]
    readonly property string categoryWord: {
        for (let i = 0; i < settingsDialog.categories.length; i++) {
            if (settingsDialog.categories[i].key === settingsDialog.category)
                return settingsDialog.categories[i].word
        }
        return ""
    }

    /// The tab this screen reads git through: the avatar candidates come off its graph, the merge editor off its
    /// working tree. The settings themselves are global, but "whose commits are these" and "what would git launch"
    /// are read where the person is.
    property var curPage: null

    /// The strip, for the git category's repository group: it offers the repositories standing in it, and lands on
    /// the one the reader is looking at. Not `curPage` — that is the tab in front, and that group is the one place in
    /// the window where the answer may be about a repository nobody is looking at.
    required property TabsModel tabsModel

    /// Opens the screen on one category. The only way in — an `open()` that left the category where the last reader
    /// put it would answer a different question than the one the entry asked.
    function openAt(which) {
        settingsDialog.category = which
        settingsDialog.open()
    }

    /// Automation: the rail row for `which`, pressed through its own handler (`settings-switch`). Answers whether
    /// there was such a row — the run waits rather than counting a press it never made.
    function autoTapCategory(which) {
        for (let i = 0; i < categoryRepeater.count; i++) {
            const row = categoryRepeater.itemAt(i)
            if (row && row.modelData.key === which) {
                row.tap()
                return true
            }
        }
        return false
    }
    /// Automation: which chapters the screen is actually showing, for the verb that presses the rail — the category
    /// property is the input side, and a run that read it back would be reporting its own press.
    readonly property bool autoAppShown: appPane.autoAppShown
    readonly property bool autoGitShown: gitPane.visible

    // ---- what the application category answers for, forwarded ------------
    // The avatar verbs are the window's, so they are finished by `WindowAutoActDriver` and ask the screen; the screen
    // passes the question on to the pane that owns the list.
    readonly property int autoAvatarRows: appPane.autoAvatarRows
    readonly property bool autoAvatarComboOpen: appPane.autoAvatarComboOpen
    function autoAvatarRowLit(at) {
        return appPane.autoAvatarRowLit(at)
    }
    function autoAvatarRowPainted(at) {
        return appPane.autoAvatarRowPainted(at)
    }
    function autoAvatarOfferCombo() {
        appPane.autoAvatarOfferCombo()
    }
    function autoAvatarHoldRemove(at) {
        return appPane.autoAvatarHoldRemove(at)
    }

    // ---- what the git category answers for, forwarded ---------------------
    // The half of the screen that talks to git lives in `SettingsGitPane`, both of its groups; the window's harness
    // asks the screen, so the screen passes the question on. `opened` is the screen's to add — a pane that is only
    // hidden still answers.
    readonly property bool autoToolsLoadingReady: settingsDialog.opened && gitPane.autoToolsLoadingReady
    readonly property bool autoToolsSettledReady: settingsDialog.opened && gitPane.autoToolsSettledReady
    function reportTool() {
        gitPane.reportTool()
    }
    readonly property bool autoRepoReady: settingsDialog.opened && gitPane.autoRepoReady
    readonly property bool autoRepoComboOpen: gitPane.autoRepoComboOpen
    readonly property int autoRepoRows: gitPane.autoRepoRows
    function autoOfferRepos() {
        gitPane.autoOfferRepos()
    }
    // The line-ending chapter of that same group. **Only the repository level is ever driven**: the group above it
    // writes `--global`, which is the machine's own file and not a run's to touch (verify-ui スキル).
    readonly property bool autoRepoEndingsReady: settingsDialog.opened && gitPane.autoRepoEndingsReady
    readonly property string autoRepoEndingHeld: gitPane.autoRepoEndingHeld
    /// The global chapter's own read has answered. Reported beside the repository one rather than driven: the two
    /// chapters are one wiring asked at two levels, and the level nothing may write is the level a picture is the
    /// only other evidence for.
    readonly property bool autoGlobalEndingsAnswered: settingsDialog.opened && gitPane.autoEndingsAnswered
    function autoPickRepoEnding(value) {
        return gitPane.autoPickRepoEnding(value)
    }
    /// Sends the chapters to their foot, where the group this run is about stands. The screen is two groups deep and
    /// the window is not that tall (`settings_fit`), so a run that photographed the resting position would be
    /// photographing the group above the one it just wrote into.
    function autoShowChapterFoot() {
        chapters.contentY = Math.max(0, chapterCol.implicitHeight - chapters.height)
    }
    function reportRepoEndings() {
        gitPane.reportEndings()
        gitPane.reportRepoEndings()
    }
    /// Automation: shows the repository standing at `at` in the strip, through the same call a pick from the list
    /// makes. Answers whether there was such a row.
    function autoShowRepoAt(at) {
        return gitPane.autoShowRepoAt(at)
    }
    function reportRepo() {
        gitPane.reportRepo()
        // Whether every chapter can be got to: they fit, or the bar that sends them is standing. **A photograph
        // cannot say it** — a column cut off at the window's edge is drawn exactly like one that ends there (the
        // blind spot `details-fit` exists for), and the one thing that silently breaks it is a content height read
        // off implicit sizes that a wrapping label under-reports.
        AppBackend.report("settings_fit reach="
                          + (chapterCol.implicitHeight <= chapters.height || chaptersBar.visible)
                          + " content=" + Math.round(chapterCol.implicitHeight)
                          + " view=" + Math.round(chapters.height)
                          + " bar=" + chaptersBar.visible)
    }

    /// The author an avatar's badge was pressed on. The screen opens
    /// already carrying whom it is about, so the only thing left to do is
    /// name the picture.
    property string prefillName: ""
    property string prefillEmail: ""

    onOpened: {
        // Nothing is being asked yet, whatever the last reader left standing.
        settingsDialog.askingLeave = false
        appPane.load()
        gitPane.loadIdentity()
        gitPane.loadTool()
        // One `git config` against the user's own file, so it rides the screen opening rather than the category
        // showing — the same order of cost as the identity read above.
        gitPane.loadEndings()
        // The git category's repository group lands on the one the reader is looking at, and does it here rather than
        // on the category showing: "first, the repository I am in" is about the screen opening, not about which
        // category is read first — and coming back to the category would otherwise throw away the repository they had
        // chosen. Two `git config` reads, which is the same order of cost as the identity read above; the
        // eight-second one (`--tool-help`) still waits for the category itself.
        gitPane.landOnFront()
        // Opened from an avatar, the caret is already spoken for and the screen has none left to place.
        if (!appPane.focusPrefill()) {
            if (settingsDialog.category === "git")
                gitPane.focusIdentity()
            else
                appPane.focusFetch()
        }
        // Through the press rather than the popup: what these two are for
        // is the state a finger on the field leaves behind, and opening
        // the screen from here would photograph that just as well with the
        // wiring cut. Last, because a press also takes the caret — the
        // focus settled just above is the one a person would be taking it
        // from.
        if (AppBackend.autoAct === "settings-tools" || AppBackend.autoAct === "settings-tools-loading")
            gitPane.pressToolField()
    }
    // Escape and the button are the same exit, so both leave the fields
    // written: with no Cancel there is nothing for a discard to mean. The
    // identity is the exception — git cannot be given both of its keys in
    // one go, so it keeps a Save of its own in the chapter it belongs to.
    onClosed: {
        settingsDialog.askingLeave = false
        settingsDialog.applyFields()
        settingsDialog.prefillName = ""
        settingsDialog.prefillEmail = ""
    }
    // Only the way out writes them, and each category writes its own: a field finished with in one of them has no
    // business queueing a `git config` for another.
    function applyFields() {
        appPane.applyFields()
        gitPane.applyTool()
    }

    /// An identity chapter is holding an edit git has not been given, and the reader is on their way out. **The one
    /// thing on this screen that can be lost** — every other field writes as it is finished with, so this is the
    /// only place a Save stands between what is typed and what git holds (規約 §設定の画面).
    readonly property int unsavedIdentities: gitPane.unsavedIdentities
    /// The same, told apart: the two chapters read out of different files, so a way out that stopped has to be able
    /// to say which of them stopped it.
    readonly property bool autoUnsavedGlobal: gitPane.unsavedIsGlobal
    readonly property bool autoUnsavedRepo: gitPane.unsavedIsRepo
    /// How wide the screen's one block is: the rail, the line beside it, and the column of chapters, with the same
    /// step on both sides of that line. Everything on the screen that is not a full-width rule is laid inside it.
    /// **Symmetric on purpose**: one `spaceXxl` of air outside the rail, and one on the far side of the column for
    /// the bar to stand in. Centre the block and the ink is centred with it — a lane on one side only would put
    /// everything the reader looks at that far left of the middle.
    readonly property real blockWidth: 2 * Theme.spaceXxl + Theme.settingsRailWidth + 2 * Theme.spaceLg
                                       + Theme.borderWidth + Theme.textWidth

    /// A way out was taken over an unsaved identity and turned down. **The mark is armed, not the screen blocked**:
    /// the reader was shown what is holding it and the next press goes through (規約 §設定の画面).
    property bool askingLeave: false

    /// The way out, for the two things that take it: the `✕` in the corner and Escape. One function so they cannot
    /// come apart, and so a run enters the same road a hand does (規約 §UI 自動化の因果性).
    ///
    /// **The first press over an unsaved identity does not close.** It puts the reader in front of the chapter that
    /// is holding it — the right category, scrolled to the boxes — and turns the mark. Nothing is asked in words and
    /// no second window opens: what a person needs at that moment is to *see* the thing, and the Save it needs is
    /// standing right there. **The second press closes**, because a way out that can be refused twice is not a way
    /// out. Leaving puts the boxes back, so a screen opened again is not still offering the edit that was dropped.
    function escapeOut() {
        if (settingsDialog.unsavedIdentities > 0 && !settingsDialog.askingLeave) {
            settingsDialog.category = "git"
            settingsDialog.askingLeave = true
            settingsDialog.showUnsaved()
            return
        }
        if (settingsDialog.unsavedIdentities > 0)
            gitPane.dropUnsavedIdentities()
        settingsDialog.close()
    }
    /// Sends the chapters to the boxes that are holding the edit. Mapped rather than measured: the chapters are two
    /// components deep and only the column they are laid into knows where they ended up.
    function showUnsaved() {
        const item = gitPane.unsavedIdentityItem()
        if (!item)
            return
        const at = item.mapToItem(chapterCol, 0, 0).y
        chapters.contentY = Math.max(0, Math.min(at - Theme.spaceXl,
                                                 chapterCol.implicitHeight - chapters.height))
    }
    /// Typing again takes the arming off: the mark is about a press the reader made, not about the state of the
    /// boxes, and an edit made after it is one they have not been shown yet.
    onUnsavedIdentitiesChanged: settingsDialog.askingLeave = false
    /// Automation: leaves the global identity holding an edit, so the way out has something to stop for.
    function autoTypeIdentity(text) {
        gitPane.autoTypeIdentity(text)
    }

    // Two bands, and the rule between them is what says the top one does not move (規約 §設定の画面). It runs the
    // whole width — the screen covers the window, so a line stopped short of the frame would read as an unfinished
    // one rather than as the edge of a band. **What is inside the bands does not**: both are laid in the one block
    // the screen is centred on, so the title stands over the rail and the way out over the column it closes. That is
    // what the padding here is 0 for — each band carries its own, and the rule carries none.
    padding: 0

    contentItem: ColumnLayout {
        spacing: 0

        // Escape is heard as a shortcut rather than left to the popup's own `closePolicy`, which needs the key to
        // reach the screen through whatever holds the caret. Owned by the screen, so Qt hands it over only while
        // this is the topmost thing open: a list standing over it still takes the first press for itself and the
        // screen stays (qmltestrunner 実測, `tst_esc7`).
        Shortcut {
            // `sequences` rather than `sequence`: Cancel is more than one key on some platforms, and binding the
            // single form takes only the first of them (Qt warns about exactly this).
            sequences: [StandardKey.Cancel]
            enabled: settingsDialog.opened
            onActivated: settingsDialog.escapeOut()
        }

        // ---- the header band -------------------------------------------------
        // Laid in the same block the chapters are, so the title stands over the rail and the way out over the column
        // it closes (`SettingsHeader`). Its own file for the reason the panes are: this screen is at the length it
        // is held to, and the band is a whole thing rather than a line of it.
        SettingsHeader {
            Layout.maximumWidth: settingsDialog.blockWidth
            Layout.alignment: Qt.AlignHCenter
            word: settingsDialog.categoryWord
            unsaved: settingsDialog.unsavedIdentities > 0
            armed: settingsDialog.askingLeave
            onClosed: settingsDialog.escapeOut()
        }
        // The band's edge is the window's, so this one line is not laid in the block.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }

        // ---- the categories and their chapters -------------------------------
        // **Centred, and the margins are what a narrowing window eats first** (2026-08-30 ユーザー指示). The block
        // is a fixed thing — a rail of a known width beside a column set to the width words are read at — so on a
        // wide window the leftover is air on both sides rather than a screen hanging off the left edge. Narrower
        // than the block, `fillWidth` takes over and the column gives way; `blockWidth` is the ceiling, not a floor.
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.maximumWidth: settingsDialog.blockWidth
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Theme.spaceLg
            Layout.bottomMargin: Theme.spaceLg
            spacing: Theme.spaceLg

            // The categories. A list of names down the left of a screen, lit the way the sidebar's rows are
            // (規約 §左メニューの所作) — the same kind of thing in the same clothes. **Not the sidebar's width**
            // (2026-08-30 ユーザー報告「左メニューが長すぎ」): 260 is for a column of names this app did not write
            // — branches, remotes, tags, whatever anybody called them — and this one holds a handful of words it
            // did. `settingsRailWidth` leaves room for the longest a category is going to be. Fixed rather than
            // fitted, because a layout inside a layout fills by default and a rail that took whatever the chapters
            // did not want would move every time the category changed.
            ColumnLayout {
                Layout.fillWidth: false
                Layout.leftMargin: Theme.spaceXxl
                Layout.preferredWidth: Theme.settingsRailWidth
                Layout.alignment: Qt.AlignTop
                spacing: 0
                Repeater {
                    id: categoryRepeater
                    model: settingsDialog.categories
                    delegate: Rectangle {
                        id: categoryRow
                        required property var modelData
                        readonly property bool current: settingsDialog.category === categoryRow.modelData.key
                        /// What a press on this row does. The handler below is one line onto it so a run enters the
                        /// same road a hand does (規約 §UI 自動化の因果性).
                        function tap() {
                            settingsDialog.category = categoryRow.modelData.key
                        }
                        Layout.fillWidth: true
                        implicitHeight: Theme.rowHeight
                        color: categoryRow.current
                               ? Theme.bgSelected
                               : categoryHover.hovered ? Theme.bgHover : "transparent"
                        // Which category is holding something git has not been given. **The rail is where it has to
                        // be said**: the question at the foot names the chapter, but a reader standing in the other
                        // category cannot see either of them — and this is the one column on screen that is always
                        // showing both (2026-08-30. The shape JetBrains marks a modified settings page with).
                        NavIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.spaceSm
                            width: Theme.iconSm
                            height: Theme.iconSm
                            kind: "bang"
                            tint: Theme.warning
                            visible: categoryRow.modelData.key === "git"
                                     && settingsDialog.unsavedIdentities > 0
                        }
                        Label {
                            anchors.verticalCenter: parent.verticalCenter
                            x: Theme.spaceSm
                            text: categoryRow.modelData.word
                            font.pixelSize: Theme.fontMd
                            // **Every row is `textPrimary`, standing or not** (2026-08-30 ユーザー報告
                            // 「左メニューの文字が弱すぎる」). What says which one is showing is the wash under it,
                            // exactly as in the left menu these rows are dressed as — there a row does not go dim
                            // for not being the one selected. Dimming the other one also said the wrong thing: the
                            // category nobody is in is the one there is any reason to press (規約 §無効
                            // 「選ばれていないことを無効の色で言わない」).
                            color: Theme.textPrimary
                        }
                        HoverHandler {
                            id: categoryHover
                        }
                        TapHandler {
                            onTapped: categoryRow.tap()
                        }
                    }
                }
            }
            // The line between the categories and what they open, drawn like every other divider in the window.
            Rectangle {
                Layout.fillHeight: true
                implicitWidth: Theme.borderWidth
                color: Theme.borderSubtle
            }

            // The chapters, sent when they do not fit. A screen that fills the window is still a fixed height, and the
            // git category is two groups deep — so the thing a card was avoided for (規約 §設定の画面) arrives here
            // anyway, and this is where it can be answered without the screen resizing itself under the reader.
            //
            // **The panels' slab, with the idle step this ground needs** (デザイン規約 §ペインのスクロールバー.
            // 2026-08-30 ユーザー指示 =「settings のスクロールバーは意匠を足してよい」). The slab's three states are
            // counted up from whatever it stands on, and the default idle is the step above a *pane's* ground —
            // which on this screen is the ground exactly, so a resting bar was not there at all (実測: 辺の 5px が
            // `#0F172A`). One step further up (`borderSubtle`) is the same rule read against this ground, and it is
            // what lets the reader see there is more to read before touching anything. The floating thumb could not
            // say that: three tenths of one ink over `bgElevated` is barely a colour.
            //
            // **The bar stands at the window's edge, not against the chapters.** The band's right inset is spent
            // inside this view rather than outside it, so the room the reader can see to the right of the form is
            // where the bar goes (2026-08-30 ユーザー報告「余白が余ってるのにピッタピタにくっつけるのをやめて」) —
            // the chapters keep their own right edge, level with the `✕`, and nothing of theirs comes
            // near the ink. `scrollBarGutter` is not what does it: a nine-pixel gutter clears the thumb and nothing
            // more, which is the same "just barely" in a smaller size.
            // **No margin of its own.** The row's `spacing` is the step on both sides of the line, and an extra one
            // here put the chapters eight pixels further from it than the rail is — a difference small enough to
            // read as a mistake rather than as a choice (2026-08-30 ユーザー報告).
            Flickable {
                id: chapters
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: width
                contentHeight: chapterCol.implicitHeight
                // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: PaneScrollBar {
                    id: chaptersBar
                    idleColor: Theme.borderSubtle
                }

                ColumnLayout {
                    id: chapterCol
                    // Width rather than a margin: a `Flickable`'s content item is not a layout, so `Layout.*` on this
                    // one would be read by nobody. **The band's right inset is spent here** — the view runs to the
                    // window's edge so the bar can stand there — **and the column stops at the width a run of words
                    // is set at** (`textWidth`. 2026-08-30).
                    //
                    // A settings screen is read, and a sentence set across 1200 pixels is one the eye loses its
                    // place returning from: at `fontMd` that is around 180 characters, twice what a line should be.
                    // **The boxes stop there too** rather than only the prose — a form whose inputs are twice the
                    // width of the sentences explaining them reads as two columns that happen to be stacked, and a
                    // box a thousand pixels wide for a person's name is not asking for a name. Every settings screen
                    // worth copying does this (VS Code, Windows 11, GitHub all cap the column and leave the rest of
                    // a wide window empty); what fills the space here is the bar, at the far edge where it belongs.
                    width: Math.min(chapters.width - Theme.spaceXxl, Theme.textWidth)
                    spacing: Theme.spaceXl

                    SettingsAppPane {
                        id: appPane
                        visible: settingsDialog.category === "app"
                        curPage: settingsDialog.curPage
                        prefillName: settingsDialog.prefillName
                        prefillEmail: settingsDialog.prefillEmail
                        onAccepted: settingsDialog.escapeOut()
                    }

                    SettingsGitPane {
                        id: gitPane
                        visible: settingsDialog.category === "git"
                        curPage: settingsDialog.curPage
                        tabsModel: settingsDialog.tabsModel
                        // Its slow read is asked for by this, and by nothing else — being hidden is what says the reader
                        // did not ask for it (規約 §設定の画面). Following git is the other one: that goes on wherever the
                        // reader is standing.
                        showing: settingsDialog.opened && settingsDialog.category === "git"
                        screenOpen: settingsDialog.opened
                        onAccepted: settingsDialog.escapeOut()
                    }
                }
            }
        }
        // **No foot.** The `OK` that stood here closed a screen that had already written everything it was going to
        // write, which is a button for confirming nothing (2026-08-30 ユーザー判断). What is left is the way out in
        // the corner, and a way out does not need a second copy of itself along the bottom edge — every settings
        // screen worth copying (VS Code, Windows 11, the browsers) ends the same way: content to the bottom of the
        // window and nothing under it.
    }
}

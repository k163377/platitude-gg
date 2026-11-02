import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Every setting this app can change, on one window-filling screen (user decision — ask before changing): a card sized
// to its contents grows a scroll bar the moment one chapter grows, so the identity gate (`IdentityDialog`) is the one
// card left of this family. Categories down the left, split by where a value is stored (Platitude GG's own file /
// git's), and inside `Git` by how far it reaches (規約 §設定の画面) — one file per category (`SettingsAppPane` /
// `SettingsGitPane`), and in `Git` one more level, the groups (`SettingsGroup`: `GLOBAL`, and `REPOSITORY OVERRIDE` =
// `SettingsRepoPane`). **This screen keeps only the rail and the relaying**: the chapters, the fields and the reads
// and writes a category makes are its pane's. Chapters are `SettingsSection`, a field's heading `LabeledField`,
// explanations `HelpText`.
//
// Every word in the chapter column is a field (`CardText`), headings, captions and units included — the column is a
// sweep surface (規約 §右のペインの字は掴める), and a word that is not a field is a dead spot there: a press on it starts
// the selection on another line. The one chip that is a field is the git version (`SettingsAppPane`).
//
// Headless: `settings` / `settings-git` / `settings-switch` (the one road that presses the rail) / `settings-repo` /
// `settings-repo-pick` / `settings-escape` / `settings-leave`.
AppDialog {
    id: settingsDialog

    fills: true

    /// Which category is showing — `"app"` or `"git"`.
    property string category: "app"

    /// The categories, their words and marks. One list for the rail and the title, so the two cannot name a category
    /// differently; both wear the mark and keep the word. `Application` is a window (`app-window`, banded: the bare
    /// square is the maximize button's), `Git` a branch; the title's own mark is the gear (`SettingsHeader`).
    readonly property var categories: [{ key: "app", word: qsTr("Application"), icon: "app-window" },
                                       { key: "git", word: qsTr("Git"), icon: "branch" }]
    /// The entry for `category`, looked up once for both the word and the mark; undefined when no row matches.
    readonly property var categoryEntry: {
        for (const entry of settingsDialog.categories) {
            if (entry.key === settingsDialog.category)
                return entry
        }
        return undefined
    }
    readonly property string categoryIcon: settingsDialog.categoryEntry ? settingsDialog.categoryEntry.icon : ""
    readonly property string categoryWord: settingsDialog.categoryEntry ? settingsDialog.categoryEntry.word : ""

    /// The tab this screen reads git through: the settings are global, but the avatar candidates and the merge editor
    /// are read where the person is.
    property var curPage: null

    /// The strip, for the git category's repository group — which may be about a repository other than `curPage`.
    required property TabsModel tabsModel

    /// Opens the screen on one category. The only way in: a bare `open()` would keep the last reader's category. Every
    /// door meets here (`WindowDialogSeat.openSettingsAt`): the app menu's `Settings…` (`"app"`), the toolbar's
    /// identity badge (`"git"`), the conflict menu's `Edit in <merge editor>…` (`RepoPage.gitSettingsRequested`,
    /// `"git"`) and an avatar's pen badge (`"app"`). `Git` is opened by name only from the two doors called about one
    /// of its items; the app menu's row names nothing inside a category, so it opens at the head.
    function openAt(which) {
        settingsDialog.category = which
        settingsDialog.open()
    }

    /// Automation: presses the rail row for `which` through its own handler (`settings-switch`); false while there is
    /// no such row.
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
    /// The two panes, handed over whole for automation (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」).
    /// A hidden pane still answers — pair it with [`opened`] for "the screen is showing this".
    readonly property alias autoAppPane: appPane
    readonly property alias autoGitPane: gitPane
    /// The chapters' sweep hand, named so a run can enter it (`settings-sweep`).
    readonly property alias autoChapterHand: chapterHand

    /// Automation: sends the chapters to their foot, where the repository group stands — the resting position
    /// shows the group above it.
    function autoShowChapterFoot() {
        chapters.contentY = settingsDialog.chapterFoot()
    }
    /// Automation: whether the chapters stand at their foot as the column is now — it can grow after a send.
    function autoAtChapterFoot() {
        return chapters.contentY === settingsDialog.chapterFoot()
    }
    /// The `contentY` that brings the chapters' last line to the view's bottom.
    function chapterFoot() {
        return Math.max(0, chapterCol.implicitHeight - chapters.height)
    }
    /// Automation: the chapters' height, the room they have, and whether their bar stands — whether every chapter can
    /// be reached, which a photograph cannot say. What breaks it silently is a wrapping label under-reporting its
    /// implicit height.
    readonly property real chaptersContent: chapterCol.implicitHeight
    readonly property real chaptersView: chapters.height
    readonly property bool chaptersBarShown: chaptersBar.visible
    /// Automation: the chapters' middle-button hand (a middle button cannot be injected) and where it has sent them.
    readonly property alias autoMiddleHand: middleHand
    readonly property real chaptersAt: chapters.contentY

    /// The merge editor's list comes down as the screen opens, through the field's own press. Written from outside
    /// before opening.
    property bool pressToolOnOpen: false

    /// The author an avatar's badge was pressed on, so the screen opens already knowing whom it is about.
    property string prefillName: ""
    property string prefillEmail: ""

    onOpened: {
        settingsDialog.askingLeave = false
        appPane.load()
        gitPane.loadIdentity()
        gitPane.loadTool()
        // On open, not on entering the category — that would throw away a repository the reader had chosen. Two
        // `git config` reads; the slow `--tool-help` still waits for the category (`SettingsGitPane.askTools`).
        gitPane.landOnFront()
        // The screen opens to be read (user decision — ask before changing): a caret dropped in the first chapter
        // would take the reader's next keystroke into a value they did not come for. The one exception is the
        // avatar's badge, a door whose only errand is naming a picture: this is the one caret call here
        // (`IdentityFields.focusName` is the gate's — a card that is a question takes the caret).
        appPane.focusPrefill()
        // Through the press — opening the list directly would look the same with the wiring cut. Last, because a
        // press also takes the caret.
        if (settingsDialog.pressToolOnOpen)
            gitPane.pressToolField()
    }
    // Every exit writes the fields; there is no Cancel. Each field writes as its edit is finished (focus out / Enter /
    // a row picked); the identity keeps its own Save (`SettingsGitPane`).
    onClosed: {
        settingsDialog.askingLeave = false
        settingsDialog.applyFields()
        settingsDialog.prefillName = ""
        settingsDialog.prefillEmail = ""
        // The moment a setting changed outside the app is read again (rebase.updateRefs, which decides the drop's hold
        // — `RepoSession::reread_update_refs`); nothing watches for it otherwise.
        AppBackend.settingsClosed()
    }
    // Each category writes its own, so a field finished in one never queues a `git config` for the other.
    function applyFields() {
        appPane.applyFields()
        gitPane.applyTool()
    }

    /// How many identity chapters hold an edit git has not been given — the one thing on this screen that can be
    /// lost (`IdentityFields.dirty`).
    readonly property int unsavedIdentities: gitPane.unsavedIdentities
    /// The screen's one block, centred (user decision — ask before changing): `spaceXxl` + rail + `spaceLg` + line +
    /// `spaceLg` + chapters + `spaceXxl`, the same air at both ends so the ink centres with the block; the right
    /// `spaceXxl` is the bar's lane. A narrowing window loses the outer air first; the block is a ceiling. The title
    /// shares it; the exit stays at the window edge.
    readonly property real blockWidth: 2 * Theme.spaceXxl + Theme.settingsRailWidth + 2 * Theme.spaceLg
                                       + Theme.borderWidth + Theme.textWidth

    /// A way out over an unsaved identity was turned down once: the mark is armed and the next press goes through.
    property bool askingLeave: false

    /// A way out over a git waiting to be applied was turned down. Unlike [`askingLeave`] it never lets a press
    /// through; it falls with the offer. While the offer stands the screen holds on until it is answered (user
    /// decision — ask before changing): the identity holds an edit the reader may mean to drop, this a path already
    /// written, and leaving with the window's git and the setting apart is the very state that chapter exists to
    /// prevent.
    property bool askingGitPath: false
    onOpenedChanged: if (!settingsDialog.opened) settingsDialog.askingGitPath = false
    Connections {
        target: AppBackend
        function onGitPathChanged() {
            if (!AppBackend.gitPathOffersRestart)
                settingsDialog.askingGitPath = false
        }
    }

    /// Sends the chapters to the git box, the first chapter of its category (`SettingsAppPane`).
    function showGitPath() {
        chapters.contentY = 0
    }

    /// The way out for both the `✕` and Escape — one function so they cannot come apart. Over an unsaved identity
    /// the first press shows the chapter (user decision — ask before changing): the category moves to `Git`, the
    /// chapters scroll to it and the `✕` arms — asked where the thing is (規約 §可否・警告の出し場所). The second press
    /// closes: an exit that refuses twice is no exit. Closing puts the boxes back to git's answer, so a reopened
    /// screen does not still hold the edit it dropped. The unsaved edit is said before any press, in three places
    /// (user decision): `IdentityFields`' `Not given to git yet.` over its Save, the `✕` in `warning`
    /// (`SettingsHeader`) and the `!` on its category's rail row.
    function escapeOut() {
        // A chosen git holds the screen shut on every press (`askingGitPath`): the category moves to `Application`,
        // the chapter comes to the head and the `✕` arms. The way back is the box.
        if (AppBackend.gitPathOffersRestart) {
            settingsDialog.category = "app"
            settingsDialog.askingGitPath = true
            settingsDialog.showGitPath()
            return
        }
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
    /// Sends the chapters to the boxes holding the edit — mapped, since they sit two components deep in the column.
    function showUnsaved() {
        const item = gitPane.unsavedIdentityItem()
        if (!item)
            return
        const at = item.mapToItem(chapterCol, 0, 0).y
        chapters.contentY = Math.max(0, Math.min(at - Theme.spaceXl,
                                                 chapterCol.implicitHeight - chapters.height))
    }
    /// Typing again disarms: the armed mark says the reader pressed, and an edit made after the press has not been
    /// shown yet.
    onUnsavedIdentitiesChanged: settingsDialog.askingLeave = false

    // Zero: two bands — title and exit, then the chapters — and the `borderDefault` rule between them runs edge to
    // edge (a rule stopped `spaceXxl` short reads as half drawn), so each band keeps its own air (sides `spaceXxl`,
    // top and bottom `spaceLg`).
    padding: 0

    contentItem: ColumnLayout {
        spacing: 0

        // A shortcut, not `closePolicy`: that needs the key to get past whatever holds the caret. Qt hands it over
        // only while the screen is topmost, so an open list still takes the first press and the screen stays.
        Shortcut {
            // `sequences`: Cancel is several keys on some platforms, and `sequence` takes only the first.
            sequences: [StandardKey.Cancel]
            enabled: settingsDialog.opened
            // A middle-click scroll going on is put down first, as on the page (`RepoPage.escapePressed`).
            onActivated: {
                if (!MiddleHand.stop())
                    settingsDialog.escapeOut()
            }
        }

        // ---- the header band -------------------------------------------------
        SettingsHeader {
            blockWidth: settingsDialog.blockWidth
            word: settingsDialog.categoryWord
            categoryIcon: settingsDialog.categoryIcon
            unsaved: settingsDialog.unsavedIdentities > 0
            armed: settingsDialog.askingLeave || settingsDialog.askingGitPath
            onClosed: settingsDialog.escapeOut()
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }

        // ---- the categories and their chapters -------------------------------
        // Centred with `blockWidth` as the ceiling: a narrowing window eats the side air first, then the column.
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.maximumWidth: settingsDialog.blockWidth
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Theme.spaceLg
            Layout.bottomMargin: Theme.spaceLg
            spacing: Theme.spaceLg

            // The categories, dressed as the sidebar's rows (規約 §左メニューの所作): `rowHeight`, `bgSelected` for the
            // one showing, `bgHover`, `fontMd`; only the width is the screen's own (`settingsRailWidth`).
            // `fillWidth: false`: a nested layout fills by default, and a rail taking what the chapters leave would
            // move on every category change.
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
                        /// A git waiting to be applied locks the other category (user decision — ask before
                        /// changing): with the exit already shut, offering other settings would offer the settings of a
                        /// window about to be replaced. The current row stays live — greying it would grey the very
                        /// thing being read.
                        enabled: categoryRow.current || !AppBackend.gitPathOffersRestart
                        /// What a press on this row does — the handler and automation both come through here.
                        function tap() {
                            if (!categoryRow.enabled)
                                return
                            settingsDialog.category = categoryRow.modelData.key
                        }
                        Layout.fillWidth: true
                        implicitHeight: Theme.rowHeight
                        color: categoryRow.current
                               ? Theme.bgSelected
                               : categoryHover.hovered ? Theme.bgHover : "transparent"
                        // The head of the row's words, a category named as a heading is (デザイン規約 §アイコンの席 の
                        // 文頭記号の見出し): a step over their capitals and on their middle, its box their line.
                        NavIcon {
                            id: categoryMark
                            anchors.verticalCenter: parent.verticalCenter
                            x: Theme.spaceSm
                            width: Theme.iconMd
                            height: categoryWord.height
                            set: "head"
                            size: categoryWord.font.pixelSize
                            wordBase: categoryWord.baselineOffset
                            kind: categoryRow.modelData.icon
                            tint: categoryRow.enabled ? Theme.textPrimary : Theme.textMuted
                        }
                        Label {
                            id: categoryWord
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: categoryMark.right
                            anchors.leftMargin: Theme.spaceSm
                            text: categoryRow.modelData.word
                            font.pixelSize: Theme.fontMd
                            // `textPrimary` selected or not — the wash says which is showing; dimming the other
                            // would darken the one a press goes to (§無効「選ばれていない側は自分の Dim」). Muted only
                            // when the row cannot be pressed.
                            color: categoryRow.enabled ? Theme.textPrimary : Theme.textMuted
                        }
                        // The category holding an unsaved identity (user decision — ask before changing): the rail is
                        // the one column always showing both, and a reader in the other category sees neither of the
                        // other two marks (`escapeOut`). On the word's right shoulder, as `NameCell` seats a file's
                        // mark (規約 §git 用語のコード表記「`!` の席」).
                        NavIcon {
                            x: categoryWord.x + categoryWord.implicitWidth - Theme.spaceXs / 2
                            y: categoryWord.y
                            width: Theme.iconSm
                            height: Theme.iconSm
                            kind: "bang"
                            tint: Theme.warning
                            visible: categoryRow.modelData.key === "git"
                                     && settingsDialog.unsavedIdentities > 0
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
            Rectangle {
                Layout.fillHeight: true
                implicitWidth: Theme.borderWidth
                color: Theme.borderSubtle
            }

            // The chapters scroll — `Git` holds two groups. The bar stands at the window's edge with the band's right
            // inset spent inside the view, out of the content's width: a 9px gutter dodging the thumb would set the bar
            // 1px off the words with 32px free to their right. So the chapters' right edge lines up with the `✕`, a
            // band's width of air before the bar, and no `scrollBarGutter` on this side either. Its left air is the
            // row's `spacing`: an extra margin here would set the chapters further from the line than the rail.
            Flickable {
                id: chapters
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: width
                // `implicitHeight`, not `height`: until the layout has settled the wrapped labels' heights the
                // column's `height` comes out short, and an overflowing screen raises no bar (`contentHeight <=
                // height`). Judged by `settings_fit reach=` (`settings-repo`).
                contentHeight: chapterCol.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: PaneScrollBar {
                    id: chaptersBar
                    // The pinned kind, its idle step recounted from this ground (user decision — ask before
                    // changing): the three steps count from the ground the board stands on (規約 §ペインのスクロール
                    // バー), and the default `bgElevated` is this ground's own colour, so the bar would vanish; here
                    // they are `borderSubtle` / `borderDefault` / `borderStrong`. A see-through thumb is no substitute:
                    // it cannot say "more below" before a hand touches it.
                    idleColor: Theme.borderSubtle
                }
                // On the view's own frame, not in its content, which travels with the scroll. The boxes keep a middle
                // press only where it pastes (`MiddleAutoScroll.claimedAt`).
                MiddleAutoScroll {
                    id: middleHand
                    parent: chapters
                    anchors.fill: parent
                    visible: chaptersBar.visible
                    onDrifted: dy => chapters.contentY =
                        Math.max(0, Math.min(chapters.contentY + dy, chapters.contentHeight - chapters.height))
                }

                // The chapters' sweep hand (規約 §右のペインの字は掴める). Under the column, so it gets only the
                // presses nothing else took; inside the view, because a `Flickable` hands a press to its children
                // first and keeps what they leave (`SweepPad`). With the hand over the whole content, this surface
                // gives up panning by drag; the wheel and the bar still scroll.
                SweepPad {
                    id: chapterHand
                    anchors.fill: parent
                    content: chapterCol
                }

                ColumnLayout {
                    id: chapterCol
                    // `width`, not `Layout.*`: a `Flickable`'s content item is not a layout. Less the band's right
                    // inset, and capped at `textWidth` (user decision — ask before changing): stretched across a
                    // window-filling screen a `fontMd` line runs near 180 characters and the eye loses the next line's
                    // head. Boxes stop with the sentences — full-width boxes under capped sentences split a chapter
                    // into two columns. The free space on the right is correct; the bar stands in it.
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
                        // `showing` asks for the slow read; `screenOpen` keeps the boxes following git either way.
                        showing: settingsDialog.opened && settingsDialog.category === "git"
                        screenOpen: settingsDialog.opened
                        onAccepted: settingsDialog.escapeOut()
                    }
                }
            }
        }
        // No footer: the screen ends with its contents (user decision — ask before changing). Every field has
        // already written, so an `OK` would confirm nothing.
    }
}

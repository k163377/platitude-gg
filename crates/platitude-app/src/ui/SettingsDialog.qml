import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Every setting this app can change, on one window-filling screen: categories down the left, split by where a value
// is stored (Platitude GG's own file / git's), and inside `Git` by how far it reaches (規約 §設定の画面).
AppDialog {
    id: settingsDialog

    fills: true

    /// Which category is showing — `"app"` or `"git"`.
    property string category: "app"

    /// The categories, their words and marks. One list for the rail and the title, so the two cannot name a category
    /// differently.
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

    /// Opens the screen on one category. The only way in: a bare `open()` would keep the last reader's category.
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
        chapters.contentY = Math.max(0, chapterCol.implicitHeight - chapters.height)
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
        // `git config` reads; the slow `--tool-help` still waits for the category (規約 §設定の画面).
        gitPane.landOnFront()
        // No caret, except from the avatar's badge, which opened the screen to name a picture (規約 §設定の画面).
        appPane.focusPrefill()
        // Through the press — opening the list directly would look the same with the wiring cut. Last, because a
        // press also takes the caret.
        if (settingsDialog.pressToolOnOpen)
            gitPane.pressToolField()
    }
    // Every exit writes the fields; there is no Cancel. The identity keeps its own Save (規約 §設定の画面).
    onClosed: {
        settingsDialog.askingLeave = false
        settingsDialog.applyFields()
        settingsDialog.prefillName = ""
        settingsDialog.prefillEmail = ""
    }
    // Each category writes its own, so a field finished in one never queues a `git config` for the other.
    function applyFields() {
        appPane.applyFields()
        gitPane.applyTool()
    }

    /// How many identity chapters hold an edit git has not been given — the one thing on this screen that can be
    /// lost (規約 §設定の画面).
    readonly property int unsavedIdentities: gitPane.unsavedIdentities
    /// The screen's one block, centred: `spaceXxl` + rail + line + chapters + `spaceXxl`, symmetric so the ink
    /// centres with it (規約 §設定の画面). The title shares it; the exit stays at the window edge.
    readonly property real blockWidth: 2 * Theme.spaceXxl + Theme.settingsRailWidth + 2 * Theme.spaceLg
                                       + Theme.borderWidth + Theme.textWidth

    /// A way out over an unsaved identity was turned down once: the mark is armed and the next press goes through.
    property bool askingLeave: false

    /// A way out over a git waiting to be applied was turned down. Unlike [`askingLeave`] it never lets a press
    /// through; it falls with the offer.
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
    /// the first press shows the chapter and arms the mark; the second closes and puts the boxes back
    /// (規約 §設定の画面).
    function escapeOut() {
        // A chosen git holds the screen shut on every press: the path is already written, and leaving would keep
        // this window on another git. The way back is the box.
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
    /// Typing again disarms: an edit made after the press has not been shown yet.
    onUnsavedIdentitiesChanged: settingsDialog.askingLeave = false

    // Zero: the rule between the two bands runs edge to edge, and each band keeps its own air (規約 §設定の画面).
    padding: 0

    contentItem: ColumnLayout {
        spacing: 0

        // A shortcut, not `closePolicy`: that needs the key to get past whatever holds the caret. Qt hands it over
        // only while the screen is topmost, so an open list still takes the first press (規約 §設定の画面).
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

            // The categories, dressed as the sidebar's rows (規約 §設定の画面). `fillWidth: false`: a nested layout
            // fills by default, and a rail taking what the chapters leave would move on every category change.
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
                        /// A git waiting to be applied locks the other category; the current row stays live so it
                        /// is not greyed (規約 §設定の画面).
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
                        NavIcon {
                            id: categoryMark
                            anchors.verticalCenter: parent.verticalCenter
                            x: Theme.spaceSm
                            width: Theme.iconMd
                            height: Theme.iconMd
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
                            // `textPrimary` selected or not — the wash says which is showing (規約 §設定の画面);
                            // muted only when the row cannot be pressed (§無効).
                            color: categoryRow.enabled ? Theme.textPrimary : Theme.textMuted
                        }
                        // The category holding an unsaved identity: the rail is the one column always showing both
                        // (規約 §設定の画面). On the word's right shoulder, as `NameCell` seats a file's mark
                        // (規約 §git 用語のコード表記「`!` の席」).
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

            // The bar's idle step is recounted from this ground, and it stands at the window's edge with the band's
            // right inset spent inside the view (規約 §設定の画面). Its left air is the row's `spacing`: an extra
            // margin here would set the chapters further from the line than the rail.
            Flickable {
                id: chapters
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: width
                contentHeight: chapterCol.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: PaneScrollBar {
                    id: chaptersBar
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
                // first and keeps what they leave.
                SweepPad {
                    id: chapterHand
                    anchors.fill: parent
                    content: chapterCol
                }

                ColumnLayout {
                    id: chapterCol
                    // `width`, not `Layout.*`: a `Flickable`'s content item is not a layout. Less the band's right
                    // inset, and capped at `textWidth` for sentences and boxes alike (規約 §設定の画面).
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
        // No footer: every field has already written, so an `OK` would confirm nothing (規約 §設定の画面).
    }
}

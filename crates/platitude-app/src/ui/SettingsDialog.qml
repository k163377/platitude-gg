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

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spaceLg

            // The categories. A list of names down the left of a screen, lit the way the sidebar's rows are
            // (規約 §左メニューの所作) — the same kind of thing in the same clothes, and at the same width
            // (規約 §レイアウト初期値 サイドバー幅). Fixed, because a layout inside a layout fills by default and a
            // rail that took the width the chapters did not want would move every time the category changed.
            ColumnLayout {
                Layout.fillWidth: false
                Layout.preferredWidth: 260
                Layout.alignment: Qt.AlignTop
                spacing: 0
                Repeater {
                    id: categoryRepeater
                    model: [{ key: "app", word: qsTr("Application") }, { key: "git", word: qsTr("Git") }]
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
                        Label {
                            anchors.verticalCenter: parent.verticalCenter
                            x: Theme.spaceSm
                            text: categoryRow.modelData.word
                            font.pixelSize: Theme.fontMd
                            color: categoryRow.current ? Theme.textPrimary : Theme.textSecondary
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
            // **The floating bar, not the panels' slab** (デザイン規約 §スクロールバー / §QML 実装ルール のバーの
            // 選び方). The slab says its idle state with `bgElevated`, which is the step above a *pane's* ground —
            // and this screen's ground is `bgElevated` itself, so an idle slab here is the ground exactly (実測: the
            // five pixels at the edge came back `#0F172A`, and a bar that is meant to dim rather than vanish had
            // vanished). The translucent thumb is the one with a reading over a card, and this edge can take it: the
            // ink reaches eight pixels in, where the combo's chevron starts, and what it passes over below that is
            // the inside of a box near its own right frame.
            Flickable {
                id: chapters
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: Theme.spaceSm
                clip: true
                contentWidth: width
                contentHeight: chapterCol.implicitHeight
                // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AutoScrollBar { id: chaptersBar }

                ColumnLayout {
                    id: chapterCol
                    // Width rather than a margin: a `Flickable`'s content item is not a layout, so `Layout.*` on this
                    // one would be read by nobody. No gutter — a floating bar does not take one (規約 §余白).
                    width: chapters.width
                    spacing: Theme.spaceXl

                    SettingsAppPane {
                        id: appPane
                        visible: settingsDialog.category === "app"
                        curPage: settingsDialog.curPage
                        prefillName: settingsDialog.prefillName
                        prefillEmail: settingsDialog.prefillEmail
                        onAccepted: settingsDialog.close()
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
                        onAccepted: settingsDialog.close()
                    }
                }
            }
        }
        // One button, because there is only one thing left for a button to
        // do. A Cancel here would promise to put back a picture that was
        // assigned the moment it was named, and a Save would claim credit
        // for writes that already happened — which is also why the one
        // button carries no check.
        DialogActions {
            acceptText: qsTr("OK")
            onAccepted: settingsDialog.close()
        }
    }
}

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Every setting this app can change, on one screen with the categories down the left: what Platitude GG keeps in its
// own file, and what it writes into git's configuration for every other git on this computer to read. The two are
// kept apart because where a value is stored is the one thing about it a reader cannot see — so it is the split the
// screen is built on, and each category says it in a line of its own.
//
// The whole window, not a card: a settings screen is read rather than answered, and a card sized to its own content
// grows a scrollbar as soon as one chapter does. The one card left in this family is the identity gate
// (`IdentityDialog`), which is a question.
AppDialog {
    id: settingsDialog

    fills: true

    /// Which category is showing — `"app"` or `"git"`. The two entries in the app menu are two doors into this one
    /// screen, and the door decides which of them the reader lands on.
    property string category: "app"

    /// The tab this screen reads git through: the avatar candidates come off its graph, the merge editor off its
    /// working tree. The settings themselves are global, but "whose commits are these" and "what would git launch"
    /// are read where the person is.
    property var curPage: null

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
    // The half of the screen that talks to git lives in `SettingsGitPane`; the window's harness asks the screen, so
    // the screen passes the question on. `opened` is the screen's to add — a pane that is only hidden still answers.
    readonly property bool autoToolsLoadingReady: settingsDialog.opened && gitPane.autoToolsLoadingReady
    readonly property bool autoToolsSettledReady: settingsDialog.opened && gitPane.autoToolsSettledReady
    function reportTool() {
        gitPane.reportTool()
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

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: Theme.spaceSm
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
                    // Its slow read is asked for by this, and by nothing else — being hidden is what says the reader
                    // did not ask for it (規約 §設定の画面). Following git is the other one: that goes on wherever the
                    // reader is standing.
                    showing: settingsDialog.opened && settingsDialog.category === "git"
                    screenOpen: settingsDialog.opened
                    onAccepted: settingsDialog.close()
                }
                Item { Layout.fillHeight: true }
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

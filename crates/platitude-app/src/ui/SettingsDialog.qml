import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
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
    readonly property bool autoAppShown: fetchField.visible
    readonly property bool autoGitShown: gitPane.visible

    // ---- what the git category answers for, forwarded ---------------------
    // The half of the screen that talks to git lives in `SettingsGitPane`; the window's harness asks the screen, so
    // the screen passes the question on. `opened` is the screen's to add — a pane that is only hidden still answers.
    readonly property bool autoToolsLoadingReady: settingsDialog.opened && gitPane.autoToolsLoadingReady
    readonly property bool autoToolsSettledReady: settingsDialog.opened && gitPane.autoToolsSettledReady
    function reportTool() {
        gitPane.reportTool()
    }

    // ---- app: the avatars -----------------------------------------------
    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs wait on, and the two moves they have no hand to make. The screen is the window's, so
    /// those verbs are finished by `WindowAutoActDriver`; everything here is the list's own output side — the rows the
    /// store answered the filing with, the picture inside the first of them, that row's `lit`, and the candidate
    /// list's own `opened`.
    readonly property int autoAvatarRows: avatarRepeater.count
    readonly property bool autoAvatarComboOpen: avatarWho.popup.opened
    function autoAvatarRowLit(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.lit
    }
    function autoAvatarRowPainted(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.pictureReady()
    }
    /// The door a press uses, so the list comes down the way it does under a hand — including the turn of the loop the
    /// field puts between the press and the list (`AppCombo.pressField`).
    function autoAvatarOfferCombo() {
        avatarWho.pressField()
    }
    /// Runs the row's hold to its end. False where the list has no such row yet, so the caller waits instead of
    /// counting a press it never made.
    function autoAvatarHoldRemove(at) {
        const row = avatarRepeater.itemAt(at)
        if (!row)
            return false
        row.holdRemove()
        return true
    }

    /// The author an avatar's badge was pressed on. The screen opens
    /// already carrying whom it is about, so the only thing left to do is
    /// name the picture.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Assignments as the settings file holds them: address, name, URL — U+001F between records, U+001E between a
    /// record's fields, the same convention every packed list carries (`encode::RECORD_SEP` / `FIELD_SEP`).
    readonly property var assigned: {
        const packed = AppBackend.avatars
        if (packed === "")
            return []
        return packed.split(String.fromCharCode(31)).map(record => {
            const parts = record.split(String.fromCharCode(30))
            return { email: parts[0], name: parts[1] || parts[0], url: parts[2] }
        })
    }

    /// Who the entry offers: the authors of the repository being looked
    /// at. Read when the screen opens rather than bound, because the graph
    /// keeps moving and a list that reordered itself under an open popup
    /// would be answering a question nobody asked.
    property var authorChoices: []
    function readAuthorChoices() {
        // The model dedupes, sorts and formats (`GraphModel.author_choices`); the prefill rides along so its address
        // is not repeated below. With no page there are no rows to offer — the prefill still leads the list.
        const packed = settingsDialog.curPage
                     ? settingsDialog.curPage.pageGraph.authorChoices(settingsDialog.prefillName,
                                                                      settingsDialog.prefillEmail)
                     : settingsDialog.prefillEmail !== ""
                       ? settingsDialog.prefillName + " <" + settingsDialog.prefillEmail + ">" : ""
        settingsDialog.authorChoices = packed === "" ? [] : packed.split(String.fromCharCode(31))
    }

    /// The address the picker will file under, pulled back out of what the
    /// entry is showing. The list writes `Name <address>`; a person typing
    /// their own may write either half, and an address is the one with an
    /// `@` in it (`platitude_core::trailers::split_identity`).
    readonly property string chosenEmail: GitFacts.identityEmailOf(avatarWho.wanted)
    readonly property string chosenName: GitFacts.identityNameOf(avatarWho.wanted)

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0 ? String(AppBackend.autoFetchMinutes) : ""
        gitPane.loadIdentity()
        gitPane.loadTool()
        // Headless has no pointer to put on a row's Remove, and the lit
        // button is what the dim/bright pair is photographed by.
        settingsDialog.pointedAtRow = AppBackend.autoAct === "avatar-row-lit" ? 0 : -1
        settingsDialog.readAuthorChoices()
        // Opened from an avatar, the first thing left to do is name the
        // picture, so the focus goes there rather than to the top field.
        if (settingsDialog.prefillEmail !== "") {
            const who = settingsDialog.prefillName + " <" + settingsDialog.prefillEmail + ">"
            avatarWho.wanted = who
            avatarWho.editText = who
            chooseAvatar.forceActiveFocus()
        } else {
            avatarWho.wanted = ""
            avatarWho.editText = ""
            if (settingsDialog.category === "git")
                gitPane.focusIdentity()
            else
                fetchField.forceActiveFocus()
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
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    function applyFetch() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
    }
    // Only the way out writes both: the two categories have separate owners, and a field finished with in one of them
    // has no business queueing a `git config` for the other.
    function applyFields() {
        settingsDialog.applyFetch()
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

                // Where this category's values live, said before the chapters rather than in a line at the foot. The
                // git category says the same about itself in its own file, and between them the two sentences are the
                // whole of the difference between the categories.
                Label {
                    visible: settingsDialog.category === "app"
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    text: qsTr("Kept by Platitude GG in its own settings file. Nothing here is written to your git configuration.")
                }

                SettingsSection {
                    visible: settingsDialog.category === "app"
                    caption: qsTr("AUTOMATIC FETCH")
                    LabeledField {
                        caption: qsTr("Interval")
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceSm
                            FormField {
                                id: fetchField
                                implicitWidth: 160
                                placeholderText: qsTr("off")
                                inputMethodHints: Qt.ImhDigitsOnly
                                validator: IntValidator {
                                    bottom: 1
                                    top: AppBackend.autoFetchMaxMinutes
                                }
                                // Written when the field is done with — on Enter, and
                                // on the focus leaving it — rather than per keystroke,
                                // which would run through "1" on the way to "10".
                                onEditingFinished: settingsDialog.applyFetch()
                                onAccepted: settingsDialog.close()
                            }
                            Label {
                                text: qsTr("minutes")
                                color: Theme.textSecondary
                            }
                            Item { Layout.fillWidth: true }
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
                        text: qsTr("Runs git fetch --prune on every open repository, at most once per interval. Empty means off; %1 minutes is the longest interval.")
                              .arg(AppBackend.autoFetchMaxMinutes)
                    }
                }

                SettingsSection {
                    visible: settingsDialog.category === "app"
                    caption: qsTr("AVATARS")
                    Repeater {
                        id: avatarRepeater
                        /// The one column every row's name is laid into, as wide
                        /// as the widest of them, so the address beside it starts
                        /// on the same x down the whole list and the chapter reads
                        /// as a table rather than as a stack of sentences. The
                        /// same shape `AppMenu.codeColW` uses for its chips.
                        readonly property real nameColW: {
                            let widest = 0
                            for (let i = 0; i < avatarRepeater.count; i++) {
                                const row = avatarRepeater.itemAt(i)
                                if (row && row.nameSeat !== undefined)
                                    widest = Math.max(widest, row.nameSeat)
                            }
                            return widest
                        }
                        model: settingsDialog.assigned
                        // One row per assignment (`AvatarAssignRow`); the shared name column and the pointer stand-in
                        // are handed down, the rest the row reads off its own record.
                        delegate: AvatarAssignRow {
                            Layout.fillWidth: true
                            nameColW: avatarRepeater.nameColW
                            pointedAtRow: settingsDialog.pointedAtRow
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spaceSm
                        AppCombo {
                            id: avatarWho
                            Layout.fillWidth: true
                            placeholder: qsTr("name or email")
                            model: settingsDialog.authorChoices
                        }
                        ActionButton {
                            id: chooseAvatar
                            implicitHeight: Theme.controlHeight
                            // One word for the whole feature, and it is the word
                            // this shelf already uses (デザイン規約 §アバターを
                            // 与える). No article: the row names a kind about to be
                            // chosen, not something already on screen (§長さ).
                            text: qsTr("Choose avatar…")
                            // A plain frame rather than bare: bare is for the
                            // answer standing beside a framed one, read as the
                            // pair it is in, and this one stands in a form row
                            // next to a combo (規約 §肯定側のボタン). Opened from
                            // an avatar, what says which errand this is is the
                            // combo beside it already carrying that name — not a
                            // colour on the button.
                            frameColor: Theme.borderDefault
                            activeFocusOnTab: true
                            enabled: settingsDialog.chosenEmail !== ""
                            onActivated: avatarPicker.open()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: AppBackend.avatarError !== ""
                        wrapMode: Text.Wrap
                        color: Theme.danger
                        font.pixelSize: Theme.fontSm
                        text: AppBackend.avatarError
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
                        text: qsTr("An avatar for anyone whose commits you read. The list offers this repository's authors. Nothing is fetched: it comes from one of your own files, copied in beside these settings.")
                    }
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

        FileDialog {
            id: avatarPicker
            title: qsTr("Choose avatar")
            // The patterns come from the store so the dialog and the store
            // cannot drift apart; the words in front of them are ours, so
            // they live here rather than in Rust (CLAUDE.md 文言規約). They
            // name the two formats rather than a category, because every
            // category word for a file is a second name for the avatar
            // (デザイン規約 §アバターを与える) — and the formats are pinned
            // by that same section, so `avatar::EXTENSIONS` cannot grow one
            // this line does not know about without the rules moving first.
            nameFilters: [qsTr("PNG and JPEG (%1)").arg(AppBackend.avatarPatterns)]
            onAccepted: AppBackend.assignAvatar(settingsDialog.chosenEmail, settingsDialog.chosenName,
                                                selectedFile.toString())
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

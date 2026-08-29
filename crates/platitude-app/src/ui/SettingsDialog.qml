import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import QtQuick.Layouts
import platitude
import platitude.ui

// Settings that apply to every open repository: how often this computer
// should talk to remotes at all, and which editor it hands a conflict to.
AppDialog {
    id: settingsDialog

    /// The tab whose repository answers for the merge tool. The setting is
    /// global, but "what would git launch" is read where the person is.
    property var curPage: null
    readonly property string mergeTool: settingsDialog.curPage ? settingsDialog.curPage.pageWt.mergeTool : ""
    /// Names to offer, packed the way the graph's label records are.
    readonly property var toolChoices: {
        const packed = settingsDialog.curPage ? settingsDialog.curPage.pageTab.mergeTools : ""
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false
    // Automation can photograph both phases without racing a wall clock.
    // The loading latch is raised only after the real model reports an
    // outstanding tool read, then keeps that observed visual state alive
    // until grabToImage has finished.
    property bool autoToolLoadingLatched: false
    readonly property bool autoToolsLoadingReady:
        settingsDialog.opened && settingsDialog.autoToolLoadingLatched && toolField.popup.opened
    readonly property bool autoToolsSettledReady: settingsDialog.opened && settingsDialog.curPage
        && !settingsDialog.curPage.pageTab.mergeToolsLoading
        && settingsDialog.toolChoices.length > 0 && toolField.popup.opened

    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs wait on, and the two moves they have no hand to make. The card is the window's, so those
    /// verbs are finished by `WindowAutoActDriver`; everything here is the list's own output side — the rows the store
    /// answered the filing with, the picture inside the first of them, that row's `lit`, and the candidate list's own
    /// `opened`.
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

    /// The author an avatar's badge was pressed on. The card opens
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
    /// at. Read when the card opens rather than bound, because the graph
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

    /// Smoke hook. The candidates arrive in two waves and the configured
    /// name in a third, so the value has three chances to be knocked out
    /// by something that is not a person — report it at each.
    function reportTool() {
        if (AppBackend.autoAct === "settings-tools" || AppBackend.autoAct === "settings-tools-loading")
            AppBackend.report("merge_editor wanted=" + toolField.wanted + " shown=" + toolField.editText
                              + " configured=" + settingsDialog.mergeTool + " settled=" + (settingsDialog.curPage
                                  && !settingsDialog.curPage.pageTab.mergeToolsLoading)
                              + " loading=" + toolField.loading + " open=" + toolField.popup.opened
                              + " typing=" + toolField.typing
                              + " choices=" + settingsDialog.toolChoices.length)
    }
    onToolChoicesChanged: settingsDialog.reportTool()

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0 ? String(AppBackend.autoFetchMinutes) : ""
        toolField.wanted = settingsDialog.mergeTool
        settingsDialog.toolTouched = false
        settingsDialog.autoToolLoadingLatched = false
        // Headless has no pointer to put on a row's Remove, and the lit
        // button is what the dim/bright pair is photographed by.
        settingsDialog.pointedAtRow = AppBackend.autoAct === "avatar-row-lit" ? 0 : -1
        settingsDialog.readAuthorChoices()
        if (settingsDialog.curPage) {
            // The status refresh only names the configured tool where
            // something is conflicted, so ask for it. The candidates are
            // a separate, far slower read — hence the turning indicator.
            settingsDialog.curPage.pageTab.askMergeTool()
            settingsDialog.curPage.pageTab.askMergeTools()
            if (AppBackend.autoAct === "settings-tools-loading" && settingsDialog.curPage.pageTab.mergeToolsLoading)
                settingsDialog.autoToolLoadingLatched = true
        }
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
            fetchField.forceActiveFocus()
        }
        // Through the press rather than the popup: what these two are for
        // is the state a finger on the field leaves behind, and opening
        // the card from here would photograph that just as well with the
        // wiring cut. Last, because a press also takes the caret — the
        // focus settled just above is the one a person would be taking it
        // from.
        if (AppBackend.autoAct === "settings-tools" || AppBackend.autoAct === "settings-tools-loading")
            toolField.pressField()
    }
    // Escape and the button are the same exit, so both leave the fields
    // written: with no Cancel there is nothing for a discard to mean.
    onClosed: {
        settingsDialog.applyFields()
        settingsDialog.prefillName = ""
        settingsDialog.prefillEmail = ""
    }
    onMergeToolChanged: {
        if (settingsDialog.opened && !settingsDialog.toolTouched) {
            toolField.wanted = settingsDialog.mergeTool
            settingsDialog.toolTouched = false
        }
        settingsDialog.reportTool()
    }
    Connections {
        target: settingsDialog.curPage ? settingsDialog.curPage.pageTab : null
        function onMergeToolsLoadingChanged() {
            if (AppBackend.autoAct === "settings-tools-loading" && settingsDialog.curPage.pageTab.mergeToolsLoading)
                settingsDialog.autoToolLoadingLatched = true
        }
    }
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    //
    // Nothing here waits for a button. A picture is assigned the moment
    // it is named, so a Save that governed the other two would be telling
    // the truth about half of this card; the fields write as they are
    // finished with instead, and the one button left only dismisses it.
    // The tool is written only where it would change what git answers
    // with. Pressing the field is not choosing anything, and the write for
    // "the same as now" is not free: an empty field asks git to unset a
    // key, which fails when the key was never there in the first place
    // (2026-08-21 ユーザー報告 — the log raised itself over the card
    // closing).
    function applyFields() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
        if (settingsDialog.curPage && toolField.wanted !== settingsDialog.mergeTool)
            settingsDialog.curPage.pageTab.setMergeTool(toolField.wanted)
    }
    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        LabeledField {
            caption: qsTr("Fetch automatically")
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
                    onEditingFinished: settingsDialog.applyFields()
                    onAccepted: settingsDialog.close()
                }
                Label {
                    text: qsTr("minutes")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
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
        LabeledField {
            caption: qsTr("Merge editor")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                // As wide as the dialog gives it: a tool's name has no
                // fixed length, and an input only takes a fixed width
                // when its content does (デザイン規約 §レイアウト初期値).
                AppCombo {
                    id: toolField
                    Layout.fillWidth: true
                    placeholder: qsTr("none")
                    // The popup is opened by the dialog's actual `opened`
                    // edge above. Loading stays latched only for the
                    // automation verb that deliberately photographs it.
                    loading: settingsDialog.autoToolLoadingLatched || (settingsDialog.curPage
                                 && settingsDialog.curPage.pageTab.mergeToolsLoading)
                    model: settingsDialog.toolChoices
                    onWantedChanged: settingsDialog.toolTouched = true
                    // A row picked from the list is a finished answer;
                    // free text waits for Enter or for the card to close.
                    onActivated: settingsDialog.applyFields()
                    onAccepted: settingsDialog.close()
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                // Named rather than picked from a list: the only way to
                // enumerate them is `git mergetool --tool-help`, whose
                // output is laid out for a person to read.
                text: qsTr("Which tool opens a conflicted file. It must not need a console — this app gives git none, so vimdiff and its kind cannot run.")
            }
        }
        LabeledField {
            caption: qsTr("Avatars")
            Repeater {
                id: avatarRepeater
                /// The one column every row's name is laid into, as wide
                /// as the widest of them, so the address beside it starts
                /// on the same x down the whole list and the section reads
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
                // One row per assignment (`AvatarAssignRow`); the shared name column and the pointer stand-in are
                // handed down, the rest the row reads off its own record.
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
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            // Two settings, two homes: one this app keeps for itself, one
            // git keeps where every other git on this computer can see it.
            text: qsTr("The fetch interval is stored by Platitude GG. The merge editor is stored by git as merge.guitool, where every other git on this computer sees it.")
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

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
    readonly property string mergeTool:
        settingsDialog.curPage ? settingsDialog.curPage.pageWt.mergeTool : ""
    /// Names to offer, packed the way the graph's label records are.
    readonly property var toolChoices: {
        const packed = settingsDialog.curPage
                       ? settingsDialog.curPage.pageTab.mergeTools : ""
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false

    /// Stands in for the pointer on one row, which headless cannot inject.
    property int pointedAtRow: -1

    /// The author an avatar's badge was pressed on. The card opens
    /// already carrying whom it is about, so the only thing left to do is
    /// name the picture.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Assignments as the settings file holds them: address, name, URL.
    readonly property var assigned: {
        const packed = AppBackend.avatars
        if (packed === "")
            return []
        return packed.split(String.fromCharCode(30)).map(record => {
            const parts = record.split(String.fromCharCode(31))
            return { email: parts[0], name: parts[1] || parts[0], url: parts[2] }
        })
    }

    /// Who the entry offers: the authors of the repository being looked
    /// at. Read when the card opens rather than bound, because the graph
    /// keeps moving and a list that reordered itself under an open popup
    /// would be answering a question nobody asked.
    property var authorChoices: []
    function readAuthorChoices() {
        const packed = settingsDialog.curPage
                       ? settingsDialog.curPage.pageGraph.authorChoices() : ""
        const seen = {}
        const out = []
        if (settingsDialog.prefillEmail !== "") {
            out.push(settingsDialog.prefillName + " <"
                     + settingsDialog.prefillEmail + ">")
            seen[settingsDialog.prefillEmail] = true
        }
        if (packed !== "") {
            for (const record of packed.split(String.fromCharCode(30))) {
                const parts = record.split(String.fromCharCode(31))
                if (seen[parts[1]])
                    continue
                seen[parts[1]] = true
                out.push(parts[0] + " <" + parts[1] + ">")
            }
        }
        settingsDialog.authorChoices = out
    }

    /// The address the picker will file under, pulled back out of what the
    /// entry is showing. The list writes `Name <address>`; a person typing
    /// their own may write either half, and an address is the one with an
    /// `@` in it.
    readonly property string chosenEmail: {
        const text = avatarWho.wanted.trim()
        const open = text.lastIndexOf("<")
        const close = text.lastIndexOf(">")
        const inner = (open >= 0 && close > open)
                      ? text.substring(open + 1, close).trim() : text
        return inner.indexOf("@") > 0 ? inner : ""
    }
    readonly property string chosenName: {
        const text = avatarWho.wanted.trim()
        const open = text.lastIndexOf("<")
        return open > 0 ? text.substring(0, open).trim() : ""
    }

    /// Smoke hook. The candidates arrive in two waves and the configured
    /// name in a third, so the value has three chances to be knocked out
    /// by something that is not a person — report it at each.
    function reportTool() {
        if (AppBackend.autoAct === "settings-tools")
            AppBackend.report("merge_editor wanted=" + toolField.wanted
                              + " shown=" + toolField.editText
                              + " configured=" + settingsDialog.mergeTool)
    }
    onToolChoicesChanged: settingsDialog.reportTool()

    onOpened: {
        fetchField.text = AppBackend.autoFetchMinutes > 0
                          ? String(AppBackend.autoFetchMinutes) : ""
        toolField.wanted = settingsDialog.mergeTool
        settingsDialog.toolTouched = false
        // Headless has no pointer to put on a row, and the lit row is
        // what the dim/bright pair is photographed by.
        settingsDialog.pointedAtRow =
            AppBackend.autoAct === "avatar-row-lit" ? 0 : -1
        settingsDialog.readAuthorChoices()
        if (settingsDialog.curPage) {
            // The status refresh only names the configured tool where
            // something is conflicted, so ask for it. The candidates are
            // a separate, far slower read — hence the turning indicator.
            settingsDialog.curPage.pageTab.askMergeTool()
            settingsDialog.curPage.pageTab.askMergeTools()
        }
        // Opened from an avatar, the first thing left to do is name the
        // picture, so the focus goes there rather than to the top field.
        if (settingsDialog.prefillEmail !== "") {
            const who = settingsDialog.prefillName + " <"
                        + settingsDialog.prefillEmail + ">"
            avatarWho.wanted = who
            avatarWho.editText = who
            chooseImage.forceActiveFocus()
        } else {
            avatarWho.wanted = ""
            avatarWho.editText = ""
            fetchField.forceActiveFocus()
        }
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
    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    //
    // Nothing here waits for a button. A picture is assigned the moment
    // it is named, so a Save that governed the other two would be telling
    // the truth about half of this card; the fields write as they are
    // finished with instead, and the one button left only dismisses it.
    function applyFields() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0
                                                              : Number(fetchField.text))
        if (settingsDialog.toolTouched && settingsDialog.curPage)
            settingsDialog.curPage.pageTab.setMergeTool(toolField.wanted)
    }
    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Fetch automatically")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
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
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Merge editor")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
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
                    // Smoke hook: the popup is drawn here rather than by
                    // Fusion, so it needs its own look at (PG_AUTO_ACT).
                    Timer {
                        running: settingsDialog.opened
                                 && AppBackend.autoAct === "settings-tools"
                        interval: 400
                        onTriggered: toolField.popup.open()
                    }
                    loading: settingsDialog.curPage
                             && settingsDialog.curPage.pageTab.mergeToolsLoading
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
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Avatars")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
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
                delegate: RowLayout {
                    id: avatarRow
                    Layout.fillWidth: true
                    spacing: Theme.spaceSm
                    /// The hand is on this row: the pointer, the keyboard's
                    /// focus (the hold's other hand — デザイン規約 §長押し,
                    /// and the row it is about is just as settled), or
                    /// headless having put it there for a shot.
                    readonly property bool lit:
                        rowHover.hovered || unsetButton.activeFocus
                        || settingsDialog.pointedAtRow === index
                    // A handler, not a MouseArea: a MouseArea is an Item,
                    // so a layout gives it a seat of its own and every
                    // column after it starts a gap further right — 
                    // anchoring it over the row only turns that into
                    // undefined behaviour (the engine says so out loud).
                    // A handler is not an Item and takes no seat.
                    HoverHandler {
                        id: rowHover
                    }
                    IdentIcon {
                        imageUrl: modelData.url
                        width: Theme.iconLg
                        height: Theme.iconLg
                        Layout.preferredWidth: Theme.iconLg
                        Layout.preferredHeight: Theme.iconLg
                    }
                    /// The width this row asks the shared name column to
                    /// hold — its own glyphs, and nothing for the column's
                    /// spare width, which stays air.
                    readonly property real nameSeat: nameLabel.implicitWidth
                    Label {
                        id: nameLabel
                        text: modelData.name
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontMd
                        Layout.preferredWidth: avatarRepeater.nameColW
                        elide: Text.ElideRight
                    }
                    Label {
                        Layout.fillWidth: true
                        text: modelData.email
                        elide: Text.ElideRight
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
                    }
                    // Held, not clicked: this card writes as it is worked
                    // rather than on a Save, so the gesture is the only
                    // thing standing between a stray click and a picture
                    // that has to be found again (デザイン規約 §長押し).
                    //
                    // Red only where the hand is (デザイン規約 §状態): a
                    // standing state colour is for saying that the usual
                    // move is not available, and everything a settings card
                    // does is the usual move — a red word per row says
                    // nothing and thins out the warnings that mean it. The
                    // frame is what carries "this is a control" at rest:
                    // the other three columns are data, and a bare word
                    // among them reads as a fourth one. `*Dim` belongs on
                    // that frame rather than on the word, which is the one
                    // use §暗く落とした段 allows for it. Kept in the layout
                    // either way, so the address beside it does not
                    // re-elide as the pointer crosses the list.
                    ActionButton {
                        id: unsetButton
                        text: qsTr("Remove")
                        font.pixelSize: Theme.fontSm
                        tone: avatarRow.lit ? Theme.danger
                                            : Theme.textSecondary
                        frameColor: avatarRow.lit ? Theme.dangerDim
                                                  : Theme.borderSubtle
                        holdMs: Metrics.holdMs
                        holdTone: Theme.danger
                        onHeld: AppBackend.removeAvatar(modelData.email)
                        Timer {
                            running: settingsDialog.opened
                                     && AppBackend.autoAct === "avatar-remove"
                                     && index === 0
                            interval: 500
                            onTriggered: unsetButton.completeHold()
                        }
                    }
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
                    Timer {
                        running: settingsDialog.opened
                                 && AppBackend.autoAct === "avatar-combo"
                        interval: 400
                        onTriggered: avatarWho.popup.open()
                    }
                }
                ActionButton {
                    id: chooseImage
                    implicitHeight: Theme.controlHeight
                    text: qsTr("Choose image…")
                    // Opened from an avatar this already holds the focus,
                    // so the whole errand is one press and the picker —
                    // and the accent says so, since the frame is what
                    // names the affirmative here (規約 §肯定側のボタン).
                    // Away from the accent it keeps a plain frame rather
                    // than going bare: bare is for the answer standing
                    // beside a framed one, read as the pair it is in, and
                    // this one stands in a form row next to a combo.
                    frameColor: settingsDialog.prefillEmail !== "" && enabled
                                ? Theme.accent : Theme.borderDefault
                    activeFocusOnTab: true
                    enabled: settingsDialog.chosenEmail !== ""
                    onActivated: picturePicker.open()
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
                text: qsTr("A picture for anyone whose commits you read. The list offers this repository's authors. Nothing is fetched: the picture is one of your own files, copied in beside these settings.")
            }
        }
        FileDialog {
            id: picturePicker
            title: qsTr("Choose a picture")
            nameFilters: [AppBackend.avatarFilters]
            onAccepted: AppBackend.assignAvatar(settingsDialog.chosenEmail,
                                                settingsDialog.chosenName,
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
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            // One button, because there is only one thing left for a
            // button to do. A Cancel here would promise to put back a
            // picture that was assigned the moment it was named, and a
            // Save would claim credit for writes that already happened.
            ActionButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("OK")
                // The accent goes to whichever of the two the dialog was
                // opened for, and the other keeps a plain frame — the
                // same one expression, read the other way round
                // (`chooseImage`). No icon: a check would claim credit
                // for writes that already happened, which is the very
                // thing the comment above says a `Save` here must not do.
                frameColor: settingsDialog.prefillEmail === ""
                            ? Theme.accent : Theme.borderDefault
                activeFocusOnTab: true
                onActivated: settingsDialog.close()
            }
        }
    }
}

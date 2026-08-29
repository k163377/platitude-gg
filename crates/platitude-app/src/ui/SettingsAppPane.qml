import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Application` category: everything Platitude GG keeps in its own settings file, and the
// sentence that says so (規約 §設定の画面). A file of its own for the reason the other two categories have one — a
// category is one place values are stored, and this one's place is not git's.
ColumnLayout {
    id: pane

    /// The tab the avatar candidates are read off: the authors of the repository being looked at. The settings
    /// themselves are the application's, but "whose commits are these" is read where the person is.
    property var curPage: null

    /// The author an avatar's badge was pressed on, handed down by the screen — which is where the entry puts it.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Enter in one of the number fields. The way out belongs to the screen, so it is asked for rather than taken.
    signal accepted()

    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs wait on, and the two moves they have no hand to make. The screen is the window's, so
    /// those verbs are finished by `WindowAutoActDriver`; everything here is the list's own output side — the rows the
    /// store answered the filing with, the picture inside the first of them, that row's `lit`, and the candidate
    /// list's own `opened`.
    readonly property int autoAvatarRows: avatarRepeater.count
    readonly property bool autoAvatarComboOpen: avatarWho.popup.opened
    /// Which chapters this category is actually showing, for the verb that presses the rail — the screen's `category`
    /// is the input side, and a run that read it back would be reporting its own press.
    readonly property bool autoAppShown: fetchField.visible
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
        const packed = pane.curPage
                     ? pane.curPage.pageGraph.authorChoices(pane.prefillName, pane.prefillEmail)
                     : pane.prefillEmail !== ""
                       ? pane.prefillName + " <" + pane.prefillEmail + ">" : ""
        pane.authorChoices = packed === "" ? [] : packed.split(String.fromCharCode(31))
    }

    /// The address the picker will file under, pulled back out of what the
    /// entry is showing. The list writes `Name <address>`; a person typing
    /// their own may write either half, and an address is the one with an
    /// `@` in it (`platitude_core::trailers::split_identity`).
    readonly property string chosenEmail: GitFacts.identityEmailOf(avatarWho.wanted)
    readonly property string chosenName: GitFacts.identityNameOf(avatarWho.wanted)

    /// Puts the fields back to what the store says, for the screen that just opened.
    function load() {
        fetchField.text = AppBackend.autoFetchMinutes > 0 ? String(AppBackend.autoFetchMinutes) : ""
        wholeHistoryBox.checked = AppBackend.initialCommits === 0
        commitsField.text = AppBackend.initialCommits > 0 ? String(AppBackend.initialCommits) : ""
        // Headless has no pointer to put on a row's Remove, and the lit
        // button is what the dim/bright pair is photographed by.
        pane.pointedAtRow = AppBackend.autoAct === "avatar-row-lit" ? 0 : -1
        pane.readAuthorChoices()
    }
    /// Opened from an avatar, the first thing left to do is name the
    /// picture, so the focus goes there rather than to the top field.
    /// Answers whether it took the caret, so the screen knows whether it
    /// still has one to place.
    function focusPrefill() {
        if (pane.prefillEmail === "") {
            avatarWho.wanted = ""
            avatarWho.editText = ""
            return false
        }
        const who = pane.prefillName + " <" + pane.prefillEmail + ">"
        avatarWho.wanted = who
        avatarWho.editText = who
        chooseAvatar.forceActiveFocus()
        return true
    }
    function focusFetch() {
        fetchField.forceActiveFocus()
    }

    // An empty field is the off switch — nothing to type is the
    // clearest way to say "do not do this".
    function applyFetch() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
    }
    // Two controls, one value: the box asks for no window at all (core's `0`), and the field answers only while the
    // box is clear. Written from here for the same reason the interval is — a validator with a floor calls an empty
    // string unacceptable, so `onEditingFinished` never fires for one, and an emptied field would otherwise never be
    // written back. Empty is the default rather than a third meaning; the placeholder is that number.
    function applyCommits() {
        AppBackend.setInitialCommits(wholeHistoryBox.checked ? 0
                                     : commitsField.text === "" ? AppBackend.initialCommitsDefault
                                     : Number(commitsField.text))
    }
    /// Both of this category's fields, for the way out of the screen.
    function applyFields() {
        pane.applyFetch()
        pane.applyCommits()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    // Where this category's values live, said before the chapters rather than in a line at the foot. The other two
    // categories say the same about themselves in their own files, and between them those sentences are the whole of
    // the difference between the categories.
    Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        color: Theme.textSecondary
        font.pixelSize: Theme.fontSm
        text: qsTr("Kept by Platitude GG in its own settings file. Nothing here is written to your git configuration.")
    }

    SettingsSection {
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
                    onEditingFinished: pane.applyFetch()
                    onAccepted: pane.accepted()
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
        caption: qsTr("COMMIT GRAPH")
        LabeledField {
            caption: qsTr("Initial commits")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: commitsField
                    implicitWidth: 160
                    // The number does not apply while the whole history is asked for, and §無効 is how that is said.
                    // The text stays put, so unchecking gives the reader their own number back rather than a blank.
                    enabled: !wholeHistoryBox.checked
                    // What an empty field will be read as, shown rather than explained.
                    placeholderText: String(AppBackend.initialCommitsDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    // No `top`: the ceiling is the property's own type (`session::log_limit`), so a number a person
                    // typed on purpose is one the walk answers.
                    validator: IntValidator {
                        bottom: AppBackend.initialCommitsMin
                    }
                    onEditingFinished: pane.applyCommits()
                    onAccepted: pane.accepted()
                }
                Label {
                    text: qsTr("commits")
                    color: commitsField.enabled ? Theme.textSecondary : Theme.textMuted
                }
                Item { Layout.fillWidth: true }
            }
        }
        // The other answer a count of commits can have, and it is not a number — so it is not spelled as one. An
        // empty field would have had to carry it, and an empty field already means "the default" in the box right
        // above (規約 §設定の画面).
        AppCheckBox {
            id: wholeHistoryBox
            text: qsTr("Load the whole history")
            onToggled: pane.applyCommits()
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            text: qsTr("How much history a graph opens with. The rest is loaded from the row at the bottom of it, a quarter of this at a time — with the whole history there is no such row. %1 is the smallest window.")
                  .arg(AppBackend.initialCommitsMin)
        }
    }

    SettingsSection {
        caption: qsTr("AVATARS")
        Repeater {
            id: avatarRepeater
            /// The one column every row's name is laid into, as wide
            /// as the widest of them, so the address beside it starts
            /// on the same x down the whole list and the chapter reads
            /// as a table rather than as a stack of sentences. The
            /// same shape `AppMenu.codeColW` uses for its chips.
            ///
            /// Settled by the rows' own seat changes rather than bound:
            /// the binding form only re-ran on `count`, and a same-size
            /// reassignment replaces every row without moving it.
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
            // One row per assignment (`AvatarAssignRow`); the shared name column and the pointer stand-in
            // are handed down, the rest the row reads off its own record.
            delegate: AvatarAssignRow {
                Layout.fillWidth: true
                nameColW: avatarRepeater.nameColW
                pointedAtRow: pane.pointedAtRow
                onNameSeatChanged: avatarRepeater.settleNameColW()
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
                enabled: pane.chosenEmail !== ""
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
        onAccepted: AppBackend.assignAvatar(pane.chosenEmail, pane.chosenName, selectedFile.toString())
    }
}

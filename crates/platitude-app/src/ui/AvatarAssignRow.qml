import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// One assigned avatar in the settings list — picture, name, address, held Remove (`SettingsAppPane`'s AVATARS).
//
// `Remove` is held (規約 §回数押し): the write is immediate, with no Save or Cancel to take it back. Its words and frame
// go red only on the row under the hand: at rest `textSecondary` words in a `borderSubtle` frame, under the hand
// `danger` words in a `dangerDim` frame, the fill `danger` while held. Changing a setting is ordinary work, so no
// standing state colour (規約 §状態); the frame says "this is an action" all the time, so the greyed words never read as
// a fourth column of data. Not `dangerDim` words at rest: `*Dim` is never a text colour (規約 §暗く落とした段). **The
// press mark stays `danger` at rest** (user decision — ask before changing; the exception in 規約 §回数押しの色): the mark
// is the one place that says what pressing through loses, and grey it would make a removal nothing undoes look like
// any button. The column is always laid out, so a pointer crossing rows never moves where the address is cut; the
// frame makes the row taller, accepted for a list of a few rows in the settings screen.
RowLayout {
    id: avatarRow

    /// The assignment this row shows ({ email, name, url } — `SettingsAppPane.assigned`) and its place in the list.
    /// Declared: a delegate in a file of its own is only handed the roles it names.
    required property var modelData
    required property int index
    /// The one column every row's name is laid into, as wide as the widest of them (`avatarRepeater.nameColW`).
    property real nameColW: 0
    /// Stands in for the pointer on one row's Remove, which headless cannot inject (`SettingsAppPane.pointedAtRow`).
    property int pointedAtRow: -1
    /// The person whose removal this row's last press took, until its floor has passed.
    property string removalTaken: ""

    /// Remove pressed through: the removal of this person's picture as it is now, for the list to keep.
    signal removeAccepted(string email, string url)
    /// Its floor has passed: the list gives the removal it keeps for `email`, if it still stands.
    signal removeLanded(string email)

    spacing: Theme.spaceSm
    /// The hand is on this row's Remove: the pointer, keyboard focus (デザイン規約 §回数押し) or the headless stand-in.
    /// The button's own hover, not the row's — a word that reddens three columns away is about none of them.
    readonly property bool lit: unsetButton.hovered || unsetButton.activeFocus
        || avatarRow.pointedAtRow === avatarRow.index
    /// Automation: the picture the list is photographed for, and the hold it has no hand to make.
    function pictureReady() {
        return rowFace.pictureReady()
    }
    function holdRemove() {
        return unsetButton.completeHold()
    }
    IdentIcon {
        id: rowFace
        imageUrl: avatarRow.modelData.url
        width: Theme.iconLg
        height: Theme.iconLg
        Layout.preferredWidth: Theme.iconLg
        Layout.preferredHeight: Theme.iconLg
        // A picture heading the name, on its capitals' middle (デザイン規約 §アイコンの席).
        transform: Translate {
            y: WordFace.capitalDropOf(nameLabel.baselineOffset, nameLabel.pixelSize, nameLabel.height)
        }
    }
    /// The width this row asks of the shared name column — its own glyphs only.
    readonly property real nameSeat: nameLabel.implicitWidth
    // Fields cut by clipping, so a drag takes the whole value (`LineText`, 規約 §右のペインの字は掴める). The cut mark
    // stands on the settings screen's own ground, `bgElevated`.
    LineText {
        id: nameLabel
        text: avatarRow.modelData.name
        color: Theme.textPrimary
        pixelSize: Theme.fontMd
        ground: Theme.bgElevated
        Layout.preferredWidth: avatarRow.nameColW
    }
    // `textSecondary`, not `textMuted` — muted is the disabled look (デザイン規約 §テキスト / §無効).
    LineText {
        Layout.fillWidth: true
        text: avatarRow.modelData.email
        color: Theme.textSecondary
        pixelSize: Theme.fontSm
        ground: Theme.bgElevated
    }
    // Held, its word and frame red only under the hand, framed at rest, and always laid out; the press mark wears the
    // fill's red at rest too (see the header). `danger`, not `warning`: the picture lies outside the
    // repository, and no discard record brings it back (user decision — ask before changing).
    ActionButton {
        id: unsetButton
        text: qsTr("Remove")
        font.pixelSize: Theme.fontMd
        tone: avatarRow.lit ? Theme.danger : Theme.textSecondary
        frameColor: avatarRow.lit ? Theme.dangerDim : Theme.borderSubtle
        holdPresses: Metrics.holdPressesWarned
        holdTone: Theme.danger
        // Whose face this press is aimed at (`HoldDriver.premise`): an assignment landing while the hand is here
        // rebuilds the list, and a delegate re-used for another row would take that person's instead.
        premise: avatarRow.modelData.email
        // The row goes with the entry, in the same turn: the removal is taken at the last press — this person, this
        // picture — and given once the full fill has been seen. The list keeps it (`AvatarRemovals`): a row
        // rebuilt meanwhile would take it with it.
        onHeld: {
            avatarRow.removalTaken = avatarRow.modelData.email
            avatarRow.removeAccepted(avatarRow.modelData.email, avatarRow.modelData.url)
        }
        onLanded: {
            const email = avatarRow.removalTaken
            avatarRow.removalTaken = ""
            if (email !== "")
                avatarRow.removeLanded(email)
        }
    }
}

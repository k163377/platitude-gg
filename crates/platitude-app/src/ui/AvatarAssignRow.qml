import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// One assigned avatar in the settings list — picture, name, address, held Remove (デザイン規約 §アバターを与える).
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

    spacing: Theme.spaceSm
    /// The hand is on this row's Remove: the pointer, keyboard focus (デザイン規約 §長押し) or the headless stand-in.
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
    // Held, red only under the hand, framed at rest, and always laid out (デザイン規約 §アバターを与える).
    ActionButton {
        id: unsetButton
        text: qsTr("Remove")
        font.pixelSize: Theme.fontMd
        tone: avatarRow.lit ? Theme.danger : Theme.textSecondary
        frameColor: avatarRow.lit ? Theme.dangerDim : Theme.borderSubtle
        holdMs: Metrics.holdMs
        holdTone: Theme.danger
        // Whose face this press is aimed at (`HoldDriver.premise`): an assignment landing while the hand is here
        // rebuilds the list, and a delegate re-used for another row would take that person's instead.
        premise: avatarRow.modelData.email
        onHeld: AppBackend.removeAvatar(avatarRow.modelData.email)
    }
}

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import platitude
import platitude.ui

// The window's two biggest dialogs — the settings screen and the clone box — with the model and the folder chooser
// the clone owns. Together in one seat because every way into either goes through the two doors below, which is what
// a later change needs: a door can decide when the screen behind it is built at all.
//
// An `Item` that fills the window: the dialogs are popups whose sizing reads their parent
// (`AppDialog.fills`), and this seat is that parent now.
Item {
    id: seat

    /// The page in front, for where the clone box opens and what the settings screen shows (`RepoPage`).
    required property var curPage
    required property var tabsModel

    /// The three the window's harness reaches for by name (`WindowAutoActDriver`).
    readonly property alias cloneModel: cloneModel
    readonly property alias cloneDialog: cloneDialog
    readonly property alias settingsDialog: settingsDialog
    /// Whether the settings screen is up — the identity gate holds itself back behind it (`IdentityGate`).
    readonly property bool settingsOpened: settingsDialog.opened

    /// The clone box, opened beside the repository that is already open — a second working copy usually goes where
    /// the first one is, which is the reading `openRepositoryPicker` starts from as well. With no tab open there is
    /// nothing to be beside, and the platform dialog's own folder is what stands in.
    function startClone() {
        const near = seat.curPage !== null ? seat.curPage.pageTab.pickerFolderUrl : ""
        cloneDialog.start(near !== "" ? near : cloneFolderDialog.currentFolder.toString())
    }
    /// Every door into the settings comes through here.
    function openSettingsAt(category) {
        settingsDialog.openAt(category)
    }
    /// The same door, told whom it was opened on before it opens (デザイン規約 §アバターを与える).
    function openAvatarSettings(name, email) {
        settingsDialog.prefillName = name
        settingsDialog.prefillEmail = email
        settingsDialog.openAt("app")
    }

    // Fetching a repository that has no tab yet, and the folder chooser that belongs to it. A second FolderDialog
    // rather than the picker's: the two ask different questions ("which repository" / "which folder to put one in"),
    // and sharing one would carry each answer into the other's next opening.
    CloneModel {
        id: cloneModel
        // The clone landed, so the box that asked for it goes and the strip does what it always does with a folder
        // that opens. The two objects are joined here rather than to each other: fetching a repository happens before
        // there is a tab, and opening one is the strip's ordinary job (`models/clone.rs`).
        onCloneDone: path => {
            cloneDialog.landed()
            seat.tabsModel.openRepositoryPath(path)
        }
        onCloneFailed: message => cloneDialog.said(message)
    }
    CloneDialog {
        id: cloneDialog
        cloning: cloneModel.cloning
        onSubmitted: (url, parentUrl, name) => cloneModel.cloneRepository(url, parentUrl, name)
        // Every road out of the box comes through here, Escape included: a clone still on the network is stopped by
        // the same press that takes away the only place its answer could have landed.
        onCancelled: cloneModel.cancelClone()
        onChooseFolder: near => {
            if (near !== "")
                cloneFolderDialog.currentFolder = near
            cloneFolderDialog.open()
        }
    }
    FolderDialog {
        id: cloneFolderDialog
        title: qsTr("Where to put it")
        onAccepted: cloneDialog.setFolder(selectedFolder.toString())
    }

    SettingsDialog {
        id: settingsDialog
        curPage: seat.curPage
        tabsModel: seat.tabsModel
    }
}

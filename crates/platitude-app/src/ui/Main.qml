// platitude-gg main window. Presentation only: every model row arrives
// precomputed from Rust; the only JS here decodes compact draw/chip tokens
// and formats dates for display.
//
// All colors / fonts / dimensions come from the Theme singleton
// (internal-docs/デザイン規約.md is the source of truth).
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Dialogs
import platitude
import platitude.ui

ApplicationWindow {
    id: root
    width: 1440
    height: 900
    visible: true
    title: qsTr("platitude-gg")
    color: Theme.bgBase
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // Graph-geometry / interaction constants still being tuned. This block
    // is their single source of truth for now (デザイン規約.md §運用の例外);
    // values follow the guideline principles (4px grid, token reuse) and
    // graduate into the document once they stabilize.
    readonly property int laneW: Theme.iconLg
    readonly property int laneInset: Theme.spaceXs
    readonly property int nodeIcon: Theme.iconLg
    readonly property int laneStroke: 2
    readonly property real iconStroke: 1.5
    readonly property real identiconFill: 0.72
    readonly property int wheelRows: 6
    readonly property real middleScrollGain: 0.12
    readonly property int labelColW: 152
    readonly property int graphDefaultLanes: 12
    readonly property int detailsAvatar: 40
    readonly property int anchorDelayMs: 50
    readonly property var laneDash: [1, 1]

    // Image-preview zoom steps: a small image draws at a fixed integer
    // scale picked from its natural size (never from the window), then
    // fit-to-frame shrinking still wins when space runs out. Small icons
    // land in a readable 128-256px band.
    function previewZoomFor(maxSide) {
        if (maxSide <= 0)
            return 1
        if (maxSide < 32)
            return 8
        if (maxSide < 64)
            return 4
        if (maxSide < 128)
            return 2
        return 1
    }

    palette {
        window: Theme.bgBase
        windowText: Theme.textPrimary
        base: Theme.bgBase
        text: Theme.textPrimary
        button: Theme.bgElevated
        buttonText: Theme.textPrimary
        highlight: Theme.accent
        highlightedText: Theme.textOnAccent
        placeholderText: Theme.textMuted
        mid: Theme.borderDefault
        dark: Theme.bgBase
        light: Theme.borderDefault
    }

    // Window focus is a refresh trigger (refs/status/stash only).
    property int focusEpoch: 0
    onActiveChanged: if (active) focusEpoch++

    // Identity dialog: opens on startup when git has no name and email to
    // put on a commit, and on demand from the app menu or the toolbar
    // badge. "Not now" leaves the app fully usable — reading a repository
    // needs no identity.
    property bool identityDismissed: false
    property bool identityEditing: false
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (identityEditing
                                               || (AppBackend.identityState === "missing"
                                                   && !identityDismissed))
    function dismissIdentity() {
        identityEditing = false
        identityDismissed = true
    }
    // Screenshot hook: PG_AUTO_IDENTITY="edit" opens the dialog on an
    // identity that is already set, which is otherwise a menu action.
    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (AppBackend.autoIdentity === "edit"
                    && AppBackend.identityState === "ready"
                    && !root.identityDismissed)
                root.identityEditing = true
        }
    }

    // Frame counter for the scroll benchmark (PG_AUTO_SCROLL=1).
    property int frameCounter: 0
    onFrameSwapped: frameCounter++

    // Clipboard access for copy buttons (QML has no direct clipboard API).
    function copyText(value) {
        clipboardEdit.text = value
        clipboardEdit.selectAll()
        clipboardEdit.copy()
    }
    TextEdit {
        id: clipboardEdit
        visible: false
    }

    TabsModel {
        id: tabsModel
    }

    // The RepoPage of the active tab (the toolbar's right-side controls
    // act on it).
    readonly property var curPage: (pageRepeater.count > 0
                                    && tabsModel.currentIndex >= 0
                                    && tabsModel.currentIndex < pageRepeater.count)
                                   ? pageRepeater.itemAt(tabsModel.currentIndex) : null

    FolderDialog {
        id: folderDialog
        title: qsTr("Open repository")
        onAccepted: tabsModel.openRepositoryUrl(selectedFolder.toString())
    }

    Component.onCompleted: {
        AppBackend.initialize()
        if (AppBackend.autoOpen !== "") {
            // Multiple repositories separated by ';' open as tabs in order.
            const paths = AppBackend.autoOpen.split(";")
            for (let i = 0; i < paths.length; i++) {
                if (paths[i] !== "")
                    tabsModel.openRepositoryPath(paths[i])
            }
        }
        if (AppBackend.autoQuitMs > 0)
            quitTimer.start()
        if (AppBackend.shotDir !== "")
            shotTimer.start()
    }
    Timer {
        id: quitTimer
        interval: Math.max(AppBackend.autoQuitMs, 1)
        onTriggered: Qt.quit()
    }
    Timer {
        id: shotTimer
        interval: AppBackend.autoQuitMs > 800 ? AppBackend.autoQuitMs - 800 : 3500
        onTriggered: {
            const path = AppBackend.shotDir + "/app.png"
            // Popups (the identity dialog) render in the window overlay,
            // outside this subtree: capturing those needs a window-level
            // screenshot from outside the process.
            const ok = mainUi.grabToImage(function (res) {
                const saved = res.saveToFile(path)
                console.warn("screenshot saved=" + saved + " path=" + path)
                if (AppBackend.autoQuitMs <= 0)
                    Qt.quit()
            })
            if (!ok)
                console.warn("grabToImage returned false")
        }
    }

    // ---- git gate --------------------------------------------------------
    Item {
        anchors.fill: parent
        visible: AppBackend.gitState !== "ok"
        Column {
            anchors.centerIn: parent
            spacing: Theme.spaceLg
            width: Math.min(640, root.width - 2 * Theme.spaceXxl)
            Label {
                text: qsTr("platitude-gg")
                font.pixelSize: Theme.fontXl
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }
            BusyIndicator {
                visible: AppBackend.gitState === "checking"
                running: visible
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                visible: AppBackend.gitState === "missing"
                text: qsTr("git was not found on PATH. Install git 2.43 or newer and restart.")
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            Label {
                visible: AppBackend.gitState === "unsupported" || AppBackend.gitState === "error"
                text: AppBackend.gitError
                color: Theme.danger
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
        }
    }

    // ---- identity dialog -------------------------------------------------
    // Opened and closed from the state above rather than by binding
    // `visible`: Escape closes a popup imperatively, which would overwrite
    // such a binding and leave the menu entry unable to open it again.
    // Closing for any reason answers the state, so the two stay in step.
    Dialog {
        id: identityDialog
        anchors.centerIn: parent
        width: Math.min(640, root.width - 2 * Theme.spaceXxl)
        modal: true
        closePolicy: Popup.CloseOnEscape
        focus: true
        onClosed: root.dismissIdentity()
        Connections {
            target: root
            function onIdentityWantedChanged() {
                if (root.identityWanted)
                    identityDialog.open()
                else
                    identityDialog.close()
            }
        }
        padding: Theme.spaceXxl
        header: null
        footer: null
        background: Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }

        // A write is in flight that this dialog asked for. Only then does
        // a finished write close it — the notification is shared with the
        // startup check, which must not close the dialog under the user.
        property bool saving: false

        function submit() {
            if (!saveButton.enabled)
                return
            identityDialog.saving = true
            AppBackend.saveIdentity(nameField.text, emailField.text)
        }

        onOpened: {
            nameField.text = AppBackend.identityName
            emailField.text = AppBackend.identityEmail
            nameField.forceActiveFocus()
            if (AppBackend.autoIdentity !== "")
                Qt.callLater(identityDialog.applyAutoIdentity)
        }

        // Screenshot hook: PG_AUTO_IDENTITY="<name>|<email>" fills the
        // fields, PG_AUTO_IDENTITY_SAVE=1 submits them, "skip" answers
        // "Not now" to show the state behind the dialog, and "edit" leaves
        // an identity that is already set as it is.
        function applyAutoIdentity() {
            if (AppBackend.autoIdentity === "skip") {
                root.dismissIdentity()
                return
            }
            if (AppBackend.autoIdentity === "edit")
                return
            const parts = AppBackend.autoIdentity.split("|")
            nameField.text = parts[0]
            emailField.text = parts.length > 1 ? parts[1] : ""
            if (AppBackend.autoIdentitySave)
                identityDialog.submit()
        }

        Connections {
            target: AppBackend
            function onIdentityChanged() {
                if (!identityDialog.saving || AppBackend.identityBusy)
                    return
                identityDialog.saving = false
                if (AppBackend.identityError === "")
                    root.identityEditing = false
            }
        }

        contentItem: ColumnLayout {
            spacing: Theme.spaceLg

            Label {
                text: AppBackend.identityState === "ready" ? qsTr("Your identity")
                                                           : qsTr("Set up your identity")
                font.pixelSize: Theme.fontXl
                font.weight: Font.DemiBold
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textSecondary
                text: qsTr("git records a name and an email address on every commit you "
                           + "make. They are stored in your git configuration — "
                           + "platitude-gg keeps no copy of them.")
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Label {
                    text: qsTr("Name")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                FormField {
                    id: nameField
                    Layout.fillWidth: true
                    placeholderText: qsTr("Ada Lovelace")
                    onAccepted: identityDialog.submit()
                    // Not validation — these are the characters git drops
                    // when it builds an author line, and keeping them out
                    // stops the configuration from disagreeing with what
                    // commits show. Everything else is the user's business.
                    validator: RegularExpressionValidator {
                        regularExpression: /[^<>\r\n]*/
                    }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Label {
                    text: qsTr("Email address")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                FormField {
                    id: emailField
                    Layout.fillWidth: true
                    placeholderText: qsTr("ada@example.com")
                    onAccepted: identityDialog.submit()
                    validator: RegularExpressionValidator {
                        regularExpression: /[^<>\r\n]*/
                    }
                }
            }

            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Saved for every repository on this computer "
                           + "(user.name and user.email).")
            }
            // git's own message, unedited.
            Label {
                Layout.fillWidth: true
                visible: AppBackend.identityError !== ""
                wrapMode: Text.Wrap
                color: Theme.danger
                text: AppBackend.identityError
            }

            RowLayout {
                Layout.alignment: Qt.AlignRight
                spacing: Theme.spaceSm
                HoverButton {
                    implicitHeight: Theme.controlHeight
                    text: root.identityEditing ? qsTr("Cancel") : qsTr("Not now")
                    onClicked: root.dismissIdentity()
                }
                HoverButton {
                    id: saveButton
                    implicitHeight: Theme.controlHeight
                    highlighted: true
                    text: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
                    enabled: !AppBackend.identityBusy
                             && nameField.text.trim() !== ""
                             && emailField.text.trim() !== ""
                    onClicked: identityDialog.submit()
                }
            }
        }
    }

    // ---- confirmation ----------------------------------------------------
    // One dialog for every "this cannot be taken back" question: rewriting
    // history a remote already has, or a push that overwrites one. The
    // caller supplies the wording and what to run on yes, so the phrasing
    // stays next to the operation it describes.
    //
    // Confirmation is reserved for the irreversible. Everyday operations
    // (stage, commit, switch) ask nothing — a prompt on each of those
    // teaches people to dismiss prompts.
    function confirm(heading, detail, acceptText, action) {
        confirmDialog.heading = heading
        confirmDialog.detail = detail
        confirmDialog.acceptText = acceptText
        confirmDialog.action = action
        confirmDialog.open()
    }
    Dialog {
        id: confirmDialog
        anchors.centerIn: parent
        width: Math.min(640, root.width - 2 * Theme.spaceXxl)
        modal: true
        closePolicy: Popup.CloseOnEscape
        focus: true
        padding: Theme.spaceXxl
        header: null
        footer: null
        property string heading: ""
        property string detail: ""
        property string acceptText: ""
        property var action: null
        background: Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }
        contentItem: ColumnLayout {
            spacing: Theme.spaceLg
            Label {
                text: confirmDialog.heading
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            Label {
                text: confirmDialog.detail
                color: Theme.textSecondary
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                spacing: Theme.spaceSm
                HoverButton {
                    implicitHeight: Theme.controlHeight
                    text: qsTr("Cancel")
                    onClicked: confirmDialog.close()
                }
                HoverButton {
                    implicitHeight: Theme.controlHeight
                    highlighted: true
                    text: confirmDialog.acceptText
                    onClicked: {
                        const run = confirmDialog.action
                        confirmDialog.close()
                        if (run)
                            run()
                    }
                }
            }
        }
    }

    // ---- settings --------------------------------------------------------
    // One setting so far. It applies to every open repository, because the
    // question is how often this computer should talk to remotes at all.
    Dialog {
        id: settingsDialog
        anchors.centerIn: parent
        width: Math.min(640, root.width - 2 * Theme.spaceXxl)
        modal: true
        closePolicy: Popup.CloseOnEscape
        focus: true
        padding: Theme.spaceXxl
        header: null
        footer: null
        background: Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }
        onOpened: {
            fetchField.text = AppBackend.autoFetchMinutes > 0
                              ? String(AppBackend.autoFetchMinutes) : ""
            fetchField.forceActiveFocus()
        }
        // An empty field is the off switch — nothing to type is the
        // clearest way to say "do not do this".
        function apply() {
            AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0
                                                                  : Number(fetchField.text))
            settingsDialog.close()
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
                        onAccepted: settingsDialog.apply()
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
                    text: qsTr("Runs git fetch --prune on every open repository, at "
                               + "most once per interval. Leave it empty to switch it "
                               + "off; %1 minutes is the longest interval offered.")
                          .arg(AppBackend.autoFetchMaxMinutes)
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Settings are not stored yet, so this returns to its "
                           + "default the next time platitude-gg starts.")
            }
            RowLayout {
                Layout.alignment: Qt.AlignRight
                spacing: Theme.spaceSm
                HoverButton {
                    implicitHeight: Theme.controlHeight
                    text: qsTr("Cancel")
                    onClicked: settingsDialog.close()
                }
                HoverButton {
                    implicitHeight: Theme.controlHeight
                    highlighted: true
                    text: qsTr("Save")
                    onClicked: settingsDialog.apply()
                }
            }
        }
    }

    // ---- main ------------------------------------------------------------
    ColumnLayout {
        id: mainUi
        anchors.fill: parent
        spacing: 0
        visible: AppBackend.gitState === "ok"

        // Top toolbar: prominent tabs and the per-repository controls
        // share one row.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.toolbarHeight
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm
                // App menu (Claude-Desktop-style hamburger); most entries
                // are placeholders until their phases land.
                HoverToolButton {
                    id: menuButton
                    text: "☰"
                    font.pixelSize: Theme.fontLg
                    Layout.leftMargin: Theme.spaceXs
                    Layout.alignment: Qt.AlignVCenter
                    padding: 0
                    implicitWidth: Theme.spaceXl
                    implicitHeight: Theme.spaceXl
                    onClicked: appMenu.open()
                    Menu {
                        id: appMenu
                        y: menuButton.height
                        MenuItem {
                            text: qsTr("Open repository…")
                            onTriggered: folderDialog.open()
                        }
                        MenuItem {
                            text: qsTr("Clone repository…")
                            enabled: false
                        }
                        MenuSeparator {}
                        MenuItem {
                            text: qsTr("Identity…")
                            onTriggered: root.identityEditing = true
                        }
                        MenuItem {
                            text: qsTr("Settings…")
                            onTriggered: settingsDialog.open()
                        }
                        MenuItem {
                            text: qsTr("About platitude-gg")
                            enabled: false
                        }
                        MenuSeparator {}
                        MenuItem {
                            text: qsTr("Exit")
                            onTriggered: Qt.quit()
                        }
                    }
                }
                // Plain Row tabs (no TabBar): full control of the geometry
                // so the selected underline sits exactly on the toolbar's
                // bottom edge with no styling leftovers beneath it.
                Row {
                    id: tabRow
                    Layout.fillHeight: true
                    spacing: 0
                    Repeater {
                        model: tabsModel
                        Rectangle {
                            id: tabItem
                            required property int index
                            required property int tab_id
                            required property string title
                            required property string repo_path
                            readonly property bool current: tabsModel.currentIndex === index
                            width: tabContent.implicitWidth + 2 * Theme.spaceSm
                            height: tabRow.height
                            color: current ? Theme.bgSelected : "transparent"
                            MouseArea {
                                id: tabMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: tabsModel.setCurrentIndex(tabItem.index)
                            }
                            Rectangle {
                                anchors.fill: parent
                                color: Theme.bgHover
                                visible: tabMouse.containsMouse && !tabItem.current
                            }
                            Rectangle {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: 2 * Theme.borderWidth
                                color: Theme.accent
                                visible: tabItem.current
                            }
                            RowLayout {
                                id: tabContent
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: tabItem.title
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 180
                                    Layout.fillHeight: true
                                    verticalAlignment: Text.AlignVCenter
                                    font.weight: tabItem.current ? Font.DemiBold : Font.Normal
                                    color: tabItem.current ? Theme.textPrimary
                                                           : Theme.textSecondary
                                }
                                HoverToolButton {
                                    text: "×"
                                    padding: 0
                                    Layout.alignment: Qt.AlignVCenter
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    onClicked: tabsModel.closeTab(tabItem.tab_id)
                                }
                            }
                        }
                    }
                }
                HoverToolButton {
                    text: "+"
                    font.pixelSize: Theme.fontLg
                    onClicked: folderDialog.open()
                }
                Item { Layout.fillWidth: true }

                // Transient state of the current repository.
                Rectangle {
                    visible: root.curPage !== null && root.curPage.pageWt.opText !== ""
                    color: "transparent"
                    border.color: Theme.warning
                    border.width: Theme.borderWidth
                    radius: Theme.radiusSm
                    implicitHeight: Theme.iconLg
                    implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
                    Label {
                        id: opLabel
                        anchors.centerIn: parent
                        text: root.curPage !== null ? root.curPage.pageWt.opText : ""
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
                Rectangle {
                    visible: root.curPage !== null && root.curPage.pageWt.hasConflicts
                    color: Theme.danger
                    radius: Theme.radiusSm
                    implicitHeight: Theme.iconLg
                    implicitWidth: conflictLabel.implicitWidth + 2 * Theme.spaceXs
                    Label {
                        id: conflictLabel
                        anchors.centerIn: parent
                        text: qsTr("CONFLICTS")
                        color: Theme.textOnAccent
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
                // Nothing to attribute commits to. Kept next to the other
                // repository-state badges so the way back to the setup
                // screen stays visible after "Not now".
                Rectangle {
                    visible: AppBackend.identityState === "missing"
                             || (root.curPage !== null
                                 && !root.curPage.pageTab.identityReady)
                    color: "transparent"
                    border.color: Theme.warning
                    border.width: Theme.borderWidth
                    radius: Theme.radiusSm
                    implicitHeight: Theme.iconLg
                    implicitWidth: identityBadge.implicitWidth + 2 * Theme.spaceXs
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: identityBadgeMouse.containsMouse
                    }
                    Label {
                        id: identityBadge
                        anchors.centerIn: parent
                        text: qsTr("SET IDENTITY")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                    MouseArea {
                        id: identityBadgeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.identityEditing = true
                        ToolTip.visible: containsMouse
                        ToolTip.delay: 600
                        ToolTip.text: qsTr("git has no name or email to record on commits")
                    }
                }
                Label {
                    visible: root.curPage !== null && root.curPage.pageTab.lastError !== ""
                    text: root.curPage !== null ? root.curPage.pageTab.lastError : ""
                    color: Theme.danger
                    elide: Text.ElideRight
                    Layout.maximumWidth: 320
                    font.pixelSize: Theme.fontSm
                    background: Rectangle {
                        color: Theme.bgHover
                        visible: errorClearMouse.containsMouse
                    }
                    MouseArea {
                        id: errorClearMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: if (root.curPage !== null) root.curPage.pageTab.clearLastError()
                    }
                }
                // Auto fetch: quiet by design. A machine that is simply
                // offline fails here once a minute, and that belongs in a
                // tooltip rather than in the error line.
                Label {
                    readonly property bool failing: root.curPage !== null
                                                    && root.curPage.pageTab.autoFetchError !== ""
                    text: "↻"
                    color: root.curPage !== null && root.curPage.pageTab.autoFetchRunning
                           ? Theme.accent
                           : failing ? Theme.warning
                           : AppBackend.autoFetchMinutes > 0 ? Theme.textMuted
                           : Theme.borderDefault
                    font.pixelSize: Theme.fontMd
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: fetchIndicatorMouse.containsMouse
                    }
                    MouseArea {
                        id: fetchIndicatorMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: settingsDialog.open()
                        ToolTip.visible: containsMouse
                        ToolTip.delay: 600
                        ToolTip.text: parent.failing
                                      ? root.curPage.pageTab.autoFetchError
                                      : AppBackend.autoFetchMinutes > 0
                                        ? qsTr("Fetching every %n minute(s)", "",
                                               AppBackend.autoFetchMinutes)
                                        : qsTr("Automatic fetching is off")
                    }
                }
                // Reserved: search box (backlog)
                SlimField {
                    enabled: false
                    opacity: 0.35
                    placeholderText: qsTr("Search")
                    implicitWidth: 160
                }
                ActionButton {
                    kind: "fetch"
                    text: qsTr("Fetch")
                    enabled: root.curPage !== null
                             && root.curPage.pageTab.remoteCount > 0
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Fetch every remote now, pruning branches "
                                       + "they no longer have")
                    onClicked: root.curPage.pageTab.fetch("")
                }
                ActionButton {
                    kind: "push"
                    text: qsTr("Push")
                    enabled: root.curPage !== null && root.curPage.canPush
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: root.curPage === null ? ""
                                  : root.curPage.pageWt.upstream !== ""
                                    ? qsTr("Push this branch to %1")
                                      .arg(root.curPage.pageWt.upstream)
                                    : qsTr("Publish this branch as %1")
                                      .arg(root.curPage.pushTargetLabel)
                    onClicked: root.curPage.pushNow()
                    // Overwriting a remote's history is the one push that
                    // needs asking about, so it lives behind its own entry.
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.RightButton
                        onClicked: pushMenu.popup()
                    }
                    Menu {
                        id: pushMenu
                        MenuItem {
                            text: qsTr("Force push…")
                            enabled: root.curPage !== null && root.curPage.canPush
                            onTriggered: root.curPage.forcePushNow()
                        }
                    }
                }
                // Local re-read only (no network).
                HoverToolButton {
                    text: qsTr("Reload")
                    enabled: root.curPage !== null
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Re-read this repository from disk (does not fetch)")
                    onClicked: root.curPage.pageTab.refreshAll()
                }
            }
        }

        // Divider under the tab toolbar — same look as the pane splitters.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            color: Theme.borderSubtle
        }

        // Nothing open: the window keeps its usual three-pane shape with
        // every pane empty, and the way in sits where the graph goes.
        // Built only while it is needed, so an app that starts with tabs
        // never pays for it.
        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            active: tabsModel.currentIndex < 0
            visible: active
            sourceComponent: Component {
                RepoPage {
                    index: -1
                    tab_id: -1
                }
            }
        }

        // Repository pages (one per tab, kept alive for instant switching)
        StackLayout {
            id: pages
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: tabsModel.currentIndex >= 0
            currentIndex: Math.max(0, tabsModel.currentIndex)
            Repeater {
                id: pageRepeater
                model: tabsModel
                RepoPage {}
            }
        }

        // Bottom edge: the same splitter-style divider closes the window;
        // the git version floats above it bottom-right as faint bare text.
        Item {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            Rectangle {
                anchors.fill: parent
                color: Theme.borderSubtle
            }
            Label {
                visible: AppBackend.gitVersion !== ""
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.rightMargin: Theme.spaceSm
                anchors.bottomMargin: Theme.splitterWidth + Theme.spaceXs
                text: qsTr("git %1").arg(AppBackend.gitVersion)
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // ======================================================================
    // One repository page
    // ======================================================================
    component RepoPage: Item {
        id: page
        required property int index
        required property int tab_id
        // No repository behind this page (tab_id -1): the chrome renders
        // with empty models and the graph column offers the way in.
        readonly property bool blank: tab_id < 0

        property string selectedOid: ""

        // Adjustable column widths (labels / graph); -1 = automatic. The
        // graph column caps at a default lane count and scrolls
        // horizontally when the full graph is wider.
        property real labelWManual: -1
        property real graphColWManual: -1
        readonly property real labelW: labelWManual >= 0 ? labelWManual : root.labelColW
        readonly property real graphFullW: root.laneInset
                                           + Math.max(1, graphModel.maxLanes) * root.laneW
                                           + Theme.spaceSm
        readonly property real graphColW: Math.min(graphFullW,
            graphColWManual >= 0 ? Math.max(graphColWManual, root.laneInset + root.laneW)
                                 : root.laneInset + root.graphDefaultLanes * root.laneW
                                   + Theme.spaceSm)
        property real graphX: 0
        readonly property real graphXMax: Math.max(0, graphFullW - graphColW)
        onGraphXMaxChanged: graphX = Math.min(graphX, graphXMax)

        // Right pane switches to the working-tree (WIP) view.
        property bool wipShown: false
        // Selected stash row's reflog selector ("" = not a stash).
        property string selectedStashRef: ""
        function showWip() {
            page.wipShown = true
            page.selectedOid = ""
            page.selectedStashRef = ""
            page.closeDiff()
            page.refreshHeadPublished()
        }

        // ---- commit editor -------------------------------------------
        // `amending` mirrors the checkbox so the page can act on it
        // without reaching into the delegate tree.
        property bool amending: false
        // Whether HEAD is already on a remote. Amending it rewrites
        // something other people may have, so that gets confirmed.
        property bool headPublished: false
        readonly property string headRange: "HEAD^!"
        function refreshHeadPublished() {
            if (repoTab.state === "open")
                repoTab.checkPublish(page.headRange)
        }
        Connections {
            target: repoTab
            function onChanged() {
                // One shared answer slot, so each consumer only reads the
                // reply to the range it asked about.
                if (repoTab.publishRange === page.headRange) {
                    page.headPublished = repoTab.publishPublished > 0
                } else if (page.menuOid !== ""
                           && repoTab.publishRange === page.menuOid + "^!") {
                    page.menuPublished = repoTab.publishPublished > 0
                    page.menuPublishKnown = true
                }
                page.absorbHeadMessage()
                page.absorbWriteResult()
            }
        }

        // Turning amend on starts the editor from HEAD's message; turning
        // it off empties it again, since the text belonged to that commit.
        property int seenHeadMessageSeq: 0
        property bool wantHeadMessage: false
        function amendToggled(on) {
            page.amending = on
            if (on) {
                page.wantHeadMessage = true
                repoTab.requestHeadMessage()
            } else {
                page.clearCommitEditor()
            }
        }
        function absorbHeadMessage() {
            if (repoTab.headMessageSeq === page.seenHeadMessageSeq)
                return
            page.seenHeadMessageSeq = repoTab.headMessageSeq
            if (!page.wantHeadMessage)
                return
            page.wantHeadMessage = false
            wipSubject.text = repoTab.headSubject
            wipBody.text = repoTab.headBody
        }
        function clearCommitEditor() {
            wipSubject.text = ""
            wipBody.text = ""
        }

        function commitNow() {
            if (page.amending && page.headPublished) {
                root.confirm(
                    qsTr("Rewrite a commit that is already on a remote?"),
                    qsTr("The last commit has been pushed. Amending replaces it "
                         + "with a different one, so anyone who already has it "
                         + "will be out of step until they reset."),
                    qsTr("Amend anyway"), page.doCommit)
                return
            }
            page.doCommit()
        }
        function doCommit() {
            repoTab.commit(wipSubject.text, wipBody.text, page.amending)
        }

        // ---- moving between branches and commits ----------------------
        // Terminology is deliberate: git runs `switch` / `restore`, and
        // the UI says "Switch to" (デザイン規約 §用語).
        //
        // A move with uncommitted changes asks what to do with them
        // instead of silently carrying them along: "leave them here"
        // stashes first (the default — arriving on another branch with
        // unexplained changes is how accidents start), "bring them along"
        // is git's own behaviour.
        readonly property bool treeDirty: workTree.stagedCount > 0
                                          || workTree.unstagedCount > 0
                                          || workTree.untrackedCount > 0
        // What the pending move is: kind is "branch" / "remote" / "commit".
        property string moveKind: ""
        property string moveTarget: ""
        property string moveLabel: ""

        function switchTo(kind, target, label) {
            page.moveKind = kind
            page.moveTarget = target
            page.moveLabel = label
            if (page.treeDirty)
                dirtySwitchDialog.open()
            else
                page.runSwitch(false)
        }
        function runSwitch(stashFirst) {
            if (page.moveKind === "branch")
                repoTab.checkoutBranch(page.moveTarget, stashFirst)
            else if (page.moveKind === "remote")
                repoTab.checkoutRemote(page.moveTarget,
                                       repoTab.localNameFor(page.moveTarget), stashFirst)
            else if (page.moveKind === "commit")
                repoTab.checkoutDetached(page.moveTarget, stashFirst)
            page.moveKind = ""
        }

        Dialog {
            id: dirtySwitchDialog
            anchors.centerIn: parent
            width: Math.min(640, root.width - 2 * Theme.spaceXxl)
            modal: true
            closePolicy: Popup.CloseOnEscape
            focus: true
            padding: Theme.spaceXxl
            header: null
            footer: null
            background: Rectangle {
                color: Theme.bgElevated
                radius: Theme.radiusMd
                border.color: Theme.borderDefault
                border.width: Theme.borderWidth
            }
            contentItem: ColumnLayout {
                spacing: Theme.spaceLg
                Label {
                    text: qsTr("You have uncommitted changes")
                    font.pixelSize: Theme.fontLg
                    font.weight: Font.DemiBold
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.textSecondary
                    text: qsTr("Switching to %1 can either leave them where they "
                               + "are or take them with you.").arg(page.moveLabel)
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceSm
                    HoverButton {
                        Layout.fillWidth: true
                        implicitHeight: Theme.controlHeight
                        highlighted: true
                        text: qsTr("Leave my changes on %1")
                              .arg(workTree.detached ? qsTr("this commit") : workTree.branch)
                        onClicked: {
                            dirtySwitchDialog.close()
                            page.runSwitch(true)
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
                        text: qsTr("Stashes them first; they wait in STASHES until "
                                   + "you apply them again.")
                    }
                    HoverButton {
                        Layout.fillWidth: true
                        implicitHeight: Theme.controlHeight
                        text: qsTr("Bring my changes to %1").arg(page.moveLabel)
                        onClicked: {
                            dirtySwitchDialog.close()
                            page.runSwitch(false)
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
                        text: qsTr("Carries them over; git refuses the switch if they "
                                   + "would collide with what is there.")
                    }
                }
                HoverButton {
                    Layout.alignment: Qt.AlignRight
                    implicitHeight: Theme.controlHeight
                    text: qsTr("Cancel")
                    onClicked: dirtySwitchDialog.close()
                }
            }
        }

        // ---- push ------------------------------------------------------
        readonly property string pushTargetLabel:
            workTree.upstream !== "" ? workTree.upstream
                                     : repoTab.defaultRemote + "/" + workTree.branch
        readonly property bool canPush: repoTab.state === "open"
                                        && !workTree.detached
                                        && workTree.branch !== ""
                                        && repoTab.remoteCount > 0
                                        && repoTab.busyCount === 0
        function pushNow() {
            repoTab.pushCurrent("", "")
        }
        function forcePushNow() {
            root.confirm(
                qsTr("Overwrite %1 with this branch?").arg(page.pushTargetLabel),
                qsTr("A force push replaces the remote branch's history with "
                     + "yours. Commits only the remote has are lost, and anyone "
                     + "who already pulled them keeps a history that no longer "
                     + "matches.\n\nThe push is refused if the remote moved since "
                     + "this window last saw it."),
                qsTr("Force push"),
                // A lease pinned to the commit actually on screen: a
                // background fetch must not turn this into a plain force.
                function () { repoTab.pushCurrent("lease", page.upstreamOid()) })
        }
        /// Commit the remote-tracking branch points at, as shown here.
        function upstreamOid() {
            return workTree.upstream !== ""
                   ? remotesModel.oidOfName(workTree.upstream) : ""
        }

        // ---- context menu on a branch row ------------------------------
        property string menuRefName: ""
        property string menuRefOid: ""
        property bool menuRefRemote: false
        function openRefMenu(name, oidHex, isRemote) {
            page.menuRefName = name
            page.menuRefOid = oidHex
            page.menuRefRemote = isRemote
            refMenu.popup()
        }
        Menu {
            id: refMenu
            MenuItem {
                text: qsTr("Switch to %1").arg(page.menuRefName)
                enabled: page.menuRefName !== workTree.branch
                onTriggered: page.switchTo(page.menuRefRemote ? "remote" : "branch",
                                           page.menuRefName, page.menuRefName)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Copy commit hash")
                onTriggered: root.copyText(page.menuRefOid)
            }
        }

        // ---- context menu on a commit row ------------------------------
        property string menuOid: ""
        readonly property string menuShort: page.menuOid.substring(0, 8)
        function openCommitMenu(oidHex) {
            page.menuOid = oidHex
            // Asked as the menu opens so the rewrite warnings inside it
            // know whether this commit has already left the machine. The
            // rewriting entries stay disabled until the answer lands —
            // one `rev-list --count`, so within a frame or two.
            page.menuPublished = false
            page.menuPublishKnown = false
            repoTab.checkPublish(oidHex + "^!")
            commitMenu.popup()
        }
        // Whether the commit the menu is about is already on a remote.
        property bool menuPublished: false
        property bool menuPublishKnown: false

        Menu {
            id: commitMenu
            MenuItem {
                text: qsTr("Copy this commit onto the current branch")
                enabled: repoTab.busyCount === 0
                onTriggered: repoTab.cherryPick(page.menuOid)
            }
            MenuItem {
                text: qsTr("Switch to this commit")
                enabled: repoTab.busyCount === 0
                onTriggered: page.switchTo("commit", page.menuOid, page.menuShort)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Edit message…")
                enabled: repoTab.busyCount === 0 && page.menuPublishKnown
                onTriggered: page.editMessage(page.menuOid)
            }
            MenuItem {
                text: qsTr("Fold into the commit before it")
                enabled: repoTab.busyCount === 0 && page.menuPublishKnown
                onTriggered: page.squashCommit(page.menuOid)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Copy commit hash")
                onTriggered: root.copyText(page.menuOid)
            }
        }

        // ---- rewriting one commit --------------------------------------
        // Both of these replay history when the commit is not the newest
        // one, so both warn once the commit has been pushed.
        function rewriteWarning(action, run) {
            if (!page.menuPublished) {
                run()
                return
            }
            root.confirm(
                qsTr("Rewrite a commit that is already on a remote?"),
                qsTr("%1 has been pushed. %2 replaces it, and every commit after "
                     + "it, with different ones — anyone who already has them will "
                     + "be out of step until they reset.")
                    .arg(page.menuShort).arg(action),
                qsTr("Rewrite anyway"), run)
        }
        function squashCommit(oidHex) {
            page.rewriteWarning(qsTr("Folding it in"),
                                function () { repoTab.squashIntoParent(oidHex) })
        }
        function editMessage(oidHex) {
            messageDialog.oid = oidHex
            messageDialog.published = page.menuPublished
            messageDialog.open()
        }

        Dialog {
            id: messageDialog
            anchors.centerIn: parent
            width: Math.min(640, root.width - 2 * Theme.spaceXxl)
            modal: true
            closePolicy: Popup.CloseOnEscape
            focus: true
            padding: Theme.spaceXxl
            header: null
            footer: null
            property string oid: ""
            property bool published: false
            /// The current message has been copied in. Saving before that
            /// would replace a real message with an empty editor.
            property bool filled: false
            background: Rectangle {
                color: Theme.bgElevated
                radius: Theme.radiusMd
                border.color: Theme.borderDefault
                border.width: Theme.borderWidth
            }
            // The row that opened the menu was selected by the same click,
            // so its details are on their way; fill in when they land.
            onOpened: {
                messageDialog.filled = false
                editSubject.text = ""
                editBody.text = ""
                messageDialog.fill()
                editSubject.forceActiveFocus()
            }
            function fill() {
                if (messageDialog.filled || !messageDialog.visible
                        || detailsModel.shaHex !== messageDialog.oid)
                    return
                messageDialog.filled = true
                editSubject.text = detailsModel.messageSubject
                editBody.text = detailsModel.messageBody
            }
            Connections {
                target: detailsModel
                function onChanged() { messageDialog.fill() }
            }
            function submit() {
                if (!messageDialog.filled || editSubject.text.trim() === "")
                    return
                const run = function () {
                    repoTab.rewordCommit(messageDialog.oid, editSubject.text, editBody.text)
                }
                messageDialog.close()
                if (messageDialog.published)
                    page.rewriteWarning(qsTr("Changing its message"), run)
                else
                    run()
            }
            contentItem: ColumnLayout {
                spacing: Theme.spaceLg
                Label {
                    text: qsTr("Edit commit message")
                    font.pixelSize: Theme.fontXl
                    font.weight: Font.DemiBold
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.textSecondary
                    text: qsTr("%1 — the newest commit is amended in place; an older "
                               + "one is replayed, which gives every commit after it a "
                               + "new identity.").arg(page.menuShort)
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: editSubject.implicitHeight + Theme.spaceSm
                    color: Theme.bgBase
                    radius: Theme.radiusMd
                    border.color: Theme.borderDefault
                    border.width: Theme.borderWidth
                    TextArea {
                        id: editSubject
                        anchors.fill: parent
                        anchors.margins: Theme.spaceXs
                        wrapMode: TextArea.Wrap
                        placeholderText: qsTr("Commit summary")
                        font.pixelSize: Theme.fontLg
                        font.weight: Font.DemiBold
                        background: null
                        padding: 0
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 120
                    color: Theme.bgBase
                    radius: Theme.radiusMd
                    border.color: Theme.borderSubtle
                    border.width: Theme.borderWidth
                    ScrollView {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceXs
                        TextArea {
                            id: editBody
                            wrapMode: TextArea.Wrap
                            placeholderText: qsTr("Description")
                            font.pixelSize: Theme.fontMd
                            color: Theme.textSecondary
                            background: null
                            padding: 0
                        }
                    }
                }
                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: Theme.spaceSm
                    HoverButton {
                        implicitHeight: Theme.controlHeight
                        text: qsTr("Cancel")
                        onClicked: messageDialog.close()
                    }
                    HoverButton {
                        implicitHeight: Theme.controlHeight
                        highlighted: true
                        text: qsTr("Save message")
                        enabled: messageDialog.filled && editSubject.text.trim() !== ""
                        onClicked: messageDialog.submit()
                    }
                }
            }
        }

        // ---- smoke hook ------------------------------------------------
        // PG_AUTO_ACT runs one write operation through exactly the code
        // path a click takes, so the wiring can be proven headlessly. The
        // dispatch is equality on a bare verb; nothing here parses.
        Timer {
            id: autoActTimer
            interval: 1200
            onTriggered: page.runAutoAct()
        }
        // The diff has to arrive before a row of it can be staged.
        Timer {
            id: stageRowTimer
            interval: 800
            onTriggered: page.stageSelection(
                0, AppBackend.autoAct === "stage-line" ? 0 : -1)
        }
        function runAutoAct() {
            const act = AppBackend.autoAct
            const arg = AppBackend.autoActArg
            if (act === "commit") {
                repoTab.stageAll()
                wipSubject.text = arg
                page.commitNow()
            } else if (act === "amend") {
                // The message is supplied, so skip the prefill request
                // that would otherwise land on top of it.
                amendBox.checked = true
                page.amending = true
                wipSubject.text = arg
                page.commitNow()
            } else if (act === "switch") {
                page.switchTo("branch", arg, arg)
            } else if (act === "switch-leave") {
                // What the dirty-tree dialog's first button does.
                page.moveKind = "branch"
                page.moveTarget = arg
                page.runSwitch(true)
            } else if (act === "switch-remote") {
                page.switchTo("remote", arg, arg)
            } else if (act === "squash") {
                page.openCommitMenu(branchesModel.headOid)
                page.squashCommit(branchesModel.headOid)
            } else if (act === "reword") {
                page.openCommitMenu(branchesModel.headOid)
                repoTab.rewordCommit(branchesModel.headOid, arg, "")
            } else if (act === "cherry-pick") {
                repoTab.cherryPick(arg)
            } else if (act === "stage-hunk" || act === "stage-line") {
                page.toggleDiff("unstaged", arg, "")
                stageRowTimer.start()
            } else if (act === "push") {
                page.pushNow()
            } else if (act === "force-push") {
                repoTab.pushCurrent("lease", page.upstreamOid())
            } else if (act === "force-push-confirm") {
                // Goes through the confirmation, so nothing should be
                // pushed until someone answers it.
                page.forcePushNow()
            } else if (act === "fetch") {
                repoTab.fetch("")
            } else if (act === "preview") {
                page.toggleDiff("untracked", arg, "")
            } else if (act === "preview-unstaged") {
                page.toggleDiff("unstaged", arg, "")
            } else if (act === "preview-staged") {
                page.toggleDiff("staged", arg, "")
            } else if (act === "settings") {
                settingsDialog.open()
                AppBackend.setAutoFetchMinutes(Number(arg))
            }
            AppBackend.report("auto_act ran=" + act)
        }

        // A finished write the editor asked for: clear it only once git
        // says the commit landed, so a rejected one keeps its text.
        property int seenWriteSeq: 0
        function absorbWriteResult() {
            if (repoTab.writeSeq === page.seenWriteSeq)
                return
            page.seenWriteSeq = repoTab.writeSeq
            if (repoTab.lastWriteError !== "")
                return
            if (repoTab.lastWriteOp === "commit") {
                page.clearCommitEditor()
                amendBox.checked = false
                page.amending = false
            }
            if (repoTab.lastWriteOp === "stage" || repoTab.lastWriteOp === "unstage")
                page.reloadDiff()
            // Moving HEAD rewrites the working tree under the diff pane:
            // the file it holds may not even exist where the move landed,
            // so the center goes back to the graph that was moved through.
            if (repoTab.lastWriteOp === "checkout")
                page.closeDiff()
            page.refreshHeadPublished()
        }

        // Center area switches between the graph and a file diff. The
        // pieces are kept apart rather than parsed back out of the key:
        // a path may contain anything, colons included.
        property bool diffShown: false
        property string diffKey: ""
        property string diffKind: ""
        property string diffPath: ""
        property string diffOrigPath: ""
        // Whether the shown diff is a working-tree file (stageable).
        property bool diffFromWt: false
        readonly property bool diffStaged: page.diffKind === "staged"
        function toggleDiff(kind, path, origPath) {
            const key = kind + ":" + path
            if (page.diffShown && page.diffKey === key) {
                page.closeDiff()
                return
            }
            page.diffKey = key
            page.diffKind = kind
            page.diffPath = path
            page.diffOrigPath = origPath
            page.diffFromWt = kind !== "commit"
            if (kind === "commit")
                diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                            path, origPath)
            else
                diffModel.requestWorkTree(kind, path, origPath)
            page.diffShown = true
        }
        // Stages (or unstages) one hunk, or one line of it. The indices
        // address the diff currently on screen, so the pane is reloaded
        // afterwards: once the patch is applied the rows have moved.
        function stageSelection(hunk, line) {
            repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line)
            page.pendingDiffReload = true
        }
        property bool pendingDiffReload: false
        function reloadDiff() {
            if (!page.pendingDiffReload || !page.diffShown)
                return
            page.pendingDiffReload = false
            diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
        }

        function closeDiff() {
            page.diffShown = false
            page.diffKey = ""
            page.diffKind = ""
            page.diffPath = ""
            page.diffOrigPath = ""
            page.diffFromWt = false
            diffModel.clear()
        }

        // Sidebar section expansion (filter reveals collapsed sections).
        property bool expBranches: true
        property bool expRemotes: true
        property bool expWorktree: true
        property bool expStashes: true
        property bool expTags: true

        // The last expanded section absorbs the leftover height so the
        // sidebar packs top to bottom.
        readonly property string lastOpen: refFilter.text !== "" ? "tags"
            : expTags ? "tags"
            : expStashes ? "stashes"
            : expWorktree ? "worktree"
            : expRemotes ? "remotes"
            : expBranches ? "branches" : ""

        // Exposed for the window toolbar (acts on the active tab).
        readonly property var pageTab: repoTab
        readonly property var pageWt: workTree

        RepoTab { id: repoTab }
        GraphModel { id: graphModel }
        WorkTreeModel { id: workTree }
        DetailsModel { id: detailsModel }
        DiffModel { id: diffModel }
        NavSectionModel { id: branchesModel }
        NavSectionModel { id: remotesModel }
        NavSectionModel { id: worktreeModel }
        NavSectionModel { id: worktreesModel }
        NavSectionModel { id: stashesModel }
        NavSectionModel { id: tagsModel }

        Component.onCompleted: {
            if (page.blank)
                return // no session to attach to; every model stays empty
            repoTab.attach(page.tab_id)
            graphModel.attach(page.tab_id)
            workTree.attach(page.tab_id)
            detailsModel.attach(page.tab_id)
            diffModel.attach(page.tab_id)
            branchesModel.attachSection(page.tab_id, "branches")
            remotesModel.attachSection(page.tab_id, "remotes")
            worktreeModel.attachSection(page.tab_id, "worktree")
            worktreesModel.attachSection(page.tab_id, "worktrees")
            stashesModel.attachSection(page.tab_id, "stashes")
            tagsModel.attachSection(page.tab_id, "tags")
            if (AppBackend.autoAct !== "")
                autoActTimer.start()
        }

        Connections {
            target: root
            function onFocusEpochChanged() {
                if (page.visible && repoTab.state === "open")
                    repoTab.refreshQuick()
            }
        }

        // Selection policy: restore across the tag-swap reset, and default
        // to the current branch's newest commit on first load so the
        // details pane always shows something.
        function trySelectDefault() {
            if (page.selectedOid !== "" || page.wipShown || AppBackend.autoSelect
                    || AppBackend.autoWip || graphModel.rowTotal === 0)
                return
            // Refs decide which commit is "current" — wait for them
            // instead of guessing the newest row too early.
            if (!branchesModel.refsLoaded)
                return
            let row = branchesModel.headOid !== ""
                      ? graphModel.rowOf(branchesModel.headOid) : -1
            if (row < 0) {
                if (graphModel.loading)
                    return // the head row may still be streaming in
                row = 0 // detached / head outside the window: newest commit
            }
            graphList.currentIndex = row
            anchorTimer.restart()
            graphList.rowSelected(graphModel.oidAt(row))
        }
        // Centering must outlive the ListView's own relayout: a model
        // reset (tag swap / reload) zeroes contentY during the polish that
        // runs after our handlers, so the anchor is applied a beat later.
        Timer {
            id: anchorTimer
            interval: root.anchorDelayMs
            onTriggered: {
                if (graphList.currentIndex >= 0)
                    graphList.positionViewAtIndex(graphList.currentIndex, ListView.Center)
            }
        }
        // Each finished pass ends in one drain (the swap never shows a
        // loading edge), so watch the pass counter instead: re-resolve the
        // selection by oid and re-anchor the viewport on it.
        property int seenFinishCount: 0
        Connections {
            target: graphModel
            function onStatsChanged() {
                if (graphModel.finishCount !== page.seenFinishCount) {
                    page.seenFinishCount = graphModel.finishCount
                    if (page.selectedOid !== "") {
                        const row = graphModel.rowOf(page.selectedOid)
                        if (row >= 0) {
                            graphList.currentIndex = row
                            anchorTimer.restart()
                        }
                    }
                }
                page.trySelectDefault()
            }
        }
        Connections {
            target: branchesModel
            function onChanged() { page.trySelectDefault() }
        }
        Connections {
            target: worktreeModel
            function onChanged() {
                if (worktreeModel.total === 0 && page.wipShown)
                    page.wipShown = false
                // Smoke hook (PG_AUTO_WIP=1): open the WIP view once
                // uncommitted changes are known.
                if (AppBackend.autoWip && worktreeModel.total > 0 && !page.wipShown) {
                    graphList.currentIndex = 0
                    page.showWip()
                }
            }
        }

        // Smoke hook (PG_SCROLL_TO=top|bottom): jump the graph after the
        // final pass settles, using the same clamped math as the wheel.
        Timer {
            id: scrollToTimer
            interval: 600
            onTriggered: {
                graphList.contentY = graphList.clampY(
                    AppBackend.scrollTo === "bottom" ? 1e12 : -1e12)
            }
        }
        Connections {
            target: graphModel
            enabled: AppBackend.scrollTo !== ""
            function onStatsChanged() {
                if (graphModel.finishCount > 0)
                    scrollToTimer.restart()
            }
        }

        // Scroll benchmark (PG_AUTO_SCROLL=1): after the stream finishes,
        // animate 3000 rows over 12s and report the measured fps.
        property bool benchStarted: false
        Connections {
            target: graphModel
            enabled: AppBackend.autoScroll
            function onStatsChanged() {
                if (!page.benchStarted && !graphModel.loading && graphModel.rowTotal > 0) {
                    page.benchStarted = true
                    benchPrep.start()
                }
            }
        }
        Timer {
            id: benchPrep
            interval: 800
            onTriggered: {
                page.benchT0 = Date.now()
                page.benchFrames0 = root.frameCounter
                benchAnim.start()
            }
        }
        property real benchT0: 0
        property int benchFrames0: 0
        NumberAnimation {
            id: benchAnim
            target: graphList
            property: "contentY"
            from: 0
            to: Math.min(3000, graphModel.rowTotal - 40) * Theme.rowHeight
            duration: 12000
            onStopped: {
                const secs = (Date.now() - page.benchT0) / 1000
                const frames = root.frameCounter - page.benchFrames0
                AppBackend.report("scroll_bench fps=" + (frames / secs).toFixed(1)
                                  + " rows=" + graphModel.rowTotal)
            }
        }

        // Automation (PG_AUTO_SELECT=1): select the newest row, then open
        // the first changed file's diff — exercises the full pipeline for
        // screenshot-based smoke tests.
        property bool autoSelected: false
        Connections {
            target: graphModel
            enabled: AppBackend.autoSelect
            function onStatsChanged() {
                if (!page.autoSelected && graphModel.rowTotal > 0) {
                    page.autoSelected = true
                    graphList.currentIndex = 0
                    graphList.rowSelected(graphModel.oidAt(0))
                }
            }
        }
        Connections {
            target: detailsModel
            enabled: AppBackend.autoSelect
            function onChanged() {
                if (detailsModel.shaHex !== "" && fileList.count > 0 && diffModel.title === "")
                    page.toggleDiff("commit", detailsModel.filePathAt(0),
                                    detailsModel.fileOrigPathAt(0))
            }
        }

        // Open failed: show git's own message
        Column {
            anchors.centerIn: parent
            visible: repoTab.state === "error"
            spacing: Theme.spaceMd
            width: Math.min(700, page.width - 2 * Theme.spaceXl)
            Label {
                text: qsTr("Could not open this folder as a git repository")
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                text: repoTab.error
                color: Theme.danger
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            HoverButton {
                text: qsTr("Close tab")
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: tabsModel.closeTab(page.tab_id)
            }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            visible: repoTab.state !== "error"

            // ---- three-pane layout --------------------------------------
            // (repository state / search / reload live in the window
            // toolbar, next to the tabs)
            SplitView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                orientation: Qt.Horizontal
                handle: Rectangle {
                    implicitWidth: Theme.splitterWidth
                    implicitHeight: Theme.splitterWidth
                    color: Theme.borderSubtle
                }

                // Navigation sidebar: fixed section headers, each section
                // scrolls inside its own list.
                Rectangle {
                    SplitView.preferredWidth: 260
                    SplitView.minimumWidth: 180
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        // Frameless, full-width filter: the sidebar is
                        // already enclosed by dividers, so the input only
                        // keeps a hairline underline (accent on focus).
                        // It is this pane's header band, so it takes the
                        // header height — the other panes' headers and the
                        // first row under each of them line up with it.
                        TextField {
                            id: refFilter
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            font.pixelSize: Theme.fontMd
                            leftPadding: Theme.spaceSm
                            rightPadding: Theme.spaceSm
                            topPadding: 0
                            bottomPadding: 0
                            placeholderText: qsTr("Filter")
                            background: Rectangle {
                                color: "transparent"
                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    height: Theme.borderWidth
                                    color: refFilter.activeFocus ? Theme.borderFocus
                                                                 : Theme.borderSubtle
                                }
                            }
                            onTextChanged: {
                                branchesModel.setFilter(text)
                                remotesModel.setFilter(text)
                                worktreesModel.setFilter(text)
                                stashesModel.setFilter(text)
                                tagsModel.setFilter(text)
                            }
                        }

                        NavHeader {
                            caption: qsTr("BRANCHES")
                            iconKind: "branch"
                            iconTint: Theme.accent
                            count: branchesModel.total
                            expanded: page.expBranches || refFilter.text !== ""
                            onToggled: page.expBranches = !page.expBranches
                        }
                        // Current branch pinned under the header (it stays
                        // in the list too, highlighted). Replaces the old
                        // top-left branch display.
                        Rectangle {
                            visible: (page.expBranches || refFilter.text !== "")
                                     && (branchesModel.headName !== "" || workTree.detached)
                            Layout.fillWidth: true
                            implicitHeight: Theme.rowHeight
                            color: Theme.bgElevated
                            Rectangle {
                                anchors.fill: parent
                                color: Theme.bgHover
                                visible: headRowMouse.containsMouse
                            }
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceMd
                                anchors.rightMargin: Theme.spaceSm
                                spacing: Theme.spaceXs
                                // "You are here" marker, sharing the fold
                                // arrows' column so the sidebar lines up.
                                NavIcon {
                                    kind: "check"
                                    tint: Theme.accent
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: workTree.detached ? qsTr("DETACHED HEAD")
                                                            : branchesModel.headName
                                    color: workTree.detached ? Theme.warning : Theme.textLink
                                    font.weight: Font.DemiBold
                                    font.pixelSize: Theme.fontMd
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    visible: !workTree.detached && workTree.upstream !== ""
                                    text: "↑" + workTree.ahead + " ↓" + workTree.behind
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontSm
                                }
                                NavIcon {
                                    visible: !workTree.detached
                                             && (branchesModel.headHasRemote
                                                 || branchesModel.headHasPr)
                                    kind: branchesModel.headHasPr ? "pr" : "remote"
                                    tint: branchesModel.headHasPr ? Theme.success
                                                                  : Theme.textSecondary
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                            }
                            MouseArea {
                                id: headRowMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                enabled: branchesModel.headOid !== ""
                                onClicked: page.jumpToRef(branchesModel.headOid)
                            }
                        }
                        NavList {
                            sectionModel: branchesModel
                            expanded: page.expBranches || refFilter.text !== ""
                            kindHint: "branch"
                            stretch: page.lastOpen === "branches"
                            headTrack: workTree.upstream !== ""
                                       ? "↑" + workTree.ahead + " ↓" + workTree.behind : ""
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                            onRefMenuRequested: (name, oidHex) =>
                                page.openRefMenu(name, oidHex, false)
                        }

                        NavHeader {
                            caption: qsTr("REMOTES")
                            iconKind: "remote"
                            iconTint: Theme.textSecondary
                            count: remotesModel.total
                            expanded: page.expRemotes || refFilter.text !== ""
                            onToggled: page.expRemotes = !page.expRemotes
                        }
                        NavList {
                            sectionModel: remotesModel
                            expanded: page.expRemotes || refFilter.text !== ""
                            kindHint: "remote"
                            stretch: page.lastOpen === "remotes"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                            onRefMenuRequested: (name, oidHex) =>
                                page.openRefMenu(name, oidHex, true)
                        }

                        // git worktrees (checkouts), GitKraken-style; the
                        // changed-file lists live in the right pane's WIP
                        // view. Clicking one opens it as a new tab.
                        NavHeader {
                            caption: qsTr("WORKTREES")
                            iconKind: "tree"
                            iconTint: Theme.success
                            count: worktreesModel.total
                            expanded: page.expWorktree || refFilter.text !== ""
                            onToggled: page.expWorktree = !page.expWorktree
                        }
                        NavList {
                            sectionModel: worktreesModel
                            expanded: page.expWorktree || refFilter.text !== ""
                            kindHint: "worktree"
                            stretch: page.lastOpen === "worktree"
                            onFileActivated: (bucket, path, origPath) =>
                                tabsModel.openRepositoryPath(path)
                        }

                        NavHeader {
                            caption: qsTr("STASHES")
                            iconKind: "stash"
                            iconTint: Theme.textSecondary
                            count: stashesModel.total
                            expanded: page.expStashes || refFilter.text !== ""
                            onToggled: page.expStashes = !page.expStashes
                        }
                        NavList {
                            sectionModel: stashesModel
                            expanded: page.expStashes || refFilter.text !== ""
                            kindHint: "stash"
                            stretch: page.lastOpen === "stashes"
                            // A stash is a commit: clicking shows its
                            // stashed changes in the details pane.
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }

                        NavHeader {
                            caption: qsTr("TAGS")
                            iconKind: "tag"
                            iconTint: Theme.warning
                            count: tagsModel.total
                            expanded: page.expTags || refFilter.text !== ""
                            onToggled: page.expTags = !page.expTags
                            showTagToggle: true
                            tagsShown: repoTab.tagsShown
                            onTagsToggled: shown => repoTab.setTagsShown(shown)
                        }
                        NavList {
                            sectionModel: tagsModel
                            expanded: page.expTags || refFilter.text !== ""
                            kindHint: "tag"
                            stretch: page.lastOpen === "tags"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }
                    }
                }

                // Center: commit graph ⇄ file diff
                StackLayout {
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 420
                    currentIndex: page.diffShown ? 1 : 0

                    // -- graph --
                    Rectangle {
                        color: Theme.bgSurface
                        // Divider hover-lines live under the list so the
                        // message ticks and lane strokes stay in front.
                        Rectangle {
                            x: labelDivider.x + Theme.borderWidth
                            width: Theme.splitterWidth - 2 * Theme.borderWidth
                            height: parent.height
                            color: Theme.borderStrong
                            visible: labelDivider.containsMouse || labelDivider.pressed
                        }
                        Rectangle {
                            x: graphDivider.x + Theme.borderWidth
                            width: Theme.splitterWidth - 2 * Theme.borderWidth
                            height: parent.height
                            color: Theme.borderStrong
                            visible: graphDivider.containsMouse || graphDivider.pressed
                        }
                        ListView {
                            id: graphList
                            anchors.fill: parent
                            clip: true
                            model: graphModel
                            reuseItems: true
                            boundsBehavior: Flickable.StopAtBounds
                            flickDeceleration: 8000
                            maximumFlickVelocity: 9000
                            ScrollBar.vertical: ScrollBar {}
                            // The graph is the one pane with no header band;
                            // the gap stands in for one, so its first row
                            // starts on the same line as the first row of
                            // the sidebar and the details pane.
                            topMargin: Theme.headerHeight
                            // Bridge into the page scope for the shared
                            // delegate (inline components cannot see page ids).
                            property real labelWidth: page.labelW
                            property real graphColWidth: page.graphColW
                            property real graphFullWidth: page.graphFullW
                            property real graphXOffset: page.graphX
                            property int wipCount: worktreeModel.total
                            signal rowSelected(string oidHex)
                            signal rowMenuRequested(string oidHex)
                            onRowMenuRequested: oidHex => page.openCommitMenu(oidHex)
                            onRowSelected: oidHex => {
                                // The all-zero id is the synthetic WIP row.
                                if (oidHex !== "" && !/[^0]/.test(oidHex)) {
                                    page.showWip()
                                    return
                                }
                                page.wipShown = false
                                page.selectedOid = oidHex
                                page.selectedStashRef = graphModel.stashRefOf(oidHex)
                                detailsModel.request(oidHex)
                                page.closeDiff()
                            }
                            delegate: GraphRowDelegate {}
                            // Window cut: lanes keep running through the
                            // footer and the message sits where subjects go.
                            footer: Item {
                                width: graphList.width
                                height: graphModel.truncated ? 2 * Theme.rowHeight : 0
                                visible: graphModel.truncated
                                Item {
                                    x: graphList.labelWidth
                                    width: graphList.graphColWidth
                                    height: parent.height
                                    clip: true
                                    Canvas {
                                        id: tailCanvas
                                        x: -graphList.graphXOffset
                                        width: graphList.graphFullWidth
                                        height: parent.height
                                        onPaint: {
                                            const ctx = getContext("2d")
                                            ctx.clearRect(0, 0, width, height)
                                            if (graphModel.tailGeometry === "")
                                                return
                                            ctx.lineWidth = root.laneStroke
                                            ctx.globalAlpha = 0.45
                                            const toks = graphModel.tailGeometry.split(";")
                                            for (let n = 0; n < toks.length; n++) {
                                                const dot = toks[n].indexOf(".")
                                                const lane = parseInt(toks[n].substring(0, dot))
                                                const color = parseInt(toks[n].substring(dot + 1))
                                                const x = root.laneInset + lane * root.laneW + root.laneW / 2
                                                ctx.strokeStyle = Theme.graphLane[color % Theme.graphLane.length]
                                                ctx.beginPath()
                                                ctx.moveTo(x, 0)
                                                ctx.lineTo(x, height)
                                                ctx.stroke()
                                            }
                                        }
                                        Connections {
                                            target: graphModel
                                            function onStatsChanged() { tailCanvas.requestPaint() }
                                        }
                                    }
                                }
                                Label {
                                    x: graphList.labelWidth + graphList.graphColWidth
                                    anchors.verticalCenter: parent.verticalCenter
                                    leftPadding: Theme.spaceSm
                                    text: qsTr("Showing the first %L1 commits — older history is not loaded")
                                          .arg(graphModel.rowTotal)
                                    color: Theme.warning
                                    font.pixelSize: Theme.fontMd
                                    font.weight: Font.DemiBold
                                }
                            }
                            // Manual contentY math must respect originY:
                            // after positionViewAtIndex jumps, the ListView
                            // shifts its coordinate origin as item positions
                            // are fixed up, so [0, contentHeight-height] no
                            // longer matches the real scroll range (top rows
                            // become unreachable, the bottom overshoots the
                            // truncation footer).
                            function clampY(y) {
                                // topMargin lives above the content origin —
                                // forgetting it makes the top gap
                                // unreachable by wheel after any scroll.
                                const minY = graphList.originY - graphList.topMargin
                                const maxY = Math.max(minY, graphList.originY
                                                            + graphList.contentHeight
                                                            - graphList.height
                                                            + graphList.bottomMargin)
                                return Math.max(minY, Math.min(y, maxY))
                            }
                            // Mouse wheels scroll a fixed number of rows per
                            // notch; touchpads keep native Flickable panning.
                            WheelHandler {
                                acceptedDevices: PointerDevice.Mouse
                                onWheel: event => {
                                    // Wheel input exits middle-click
                                    // autoscroll (Chrome-like behavior).
                                    graphArea.autoScrolling = false
                                    graphList.cancelFlick()
                                    if (event.angleDelta.x !== 0)
                                        page.graphX = Math.max(0, Math.min(
                                            page.graphX - event.angleDelta.x / 2, page.graphXMax))
                                    const step = (event.angleDelta.y / 120)
                                               * root.wheelRows * Theme.rowHeight
                                    graphList.contentY = graphList.clampY(graphList.contentY - step)
                                }
                            }
                        }
                        // Middle-click toggles autoscroll mode: the pointer
                        // distance from the anchor sets the speed; any click
                        // exits.
                        property bool autoScrolling: false
                        property real autoAnchorX: 0
                        property real autoAnchorY: 0
                        property real autoCurrentX: 0
                        property real autoCurrentY: 0
                        id: graphArea
                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.MiddleButton
                            onClicked: mouse => {
                                graphArea.autoAnchorX = mouse.x
                                graphArea.autoAnchorY = mouse.y
                                graphArea.autoCurrentX = mouse.x
                                graphArea.autoCurrentY = mouse.y
                                graphArea.autoScrolling = true
                            }
                        }
                        MouseArea {
                            visible: graphArea.autoScrolling
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.AllButtons
                            cursorShape: Qt.SizeAllCursor
                            onPositionChanged: mouse => {
                                graphArea.autoCurrentX = mouse.x
                                graphArea.autoCurrentY = mouse.y
                            }
                            onPressed: mouse => {
                                graphArea.autoScrolling = false
                                mouse.accepted = true
                            }
                            Timer {
                                running: graphArea.autoScrolling
                                interval: 16
                                repeat: true
                                onTriggered: {
                                    const delta = (graphArea.autoCurrentY - graphArea.autoAnchorY)
                                                * root.middleScrollGain
                                    graphList.contentY = graphList.clampY(graphList.contentY + delta)
                                    // Sideways drift pans the lanes.
                                    if (page.graphXMax > 0) {
                                        const dx = (graphArea.autoCurrentX - graphArea.autoAnchorX)
                                                 * root.middleScrollGain
                                        page.graphX = Math.max(0, Math.min(page.graphX + dx,
                                                                           page.graphXMax))
                                    }
                                }
                            }
                            // Anchor marker
                            Rectangle {
                                x: graphArea.autoAnchorX - Theme.iconMd / 2
                                y: graphArea.autoAnchorY - Theme.iconMd / 2
                                width: Theme.iconMd
                                height: Theme.iconMd
                                radius: Theme.iconMd / 2
                                color: "transparent"
                                border.color: Theme.borderStrong
                                border.width: Theme.borderWidth
                                Rectangle {
                                    anchors.centerIn: parent
                                    width: Theme.spaceXs
                                    height: Theme.spaceXs
                                    radius: Theme.spaceXs / 2
                                    color: Theme.borderStrong
                                }
                            }
                        }
                        // Left-drag inside the lanes pans them horizontally
                        // when they overflow; a motionless press-release
                        // still selects the row underneath.
                        MouseArea {
                            id: lanePan
                            x: page.labelW
                            width: page.graphColW
                            height: parent.height
                            z: 1
                            visible: page.graphXMax > 0
                            acceptedButtons: Qt.LeftButton
                            property real pressX: 0
                            property real startGX: 0
                            property bool panning: false
                            onPressed: mouse => {
                                pressX = mouse.x
                                startGX = page.graphX
                                panning = false
                            }
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                if (!panning && Math.abs(mouse.x - pressX) > Theme.spaceXs)
                                    panning = true
                                if (panning)
                                    page.graphX = Math.max(0, Math.min(
                                        startGX - (mouse.x - pressX), page.graphXMax))
                            }
                            onReleased: mouse => {
                                if (panning)
                                    return
                                const idx = graphList.indexAt(page.labelW + 1,
                                                              graphList.contentY + mouse.y)
                                if (idx >= 0) {
                                    graphList.currentIndex = idx
                                    graphList.rowSelected(graphModel.oidAt(idx))
                                }
                            }
                        }

                        // Draggable column dividers (labels | graph | message).
                        MouseArea {
                            id: labelDivider
                            x: page.labelW - Theme.splitterWidth / 2
                            width: Theme.splitterWidth
                            height: parent.height
                            z: 2
                            visible: !page.blank
                            hoverEnabled: true
                            cursorShape: Qt.SplitHCursor
                            preventStealing: true
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                const nx = mapToItem(graphArea, mouse.x, 0).x
                                page.labelWManual = Math.max(Theme.spaceXxl,
                                    Math.min(nx, graphArea.width - 2 * Theme.spaceXxl))
                            }
                        }
                        MouseArea {
                            id: graphDivider
                            // Sits behind the message tick column so the
                            // hover line overlaps the ticks.
                            x: page.labelW + page.graphColW + Theme.spaceSm
                               + Theme.borderWidth - Theme.splitterWidth / 2
                            width: Theme.splitterWidth
                            height: parent.height
                            z: 2
                            visible: !page.blank
                            hoverEnabled: true
                            cursorShape: Qt.SplitHCursor
                            preventStealing: true
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                const nx = mapToItem(graphArea, mouse.x, 0).x
                                              - Theme.spaceSm - Theme.borderWidth
                                page.graphColWManual = Math.max(root.laneInset + root.laneW,
                                    Math.min(nx - page.labelW,
                                             graphArea.width - page.labelW - 2 * Theme.spaceXxl))
                            }
                        }
                        // Horizontal scroll of the lanes when the full graph
                        // is wider than its column.
                        ScrollBar {
                            visible: page.graphXMax > 0
                            orientation: Qt.Horizontal
                            x: page.labelW
                            width: page.graphColW
                            anchors.bottom: parent.bottom
                            z: 2
                            size: page.graphFullW > 0 ? page.graphColW / page.graphFullW : 1
                            position: page.graphFullW > 0 ? page.graphX / page.graphFullW : 0
                            onPositionChanged: {
                                if (pressed)
                                    page.graphX = Math.max(0, Math.min(
                                        position * page.graphFullW, page.graphXMax))
                            }
                        }
                        BusyIndicator {
                            anchors.centerIn: parent
                            running: graphModel.loading && graphModel.rowTotal === 0
                        }
                        Label {
                            anchors.centerIn: parent
                            visible: graphModel.error !== ""
                            text: graphModel.error
                            color: Theme.danger
                            width: parent.width - 2 * Theme.spaceXl
                            wrapMode: Text.Wrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                        // Empty window: the one thing worth doing sits in
                        // the column that will hold the history.
                        Column {
                            anchors.centerIn: parent
                            visible: page.blank
                            spacing: Theme.spaceLg
                            Label {
                                text: qsTr("platitude-gg")
                                font.pixelSize: Theme.fontXl
                                font.weight: Font.DemiBold
                                anchors.horizontalCenter: parent.horizontalCenter
                            }
                            Label {
                                text: qsTr("A thin, fast GUI over your installed git.")
                                color: Theme.textSecondary
                                anchors.horizontalCenter: parent.horizontalCenter
                            }
                            HoverButton {
                                text: qsTr("Open repository…")
                                anchors.horizontalCenter: parent.horizontalCenter
                                onClicked: folderDialog.open()
                            }
                        }
                    }

                    // -- diff --
                    Rectangle {
                        color: Theme.bgSurface
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 0
                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: Theme.headerHeight
                                color: Theme.bgElevated
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: Theme.spaceSm
                                    anchors.rightMargin: Theme.spaceSm
                                    spacing: Theme.spaceSm
                                    Label {
                                        text: qsTr("DIFF · %1").arg(diffModel.title)
                                        font.pixelSize: Theme.fontSm
                                        font.weight: Font.DemiBold
                                        color: Theme.textSecondary
                                        elide: Text.ElideMiddle
                                        Layout.fillWidth: true
                                    }
                                    HoverToolButton {
                                        visible: page.diffFromWt
                                        text: page.diffStaged ? qsTr("Unstage file")
                                                              : qsTr("Stage file")
                                        font.pixelSize: Theme.fontSm
                                        enabled: repoTab.busyCount === 0
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 300
                                        ToolTip.text: page.diffStaged
                                            ? qsTr("Take this whole file out of the next commit")
                                            : qsTr("Put this whole file into the next commit")
                                        onClicked: {
                                            if (page.diffStaged)
                                                repoTab.unstagePath(page.diffPath)
                                            else
                                                repoTab.stagePath(page.diffPath)
                                        }
                                    }
                                    HoverToolButton {
                                        text: "×"
                                        implicitWidth: Theme.iconLg
                                        implicitHeight: Theme.iconLg
                                        padding: 0
                                        onClicked: page.closeDiff()
                                    }
                                }
                            }
                            // -- content preview: binaries summarized by
                            //    size, images rendered (added = After only,
                            //    deleted = Before only, modified = both).
                            Label {
                                visible: diffModel.previewKind === "binary"
                                         || (diffModel.isBinary
                                             && diffModel.previewKind === "")
                                Layout.margins: Theme.spaceSm
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                text: {
                                    const oldS = diffModel.previewOldSize
                                    const newS = diffModel.previewNewSize
                                    if (oldS !== "" && newS !== "")
                                        return qsTr("Binary file · %1 → %2").arg(oldS).arg(newS)
                                    if (newS !== "")
                                        return qsTr("Binary file · %1").arg(newS)
                                    if (oldS !== "")
                                        return qsTr("Binary file removed · was %1").arg(oldS)
                                    return qsTr("Binary file — no text diff")
                                }
                                color: Theme.textMuted
                            }
                            RowLayout {
                                visible: diffModel.previewKind === "image"
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.margins: Theme.spaceSm
                                spacing: Theme.spaceSm
                                ImagePreviewCell {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    label: qsTr("Before · %1").arg(diffModel.previewOldSize)
                                    url: diffModel.previewOldUrl
                                    sizeText: diffModel.previewOldSize
                                }
                                ImagePreviewCell {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    label: qsTr("After · %1").arg(diffModel.previewNewSize)
                                    url: diffModel.previewNewUrl
                                    sizeText: diffModel.previewNewSize
                                }
                            }
                            ListView {
                                id: diffList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: diffModel
                                reuseItems: true
                                boundsBehavior: Flickable.StopAtBounds
                                ScrollBar.vertical: ScrollBar {}
                                // An image with no text rows hands its space
                                // to the preview (SVG edits keep both).
                                visible: diffModel.previewKind !== "image"
                                         || count > 0
                                delegate: Rectangle {
                                    id: diffRow
                                    required property string kind
                                    required property int old_no
                                    required property int new_no
                                    required property string text
                                    required property int hunk
                                    required property int line
                                    width: diffList.width
                                    height: Theme.rowHeight
                                    color: kind === "add" ? Theme.diffAddedBg
                                           : kind === "del" ? Theme.diffRemovedBg
                                           : kind === "hunk" ? Theme.diffHunkHeaderBg
                                           : "transparent"
                                    Row {
                                        anchors.fill: parent
                                        spacing: 0
                                        Label {
                                            width: 42
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: Theme.spaceXs
                                            color: Theme.textMuted
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontSm
                                        }
                                        Label {
                                            width: 42
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: Theme.spaceXs
                                            color: Theme.textMuted
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontSm
                                        }
                                        Label {
                                            width: parent.width - 84
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.text
                                            elide: Text.ElideRight
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontMd
                                            color: diffRow.kind === "add" ? Theme.diffAddedFg
                                                   : diffRow.kind === "del" ? Theme.diffRemovedFg
                                                   : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                                                   : diffRow.kind === "meta" ? Theme.textMuted
                                                   : Theme.textPrimary
                                        }
                                    }
                                    MouseArea {
                                        id: lineHover
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.NoButton
                                        enabled: page.diffFromWt
                                    }
                                    // Hunk-level staging. The row carries the
                                    // hunk index the patch builder needs, so
                                    // what is staged is exactly what is shown.
                                    HoverToolButton {
                                        visible: page.diffFromWt && diffRow.kind === "hunk"
                                        anchors.right: parent.right
                                        anchors.rightMargin: Theme.spaceSm
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: page.diffStaged ? qsTr("Unstage hunk")
                                                              : qsTr("Stage hunk")
                                        font.pixelSize: Theme.fontSm
                                        enabled: repoTab.busyCount === 0
                                        onClicked: page.stageSelection(diffRow.hunk, -1)
                                    }
                                    // Line-level staging.
                                    Rectangle {
                                        visible: page.diffFromWt && lineHover.containsMouse
                                                 && (diffRow.kind === "add"
                                                     || diffRow.kind === "del")
                                        x: Theme.spaceXs
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Theme.iconMd
                                        height: Theme.iconMd
                                        radius: Theme.radiusSm
                                        color: Theme.bgElevated
                                        border.color: Theme.borderStrong
                                        border.width: Theme.borderWidth
                                        ToolTip.visible: stageLineHover.containsMouse
                                        ToolTip.delay: 300
                                        ToolTip.text: page.diffStaged ? qsTr("Unstage this line")
                                                                      : qsTr("Stage this line")
                                        Rectangle {
                                            anchors.fill: parent
                                            radius: Theme.radiusSm
                                            color: Theme.bgHover
                                            visible: stageLineHover.containsMouse
                                        }
                                        NavIcon {
                                            anchors.centerIn: parent
                                            width: Theme.iconSm
                                            height: Theme.iconSm
                                            kind: diffRow.kind === "add" ? "plus" : "minus"
                                            tint: Theme.textPrimary
                                        }
                                        MouseArea {
                                            id: stageLineHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            onClicked: page.stageSelection(diffRow.hunk,
                                                                           diffRow.line)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right side: commit details
                Rectangle {
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: 300
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0

                        // ---- working-tree (WIP) mode: the commit-editor
                        // shape plus stage/unstage affordances. The
                        // controls are visual for now — the operations
                        // land in Phase 2. ----
                        Rectangle {
                            visible: page.wipShown
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: qsTr("UNCOMMITTED CHANGES (%1)").arg(worktreeModel.total)
                                    font.pixelSize: Theme.fontSm
                                    font.weight: Font.DemiBold
                                    color: Theme.textSecondary
                                }
                                Item { Layout.fillWidth: true }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Tree view")
                                    onClicked: worktreeModel.setTreeView(true)
                                    contentItem: NavIcon {
                                        kind: "hier"
                                        tint: worktreeModel.treeView ? Theme.accent
                                                                     : Theme.textMuted
                                    }
                                }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Paths view")
                                    onClicked: worktreeModel.setTreeView(false)
                                    contentItem: NavIcon {
                                        kind: "list"
                                        tint: worktreeModel.treeView ? Theme.textMuted
                                                                     : Theme.accent
                                    }
                                }
                            }
                        }
                        // Message editor pinned on top — identical shape in
                        // commit details, amend and new-commit creation.
                        // Flush against the header band, like every other
                        // pane's first row.
                        ColumnLayout {
                            visible: page.wipShown
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            Layout.topMargin: 0
                            spacing: Theme.spaceXs
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: wipSubject.implicitHeight + Theme.spaceSm
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderDefault
                                border.width: Theme.borderWidth
                                TextArea {
                                    id: wipSubject
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    wrapMode: TextArea.Wrap
                                    placeholderText: qsTr("Commit summary")
                                    font.pixelSize: Theme.fontLg
                                    font.weight: Font.DemiBold
                                    color: Theme.textPrimary
                                    background: null
                                    padding: 0
                                }
                            }
                            // Two lines tall from the start (matches the
                            // details pane's description box).
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(Math.max(wipBody.implicitHeight,
                                                                          2 * Theme.fontMdLine)
                                                                 + Theme.spaceSm, 120)
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderSubtle
                                border.width: Theme.borderWidth
                                ScrollView {
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    TextArea {
                                        id: wipBody
                                        wrapMode: TextArea.Wrap
                                        placeholderText: qsTr("Description")
                                        font.pixelSize: Theme.fontMd
                                        color: Theme.textSecondary
                                        background: null
                                        padding: 0
                                    }
                                }
                            }
                            // Amend replaces the newest commit instead of
                            // adding one, so it starts from that commit's
                            // message rather than an empty editor.
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                CheckBox {
                                    id: amendBox
                                    text: qsTr("Amend the last commit")
                                    font.pixelSize: Theme.fontSm
                                    implicitHeight: Theme.controlHeight
                                    onToggled: page.amendToggled(checked)
                                }
                                Item { Layout.fillWidth: true }
                                // Warned about, not forbidden: git allows
                                // it and the confirmation says what it costs.
                                Label {
                                    visible: page.amending && page.headPublished
                                    text: qsTr("already pushed")
                                    color: Theme.warning
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: amendPushedHover.containsMouse
                                    ToolTip.delay: 400
                                    ToolTip.text: qsTr("The last commit is on a remote. "
                                                       + "Rewriting it would leave anyone "
                                                       + "who already has it out of step.")
                                    MouseArea {
                                        id: amendPushedHover
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.NoButton
                                    }
                                }
                            }
                            HoverButton {
                                id: commitButton
                                Layout.fillWidth: true
                                highlighted: true
                                text: page.amending
                                      ? qsTr("Amend commit (%1 staged)").arg(workTree.stagedCount)
                                      : qsTr("Commit changes (%1 staged)").arg(workTree.stagedCount)
                                // An amend can stand on its own (message
                                // only); a new commit needs staged content
                                // and a summary, and git needs an identity
                                // to attribute either one to.
                                enabled: repoTab.busyCount === 0
                                         && repoTab.identityReady
                                         && wipSubject.text.trim() !== ""
                                         && (page.amending || workTree.stagedCount > 0)
                                onClicked: page.commitNow()
                                ToolTip.visible: commitHover.containsMouse && !enabled
                                ToolTip.delay: 300
                                ToolTip.text: !repoTab.identityReady
                                              ? qsTr("git has no name or email to record on commits")
                                              : wipSubject.text.trim() === ""
                                              ? qsTr("A commit needs a summary")
                                              : qsTr("Stage something to commit")
                                MouseArea {
                                    id: commitHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.NoButton
                                }
                            }
                        }
                        ListView {
                            id: wipList
                            visible: page.wipShown
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: worktreeModel
                            reuseItems: true
                            ScrollBar.vertical: ScrollBar {}
                            // GitKraken grouping: unstaged (incl. untracked)
                            // above, staged below.
                            section.property: "group"
                            section.delegate: Rectangle {
                                id: bucketHeader
                                required property string section
                                width: wipList.width
                                height: Theme.rowHeight
                                color: Theme.bgElevated
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: Theme.spaceSm
                                    anchors.rightMargin: Theme.spaceXs
                                    spacing: Theme.spaceXs
                                    Label {
                                        text: bucketHeader.section === "staged"
                                              ? qsTr("STAGED FILES (%1)").arg(workTree.stagedCount)
                                              : bucketHeader.section === "unstaged"
                                              ? qsTr("UNSTAGED FILES (%1)")
                                                .arg(workTree.unstagedCount + workTree.untrackedCount)
                                              : qsTr("CONFLICTS")
                                        font.pixelSize: Theme.fontSm
                                        font.weight: Font.DemiBold
                                        color: bucketHeader.section === "conflicts"
                                               ? Theme.danger : Theme.textSecondary
                                    }
                                    Item { Layout.fillWidth: true }
                                    HoverToolButton {
                                        visible: bucketHeader.section !== "conflicts"
                                        text: bucketHeader.section === "staged"
                                              ? qsTr("Unstage all") : qsTr("Stage all")
                                        font.pixelSize: Theme.fontSm
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 300
                                        ToolTip.text: bucketHeader.section === "staged"
                                            ? qsTr("Empty the staging area")
                                            : qsTr("Stage every change, untracked files included")
                                        onClicked: {
                                            if (bucketHeader.section === "staged")
                                                repoTab.unstageAll()
                                            else
                                                repoTab.stageAll()
                                        }
                                    }
                                }
                            }
                            delegate: NavItemDelegate {
                                listWidth: wipList.width
                                kindHint: "wt"
                                showStage: true
                                onFileClicked: (bucket, path, origPath) =>
                                    page.toggleDiff(bucket, path, origPath)
                                onFolderClicked: key => worktreeModel.toggleFolder(key)
                                onStageClicked: (bucket, path) => {
                                    if (bucket === "staged")
                                        repoTab.unstagePath(path)
                                    else
                                        repoTab.stagePath(path)
                                }
                            }
                        }

                        // ---- commit-details mode ----
                        PaneHeader {
                            visible: !page.wipShown
                            text: qsTr("COMMIT")
                        }
                        // Stash actions when the selected row is a stash.
                        Rectangle {
                            visible: !page.wipShown && page.selectedStashRef !== ""
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                NavIcon {
                                    kind: "stash"
                                    tint: Theme.textSecondary
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                                Label {
                                    text: page.selectedStashRef
                                    font.family: Theme.monoFamily
                                    font.pixelSize: Theme.fontSm
                                    color: Theme.textSecondary
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                HoverToolButton {
                                    text: qsTr("Apply")
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Apply this stash, keeping it")
                                    onClicked: repoTab.applyStash(page.selectedStashRef)
                                }
                                HoverToolButton {
                                    text: qsTr("Pop")
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Apply this stash and drop it")
                                    onClicked: {
                                        repoTab.popStash(page.selectedStashRef)
                                        page.selectedStashRef = ""
                                    }
                                }
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            Layout.topMargin: 0
                            spacing: Theme.spaceXs
                            visible: !page.wipShown && detailsModel.shaHex !== ""

                            // -- message first, like the commit editor:
                            // a prominent summary box and a dimmer
                            // description box (Phase 2 makes these editable
                            // for new commits and amend) --
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: subjectArea.implicitHeight + Theme.spaceSm
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderDefault
                                border.width: Theme.borderWidth
                                TextArea {
                                    id: subjectArea
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    readOnly: true
                                    wrapMode: TextArea.Wrap
                                    text: detailsModel.messageSubject
                                    font.pixelSize: Theme.fontLg
                                    font.weight: Font.DemiBold
                                    color: Theme.textPrimary
                                    background: null
                                    padding: 0
                                }
                            }
                            // Always shown, even empty, and two lines tall
                            // from the start — the pair mirrors the commit
                            // editor's fields.
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(Math.max(bodyArea.implicitHeight,
                                                                          2 * Theme.fontMdLine)
                                                                 + Theme.spaceSm, 120)
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderSubtle
                                border.width: Theme.borderWidth
                                ScrollView {
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    TextArea {
                                        id: bodyArea
                                        readOnly: true
                                        wrapMode: TextArea.Wrap
                                        text: detailsModel.messageBody
                                        font.pixelSize: Theme.fontMd
                                        color: Theme.textSecondary
                                        background: null
                                        padding: 0
                                    }
                                }
                            }
                            // -- author card: avatar + name/date on the
                            // left, own hash over parent hash on the right
                            // (rows aligned) --
                            RowLayout {
                                spacing: Theme.spaceSm
                                IdentIcon {
                                    code: detailsModel.avatar
                                    width: root.detailsAvatar
                                    height: root.detailsAvatar
                                }
                                ColumnLayout {
                                    spacing: 0
                                    Layout.fillWidth: true
                                    Label {
                                        id: authorLabel
                                        text: detailsModel.authorName
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        color: Theme.textPrimary
                                        font.pixelSize: Theme.fontMd
                                        font.weight: Font.DemiBold
                                        ToolTip.visible: authorHover.containsMouse
                                        ToolTip.delay: 400
                                        ToolTip.text: qsTr("Author: %1 <%2>\nCommitter: %3")
                                                      .arg(detailsModel.authorName)
                                                      .arg(detailsModel.authorEmail)
                                                      .arg(detailsModel.committer)
                                        MouseArea {
                                            id: authorHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            acceptedButtons: Qt.NoButton
                                        }
                                    }
                                    Label {
                                        id: detailsDate
                                        text: Qt.formatDateTime(new Date(detailsModel.authorTime * 1000),
                                                                "yyyy-MM-dd HH:mm")
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontSm
                                    }
                                }
                                ColumnLayout {
                                    spacing: 0
                                    Layout.alignment: Qt.AlignRight
                                    // The hash is the button, not just the
                                    // icon beside it — a 16px glyph was too
                                    // small to aim at. Hovering underlines
                                    // the hash and lights the icon so the
                                    // whole plate reads as one control.
                                    // Not a HoverToolButton: the style's panel
                                    // would make the plate taller than one
                                    // line and drop this hash out of step with
                                    // the author name beside it, so it draws
                                    // the same wash over its own flat face.
                                    ToolButton {
                                        id: hashCopy
                                        Layout.alignment: Qt.AlignRight
                                        text: detailsModel.sha8
                                        leftPadding: Theme.spaceXs
                                        rightPadding: Theme.spaceXs
                                        topPadding: 0
                                        bottomPadding: 0
                                        readonly property bool lit: hovered || visualFocus
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Copy full hash")
                                        onClicked: root.copyText(detailsModel.shaHex)
                                        background: Rectangle {
                                            radius: Theme.radiusSm
                                            color: hashCopy.down ? Theme.bgPressed
                                                 : hashCopy.lit ? Theme.bgHover
                                                 : "transparent"
                                            MouseArea {
                                                anchors.fill: parent
                                                acceptedButtons: Qt.NoButton
                                                cursorShape: Qt.PointingHandCursor
                                            }
                                            // Drawn here rather than as the
                                            // label's font underline so the
                                            // rule runs under the icon too —
                                            // the hash and the icon are one
                                            // target, so they get one line.
                                            Rectangle {
                                                visible: hashCopy.lit
                                                color: Theme.textPrimary
                                                height: Theme.borderWidth
                                                anchors.left: parent.left
                                                anchors.right: parent.right
                                                anchors.bottom: parent.bottom
                                                anchors.leftMargin: hashCopy.leftPadding
                                                anchors.rightMargin: hashCopy.rightPadding
                                            }
                                        }
                                        contentItem: RowLayout {
                                            spacing: Theme.spaceXs
                                            Label {
                                                text: hashCopy.text
                                                font.family: Theme.monoFamily
                                                font.pixelSize: Theme.fontMd
                                                color: Theme.textPrimary
                                                Layout.alignment: Qt.AlignVCenter
                                            }
                                            NavIcon {
                                                kind: "copyicon"
                                                tint: hashCopy.lit ? Theme.textPrimary
                                                                   : Theme.textSecondary
                                                Layout.alignment: Qt.AlignVCenter
                                            }
                                        }
                                    }
                                    Label {
                                        id: parentLink
                                        visible: detailsModel.parentHex !== ""
                                        Layout.alignment: Qt.AlignRight
                                        text: "← " + detailsModel.parentHex.substring(0, 8)
                                        font.family: Theme.monoFamily
                                        color: Theme.textLink
                                        font.pixelSize: Theme.fontSm
                                        font.underline: parentHover.containsMouse
                                        ToolTip.visible: parentHover.containsMouse
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Go to parent commit")
                                        MouseArea {
                                            id: parentHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: page.jumpToRef(detailsModel.parentHex)
                                        }
                                    }
                                }
                            }
                        }
                        // CHANGES header with the tree ⇄ path view toggle.
                        Rectangle {
                            visible: !page.wipShown && detailsModel.shaHex !== ""
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: qsTr("CHANGES (%1)").arg(detailsModel.fileTotal)
                                    font.pixelSize: Theme.fontSm
                                    font.weight: Font.DemiBold
                                    color: Theme.textSecondary
                                }
                                Item { Layout.fillWidth: true }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Tree view")
                                    onClicked: detailsModel.setTreeView(true)
                                    contentItem: NavIcon {
                                        kind: "hier"
                                        tint: detailsModel.treeView ? Theme.accent
                                                                    : Theme.textMuted
                                    }
                                }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Paths view")
                                    onClicked: detailsModel.setTreeView(false)
                                    contentItem: NavIcon {
                                        kind: "list"
                                        tint: detailsModel.treeView ? Theme.textMuted
                                                                    : Theme.accent
                                    }
                                }
                            }
                        }
                        ListView {
                            id: fileList
                            visible: !page.wipShown
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: detailsModel
                            reuseItems: true
                            ScrollBar.vertical: ScrollBar {}
                            delegate: FileRowDelegate {
                                listWidth: fileList.width
                                onActivated: (bucket, path, origPath) =>
                                    page.toggleDiff("commit", path, origPath)
                                onFolderToggled: key => detailsModel.toggleFolder(key)
                            }
                        }
                    }
                }
            }
        }

        function jumpToRef(oidHex) {
            const row = graphModel.rowOf(oidHex)
            if (row >= 0) {
                graphList.currentIndex = row
                graphList.positionViewAtIndex(row, ListView.Center)
            }
            // Details resolve even outside the window.
            page.wipShown = false
            page.selectedOid = oidHex
            page.selectedStashRef = graphModel.stashRefOf(oidHex)
            detailsModel.request(oidHex)
            page.closeDiff()
        }
    }

    // ======================================================================
    // Commit-graph row: [branch/tag chips][lanes + identicon node][subject]
    // — fixed-width label and graph columns keep subjects aligned.
    // ======================================================================
    component GraphRowDelegate: Item {
        id: rowItem
        required property int index
        required property string oid_hex
        required property string author
        required property double atime
        required property string subject
        required property int node_lane
        required property int node_color
        required property int avatar
        required property string geometry
        required property string labels
        required property string stash_ref

        width: ListView.view.width
        height: Theme.rowHeight

        readonly property bool selected: ListView.isCurrentItem
        // The all-zero id marks the synthetic uncommitted-changes row.
        readonly property bool isWip: oid_hex !== "" && !/[^0]/.test(oid_hex)
        // Chip records are separated by U+001F (see encode.rs). Branch-like
        // records (HEAD / local / remote) and tags get separate chips.
        readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))
        readonly property var branchRecords: labelRecords.filter(r => r[0] !== "T")
        readonly property var tagRecords: labelRecords.filter(r => r[0] === "T")

        Rectangle {
            anchors.fill: parent
            color: Theme.bgSelected
            visible: rowItem.selected
        }
        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: rowMouse.containsMouse && !rowItem.selected
        }

        onGeometryChanged: laneCanvas.requestPaint()
        onNode_laneChanged: laneCanvas.requestPaint()
        onAvatarChanged: laneCanvas.requestPaint()

        // Column widths come from the ListView (page scope).
        readonly property real labelsW: ListView.view ? ListView.view.labelWidth : root.labelColW

        RowLayout {
            anchors.fill: parent
            spacing: 0

            // Branch / tag chips, right-aligned against the graph.
            Item {
                Layout.preferredWidth: rowItem.labelsW
                Layout.fillHeight: true
                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.spaceXs
                    RefChip {
                        records: rowItem.branchRecords
                        tagStyle: false
                        maxWidth: rowItem.tagRecords.length > 0
                                  ? (rowItem.labelsW - Theme.spaceSm) / 2
                                  : rowItem.labelsW - Theme.spaceSm
                    }
                    RefChip {
                        records: rowItem.tagRecords
                        tagStyle: true
                        maxWidth: rowItem.branchRecords.length > 0
                                  ? (rowItem.labelsW - Theme.spaceSm) / 2
                                  : rowItem.labelsW - Theme.spaceSm
                    }
                }
                MouseArea {
                    id: labelHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
                ToolTip.visible: labelHover.containsMouse && rowItem.labelRecords.length > 0
                ToolTip.delay: 300
                ToolTip.text: {
                    let lines = []
                    for (let i = 0; i < rowItem.labelRecords.length; i++) {
                        const r = rowItem.labelRecords[i]
                        const icon = r[0] === "T" ? "⚑" : r[0] === "R" ? "☁"
                                   : r[0] === "H" ? "HEAD" : "⎇"
                        // Aggregated records keep their PR mark visible here.
                        const pr = r.length > 3 && r[3] === "1" ? qsTr(" · PR") : ""
                        lines.push(icon + " " + r.substring(4) + pr)
                    }
                    return lines.join("\n")
                }
            }

            // Lanes viewport: the full-width canvas slides behind a clip
            // when the graph column scrolls horizontally.
            Item {
                Layout.preferredWidth: rowItem.ListView.view
                                       ? rowItem.ListView.view.graphColWidth : 120
                Layout.fillHeight: true
                clip: true
                Canvas {
                    id: laneCanvas
                    x: rowItem.ListView.view ? -rowItem.ListView.view.graphXOffset : 0
                    width: rowItem.ListView.view
                           ? rowItem.ListView.view.graphFullWidth : 120
                    height: parent.height
                    onPaint: {
                    const ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.lineWidth = root.laneStroke
                    const laneCount = Theme.graphLane.length
                    const cx = function (l) { return root.laneInset + l * root.laneW + root.laneW / 2 }
                    const midY = height / 2
                    const nodeX = cx(rowItem.node_lane)
                    // decode precomputed draw tokens: t/i/o + lane + color
                    // (uppercase = dashed WIP edge)
                    if (rowItem.geometry !== "") {
                        const toks = rowItem.geometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const t = toks[n]
                            const k = t[0].toLowerCase()
                            const dot = t.indexOf(".")
                            const lane = parseInt(t.substring(1, dot))
                            const x = cx(lane)
                            ctx.strokeStyle = Theme.graphLane[parseInt(t.substring(dot + 1)) % laneCount]
                            ctx.setLineDash(t[0] === k ? [] : root.laneDash)
                            ctx.beginPath()
                            if (k === "t") {
                                ctx.moveTo(x, 0)
                                ctx.lineTo(x, height)
                            } else if (k === "i") {
                                ctx.moveTo(x, 0)
                                ctx.bezierCurveTo(x, midY * 0.66, nodeX, midY * 0.34, nodeX, midY)
                            } else {
                                ctx.moveTo(nodeX, midY)
                                ctx.bezierCurveTo(nodeX, height - midY * 0.34,
                                                  x, height - midY * 0.66, x, height)
                            }
                            ctx.stroke()
                        }
                        ctx.setLineDash([])
                    }
                    // The WIP row has no commit and no author: a dashed,
                    // empty node instead of the identicon.
                    const r = root.nodeIcon / 2
                    if (rowItem.isWip) {
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = root.laneStroke
                        ctx.setLineDash(root.laneDash)
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.stroke()
                        ctx.setLineDash([])
                        return
                    }
                    // Stash rows draw the archive-box glyph instead of the
                    // author identicon.
                    if (rowItem.stash_ref !== "") {
                        ctx.fillStyle = Theme.bgElevated
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.fill()
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = root.iconStroke
                        const bw = r * 1.2
                        ctx.strokeRect(nodeX - bw / 2, midY - bw / 2, bw, bw * 0.36)
                        ctx.strokeRect(nodeX - bw * 0.4, midY - bw * 0.1, bw * 0.8, bw * 0.58)
                        // Dashed ring like the WIP node: not part of the
                        // committed history proper.
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = root.laneStroke
                        ctx.setLineDash(root.laneDash)
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.stroke()
                        ctx.setLineDash([])
                        return
                    }
                    // The commit node is the author's identicon (5x5,
                    // mirrored; local substitute for network avatars). The
                    // pattern uses only the inner part of the circle so the
                    // clip cuts less of it.
                    ctx.save()
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.clip()
                    ctx.fillStyle = Theme.bgElevated
                    ctx.fillRect(nodeX - r, midY - r, 2 * r, 2 * r)
                    ctx.fillStyle = Theme.graphLane[(rowItem.avatar >> 15) & 0x7]
                    const inner = 2 * r * root.identiconFill
                    const cell = inner / 5
                    const ox = nodeX - inner / 2
                    const oy = midY - inner / 2
                    for (let row = 0; row < 5; row++) {
                        for (let col = 0; col < 3; col++) {
                            if ((rowItem.avatar >> (row * 3 + col)) & 1) {
                                ctx.fillRect(ox + col * cell, oy + row * cell,
                                             cell + 0.5, cell + 0.5)
                                if (col < 2)
                                    ctx.fillRect(ox + (4 - col) * cell, oy + row * cell,
                                                 cell + 0.5, cell + 0.5)
                            }
                        }
                    }
                    ctx.restore()
                    ctx.strokeStyle = Theme.borderStrong
                    ctx.lineWidth = Theme.borderWidth
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.stroke()
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.spaceXs
                // Short colored tick before the message: separates rows
                // visually (deliberately not a continuous line) and echoes
                // the commit's chain color.
                Rectangle {
                    Layout.leftMargin: Theme.spaceSm
                    implicitWidth: 2 * Theme.borderWidth
                    implicitHeight: Theme.iconMd
                    radius: Theme.borderWidth
                    color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
                }
                Label {
                    Layout.fillWidth: true
                    text: rowItem.isWip
                          ? qsTr("Uncommitted changes (%1)")
                            .arg(rowItem.ListView.view ? rowItem.ListView.view.wipCount : 0)
                          : rowItem.subject
                    elide: Text.ElideRight
                    font.pixelSize: Theme.fontMd
                    color: rowItem.isWip ? Theme.textSecondary : Theme.textPrimary
                    rightPadding: Theme.spaceSm
                }
            }
        }

        MouseArea {
            id: rowMouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: mouse => {
                rowItem.ListView.view.currentIndex = rowItem.index
                rowItem.ListView.view.rowSelected(rowItem.oid_hex)
                // The synthetic WIP row is not a commit, so nothing in the
                // commit menu applies to it.
                if (mouse.button === Qt.RightButton && !rowItem.isWip)
                    rowItem.ListView.view.rowMenuRequested(rowItem.oid_hex)
            }
        }
        // Hover details: what the row no longer shows as columns.
        ToolTip.visible: rowMouse.containsMouse
        ToolTip.delay: 700
        ToolTip.text: rowItem.isWip
                      ? qsTr("Working-tree changes — not committed yet")
                      : rowItem.author + "\n"
                        + Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm") + "\n"
                        + rowItem.oid_hex.substring(0, 8)
    }

    // One aggregated chip: primary name + "+N". No icons — the kind reads
    // through color alone, matching the sidebar header tints (local =
    // accent, remote = light blue, detached HEAD = red, tag = amber);
    // tags are additionally filled while branches stay outlined.
    component RefChip: Rectangle {
        id: chip
        property var records: []
        property bool tagStyle: false
        property real maxWidth: 140

        visible: records.length > 0
        height: Theme.fontSmLine
        width: Math.min(chipContent.implicitWidth + 2 * Theme.spaceXs, maxWidth)
        radius: Theme.radiusSm
        clip: true

        readonly property string rec: records.length > 0 ? records[0] : "L000"
        readonly property string recKind: rec[0]
        readonly property bool recHead: rec[1] === "1"
        readonly property bool recPr: rec.length > 3 && rec[3] === "1"
        readonly property color chipColor: tagStyle ? Theme.warning
                                          : recKind === "R" ? Theme.textSecondary
                                          : recKind === "H" ? Theme.danger
                                          : Theme.accent

        color: tagStyle ? Theme.bgElevated : "transparent"
        border.color: chipColor
        border.width: Theme.borderWidth

        Row {
            id: chipContent
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: chip.rec.substring(4)
                color: chip.chipColor
                font.pixelSize: Theme.fontSm
                font.weight: chip.recHead ? Font.DemiBold : Font.Normal
                elide: Text.ElideRight
                width: Math.min(implicitWidth,
                                chip.maxWidth - 2 * Theme.spaceXs
                                - (chip.records.length > 1 ? Theme.spaceLg : 0)
                                - (chip.recPr ? Theme.iconSm + Theme.spaceXs : 0))
            }
            Label {
                visible: chip.records.length > 1
                text: "+" + (chip.records.length - 1)
                color: chip.chipColor
                font.pixelSize: Theme.fontSm
            }
            // PR badge: reserved width above, so it survives any elision.
            NavIcon {
                visible: chip.recPr
                anchors.verticalCenter: parent.verticalCenter
                kind: "pr"
                tint: Theme.success
                width: Theme.iconSm
                height: Theme.iconSm
            }
        }
    }

    // ======================================================================
    // Sidebar building blocks
    // ======================================================================
    component NavHeader: Rectangle {
        id: header
        property string caption
        property string iconKind: "branch"
        property color iconTint: Theme.textSecondary
        property int count: 0
        property bool expanded: true
        property bool showTagToggle: false
        property bool tagsShown: true
        signal toggled()
        signal tagsToggled(bool shown)

        Layout.fillWidth: true
        implicitHeight: Theme.rowHeight
        color: Theme.bgElevated
        // A HoverHandler rather than the MouseArea's hover: it also fires
        // over the tag toggle, so the row highlight covers the header's
        // full clickable surface.
        HoverHandler {
            id: headerHover
        }
        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: headerHover.hovered
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: header.expanded ? "▾" : "▸"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            NavIcon {
                kind: header.iconKind
                tint: header.iconTint
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
            }
            Label {
                text: header.caption
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
            Label {
                text: "(" + header.count + ")"
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
            Item { Layout.fillWidth: true }
            HoverToolButton {
                visible: header.showTagToggle
                checkable: true
                checked: header.tagsShown
                text: "⚑"
                opacity: checked ? 1.0 : 0.35
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: header.tagsShown ? qsTr("Hide tags in the graph")
                                               : qsTr("Show tags in the graph")
                onToggled: header.tagsToggled(checked)
            }
        }
        MouseArea {
            anchors.fill: parent
            // Leave the toggle button clickable.
            anchors.rightMargin: header.showTagToggle ? Theme.spaceXl : 0
            onClicked: header.toggled()
        }
    }

    component NavList: ListView {
        id: navList
        property var sectionModel
        property bool expanded: true
        property string kindHint: "branch"
        // The stretching section absorbs the sidebar's leftover height;
        // the others stay content-sized.
        property bool stretch: false
        // "↑a ↓b" of the current branch (branches section only).
        property string headTrack: ""
        signal refActivated(string oidHex)
        signal fileActivated(string bucket, string path, string origPath)
        signal refMenuRequested(string name, string oidHex)

        visible: expanded
        Layout.fillWidth: true
        Layout.fillHeight: expanded
        Layout.maximumHeight: !expanded ? 0
                              : stretch ? Number.POSITIVE_INFINITY
                              : count * Theme.rowHeight + Theme.spaceXs
        clip: true
        model: sectionModel
        reuseItems: true
        ScrollBar.vertical: ScrollBar {}
        delegate: NavItemDelegate {
            listWidth: navList.width
            kindHint: navList.kindHint
            headTrack: navList.headTrack
            onRefClicked: oidHex => navList.refActivated(oidHex)
            onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
            onFolderClicked: key => navList.sectionModel.toggleFolder(key)
            onRefMenuRequested: (name, oidHex) => navList.refMenuRequested(name, oidHex)
        }
    }

    component NavItemDelegate: Item {
        id: navRow
        required property int index
        required property string name
        required property string full
        required property string oid_hex
        required property string change
        required property string bucket
        required property string orig_path
        required property bool is_head
        required property bool has_remote
        required property bool has_pr
        required property int depth
        required property bool folder
        required property bool collapsed
        property string kindHint: "branch"
        property string headTrack: ""
        property real listWidth: 200
        // Shows the hover stage/unstage affordance (WIP view).
        property bool showStage: false

        signal refClicked(string oidHex)
        signal fileClicked(string bucket, string path, string origPath)
        signal folderClicked(string key)
        signal stageClicked(string bucket, string path)
        /// Right-click on a ref row; the page owns the menu because
        /// delegates are recycled out from under an open popup.
        signal refMenuRequested(string name, string oidHex)

        width: listWidth
        height: Theme.rowHeight

        // The current branch stays highlighted inside the list (it is
        // also pinned under the section header).
        Rectangle {
            anchors.fill: parent
            color: Theme.accentMuted
            visible: navRow.is_head && !navRow.folder
        }
        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: itemMouse.containsMouse
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceMd + navRow.depth * Theme.spaceMd
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceXs
            // Fixed-width slot so the arrow column lines up with the
            // pinned-branch check icon.
            Label {
                visible: navRow.folder
                text: navRow.collapsed ? "▸" : "▾"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                Layout.preferredWidth: Theme.iconSm + 2
                horizontalAlignment: Text.AlignHCenter
            }
            ChangeIcon {
                visible: !navRow.folder && navRow.kindHint === "wt"
                change: navRow.change
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
                ToolTip.visible: wtHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: {
                    const c = navRow.change.length > 0 ? navRow.change[0] : ""
                    const what = navRow.change.length === 2 ? qsTr("Conflicted")
                               : c === "M" ? qsTr("Modified")
                               : c === "A" ? qsTr("Added")
                               : c === "D" ? qsTr("Deleted")
                               : c === "R" ? qsTr("Renamed")
                               : c === "C" ? qsTr("Copied")
                               : c === "T" ? qsTr("Type changed")
                               : c === "?" ? qsTr("Untracked") : navRow.change
                    const where = navRow.bucket === "staged" ? qsTr("staged")
                                : navRow.bucket === "unstaged" ? qsTr("unstaged")
                                : navRow.bucket === "untracked" ? qsTr("untracked")
                                : qsTr("conflict")
                    return what + " · " + where
                }
                MouseArea {
                    id: wtHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
            Label {
                Layout.fillWidth: true
                text: navRow.name
                elide: Text.ElideMiddle
                font.weight: navRow.is_head ? Font.DemiBold : Font.Normal
                color: navRow.folder ? Theme.textSecondary
                       : navRow.is_head ? Theme.textLink : Theme.textPrimary
                font.pixelSize: Theme.fontMd
            }
            // Worktree rows: checked-out branch on the right.
            Label {
                visible: !navRow.folder && navRow.kindHint === "worktree"
                text: navRow.bucket !== "" ? navRow.bucket : qsTr("detached")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                elide: Text.ElideMiddle
                Layout.maximumWidth: navRow.listWidth / 2
            }
            // Current branch's ahead/behind, left of the state icon.
            Label {
                visible: !navRow.folder && navRow.kindHint === "branch"
                         && navRow.is_head && navRow.headTrack !== ""
                text: navRow.headTrack
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            // Branch remote state: nothing = local only, remote icon =
            // has a remote, PR icon = has a PR (real data in Phase 4;
            // PG_FAKE_PR previews the look). Remote-branch and worktree
            // rows show the PR state too.
            NavIcon {
                visible: !navRow.folder
                         && ((navRow.kindHint === "branch"
                              && (navRow.has_remote || navRow.has_pr))
                             || ((navRow.kindHint === "remote"
                                  || navRow.kindHint === "worktree")
                                 && navRow.has_pr))
                kind: navRow.has_pr ? "pr" : "remote"
                tint: navRow.has_pr ? Theme.success : Theme.textSecondary
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
                ToolTip.visible: remoteHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: navRow.has_pr ? qsTr("Has an open pull request")
                                            : qsTr("Has a remote branch")
                MouseArea {
                    id: remoteHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
        }
        MouseArea {
            id: itemMouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: mouse => {
                if (mouse.button === Qt.RightButton) {
                    // Only branch-like rows have operations behind them.
                    if (!navRow.folder && navRow.oid_hex !== ""
                            && (navRow.kindHint === "branch"
                                || navRow.kindHint === "remote"))
                        navRow.refMenuRequested(
                            navRow.full !== "" ? navRow.full : navRow.name,
                            navRow.oid_hex)
                    return
                }
                if (navRow.folder)
                    navRow.folderClicked(navRow.full)
                else if (navRow.kindHint === "wt")
                    navRow.fileClicked(navRow.bucket,
                                       navRow.full !== "" ? navRow.full : navRow.name,
                                       navRow.orig_path)
                else if (navRow.kindHint === "worktree")
                    navRow.fileClicked("worktree", navRow.full, "")
                else if (navRow.oid_hex !== "")
                    navRow.refClicked(navRow.oid_hex)
            }
        }
        // Hover stage/unstage affordance.
        HoverToolButton {
            visible: navRow.showStage && !navRow.folder
                     && (itemMouse.containsMouse || hovered)
            anchors.right: parent.right
            anchors.rightMargin: Theme.spaceXs
            anchors.verticalCenter: parent.verticalCenter
            padding: 0
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            ToolTip.visible: hovered
            ToolTip.delay: 300
            ToolTip.text: navRow.bucket === "staged" ? qsTr("Unstage file")
                                                     : qsTr("Stage file")
            onClicked: navRow.stageClicked(
                navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name)
            contentItem: NavIcon {
                kind: navRow.bucket === "staged" ? "minus" : "plus"
                tint: navRow.bucket === "staged" ? Theme.diffRemovedFg
                                                 : Theme.diffAddedFg
            }
        }
        // Nested leaves show only their last segment; hover reveals the
        // full name.
        ToolTip.visible: itemMouse.containsMouse && !navRow.folder
                         && navRow.full !== "" && navRow.full !== navRow.name
        ToolTip.delay: 700
        ToolTip.text: navRow.full
    }

    // ======================================================================
    // A changed-file row (commit file list): tree view shows indented,
    // collapsible directory rows with leaf names; path view shows flat
    // full paths.
    // ======================================================================
    component FileRowDelegate: Item {
        id: fileRow
        required property var model
        property real listWidth: 200

        readonly property string changeText: model.change ?? ""
        readonly property string nameText: model.name ?? ""
        readonly property string pathText: model.path ?? ""
        readonly property string origPathText: model.orig_path ?? ""
        readonly property bool isFolder: (model.folder ?? false) === true

        signal activated(string bucket, string path, string origPath)
        signal folderToggled(string key)

        width: listWidth
        height: Theme.rowHeight

        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: fileMouse.containsMouse
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm + (fileRow.model.depth ?? 0) * Theme.spaceMd
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceXs
            Label {
                visible: fileRow.isFolder
                text: (fileRow.model.collapsed ?? false) ? "▸" : "▾"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            ChangeIcon {
                visible: !fileRow.isFolder
                change: fileRow.changeText
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
                ToolTip.visible: changeHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: {
                    const c = fileRow.changeText.length > 0 ? fileRow.changeText[0] : ""
                    return c === "M" ? qsTr("Modified")
                         : c === "A" ? qsTr("Added")
                         : c === "D" ? qsTr("Deleted")
                         : c === "R" ? qsTr("Renamed")
                         : c === "C" ? qsTr("Copied")
                         : c === "T" ? qsTr("Type changed") : fileRow.changeText
                }
                MouseArea {
                    id: changeHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
            Label {
                Layout.fillWidth: true
                text: fileRow.isFolder || fileRow.origPathText === ""
                      ? fileRow.nameText
                      : qsTr("%1 → %2").arg(fileRow.origPathText).arg(fileRow.nameText)
                elide: Text.ElideMiddle
                font.pixelSize: Theme.fontMd
                color: fileRow.isFolder ? Theme.textSecondary : Theme.textPrimary
            }
        }
        MouseArea {
            id: fileMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: {
                if (fileRow.isFolder)
                    fileRow.folderToggled(fileRow.pathText)
                else
                    fileRow.activated("", fileRow.pathText, fileRow.origPathText)
            }
        }
        // Tree leaves show only their file name; hover reveals the path.
        ToolTip.visible: fileMouse.containsMouse && !fileRow.isFolder
                         && fileRow.nameText !== fileRow.pathText
        ToolTip.delay: 700
        ToolTip.text: fileRow.pathText
    }

    // Hand-drawn 16px-grid icons in the common git-client style (branch
    // fork, cloud remote, price-tag, archive box, tree, clock). Scaled by
    // the item size; single stroke color.
    component NavIcon: Canvas {
        id: icon
        property string kind: "branch"
        property color tint: Theme.textSecondary
        width: Theme.iconMd
        height: Theme.iconMd
        onKindChanged: requestPaint()
        onTintChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const s = width / 16
            ctx.clearRect(0, 0, width, height)
            ctx.strokeStyle = icon.tint
            ctx.fillStyle = icon.tint
            ctx.lineWidth = root.iconStroke
            ctx.lineCap = "round"
            if (icon.kind === "branch") {
                ctx.beginPath()
                ctx.moveTo(5 * s, 5 * s)
                ctx.lineTo(5 * s, 11 * s)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(11 * s, 7 * s)
                ctx.bezierCurveTo(11 * s, 9.5 * s, 8 * s, 9.5 * s, 5.8 * s, 10.2 * s)
                ctx.stroke()
                for (const c of [[5, 3.5], [5, 12.5], [11, 5]]) {
                    ctx.beginPath()
                    ctx.arc(c[0] * s, c[1] * s, 1.8 * s, 0, 2 * Math.PI)
                    ctx.stroke()
                }
            } else if (icon.kind === "remote") {
                ctx.beginPath()
                ctx.arc(6 * s, 9 * s, 3 * s, Math.PI * 0.5, Math.PI * 1.5)
                ctx.arc(8.5 * s, 6.8 * s, 3.2 * s, Math.PI * 0.95, Math.PI * 0.02, false)
                ctx.arc(11 * s, 9.4 * s, 2.6 * s, Math.PI * 1.55, Math.PI * 0.5)
                ctx.closePath()
                ctx.stroke()
            } else if (icon.kind === "tag") {
                ctx.save()
                ctx.translate(8 * s, 8.5 * s)
                ctx.rotate(Math.PI / 4)
                ctx.strokeRect(-3.6 * s, -3.6 * s, 7.2 * s, 7.2 * s)
                ctx.beginPath()
                ctx.arc(-1.4 * s, -1.4 * s, 1 * s, 0, 2 * Math.PI)
                ctx.fill()
                ctx.restore()
            } else if (icon.kind === "stash") {
                ctx.strokeRect(3 * s, 4 * s, 10 * s, 3 * s)
                ctx.strokeRect(4 * s, 7 * s, 8 * s, 6 * s)
                ctx.beginPath()
                ctx.moveTo(6.5 * s, 9.5 * s)
                ctx.lineTo(9.5 * s, 9.5 * s)
                ctx.stroke()
            } else if (icon.kind === "tree") {
                ctx.beginPath()
                ctx.arc(8 * s, 6.5 * s, 4 * s, 0, 2 * Math.PI)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(8 * s, 10.5 * s)
                ctx.lineTo(8 * s, 14 * s)
                ctx.stroke()
            } else if (icon.kind === "pen") {
                ctx.save()
                ctx.translate(8 * s, 8 * s)
                ctx.rotate(Math.PI / 4)
                ctx.strokeRect(-1.4 * s, -6 * s, 2.8 * s, 8.5 * s)
                ctx.beginPath()
                ctx.moveTo(-1.4 * s, 2.5 * s)
                ctx.lineTo(0, 5.5 * s)
                ctx.lineTo(1.4 * s, 2.5 * s)
                ctx.closePath()
                ctx.fill()
                ctx.restore()
            } else if (icon.kind === "plus") {
                ctx.beginPath()
                ctx.moveTo(8 * s, 3.5 * s)
                ctx.lineTo(8 * s, 12.5 * s)
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(12.5 * s, 8 * s)
                ctx.stroke()
            } else if (icon.kind === "minus") {
                ctx.beginPath()
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(12.5 * s, 8 * s)
                ctx.stroke()
            } else if (icon.kind === "arrow") {
                ctx.beginPath()
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(11.5 * s, 8 * s)
                ctx.moveTo(11.5 * s, 8 * s)
                ctx.lineTo(8.8 * s, 5.2 * s)
                ctx.moveTo(11.5 * s, 8 * s)
                ctx.lineTo(8.8 * s, 10.8 * s)
                ctx.stroke()
            } else if (icon.kind === "fetch" || icon.kind === "push") {
                // Network pair, drawn as one shape mirrored: the base line
                // is this repository, and the arrow either lands on it
                // (fetch) or leaves it (push).
                const down = icon.kind === "fetch"
                const tip = down ? 10 : 2.5
                const tail = down ? 2.5 : 10
                const barb = down ? tip - 3.2 : tip + 3.2
                ctx.beginPath()
                ctx.moveTo(8 * s, tail * s)
                ctx.lineTo(8 * s, tip * s)
                ctx.moveTo(4.8 * s, barb * s)
                ctx.lineTo(8 * s, tip * s)
                ctx.lineTo(11.2 * s, barb * s)
                ctx.moveTo(3.5 * s, 13 * s)
                ctx.lineTo(12.5 * s, 13 * s)
                ctx.stroke()
            } else if (icon.kind === "copyicon") {
                ctx.strokeRect(5.5 * s, 3.5 * s, 7 * s, 7 * s)
                ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
            } else if (icon.kind === "bang") {
                ctx.beginPath()
                ctx.moveTo(8 * s, 3.5 * s)
                ctx.lineTo(8 * s, 10 * s)
                ctx.stroke()
                ctx.beginPath()
                ctx.arc(8 * s, 12.8 * s, 1 * s, 0, 2 * Math.PI)
                ctx.fill()
            } else if (icon.kind === "hier") {
                ctx.strokeRect(3 * s, 3 * s, 3 * s, 3 * s)
                ctx.beginPath()
                ctx.moveTo(4.5 * s, 6 * s)
                ctx.lineTo(4.5 * s, 12 * s)
                ctx.moveTo(4.5 * s, 8 * s)
                ctx.lineTo(10 * s, 8 * s)
                ctx.moveTo(4.5 * s, 12 * s)
                ctx.lineTo(10 * s, 12 * s)
                ctx.stroke()
                ctx.strokeRect(10 * s, 6.5 * s, 3 * s, 3 * s)
                ctx.strokeRect(10 * s, 10.5 * s, 3 * s, 3 * s)
            } else if (icon.kind === "list") {
                ctx.beginPath()
                for (const ly of [4.5, 8, 11.5]) {
                    ctx.moveTo(5.5 * s, ly * s)
                    ctx.lineTo(13 * s, ly * s)
                }
                ctx.stroke()
                for (const ly of [4.5, 8, 11.5]) {
                    ctx.beginPath()
                    ctx.arc(3.4 * s, ly * s, 0.9 * s, 0, 2 * Math.PI)
                    ctx.fill()
                }
            } else if (icon.kind === "pr") {
                // GitHub-style pull request: left commit line, right elbow
                // arrow into the merge node.
                ctx.beginPath()
                ctx.moveTo(4.5 * s, 5.5 * s)
                ctx.lineTo(4.5 * s, 10.5 * s)
                ctx.stroke()
                for (const c of [[4.5, 3.8], [4.5, 12.2], [11.5, 12.2]]) {
                    ctx.beginPath()
                    ctx.arc(c[0] * s, c[1] * s, 1.7 * s, 0, 2 * Math.PI)
                    ctx.stroke()
                }
                ctx.beginPath()
                ctx.moveTo(7.4 * s, 3.8 * s)
                ctx.lineTo(9.8 * s, 3.8 * s)
                ctx.quadraticCurveTo(11.5 * s, 3.8 * s, 11.5 * s, 5.5 * s)
                ctx.lineTo(11.5 * s, 10.5 * s)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(8.8 * s, 2.5 * s)
                ctx.lineTo(7.2 * s, 3.8 * s)
                ctx.lineTo(8.8 * s, 5.1 * s)
                ctx.stroke()
            } else if (icon.kind === "check") {
                ctx.beginPath()
                ctx.moveTo(3.5 * s, 8.5 * s)
                ctx.lineTo(6.8 * s, 11.8 * s)
                ctx.lineTo(12.5 * s, 4.5 * s)
                ctx.stroke()
            } else if (icon.kind === "pin") {
                // Map pin: "you are here".
                ctx.beginPath()
                ctx.arc(8 * s, 6.2 * s, 3.4 * s, Math.PI * 0.75, Math.PI * 0.25)
                ctx.lineTo(8 * s, 13.5 * s)
                ctx.closePath()
                ctx.stroke()
                ctx.beginPath()
                ctx.arc(8 * s, 6.2 * s, 1.2 * s, 0, 2 * Math.PI)
                ctx.fill()
            } else if (icon.kind === "folder") {
                ctx.beginPath()
                ctx.moveTo(2.5 * s, 12.5 * s)
                ctx.lineTo(2.5 * s, 4.5 * s)
                ctx.lineTo(6.5 * s, 4.5 * s)
                ctx.lineTo(8 * s, 6 * s)
                ctx.lineTo(13.5 * s, 6 * s)
                ctx.lineTo(13.5 * s, 12.5 * s)
                ctx.closePath()
                ctx.stroke()
            } else if (icon.kind === "clock") {
                ctx.beginPath()
                ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(8 * s, 8 * s)
                ctx.lineTo(8 * s, 4.8 * s)
                ctx.moveTo(8 * s, 8 * s)
                ctx.lineTo(10.4 * s, 8 * s)
                ctx.stroke()
            }
        }
    }

    // Change-kind icon (pen = edit, + / − = add / delete, → = rename,
    // stacked squares = copy, ! = conflict). Edits are amber by request;
    // adds/deletes reuse the diff colors.
    component ChangeIcon: NavIcon {
        id: changeIcon
        property string change: ""
        readonly property string letter: change.length > 0 ? change[0] : ""
        readonly property bool conflict: change.length === 2
        kind: conflict ? "bang"
              : letter === "A" ? "plus"
              : letter === "D" ? "minus"
              : letter === "R" ? "arrow"
              : letter === "C" ? "copyicon"
              : letter === "?" ? "plus"
              : "pen"
        tint: conflict ? Theme.danger
              : letter === "A" ? Theme.diffAddedFg
              : letter === "D" ? Theme.diffRemovedFg
              : letter === "R" ? Theme.textLink
              : letter === "C" ? Theme.textSecondary
              : letter === "?" ? Theme.diffAddedFg
              : Theme.warning
    }

    // Fusion's built-in hover feedback is a few-percent tint that vanishes
    // on this dark palette (and a checked ToolButton shows its panel all
    // the time, hiding the hover entirely), so every clickable control
    // layers the theme's hover wash on top instead (デザイン規約: ホバーは
    // bgHover の重ね色). The wash lifts while pressed so Fusion's darker
    // pressed face stays visible.
    component HoverButton: Button {
        id: hoverButtonSelf
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusSm
            color: Theme.bgHover
            visible: hoverButtonSelf.enabled && hoverButtonSelf.hovered
                     && !hoverButtonSelf.down
        }
    }
    component HoverToolButton: ToolButton {
        id: hoverToolButtonSelf
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusSm
            color: Theme.bgHover
            visible: hoverToolButtonSelf.enabled && hoverToolButtonSelf.hovered
                     && !hoverToolButtonSelf.down
        }
    }

    // Toolbar action named twice over: an icon to find it by shape, the
    // word to be sure of it. Both halves dim together when disabled.
    component ActionButton: HoverToolButton {
        id: actionBtn
        property string kind: ""
        readonly property color fg: enabled ? Theme.textPrimary : Theme.textMuted
        contentItem: RowLayout {
            spacing: Theme.spaceXs
            NavIcon {
                kind: actionBtn.kind
                tint: actionBtn.fg
                Layout.alignment: Qt.AlignVCenter
            }
            Label {
                text: actionBtn.text
                color: actionBtn.fg
                font.pixelSize: Theme.fontMd
                Layout.alignment: Qt.AlignVCenter
            }
        }
    }

    // Author identicon (same packed code as the graph nodes).
    component IdentIcon: Canvas {
        id: ident
        property int code: 0
        width: Theme.iconLg
        height: Theme.iconLg
        onCodeChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const r = width / 2
            ctx.clearRect(0, 0, width, height)
            ctx.save()
            ctx.beginPath()
            ctx.arc(r, r, r, 0, 2 * Math.PI)
            ctx.clip()
            ctx.fillStyle = Theme.bgElevated
            ctx.fillRect(0, 0, width, height)
            ctx.fillStyle = Theme.graphLane[(ident.code >> 15) & 0x7]
            const inner = width * root.identiconFill
            const cell = inner / 5
            const o = (width - inner) / 2
            for (let row = 0; row < 5; row++) {
                for (let col = 0; col < 3; col++) {
                    if ((ident.code >> (row * 3 + col)) & 1) {
                        ctx.fillRect(o + col * cell, o + row * cell, cell + 0.5, cell + 0.5)
                        if (col < 2)
                            ctx.fillRect(o + (4 - col) * cell, o + row * cell, cell + 0.5, cell + 0.5)
                    }
                }
            }
            ctx.restore()
            ctx.strokeStyle = Theme.borderStrong
            ctx.lineWidth = Theme.borderWidth
            ctx.beginPath()
            ctx.arc(r, r, r - 0.5, 0, 2 * Math.PI)
            ctx.stroke()
        }
    }

    // Slim single-line input shared by the toolbar search and the sidebar
    // filter: thin frame, compact height.
    component SlimField: TextField {
        id: slim
        implicitHeight: Theme.iconLg
        font.pixelSize: Theme.fontMd
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceSm
        topPadding: 0
        bottomPadding: 0
        background: Rectangle {
            color: Theme.bgBase
            radius: Theme.radiusSm
            border.color: slim.activeFocus ? Theme.borderFocus : Theme.borderDefault
            border.width: Theme.borderWidth
        }
    }

    // The same frame at the height buttons and combo boxes use: for form
    // input rather than the toolbar's slim filters.
    component FormField: TextField {
        id: form
        implicitHeight: Theme.controlHeight
        font.pixelSize: Theme.fontMd
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceSm
        topPadding: 0
        bottomPadding: 0
        background: Rectangle {
            color: Theme.bgBase
            radius: Theme.radiusSm
            border.color: form.activeFocus ? Theme.borderFocus : Theme.borderDefault
            border.width: Theme.borderWidth
        }
    }

    component PaneHeader: Rectangle {
        property alias text: headerLabel.text
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        Label {
            id: headerLabel
            anchors.verticalCenter: parent.verticalCenter
            x: Theme.spaceSm
            font.pixelSize: Theme.fontSm
            font.weight: Font.DemiBold
            color: Theme.textSecondary
        }
    }

    // One side of the diff pane's image preview (Before / After). Absent
    // sides collapse (visible tracks sizeText), so an added image shows a
    // single full-width After cell and a deleted one a single Before.
    //
    // Scaling: 1:1 when the natural size fits, a fixed integer zoom for
    // small images (previewZoomFor — independent of the window), and
    // fit-to-frame shrinking as the final cap, so a tiny window never
    // overflows and never picks a different zoom, it only shrinks.
    component ImagePreviewCell: ColumnLayout {
        id: previewCell
        required property string label
        required property string url
        required property string sizeText
        visible: sizeText !== ""
        spacing: Theme.spaceXs
        // data: URLs carry the mime up front; SVG rasters scale smoothly,
        // pixel rasters must not.
        readonly property bool isVector: url.indexOf("data:image/svg") === 0
        Label {
            Layout.fillWidth: true
            text: previewCell.label
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
            elide: Text.ElideRight
        }
        Rectangle {
            id: previewFrame
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "transparent"
            border.color: Theme.borderSubtle
            border.width: Theme.borderWidth
            clip: true
            // Box the image may occupy.
            readonly property real innerW: width - 2 * Theme.spaceXs
            readonly property real innerH: height - 2 * Theme.spaceXs
            // Decoded size (0 until the image is ready).
            readonly property real naturalW: previewImage.implicitWidth
            readonly property real naturalH: previewImage.implicitHeight
            readonly property real fitScale:
                naturalW > 0 && naturalH > 0 && innerW > 0 && innerH > 0
                ? Math.min(innerW / naturalW, innerH / naturalH) : 1
            // Shrink freely; enlarge only in whole steps, never past the
            // frame and never more than the size-picked zoom.
            readonly property real displayScale: fitScale < 1
                ? fitScale
                : Math.max(1, Math.min(
                      root.previewZoomFor(Math.max(naturalW, naturalH)),
                      Math.floor(fitScale)))
            Image {
                id: previewImage
                anchors.centerIn: parent
                width: previewFrame.naturalW * previewFrame.displayScale
                height: previewFrame.naturalH * previewFrame.displayScale
                fillMode: Image.PreserveAspectFit
                source: previewCell.url
                asynchronous: true
                cache: false
                // No sourceSize: it does not cap decoding, it *rescales*
                // rasters to the given size (a 16px icon came back blurry
                // at screen width). Decode memory is already bounded by
                // the 16 MiB byte cap in platitude-core::preview.
                // Integer upscales stay crisp (pixel art); shrinking and
                // vector rasters smooth.
                smooth: previewFrame.displayScale < 1 || previewCell.isVector
                mipmap: true
                visible: status === Image.Ready
            }
            Label {
                anchors.centerIn: parent
                width: Math.min(implicitWidth, parent.width - 2 * Theme.spaceSm)
                visible: previewCell.url === ""
                         || previewImage.status === Image.Error
                text: previewCell.url === "" ? qsTr("Too large to preview")
                                             : qsTr("Preview unavailable")
                elide: Text.ElideRight
                color: Theme.textMuted
            }
        }
    }
}

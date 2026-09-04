//! platitude-gg entry point: tokio runtime + hub + QML application.

// Hide the console window of the GUI subsystem build on Windows (debug runs
// from a terminal still show logs on stderr).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod encode;
mod harness;
mod hub;
mod logsink;
mod models;
mod urlpath;
mod winframe;

use hub::Hub;
use models::{
    AppBackend, CloneModel, CommandsModel, DetailsModel, DiffModel, GitFacts, GraphModel,
    LineEndingsModel, NavSectionModel, RebasePlanModel, RepoConfigModel, RepoTab, TabsModel,
    WorkTreeModel,
};
use platitude_core::settings::{Build, Claim, Store};
use qtbridge::QApp;

#[expect(clippy::too_many_lines)]
fn main() {
    harness::start_clock();
    init_tracing();
    // Said once, before anything else can fail: a run whose window never
    // comes up, or whose stderr is all a verify-ui report keeps, still
    // names the tree it was built from (CLAUDE.md ビルド・テスト).
    let tree = models::build_tree();
    tracing::info!(
        tree = if tree.is_empty() { "-" } else { tree.as_str() },
        "build tree"
    );

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(error = %e, "failed to start the async runtime");
            std::process::exit(1);
        }
    };
    // Sole use of the two files, held for the length of the run. What the
    // taskbar's own launch entry starts lands here and goes no further:
    // two processes writing one `state.toml` overwrite each other's tabs
    // and window shape, last one out winning.
    let (store, held_elsewhere, _lock) = claim_store(Build {
        tree: &tree,
        debug: cfg!(debug_assertions),
        // Asked of the harness, not of the environment: a build without
        // one is never driven, so no `PG_*` variable left in somebody's
        // shell can take their settings away.
        driven: harness::knobs().automated,
    });
    Hub::install(runtime, store, held_elsewhere);

    let mut app = QApp::new();
    // The slug, not the product name `Platitude GG`: nothing shows this to a
    // person (the window title is QML's, and the one Qt-built dialog names
    // itself), while two machines read it verbatim — QStandardPaths joins it
    // into `~/.cache/<name>/`, and the xcb plugin makes it the WM_CLASS
    // class. A name with a space in it there buys nothing and costs a
    // desktop-file match.
    app.application_name("platitude-gg");
    // Embed the QML module (singletons + components + Main) as qrc
    // resources. Every file listed in ui/qmldir must be embedded here.
    qtbridge::include_bytes_qml!("ui/qmldir", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Theme.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Metrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Words.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Ink.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Motion.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ActionButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ActionButtonLabel.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ActionButtonSeat.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppCardFace.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppCheckBox.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppCombo.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppListView.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenuButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenuItem.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenuSeparator.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AskBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AuthorCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AutoScrollBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AvatarAssignRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AvatarButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ChangeIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ClipboardHelper.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandFetchButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandPushButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandRule.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandStashButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandStateCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandStateGroup.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandStateMetrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BandWidest.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CardText.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CoAuthorCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CoAuthorLine.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CodeChip.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ColumnDivider.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommitAuthorRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommitHoverCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommitMenuState.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommitRowMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/EolHoverCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandsPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandsTextSelect.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandsToggle.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DescriptionBox.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DetailsAuthorCards.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DetailsChangesBand.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DetailsMessageBlock.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DetailsPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/LineText.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SweepRoom.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SweepPad.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SweepBand.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/BinarySizeLine.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ConflictSideLegend.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CutName.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffCodeScroll.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffFileNotices.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffPaneHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffRowMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffRowWalk.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffScrollPlace.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffTextMetrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffTextSelect.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DialogActions.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DotMark.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FileRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FileRowMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FileRowWalk.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FindBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FocusRelease.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FoldBlock.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FormField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GitVersionCorner.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphColumnDividers.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphColumnMetrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphEmptyState.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphFind.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphHeadPin.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphLaneBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphLaneCell.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphLanePan.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphList.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphRowChips.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphRowWalk.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphTailFooter.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HarnessSeat.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HashPlate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HeadPinRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HeadTrack.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HelpText.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoldDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoldFill.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoldIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoverCardHost.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoverToolButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/InkCanvas.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentityDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentityFields.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentityGate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ImagePreviewCell.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/LabeledField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/LineEndingField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/MessageActionsRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/MessageEditor.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/MiddleAutoScroll.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NameCell.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavItemDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavList.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavNameBox.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavRail.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavRowBody.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavSections.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NoticeBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NoticeButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NoticeLine.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/OpExitCard.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/OpExitRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/OpenFailedDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/OpenFailedScreen.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PageLayout.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PaneHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PaneScrollBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PointerWatch.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PublishFlow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PublishForm.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/QuitWaitDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowQuitGate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefChip.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefListPopup.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefBranchMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefRowMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefTagMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RemoteRowMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefusalBadge.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RemoteDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ReclickGesture.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RebasePlanPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RebasePlanRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RebasePlanRunBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RepoPage.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RepoPageStack.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CloneDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CloseToolButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RowHoverHost.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SampleTimer.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SectionPeekPopup.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsAppPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsGitPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsGroup.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsRepoPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsSection.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SharedToolTip.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SidebarFilterRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SidebarPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SidebarRowGestures.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SignatureMark.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SlimField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SpinnerIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SplitBarWatch.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SplitHandleBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SplitRefusalOverlay.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/StartupGate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/StashActionsBand.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/StateBadge.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SummaryArea.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabCarry.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabItemDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabMetrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabPin.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabRun.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabStrip.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TabTitleFade.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TopBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TreeViewToggle.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/UpstreamFlow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/UpstreamForm.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WaitRing.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowBody.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowChrome.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowDialogSeat.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowShape.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WindowWaitRing.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipBucketPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipBucketHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipCommitBlock.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipTallyRow.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Main.qml", "qt/qml/platitude");
    embed_harness_qml();
    harness::install(&mut app);
    let code = app
        .register::<AppBackend>()
        .register::<GitFacts>()
        .register::<TabsModel>()
        .register::<CloneModel>()
        .register::<RepoConfigModel>()
        .register::<LineEndingsModel>()
        .register::<RepoTab>()
        .register::<GraphModel>()
        .register::<NavSectionModel>()
        .register::<WorkTreeModel>()
        .register::<DetailsModel>()
        .register::<DiffModel>()
        .register::<CommandsModel>()
        .register::<RebasePlanModel>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file("qrc:/qt/qml/platitude/ui/Main.qml")
        .run();

    Hub::shutdown();
    std::process::exit(code);
}

/// The store this run may use, the directory it was refused if it was, and
/// the lock to hold on to until the process ends.
///
/// Three ways it can go, and the third is the one worth spelling out:
///
/// * nobody else has the files — the run gets them, and the lock rides in
///   `main`'s frame so the kernel releases it however the process ends;
/// * somebody does — the run is handed an *empty* store, so a window that
///   is about to say "already running" cannot write a thing on its way out,
///   and the window says which directory it did not get;
/// * the lock could not be asked for at all (a redirected profile, a
///   network share, a filesystem that does not answer) — the run carries
///   on with the files. A lock nobody can take must never be the reason a
///   window will not open.
fn claim_store(build: Build) -> (Store, String, Option<platitude_core::settings::Lock>) {
    let store = Store::discover(build);
    match store.claim() {
        Claim::Ours(lock) => (store, String::new(), Some(lock)),
        Claim::Taken => {
            let held = store
                .lock_path()
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(|dir| dir.display().to_string())
                .unwrap_or_default();
            tracing::warn!(store = %held, "another platitude-gg is using these settings");
            (Store::ephemeral(), held, None)
        }
        Claim::Unknown(error) => {
            tracing::warn!(%error, "the settings could not be locked; carrying on");
            (store, String::new(), None)
        }
    }
}

/// stderr logging; level via `PG_LOG` (error/warn/info/debug/trace).
///
/// **Never coloured.** This stream is read by machines — `xtask perf`
/// takes the interaction and startup numbers out of it, `xtask verify-ui`
/// decides pass or fail on it — and the escapes go around the field name
/// and the `=`, so `first_chunk_ms=317` reaches a reader as
/// `first_chunk_ms\e[0m\e[2m=\e[0m317` and no substring search finds it.
/// The trap hides in a terminal, where the colouring is invisible: only
/// a pipe gets the escapes, so a run that looks clean under a shell
/// redirect still loses its numbers when xtask spawns it.
///
/// **A log line nobody can receive is not a reason to end the process.**
/// Which is the whole of why the stream is [`logsink`] rather than
/// `std::io::stderr`: the writer there answers `Ok` however the write
/// went, so the subscriber never reaches for the `eprintln!` that panics
/// on a stderr that has just failed.
///
/// `log_internal_errors` is off for the same hazard by a second road —
/// nothing can reach those `eprintln!`s through a writer that does not
/// fail, and this is what still stands between them and a window if the
/// writer above is ever put back to a bare stream.
fn init_tracing() {
    let level = match std::env::var("PG_LOG").as_deref() {
        Ok("error") => tracing::Level::ERROR,
        Ok("info") => tracing::Level::INFO,
        Ok("debug") => tracing::Level::DEBUG,
        Ok("trace") => tracing::Level::TRACE,
        _ => tracing::Level::WARN,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(logsink::sink)
        .log_internal_errors(false)
        .init();
}

/// Embeds the verification harness's QML module (`src/auto`, `platitude.auto`)
/// — the verb files, the drivers and the shot stand-ins. Nothing in a
/// shipped build, where the seats that would load them stay empty
/// (`ui/HarnessSeat.qml`).
#[cfg(feature = "automation")]
fn embed_harness_qml() {
    qtbridge::include_bytes_qml!("auto/AutoActCompletion.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActDetailsVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActDiffVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActFileRowVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActFindVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActGraphRowVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActHistoryVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActLandings.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActNavBoxVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActNavVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActPaneVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActPlanVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActPublishVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActRefVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActStepVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActSwitchVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActTipVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoActWipVerbs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/AutoShotDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/NavProbe.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/PageAutoStart.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/PageHarness.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/PagePerfDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/PageReports.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/TabProbe.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowAutoActDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowBadgeActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowBandActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowCensus.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowDialogActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowFrameActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowHarness.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowIdentityActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowPerfDriver.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowSettingsActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowShotMirrors.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowTabActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/WindowTabPinActs.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("auto/qmldir", "qt/qml/platitude");
}

#[cfg(not(feature = "automation"))]
fn embed_harness_qml() {}

//! platitude-gg entry point: tokio runtime + hub + QML application.

// Release only: debug runs keep the console for their stderr logs.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod encode;
mod harness;
mod hub;
mod logsink;
mod models;
mod ops;
mod qml_engine;
mod qrc;
mod urlpath;
mod winframe;

use hub::Hub;
use models::{
    AppBackend, CloneModel, CommandsModel, DetailsModel, DiffModel, GitFacts, GraphModel,
    LineEndingsModel, NavSectionModel, RebasePlanModel, RepoConfigModel, RepoTab, TabsModel,
    WorkTreeModel,
};
use platitude_core::settings::{Build, Claim, Store};
use qtbridge::QmlElement;
use qtbridge::qtbridge_type_lib::{QGuiApplication, QQmlApplicationEngine, QString};

#[expect(clippy::too_many_lines)]
fn main() {
    harness::start_clock();
    init_tracing();
    // Before anything that can wedge: the QML watchdog cannot fire once the
    // event loop stops turning or is left (`harness::deadline`).
    harness::watch_deadline();
    // Before any repository is opened, so the opening's own walk is already
    // a pass without the working tree's row (`harness::faults`).
    if harness::knobs().fault_hold_wip_row {
        harness::hold_the_working_tree_row();
    }
    // Before anything else can fail, so a run whose window never comes up
    // still names the tree it was built from.
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
    // Sole use of the settings files for the whole run: two processes
    // writing one `state.toml` overwrite each other's tabs and window shape.
    let (store, held_elsewhere, lock) = claim_store(Build {
        tree: &tree,
        debug: cfg!(debug_assertions),
        // From the harness: a build without one is never driven, so a stray
        // `PGG_*` in somebody's shell cannot take their settings away.
        driven: harness::knobs().automated,
    });
    Hub::install(runtime, store, held_elsewhere);

    // Held here rather than in qtbridge's `QApp`, which does not say when the
    // window's QML fails to load (`qml_engine`).
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    qml_engine::arm(engine.pin_mut());
    // The slug, not a display name: QStandardPaths joins it into
    // `~/.cache/<name>/` and xcb makes it the WM_CLASS class, where a space
    // costs a desktop-file match.
    app.pin_mut()
        .set_application_name(&QString::from("platitude-gg"));
    // Every file listed in ui/qmldir must be embedded here.
    qrc::embed!("ui/qmldir");
    qrc::embed!("ui/Theme.qml");
    qrc::embed!("ui/Metrics.qml");
    qrc::embed!("ui/Words.qml");
    qrc::embed!("ui/Ink.qml");
    qrc::embed!("ui/Motion.qml");
    qrc::embed!("ui/Hand.qml");
    qrc::embed!("ui/MiddleHand.qml");
    qrc::embed!("ui/KeyMenu.qml");
    qrc::embed!("ui/ActionButton.qml");
    qrc::embed!("ui/ActionButtonLabel.qml");
    qrc::embed!("ui/ActionButtonSeat.qml");
    qrc::embed!("ui/AppCard.qml");
    qrc::embed!("ui/AppCardFace.qml");
    qrc::embed!("ui/AppCheckBox.qml");
    qrc::embed!("ui/AppCombo.qml");
    qrc::embed!("ui/AppListView.qml");
    qrc::embed!("ui/AppDialog.qml");
    qrc::embed!("ui/AppMenu.qml");
    qrc::embed!("ui/AppMenuButton.qml");
    qrc::embed!("ui/AppMenuItem.qml");
    qrc::embed!("ui/AppMenuSeparator.qml");
    qrc::embed!("ui/AskBar.qml");
    qrc::embed!("ui/AuthorCard.qml");
    qrc::embed!("ui/AutoScrollBar.qml");
    qrc::embed!("ui/AvatarAssignRow.qml");
    qrc::embed!("ui/AvatarButton.qml");
    qrc::embed!("ui/ChangeIcon.qml");
    qrc::embed!("ui/ClipboardHelper.qml");
    qrc::embed!("ui/BandFetchButton.qml");
    qrc::embed!("ui/BandPushButton.qml");
    qrc::embed!("ui/BandRule.qml");
    qrc::embed!("ui/BandStashButton.qml");
    qrc::embed!("ui/BandStateCard.qml");
    qrc::embed!("ui/BandStateGroup.qml");
    qrc::embed!("ui/BandStateMetrics.qml");
    qrc::embed!("ui/BandStateShare.qml");
    qrc::embed!("ui/BandWidest.qml");
    qrc::embed!("ui/CardText.qml");
    qrc::embed!("ui/CarriedPane.qml");
    qrc::embed!("ui/CoAuthorCard.qml");
    qrc::embed!("ui/CoAuthorLine.qml");
    qrc::embed!("ui/ChosenCommitRow.qml");
    qrc::embed!("ui/CodeChip.qml");
    qrc::embed!("ui/ColumnDivider.qml");
    qrc::embed!("ui/CommitAuthorRow.qml");
    qrc::embed!("ui/CommitHoverCard.qml");
    qrc::embed!("ui/CommitMenuState.qml");
    qrc::embed!("ui/CommitRowMenu.qml");
    qrc::embed!("ui/EolHoverCard.qml");
    qrc::embed!("ui/CommandRowDelegate.qml");
    qrc::embed!("ui/CommandsOwner.qml");
    qrc::embed!("ui/CommandsPane.qml");
    qrc::embed!("ui/CommandsTextSelect.qml");
    qrc::embed!("ui/CommandsToggle.qml");
    qrc::embed!("ui/DescriptionBox.qml");
    qrc::embed!("ui/DetailsAuthorCards.qml");
    qrc::embed!("ui/DetailsChangesBand.qml");
    qrc::embed!("ui/DetailsMessageBlock.qml");
    qrc::embed!("ui/DetailsPane.qml");
    qrc::embed!("ui/LineRuler.qml");
    qrc::embed!("ui/LineText.qml");
    qrc::embed!("ui/SweepRoom.qml");
    qrc::embed!("ui/SweepPad.qml");
    qrc::embed!("ui/SweepBand.qml");
    qrc::embed!("ui/BinarySizeLine.qml");
    qrc::embed!("ui/ConflictSideLegend.qml");
    qrc::embed!("ui/CutName.qml");
    qrc::embed!("ui/DiffCodeScroll.qml");
    qrc::embed!("ui/DiffFileNotices.qml");
    qrc::embed!("ui/DiffPane.qml");
    qrc::embed!("ui/DiffPaneHeader.qml");
    qrc::embed!("ui/DiffReach.qml");
    qrc::embed!("ui/DiffRowDelegate.qml");
    qrc::embed!("ui/DiffRowMenu.qml");
    qrc::embed!("ui/DiffRowWalk.qml");
    qrc::embed!("ui/DiffScrollPlace.qml");
    qrc::embed!("ui/DiffTextMetrics.qml");
    qrc::embed!("ui/DiffTextSelect.qml");
    qrc::embed!("ui/DialogActions.qml");
    qrc::embed!("ui/DotMark.qml");
    qrc::embed!("ui/FieldMenu.qml");
    qrc::embed!("ui/FieldMenuSeat.qml");
    qrc::embed!("ui/FileRowDelegate.qml");
    qrc::embed!("ui/FileRowMenu.qml");
    qrc::embed!("ui/FileRowWalk.qml");
    qrc::embed!("ui/FindBar.qml");
    qrc::embed!("ui/FocusRelease.qml");
    qrc::embed!("ui/FoldBlock.qml");
    qrc::embed!("ui/FormField.qml");
    qrc::embed!("ui/GitVersionCorner.qml");
    qrc::embed!("ui/GoneBadge.qml");
    qrc::embed!("ui/GraphColumnDividers.qml");
    qrc::embed!("ui/GraphColumnMetrics.qml");
    qrc::embed!("ui/GraphEmptyState.qml");
    qrc::embed!("ui/GraphFind.qml");
    qrc::embed!("ui/GraphHeadPin.qml");
    qrc::embed!("ui/GraphLaneBar.qml");
    qrc::embed!("ui/GraphLaneCell.qml");
    qrc::embed!("ui/GraphLanePan.qml");
    qrc::embed!("ui/GraphList.qml");
    qrc::embed!("ui/GraphPane.qml");
    qrc::embed!("ui/GraphRowChips.qml");
    qrc::embed!("ui/GraphRowDelegate.qml");
    qrc::embed!("ui/GraphRowWalk.qml");
    qrc::embed!("ui/GraphTailFooter.qml");
    qrc::embed!("ui/HarnessSeat.qml");
    qrc::embed!("ui/HashPlate.qml");
    qrc::embed!("ui/HeadPinRow.qml");
    qrc::embed!("ui/HeadTrack.qml");
    qrc::embed!("ui/HelpText.qml");
    qrc::embed!("ui/HoldDriver.qml");
    qrc::embed!("ui/HoldFill.qml");
    qrc::embed!("ui/HoldIcon.qml");
    qrc::embed!("ui/HoverCardHost.qml");
    qrc::embed!("ui/HoverToolButton.qml");
    qrc::embed!("ui/IdentIcon.qml");
    qrc::embed!("ui/InkCanvas.qml");
    qrc::embed!("ui/IdentityDialog.qml");
    qrc::embed!("ui/IdentityFields.qml");
    qrc::embed!("ui/IdentityGate.qml");
    qrc::embed!("ui/ImagePreviewCell.qml");
    qrc::embed!("ui/LabeledField.qml");
    qrc::embed!("ui/LaneDissolve.qml");
    qrc::embed!("ui/LineEndingField.qml");
    qrc::embed!("ui/MessageActionsRow.qml");
    qrc::embed!("ui/MessageEditor.qml");
    qrc::embed!("ui/MiddleAutoScroll.qml");
    qrc::embed!("ui/NameCell.qml");
    qrc::embed!("ui/NavFactLine.qml");
    qrc::embed!("ui/NavFacts.qml");
    qrc::embed!("ui/NavHeader.qml");
    qrc::embed!("ui/NavIcon.qml");
    qrc::embed!("ui/NavItemDelegate.qml");
    qrc::embed!("ui/NavList.qml");
    qrc::embed!("ui/NavNameBox.qml");
    qrc::embed!("ui/NavRail.qml");
    qrc::embed!("ui/NavRowBody.qml");
    qrc::embed!("ui/NavRowFacts.qml");
    qrc::embed!("ui/NavSections.qml");
    qrc::embed!("ui/NoticeBar.qml");
    qrc::embed!("ui/NoticeButton.qml");
    qrc::embed!("ui/NoticeLine.qml");
    qrc::embed!("ui/OpExitCard.qml");
    qrc::embed!("ui/OpExitRow.qml");
    qrc::embed!("ui/OpenFailedDialog.qml");
    qrc::embed!("ui/OpenFailedScreen.qml");
    qrc::embed!("ui/OpsBranchMenu.qml");
    qrc::embed!("ui/OpsPicker.qml");
    qrc::embed!("ui/PageLayout.qml");
    qrc::embed!("ui/PaneHeader.qml");
    qrc::embed!("ui/PaneScrollBar.qml");
    qrc::embed!("ui/ParentListCard.qml");
    qrc::embed!("ui/PointerWatch.qml");
    qrc::embed!("ui/PublishFlow.qml");
    qrc::embed!("ui/PublishForm.qml");
    qrc::embed!("ui/QuitWaitDialog.qml");
    qrc::embed!("ui/WindowQuitGate.qml");
    qrc::embed!("ui/RefChip.qml");
    qrc::embed!("ui/RefChipStack.qml");
    qrc::embed!("ui/RefListPopup.qml");
    qrc::embed!("ui/RefBranchMenu.qml");
    qrc::embed!("ui/RefRowMenu.qml");
    qrc::embed!("ui/RefTagMenu.qml");
    qrc::embed!("ui/RemoteRowMenu.qml");
    qrc::embed!("ui/RefWorktreeMenu.qml");
    qrc::embed!("ui/RenameCarryFlow.qml");
    qrc::embed!("ui/RenameCarryForm.qml");
    qrc::embed!("ui/RefusalBadge.qml");
    qrc::embed!("ui/RemoteDialog.qml");
    qrc::embed!("ui/ReclickGesture.qml");
    qrc::embed!("ui/RebasePlanPane.qml");
    qrc::embed!("ui/RebasePlanRow.qml");
    qrc::embed!("ui/RebasePlanRunBar.qml");
    qrc::embed!("ui/RepoPage.qml");
    qrc::embed!("ui/RepoPageStack.qml");
    qrc::embed!("ui/CloneDialog.qml");
    qrc::embed!("ui/CloseToolButton.qml");
    qrc::embed!("ui/RowHand.qml");
    qrc::embed!("ui/RowHoverHost.qml");
    qrc::embed!("ui/SampleTimer.qml");
    qrc::embed!("ui/ScrollArrow.qml");
    qrc::embed!("ui/SectionPeekPopup.qml");
    qrc::embed!("ui/SettingsAppPane.qml");
    qrc::embed!("ui/SettingsDialog.qml");
    qrc::embed!("ui/SettingsGitPane.qml");
    qrc::embed!("ui/SettingsHeader.qml");
    qrc::embed!("ui/SettingsGroup.qml");
    qrc::embed!("ui/SettingsRepoPane.qml");
    qrc::embed!("ui/SettingsSection.qml");
    qrc::embed!("ui/SharedToolTip.qml");
    qrc::embed!("ui/SidebarFilterRow.qml");
    qrc::embed!("ui/SidebarPane.qml");
    qrc::embed!("ui/SidebarRowGestures.qml");
    qrc::embed!("ui/SignatureMark.qml");
    qrc::embed!("ui/SlimField.qml");
    qrc::embed!("ui/SpinnerIcon.qml");
    qrc::embed!("ui/SplitBarWatch.qml");
    qrc::embed!("ui/SplitHandleBar.qml");
    qrc::embed!("ui/SplitRefusalOverlay.qml");
    qrc::embed!("ui/StartupGate.qml");
    qrc::embed!("ui/StashActionsBand.qml");
    qrc::embed!("ui/StateBadge.qml");
    qrc::embed!("ui/SummaryArea.qml");
    qrc::embed!("ui/TabCarry.qml");
    qrc::embed!("ui/TabItemDelegate.qml");
    qrc::embed!("ui/TabMetrics.qml");
    qrc::embed!("ui/TabPin.qml");
    qrc::embed!("ui/TabRun.qml");
    qrc::embed!("ui/TabShare.qml");
    qrc::embed!("ui/TabStrip.qml");
    qrc::embed!("ui/TabTitleFade.qml");
    qrc::embed!("ui/TabTreeMark.qml");
    qrc::embed!("ui/TopBar.qml");
    qrc::embed!("ui/TreeViewToggle.qml");
    qrc::embed!("ui/ViewMarkButton.qml");
    qrc::embed!("ui/DiffViewToggle.qml");
    qrc::embed!("ui/DiffLineCell.qml");
    qrc::embed!("ui/DiffStageMark.qml");
    qrc::embed!("ui/UpstreamFlow.qml");
    qrc::embed!("ui/UpstreamForm.qml");
    qrc::embed!("ui/WaitRing.qml");
    qrc::embed!("ui/WindowBody.qml");
    qrc::embed!("ui/WindowChrome.qml");
    qrc::embed!("ui/WindowDialogSeat.qml");
    qrc::embed!("ui/WindowButton.qml");
    qrc::embed!("ui/WindowShape.qml");
    qrc::embed!("ui/WindowWaitRing.qml");
    qrc::embed!("ui/WipBucketPane.qml");
    qrc::embed!("ui/WheelGlide.qml");
    qrc::embed!("ui/WipBucketHeader.qml");
    qrc::embed!("ui/WipCommitBlock.qml");
    qrc::embed!("ui/WipPane.qml");
    qrc::embed!("ui/WipTallyRow.qml");
    qrc::embed!("ui/Main.qml");
    embed_harness_qml();
    harness::register_types();
    for register in [
        AppBackend::register,
        GitFacts::register,
        TabsModel::register,
        CloneModel::register,
        RepoConfigModel::register,
        LineEndingsModel::register,
        RepoTab::register,
        GraphModel::register,
        NavSectionModel::register,
        WorkTreeModel::register,
        DetailsModel::register,
        DiffModel::register,
        CommandsModel::register,
        RebasePlanModel::register,
    ] {
        register();
    }
    engine
        .pin_mut()
        .add_import_path(&QString::from("qrc:/qt/qml"));
    harness::station(harness::Station::EventLoop);
    if let Err(failed) = qml_engine::load(engine.pin_mut(), "qrc:/qt/qml/platitude/ui/Main.qml") {
        // Qt's own lines above name the file and the error; this one is the
        // run's verdict, and the exit code says it to whoever started it.
        tracing::error!(url = %failed.url, "the window's QML would not load");
        Hub::shutdown();
        drop(engine);
        qtbridge::collect_garbage();
        drop(app);
        std::process::exit(1);
    }
    let code = app.pin_mut().exec();

    harness::station(harness::Station::LeftEventLoop);
    // Read first: shutting the hub down consumes it.
    let restart = Hub::with(|hub| hub.restart_wanted()).unwrap_or(false);
    Hub::shutdown();
    harness::station(harness::Station::HubDown);
    // Qt is dropped here, after the hub (nothing left pushing at its
    // objects) and never left to `exit`: on Windows `ExitProcess` ends every
    // other thread before the libraries' detach, so Qt's static teardown
    // would run against a render thread ended mid-frame and wait for good on
    // a lock it died holding (`harness::deadline`, internal-docs/ハング調査.md).
    harness::station(harness::Station::QtTearingDown);
    // `QApp`'s order: the engine lets go of every JS wrapper, the collection
    // frees what nothing holds any more, then the application.
    drop(engine);
    qtbridge::collect_garbage();
    drop(app);
    if restart {
        // The lock goes first: `std::process::exit` runs no destructor, so
        // the successor would be turned away as "already running"
        // (`claim_store`).
        drop(lock);
        start_again();
    }
    // Last: a hang in the libraries' detach after `exit` has nothing left
    // running to report it, so only this station names the step
    // (`harness::deadline`).
    harness::station(harness::Station::Exiting);
    std::process::exit(code);
}

/// Starts this same build again, with the same arguments, and leaves it
/// running — how a setting read once at startup (which git to spawn) takes
/// effect. Called after the window and the files are let go of (`main`).
/// A failure is only logged: no window is left to say it on.
fn start_again() {
    // Undriven runs only: the successor would inherit the run's automation
    // environment, replay the verb, hold the run's settings directory and
    // outlive the parent that was supposed to bound it.
    if crate::harness::knobs().automated {
        tracing::info!("a restart was asked for; a driven run does not start one");
        return;
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            tracing::error!(%error, "cannot start again: this build has no path");
            return;
        }
    };
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match std::process::Command::new(&exe).args(&args).spawn() {
        Ok(child) => tracing::info!(pid = child.id(), path = ?exe, "started again"),
        Err(error) => tracing::error!(%error, path = ?exe, "could not start again"),
    }
}

/// The store this run may use, the directory it was refused if it was, and
/// the lock to hold on to until the process ends.
///
/// * ours — the lock rides in `main`'s frame so the kernel releases it
///   however the process ends;
/// * taken — an *empty* store, so a window about to say "already running"
///   cannot write a thing on its way out;
/// * the lock could not be asked for (a redirected profile, a network
///   share) — the run carries on with the files.
fn claim_store(build: Build) -> (Store, String, Option<platitude_core::settings::Lock>) {
    let store = Store::discover(build);
    match store.claim() {
        Claim::Ours(lock) => (store, String::new(), Some(lock)),
        Claim::Taken => {
            let held = store
                .lock_path()
                .as_deref()
                .and_then(std::path::Path::parent)
                // Shown on screen, and a directory from the OS carries
                // native separators (デザイン規約 §パスの区切り).
                .map(|dir| crate::urlpath::shown_path(&dir.display().to_string()))
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

/// stderr logging; level via `PGG_LOG` (error/warn/info/debug/trace).
///
/// Plain text: `xtask perf` and `xtask verify-ui` substring-search this
/// stream, and colour escapes split `key=value`
/// (`first_chunk_ms\e[0m\e[2m=\e[0m317`) — invisibly in a terminal.
///
/// The writer is [`logsink`], which answers `Ok` however the write went, so
/// the subscriber never reaches the `eprintln!` that panics on a failed
/// stderr. `log_internal_errors(false)` guards those `eprintln!`s again
/// should the writer ever go back to a bare stream.
fn init_tracing() {
    let level = match std::env::var("PGG_LOG").as_deref() {
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

/// Embeds the verification harness's QML module (`src/auto`, `platitude.auto`).
/// A shipped build embeds nothing, and the seats that would load it stay
/// empty (`ui/HarnessSeat.qml`).
#[cfg(feature = "automation")]
fn embed_harness_qml() {
    qrc::embed!("auto/AutoActCompletion.qml");
    qrc::embed!("auto/AutoActDetailsVerbs.qml");
    qrc::embed!("auto/AutoActDiffVerbs.qml");
    qrc::embed!("auto/AutoActDriver.qml");
    qrc::embed!("auto/AutoActFileRowVerbs.qml");
    qrc::embed!("auto/AutoActFindVerbs.qml");
    qrc::embed!("auto/AutoActGraphReclickVerbs.qml");
    qrc::embed!("auto/AutoActGraphRowVerbs.qml");
    qrc::embed!("auto/AutoActHandVerbs.qml");
    qrc::embed!("auto/AutoActHistoryVerbs.qml");
    qrc::embed!("auto/AutoActLandings.qml");
    qrc::embed!("auto/AutoActNavBoxVerbs.qml");
    qrc::embed!("auto/AutoActNavVerbs.qml");
    qrc::embed!("auto/AutoActPaneVerbs.qml");
    qrc::embed!("auto/AutoActPlanVerbs.qml");
    qrc::embed!("auto/AutoActPublishVerbs.qml");
    qrc::embed!("auto/AutoActRefVerbs.qml");
    qrc::embed!("auto/AutoActStepVerbs.qml");
    qrc::embed!("auto/AutoActKeyMenuVerbs.qml");
    qrc::embed!("auto/AutoActSplitVerbs.qml");
    qrc::embed!("auto/AutoActSwitchVerbs.qml");
    qrc::embed!("auto/AutoActTipVerbs.qml");
    qrc::embed!("auto/AutoActWipVerbs.qml");
    qrc::embed!("auto/AutoShotDriver.qml");
    qrc::embed!("auto/Awaited.qml");
    qrc::embed!("auto/NavProbe.qml");
    qrc::embed!("auto/PageAutoStart.qml");
    qrc::embed!("auto/PageHarness.qml");
    qrc::embed!("auto/PagePerfDriver.qml");
    qrc::embed!("auto/PageReports.qml");
    qrc::embed!("auto/PageSettled.qml");
    qrc::embed!("auto/TabProbe.qml");
    qrc::embed!("auto/WindowAutoActDriver.qml");
    qrc::embed!("auto/WindowBadgeActs.qml");
    qrc::embed!("auto/WindowBandActs.qml");
    qrc::embed!("auto/WindowCensus.qml");
    qrc::embed!("auto/WindowDialogActs.qml");
    qrc::embed!("auto/WindowFrameActs.qml");
    qrc::embed!("auto/WindowHarness.qml");
    qrc::embed!("auto/WindowIdentityActs.qml");
    qrc::embed!("auto/WindowOpsActs.qml");
    qrc::embed!("auto/WindowPerfDriver.qml");
    qrc::embed!("auto/WindowSettingsActs.qml");
    qrc::embed!("auto/WindowShotMirrors.qml");
    qrc::embed!("auto/WindowTabActs.qml");
    qrc::embed!("auto/WindowTabPinActs.qml");
    qrc::embed!("auto/qmldir");
}

#[cfg(not(feature = "automation"))]
fn embed_harness_qml() {}

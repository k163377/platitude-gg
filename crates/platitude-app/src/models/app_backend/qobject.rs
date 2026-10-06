//! The whole of what QML sees of `AppBackend`, and as short as this file
//! gets without a redesign.
//!
//! The `#[qobject]` block cannot be divided (structure.md §分割), so what
//! is left is the face — `qproperty!`, slot and signal signatures, and
//! their doc. Every body that could be delegated has been: window state
//! and layout to `start.rs` / `persist.rs`, assignments to `avatars.rs`,
//! startup and identity reads to `lifecycle.rs`, platform calls to
//! `crate::winframe`, measurement to `crate::harness`.
//!
//! The one step left is a redesign: `AppBackend` answers for two subjects
//! — what git and the identity are doing, and what shape the window
//! opened at — and the second could be a QObject of its own. That renames
//! twenty-one slots in every QML file that calls them (the seventeen
//! `start*` reads and the four `save*` writes that go with them), so ask
//! before starting it.

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl AppBackend {
    qproperty!("gitState", Member = git_state, Notify = git_state_changed);
    qproperty!(
        "gitVersion",
        Member = git_version,
        Notify = git_state_changed
    );
    qproperty!(
        "gitUnsupported",
        Member = git_unsupported,
        Notify = git_state_changed
    );
    qproperty!(
        "minimumGit",
        Member = minimum_git,
        Notify = git_state_changed
    );
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!(
        "identityState",
        Member = identity_state,
        Notify = identity_changed
    );
    qproperty!(
        "identityName",
        Member = identity_name,
        Notify = identity_changed
    );
    qproperty!(
        "identityEmail",
        Member = identity_email,
        Notify = identity_changed
    );
    qproperty!(
        "identityError",
        Member = identity_error,
        Notify = identity_changed
    );
    qproperty!(
        "identityBusy",
        Member = identity_busy,
        Notify = identity_changed
    );
    qproperty!(
        "identityNameSaved",
        Member = identity_name_saved,
        Notify = identity_changed
    );
    qproperty!(
        "identityEmailSaved",
        Member = identity_email_saved,
        Notify = identity_changed
    );
    qproperty!(
        "identityUnsaved",
        Member = identity_unsaved,
        Notify = identity_changed
    );
    qproperty!("systemTitleBar", Member = system_title_bar, Constant);
    qproperty!("harnessPresent", Member = harness_present, Constant);
    qproperty!(
        "autoFetchMinutes",
        Member = auto_fetch_minutes,
        Notify = settings_changed
    );
    qproperty!("autoFetchMaxMinutes", Member = auto_fetch_max, Constant);
    qproperty!(
        "gitConcurrency",
        Member = git_concurrency,
        Notify = settings_changed
    );
    qproperty!("gitConcurrencyMax", Member = git_concurrency_max, Constant);
    qproperty!(
        "gitConcurrencyDefault",
        Member = git_concurrency_default,
        Constant
    );
    // "auto" | "fixed" | "off" (`settings::WorktreesReading::word`).
    qproperty!(
        "worktreesReading",
        Member = worktrees_reading,
        Notify = settings_changed
    );
    qproperty!(
        "worktreesIntervalSecs",
        Member = worktrees_interval_secs,
        Notify = settings_changed
    );
    qproperty!(
        "worktreesIntervalMin",
        Member = worktrees_interval_min,
        Constant
    );
    qproperty!(
        "worktreesIntervalMax",
        Member = worktrees_interval_max,
        Constant
    );
    // The floors and ceilings of the reads while a repository is on screen
    // (`session::pace`), and the range a box can ask for.
    qproperty!(
        "refreshFloorSecs",
        Member = refresh_floor_secs,
        Notify = settings_changed
    );
    qproperty!(
        "refreshCeilingSecs",
        Member = refresh_ceiling_secs,
        Notify = settings_changed
    );
    qproperty!(
        "worktreesFloorSecs",
        Member = worktrees_floor_secs,
        Notify = settings_changed
    );
    qproperty!(
        "worktreesCeilingSecs",
        Member = worktrees_ceiling_secs,
        Notify = settings_changed
    );
    qproperty!("paceMinSecs", Member = pace_min_secs, Constant);
    qproperty!("paceMaxSecs", Member = pace_max_secs, Constant);
    qproperty!(
        "initialCommits",
        Member = initial_commits,
        Notify = settings_changed
    );
    qproperty!("initialCommitsMin", Member = initial_commits_min, Constant);
    qproperty!(
        "initialCommitsDefault",
        Member = initial_commits_default,
        Constant
    );
    qproperty!("gitPath", Member = git_path, Notify = git_path_changed);
    qproperty!(
        "gitPathState",
        Member = git_path_state,
        Notify = git_path_changed
    );
    qproperty!(
        "gitPathVersion",
        Member = git_path_version,
        Notify = git_path_changed
    );
    qproperty!(
        "gitPathError",
        Member = git_path_error,
        Notify = git_path_changed
    );
    qproperty!("gitPathInUse", Member = git_path_in_use, Constant);
    qproperty!(
        "gitPathOffersRestart",
        Member = git_path_offers_restart,
        Notify = git_path_changed
    );
    // The window closes on this; `main` starts the successor only once the
    // window is gone and the settings files are let go of.
    qproperty!(
        "restartWanted",
        Member = restart_wanted,
        Notify = git_path_changed
    );
    // Through a getter: a value QML wrote would not reach Rust (encode::wire_qml).
    qproperty!("avatars", Read = avatars, Notify = avatars_changed);
    fn avatars(&self) -> &Assignments {
        &self.avatars
    }
    qproperty!(
        "avatarErrorKind",
        Member = avatar_error_kind,
        Notify = avatars_changed
    );
    qproperty!(
        "avatarErrorFacts",
        Member = avatar_error_facts,
        Notify = avatars_changed
    );
    qproperty!(
        "avatarErrorSaid",
        Member = avatar_error_said,
        Notify = avatars_changed
    );
    qproperty!("avatarPatterns", Member = avatar_patterns, Constant);
    qproperty!("buildTag", Member = build_tag, Constant);
    qproperty!("buildRelease", Member = build_release, Constant);
    qproperty!("buildCommit", Member = build_commit, Constant);
    qproperty!("buildTree", Member = build_tree, Constant);
    qproperty!("alreadyRunning", Member = already_running, Constant);
    qproperty!("heldElsewhere", Member = held_elsewhere, Constant);

    /// Whether the window may go now: no git write is queued or running
    /// (`Hub::writes_settled`). A slot over hub state — the recorded
    /// exception in rules-refs/app-ui.md「アプリ終了の close ゲートは 1 本」.
    /// Fails open: a missing hub means shutdown already owns the writes
    /// and joins them.
    #[qslot]
    fn ready_to_quit(&self) -> bool {
        Hub::with(Hub::writes_settled).unwrap_or(true)
    }

    #[qsignal]
    pub(super) fn git_state_changed(&mut self);

    #[qsignal]
    pub(super) fn identity_changed(&mut self);

    #[qsignal]
    pub(super) fn settings_changed(&mut self);

    /// The path and everything its git last said — one signal, because the
    /// line under the box reads them together.
    #[qsignal]
    pub(super) fn git_path_changed(&mut self);

    /// An assignment was made or taken away. The graph rows, the details
    /// card and the settings list re-read on this: the repository did not
    /// change, so nothing else would tell them.
    #[qsignal]
    pub(super) fn avatars_changed(&mut self);

    #[qslot]
    fn assign_avatar(&mut self, email: String, name: String, file_url: String) {
        self.file_avatar(&email, &name, &file_url);
    }

    #[qslot]
    fn remove_avatar(&mut self, email: String) {
        self.unfile_avatar(&email);
    }

    #[qslot]
    fn avatar_url_for(&self, email: String) -> String {
        self.avatar_url(&email)
    }

    #[qslot]
    fn set_auto_fetch_minutes(&mut self, minutes: i32) {
        self.apply_auto_fetch_minutes(minutes);
    }

    #[qslot]
    fn set_initial_commits(&mut self, commits: i32) {
        self.apply_initial_commits(commits);
    }

    #[qslot]
    fn set_git_concurrency(&mut self, concurrency: i32) {
        self.apply_git_concurrency(concurrency);
    }

    /// How the other worktrees are read — `reading` as the file spells it —
    /// and the fixed interval in seconds, kept whatever is chosen.
    #[qslot]
    fn set_worktrees_reading(&mut self, reading: String, secs: i32) {
        self.apply_worktrees_reading(&reading, secs);
    }

    /// The shortest and the longest interval a repository on screen is
    /// read again at; zero (an emptied box) is that bound's default.
    #[qslot]
    fn set_refresh_bounds(&mut self, floor: i32, ceiling: i32) {
        self.apply_pace_bounds(Some((floor, ceiling)), None);
    }

    /// The same pair for each other worktree read automatically.
    #[qslot]
    fn set_worktrees_bounds(&mut self, floor: i32, ceiling: i32) {
        self.apply_pace_bounds(None, Some((floor, ceiling)));
    }

    /// Records which git this computer runs (a path, or empty for the one
    /// `PATH` resolves) and asks it for its version; the answer arrives on
    /// `gitPathState` and its neighbours. A path that answers nothing is
    /// still kept as written.
    #[qslot]
    fn set_git_path(&mut self, path: String) {
        self.apply_git_path(&path);
    }

    /// The settings screen has opened: asks the path it shows for its
    /// version.
    #[qslot]
    fn open_git_path_screen(&mut self) {
        self.begin_git_path_screen();
    }

    /// The restart button was held down: raises `restartWanted`, unless
    /// nothing was on offer (`AppBackend::restart_now`).
    #[qslot]
    fn apply_git_path_now(&mut self) {
        self.restart_now();
    }

    /// Asks Windows not to round the window's corners; a no-op elsewhere.
    #[qslot]
    fn square_window_corners(&self) {
        crate::winframe::square_corners();
    }

    /// Gives the window the app's icon (title bar, taskbar, Alt+Tab).
    /// Windows-only: elsewhere the desktop entry or the bundle carries it.
    #[qslot]
    fn set_window_icon(&self) {
        crate::winframe::set_icon();
    }

    /// Puts back Win+Arrow, the taskbar's menu and minimising from the
    /// taskbar button, which a window asking for none of the drawn buttons
    /// loses (P3-確認事項 §ウィンドウ chrome).
    #[qslot]
    fn keep_window_gestures(&self) {
        crate::winframe::keep_system_gestures();
        // And answer WM_NCHITTEST ourselves: Qt's own answer for this flag
        // set loses the first click and leaves hover stuck
        // (`take_frame_hit_test`).
        crate::winframe::take_frame_hit_test();
    }

    /// The band's empty runs in logical scene pixels, which the hit test
    /// calls caption (drag, snap, window menu): the run past the last tab
    /// and the seam before the window's buttons. Both in one call, so
    /// neither is left at a stale width.
    #[qslot]
    fn set_caption_strips(
        &self,
        tabs_x0: f64,
        tabs_x1: f64,
        gap_x0: f64,
        gap_x1: f64,
        bottom: f64,
    ) {
        crate::winframe::set_caption_strips([(tabs_x0, tabs_x1), (gap_x0, gap_x1)], bottom);
    }

    /// Maximises or restores through the platform: Qt maximises a
    /// frameless window by resizing it, leaving nothing to restore
    /// (`winframe::set_maximized`).
    #[qslot]
    fn set_window_maximized(&self, maximized: bool) {
        crate::winframe::set_maximized(maximized);
    }

    /// Minimises through the platform: assigning `Minimized` in Qt clears
    /// the restore-to-maximised flag, so a maximised window would come back
    /// an ordinary one (`winframe::minimize`).
    #[qslot]
    fn minimize_window(&self) {
        crate::winframe::minimize();
    }

    /// Pulls a restored window inside the work area of the screen its saved
    /// place was on, and answers whether it had to. Not QML's to do: the
    /// frame is wider than the window, and QML reports no work area
    /// (`winframe::fit_to_work_area`).
    ///
    /// `screen` is `Screen.name` (the display device), empty where the
    /// saved place names no screen this desktop still has.
    #[qslot]
    fn fit_window_to_screen(&self, screen: String) -> bool {
        crate::winframe::fit_to_work_area(&screen)
    }

    // -- window state -------------------------------------------------------
    //
    // Read once as a page or the window is built; after that the UI owns
    // the value, so these are slots. Read off the hub, so a tab opened
    // later gets the layout in force now (`Hub::state`).

    #[qslot]
    fn start_window_x(&self) -> i32 {
        Hub::with(|hub| hub.state().window.x.unwrap_or(i32::MIN)).unwrap_or(i32::MIN)
    }

    #[qslot]
    fn start_window_y(&self) -> i32 {
        Hub::with(|hub| hub.state().window.y.unwrap_or(i32::MIN)).unwrap_or(i32::MIN)
    }

    #[qslot]
    fn start_window_width(&self) -> i32 {
        with_window(|w| w.width)
    }

    #[qslot]
    fn start_window_height(&self) -> i32 {
        with_window(|w| w.height)
    }

    #[qslot]
    fn start_window_maximized(&self) -> bool {
        Hub::with(|hub| hub.state().window.maximized).unwrap_or(false)
    }

    #[qslot]
    fn start_sidebar_width(&self) -> i32 {
        with_layout(|l| l.sidebar_width)
    }

    #[qslot]
    fn start_sidebar_collapsed(&self) -> bool {
        with_flag(|l| l.sidebar_collapsed)
    }

    #[qslot]
    fn start_details_width(&self) -> i32 {
        with_layout(|l| l.details_width)
    }

    #[qslot]
    fn start_graph_labels_width(&self) -> i32 {
        with_layout(|l| l.graph_labels_width)
    }

    #[qslot]
    fn start_graph_lanes_width(&self) -> i32 {
        with_layout(|l| l.graph_lanes_width)
    }

    #[qslot]
    fn start_commands_height(&self) -> i32 {
        with_layout(|l| l.commands_height)
    }

    #[qslot]
    fn start_commands_shown(&self) -> bool {
        self.commands_shown
    }

    #[qslot]
    fn start_tags_shown(&self) -> bool {
        with_flag(|l| l.tags_shown)
    }

    #[qslot]
    fn start_wip_tree(&self) -> bool {
        with_flag(|l| l.wip_tree)
    }

    #[qslot]
    fn start_details_tree(&self) -> bool {
        with_flag(|l| l.details_tree)
    }

    #[qslot]
    fn start_diff_split(&self) -> bool {
        with_flag(|l| l.diff_split)
    }

    #[qslot]
    fn start_section(&self, name: String) -> bool {
        section_open(&name)
    }

    /// Where the window is now, reported on the layout's timer.
    #[qslot]
    fn save_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        self.write_window(x, y, width, height, maximized)
    }

    /// The layout, reported on a timer — per change would write on every
    /// frame of a splitter drag; the hub writes only what differs. Three
    /// slots, because the layout has more parts than a Qt slot takes
    /// arguments; each merges into what the hub holds.
    #[qslot]
    fn save_layout_sizes(
        &self,
        sidebar_width: i32,
        details_width: i32,
        commands_height: i32,
        graph_labels_width: i32,
        graph_lanes_width: i32,
    ) {
        self.write_layout_sizes(
            sidebar_width,
            details_width,
            commands_height,
            graph_labels_width,
            graph_lanes_width,
        )
    }

    #[qslot]
    fn save_layout_flags(
        &mut self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
        diff_split: bool,
    ) {
        self.write_layout_flags(
            sidebar_collapsed,
            commands_shown,
            tags_shown,
            wip_tree,
            details_tree,
            diff_split,
        )
    }

    #[qslot]
    fn save_sections(
        &self,
        branches: bool,
        remotes: bool,
        worktree: bool,
        stashes: bool,
        tags: bool,
    ) {
        self.write_sections(branches, remotes, worktree, stashes, tags)
    }

    /// Writes the state out if anything moved. Driven by a timer in the
    /// window and called once more as it closes.
    #[qslot]
    fn flush_state(&self) {
        Hub::with(|hub| hub.flush_state());
    }

    /// Starts the git version check (call once from QML on startup).
    #[qslot]
    fn initialize(&mut self) {
        self.start_up()
    }

    /// Reads the identity git would record on a commit, once the version
    /// gate has passed.
    pub(super) fn start_identity_check(&mut self) {
        self.begin_identity_check()
    }

    /// Records `user.name` / `user.email` globally — the answer is about
    /// the person. Core validates the values; git's own message comes back
    /// as `identityError`.
    #[qslot]
    fn save_identity(&mut self, name: String, email: String) {
        self.write_identity(name, email)
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
qml_register!(AppBackend, "AppBackend", singleton = true);

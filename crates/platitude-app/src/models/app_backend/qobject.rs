//! The whole of what QML sees of `AppBackend`, and **as short as this
//! file gets without a redesign**.
//!
//! Read this before shortening it. The `#[qobject]` block cannot be
//! divided: QMetaInfo is built per file, so one type is one block is one
//! file. What is left inside it is the face — `qproperty!` declarations,
//! slot and signal signatures, and their doc — because **every body that
//! could be delegated already has been**: the window state and the
//! layout to `start.rs` and `persist.rs`, the assignments to
//! `avatars.rs`, the startup and identity reads to `lifecycle.rs`, the
//! platform calls to `crate::winframe`, and the measurement to
//! `crate::harness`.
//!
//! **The one thing left is a redesign**: `AppBackend` answers for two
//! subjects — what git and the identity are doing, and what shape the
//! window opened at — and the second could be a QObject of its own. That
//! renames twenty slots in every QML file that calls them (the sixteen
//! `start*` reads and the four `save*` writes that go with them), so it
//! is a decision about the QML-facing object model. Ask before starting
//! it.

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
    qproperty!(
        "copiesIntervalSecs",
        Member = copies_interval_secs,
        Notify = settings_changed
    );
    qproperty!("copiesIntervalMin", Member = copies_interval_min, Constant);
    qproperty!("copiesIntervalMax", Member = copies_interval_max, Constant);
    // The page's tick binds to this: the same setting in the unit a
    // `Timer` takes, so no arithmetic stands in the binding.
    qproperty!(
        "copiesIntervalMs",
        Member = copies_interval_ms,
        Notify = settings_changed
    );
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
    // `gitPathOffersRestart` is the one state the chapter grows a button
    // and a warning for, and the same rule the press is guarded by: the
    // box holds a git this run is not on, and that git answered.
    qproperty!(
        "gitPathOffersRestart",
        Member = git_path_offers_restart,
        Notify = git_path_changed
    );
    // `restartWanted` is read by the window: `main` starts the
    // successor, and it can only do that once the window is gone and the
    // settings files have been let go of.
    qproperty!(
        "restartWanted",
        Member = restart_wanted,
        Notify = git_path_changed
    );
    qproperty!("avatars", Member = avatars, Notify = avatars_changed);
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
    qproperty!("buildTree", Member = build_tree, Constant);
    qproperty!("alreadyRunning", Member = already_running, Constant);
    qproperty!("heldElsewhere", Member = held_elsewhere, Constant);

    /// Whether the window may go now: nothing git was asked to write is
    /// still queued or running (`Hub::writes_settled`). Asked when a
    /// close is requested, and sampled by the dialog that holds the
    /// window open while the answer is no (`QuitWaitDialog`) — a slot
    /// over hub state, asked anew (the recorded exception:
    /// rules-refs/app-ui.md の close ゲート項). Fails open on purpose: a
    /// missing hub means shutdown already owns the writes, and its join
    /// is the guarantee then.
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

    /// The path, and everything the git at it last said. One signal for
    /// the four because the line under the box reads them together — a
    /// state without its version is half a sentence.
    #[qsignal]
    pub(super) fn git_path_changed(&mut self);

    /// An assignment was made or taken away. Every list already on screen
    /// re-reads itself off this — the graph rows, the details card and
    /// the settings list — because none of them can be told apart from
    /// the repository not having changed, which it did not.
    #[qsignal]
    pub(super) fn avatars_changed(&mut self);

    // The three below hand their bodies to `avatars.rs` (structure.md
    // §分割 Qt): the slot is declared here, its body lives there.
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

    // The two below hand their bodies to `persist.rs`, the way the avatar
    // slots hand theirs to `avatars.rs` (structure.md §分割 Qt): the slot
    // is declared here, its body lives there.
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

    #[qslot]
    fn set_copies_interval_secs(&mut self, secs: i32) {
        self.apply_copies_interval_secs(secs);
    }

    /// Records which git this computer runs — a path, or empty for
    /// whichever one `PATH` resolves — and asks that binary for its
    /// version. What it answers arrives on `gitPathState` and its three
    /// neighbours; every path is taken as written, because one that
    /// answers nothing is still what the reader wrote down.
    #[qslot]
    fn set_git_path(&mut self, path: String) {
        self.apply_git_path(&path);
    }

    /// The settings screen has opened: the path it is about to show is
    /// asked for its version, and nothing it does counts as a way out yet.
    #[qslot]
    fn open_git_path_screen(&mut self) {
        self.begin_git_path_screen();
    }

    /// The button offering the chosen git has been held all the way down.
    /// Puts `restartWanted` up — the window's cue to close, and `main`'s
    /// to start the successor. Refused where the screen was not offering
    /// it (`AppBackend::restart_now`).
    #[qslot]
    fn apply_git_path_now(&mut self) {
        self.restart_now();
    }

    /// Asks Windows not to round the window's corners. A no-op on the
    /// other two platforms, where the window manager is not doing it.
    #[qslot]
    fn square_window_corners(&self) {
        crate::winframe::square_corners();
    }

    /// Gives the window the app's own icon, which is what the title bar,
    /// the taskbar button and Alt+Tab read. Also Windows-only: elsewhere
    /// the icon travels with the desktop entry or the bundle.
    #[qslot]
    fn set_window_icon(&self) {
        crate::winframe::set_icon();
    }

    /// Puts back what the platform may do with the window on its own —
    /// Win+Arrow, the taskbar's menu, minimising from the taskbar button.
    /// Asking for none of the drawn buttons is what takes them away, and
    /// this app draws its own (P3-確認事項 §ウィンドウ chrome).
    #[qslot]
    fn keep_window_gestures(&self) {
        crate::winframe::keep_system_gestures();
        // And answer WM_NCHITTEST ourselves from here on: Qt 6.10's own
        // answer for this flag set synthesises input from a poll and
        // loses track of it, which is the dead first click and the
        // stuck hover (the whole story is on `take_frame_hit_test`).
        crate::winframe::take_frame_hit_test();
    }

    /// Where the band's empty runs sit, in logical scene pixels — the
    /// stretches of the window the hit test calls caption, which is what
    /// makes them drag, snap and answer a right-click with the window
    /// menu. The scene reports them; the platform does the rest.
    ///
    /// Two of them: the run past the last tab, and the seam before the
    /// window's own buttons. Both in the one call, so
    /// the platform never holds half an answer.
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

    /// Maximises the window or puts it back, through the platform. Qt
    /// maximises a frameless window by resizing it, which leaves the
    /// platform with nothing to restore — so the button did nothing
    /// while the band's double-click worked
    /// (`winframe::set_maximized`).
    #[qslot]
    fn set_window_maximized(&self, maximized: bool) {
        crate::winframe::set_maximized(maximized);
    }

    /// Puts the window down onto the taskbar, again through the
    /// platform: assigning `Minimized` tells Qt the maximise is over
    /// too, and it clears the platform's restore-to-maximised flag to
    /// match — so a maximised window came back an ordinary one
    /// (`winframe::minimize`).
    #[qslot]
    fn minimize_window(&self) {
        crate::winframe::minimize();
    }

    /// Pulls a restored window back inside the work area of the screen
    /// the saved place was on, and answers whether it had to. The scene
    /// cannot do this itself: what has to fit is the frame, which is
    /// wider than the window says it is, and the work area is not a
    /// thing QML reports (`winframe::fit_to_work_area`).
    ///
    /// `screen` is that screen's name as QML spells it
    /// (`Screen.name` — the display device), empty where the saved place
    /// names no screen this desktop still has.
    #[qslot]
    fn fit_window_to_screen(&self, screen: String) -> bool {
        crate::winframe::fit_to_work_area(&screen)
    }

    // -- window state -------------------------------------------------------
    //
    // Read once as a page or the window is built: these are where
    // something starts, and after that the UI owns the value. They
    // are slots for the same reason (規約 §QML
    // バインディングはプロパティにしか反応しない — nothing here needs
    // to react). Reading them off the hub is what makes a tab opened
    // later pick up the layout that is in force now
    // (`Hub::state`).

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

    /// Where the window is now. Reported on the same timer as the layout:
    /// a drag across the desktop is as continuous as a pane drag.
    #[qslot]
    fn save_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        self.write_window(x, y, width, height, maximized)
    }

    /// The layout, from the one place that can see all of it. Reporting
    /// each value as it changes would write on every frame of a splitter
    /// drag; the window says what it looks like on a timer, and the hub
    /// only writes a file when that differs from what is in one.
    ///
    /// Three calls, because the layout has more parts than a Qt slot
    /// takes arguments. Each merges into what the hub holds.
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

    /// Reads the identity git would record on a commit. Runs once the
    /// version gate has passed — with no usable git there is nothing to ask
    /// about, and nothing to ask it with.
    pub(super) fn start_identity_check(&mut self) {
        self.begin_identity_check()
    }

    /// Records `user.name` / `user.email` in the user's own configuration.
    ///
    /// Global is deliberate: the answer is about the person. Values are
    /// validated by the core (a line break would turn the rest of the
    /// config file into another setting), and git's own message comes
    /// back as `identityError`.
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

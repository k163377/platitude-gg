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
    qproperty!("autoIdentity", Member = auto_identity, Constant);
    qproperty!("autoIdentitySave", Member = auto_identity_save, Constant);
    qproperty!("autoOpen", Member = auto_open, Constant);
    qproperty!("shotDir", Member = shot_dir, Constant);
    qproperty!("autoWatchdogMs", Member = auto_watchdog_ms, Constant);
    qproperty!("autoSelect", Member = auto_select, Constant);
    qproperty!("autoScroll", Member = auto_scroll, Constant);
    qproperty!("autoPerf", Member = auto_perf, Constant);
    qproperty!("autoWip", Member = auto_wip, Constant);
    qproperty!("plainChrome", Member = plain_chrome, Constant);
    qproperty!("automated", Member = automated, Constant);
    qproperty!("scrollTo", Member = scroll_to, Constant);
    qproperty!("autoAct", Member = auto_act, Constant);
    qproperty!("autoActArg", Member = auto_act_arg, Constant);
    qproperty!(
        "autoFetchMinutes",
        Member = auto_fetch_minutes,
        Notify = settings_changed
    );
    qproperty!("autoFetchMaxMinutes", Member = auto_fetch_max, Constant);
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
    qproperty!("avatars", Member = avatars, Notify = avatars_changed);
    qproperty!(
        "avatarError",
        Member = avatar_error,
        Notify = avatars_changed
    );
    qproperty!("avatarPatterns", Member = avatar_patterns, Constant);
    qproperty!("buildTree", Member = build_tree, Constant);
    qproperty!("alreadyRunning", Member = already_running, Constant);
    qproperty!("heldElsewhere", Member = held_elsewhere, Constant);
    qproperty!("memReport", Member = mem_report, Constant);

    /// Writes one line of the memory breakdown, tagged with where the run
    /// had got to (`PG_MEM_REPORT=1` only).
    ///
    /// Here rather than on a timer inside Rust because the sessions live on
    /// the Qt main thread: this is the one place that can reach both them
    /// and the models' filings at a moment nothing is half-written.
    #[qslot]
    fn note_memory(&self, label: String) {
        if !crate::memprobe::enabled() {
            return;
        }
        let (session_parts, waiting) = Hub::with(|hub| {
            (
                hub.sessions()
                    .into_iter()
                    .flat_map(|(_, session)| session.heap_report())
                    .collect::<Vec<_>>(),
                hub.feed_depths(),
            )
        })
        .unwrap_or_default();
        crate::memprobe::report(&label, &session_parts, &waiting);
    }

    /// How many open tabs are holding a repository.
    ///
    /// One, however many tabs the strip has: a tab that is not in front
    /// has let go of everything it read (`Hub::release_tab`), and one that
    /// has never been in front never read anything. Read by the harness —
    /// this is the whole of what a released tab looks like from outside,
    /// and it is the one thing a picture cannot say.
    #[qslot]
    fn open_session_count(&self) -> i32 {
        Hub::with(|hub| i32::try_from(hub.sessions().len()).unwrap_or(i32::MAX)).unwrap_or(0)
    }

    /// Whether the window may go now: nothing git was asked to write is
    /// still queued or running (`Hub::writes_settled`). Asked when a
    /// close is requested, and sampled by the dialog that holds the
    /// window open while the answer is no (`QuitWaitDialog`) — a slot
    /// over hub state, never bound (the recorded exception:
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

    /// An assignment was made or taken away. Every list already on screen
    /// re-reads itself off this — the graph rows, the details card and
    /// the settings list — because none of them can be told apart from
    /// the repository not having changed, which it did not.
    #[qsignal]
    pub(super) fn avatars_changed(&mut self);

    // The three below hand their bodies to `avatars.rs` (structure.md
    // §分割 Qt): the slot has to be declared here, what it does does not.
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
    // has to be declared here, what it does does not.
    #[qslot]
    fn set_auto_fetch_minutes(&mut self, minutes: i32) {
        self.apply_auto_fetch_minutes(minutes);
    }

    #[qslot]
    fn set_initial_commits(&mut self, commits: i32) {
        self.apply_initial_commits(commits);
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
    /// Two of them: the run past the last tab, and the one the divider
    /// before the window's buttons stands in. Both in the one call, so
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

    /// Maximises the window or puts it back, through the platform rather
    /// than through `visibility`. Qt maximises a frameless window by
    /// resizing it, which leaves the platform with nothing to restore —
    /// so the button did nothing while the band's double-click worked
    /// (`winframe::set_maximized`).
    #[qslot]
    fn set_window_maximized(&self, maximized: bool) {
        crate::winframe::set_maximized(maximized);
    }

    /// Puts the window down onto the taskbar, again through the platform
    /// rather than through `visibility`: assigning `Minimized` tells Qt
    /// the maximise is over too, and it clears the platform's
    /// restore-to-maximised flag to match — so a maximised window came
    /// back an ordinary one (`winframe::minimize`).
    #[qslot]
    fn minimize_window(&self) {
        crate::winframe::minimize();
    }

    /// Pulls a restored window back inside the screen it came up on, and
    /// answers whether it had to. The scene cannot do this itself: what
    /// has to fit is the frame, which is wider than the window says it
    /// is, and the work area is not a thing QML reports
    /// (`winframe::fit_to_work_area`).
    #[qslot]
    fn fit_window_to_screen(&self) -> bool {
        crate::winframe::fit_to_work_area()
    }

    // -- window state -------------------------------------------------------
    //
    // Read once as a page or the window is built, not bound: these are
    // where something starts, and after that the UI owns the value. They
    // are slots rather than properties for the same reason (規約 §QML
    // バインディングはプロパティにしか反応しない — nothing here needs to
    // react). Reading them off the hub rather than off a copy taken at
    // startup is what makes a tab opened later pick up the layout that is
    // in force now instead of the one the app launched with.

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
        with_flag(|l| l.commands_shown)
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
    fn start_section(&self, name: String) -> bool {
        with_flag(|l| match name.as_str() {
            "branches" => l.sections.branches,
            "remotes" => l.sections.remotes,
            "worktree" => l.sections.worktree,
            "stashes" => l.sections.stashes,
            "tags" => l.sections.tags,
            _ => true,
        })
    }

    /// Where the window is now. Reported on the same timer as the layout:
    /// a drag across the desktop is as continuous as a pane drag.
    #[qslot]
    fn save_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        self.write_window(x, y, width, height, maximized)
    }

    /// The layout, from the one place that can see all of it. Reporting
    /// each value as it changes would write on every frame of a splitter
    /// drag; the window says what it looks like on a timer instead, and
    /// the hub only writes a file when that differs from what is in one.
    ///
    /// Three calls rather than one because the layout has more parts than
    /// a Qt slot takes arguments. Each merges into what the hub holds.
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
        &self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
    ) {
        self.write_layout_flags(
            sidebar_collapsed,
            commands_shown,
            tags_shown,
            wip_tree,
            details_tree,
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

    /// Benchmark/automation reporting channel (QML → tracing).
    #[qslot]
    fn report(&self, message: String) {
        tracing::info!(target: "bench", "{message}");
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
    /// Global is deliberate: the answer is about the person, not one
    /// project. Values are validated by the core (a line break would turn
    /// the rest of the config file into another setting), and git's own
    /// message comes back as `identityError`.
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

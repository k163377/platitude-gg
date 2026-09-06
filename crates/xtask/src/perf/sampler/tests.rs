use super::{Conditions, Limits, Sample, Series, WAKE_SECS, parse_host, parse_sample};

fn sample(ws: u64, fg: bool, tick: u64, app: u64, idle: u64) -> Sample {
    Sample {
        working_set: ws,
        private: ws / 2,
        display: "\\\\.\\DISPLAY2".into(),
        foreground: fg,
        interactive: true,
        dark: Some(false),
        away_ms: 0,
        minimized: false,
        windowed: true,
        // One tick of a 24-thread machine: 24 cores * 100ms in 100ns units.
        kernel: tick * 24_000_000,
        user: 0,
        idle,
        app,
        own: app,
        job: false,
    }
}

#[test]
fn a_sampler_line_parses_into_a_tick() {
    let line = "ws=123 pv=456 display=\\\\.\\DISPLAY1 win=1 fg=1 int=1 min=0 k=7 u=8 i=9 \
                app=10 own=4 job=1 dark=1";
    let parsed = parse_sample(line).expect("a whole line parses");
    assert!(parsed.job, "the children were counted");
    assert_eq!(parsed.dark, Some(true), "the display was off");
    assert_eq!(parsed.own, 4);
    assert_eq!(parsed.working_set, 123);
    assert_eq!(parsed.private, 456);
    assert_eq!(parsed.display, "\\\\.\\DISPLAY1");
    assert!(parsed.windowed && parsed.foreground && parsed.interactive && !parsed.minimized);
    assert_eq!(
        (parsed.kernel, parsed.user, parsed.idle, parsed.app),
        (7, 8, 9, 10)
    );
    // Before the window exists the display is a dash, not a name; before
    // the power broadcast answered, the screen's state is nothing.
    let bare = parse_sample("ws=1 pv=1 display=- win=0 fg=0 int=1 min=0 k=0 u=0 i=0 app=0 dark=-")
        .unwrap();
    assert!(bare.display.is_empty() && !bare.windowed && bare.interactive);
    assert_eq!(bare.dark, None);
    assert_eq!(
        parse_sample("ws=1 pv=1 dark=0").map(|s| s.dark),
        Some(Some(false))
    );
    assert!(parse_sample("PowerShell said something else entirely").is_none());
}

/// The between-runs wait reads the machine alone. A locked session is
/// the one thing it can see that the counters cannot say.
#[test]
fn a_host_line_parses_into_the_machine_alone() {
    let awake = parse_host("int=1 k=7 u=8 i=9").expect("a whole line parses");
    assert!(awake.interactive);
    assert_eq!((awake.kernel, awake.user, awake.idle), (7, 8, 9));
    let locked = parse_host("int=0 k=7 u=8 i=9").expect("a whole line parses");
    assert!(!locked.interactive);
    // Half the capacity went somewhere other than idle.
    let later = parse_host("int=1 k=17 u=8 i=14").expect("a whole line parses");
    assert!((later.busy_percent_since(&awake) - 50.0).abs() < 0.001);
    // Two reads of the same instant divide by nothing.
    assert!(awake.busy_percent_since(&awake).abs() < f64::EPSILON);
    assert!(parse_host("PowerShell said something else entirely").is_none());
}

#[test]
fn the_peak_and_the_last_reading_come_off_one_series() {
    let mut series = Series::default();
    series.absorb(sample(100, true, 1, 0, 24_000_000), 0);
    series.absorb(sample(300, true, 2, 0, 48_000_000), 0);
    series.absorb(sample(200, true, 3, 0, 72_000_000), 0);
    assert_eq!(series.peak_working_set, 300);
    assert_eq!(series.last.map(|s| s.working_set), Some(200));
    assert_eq!(series.conditions.samples, 3);
}

#[test]
fn an_idle_machine_reads_as_no_foreign_load() {
    let mut series = Series::default();
    // Every 100ns of capacity went to idle across all three ticks.
    series.absorb(sample(100, true, 1, 0, 24_000_000), 0);
    series.absorb(sample(100, true, 2, 0, 48_000_000), 0);
    assert!(series.conditions.busy_percent.abs() < 0.001);
    assert!(
        series
            .conditions
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
}

#[test]
fn work_the_measured_process_did_is_not_foreign_load() {
    let mut series = Series::default();
    // Half the machine busy, and all of it the app's own.
    series.absorb(sample(100, true, 1, 0, 24_000_000), 0);
    series.absorb(sample(100, true, 2, 12_000_000, 36_000_000), 0);
    assert!((series.conditions.busy_percent - 50.0).abs() < 0.001);
    assert!(series.conditions.foreign_percent.abs() < 0.001);
    assert!(
        series
            .conditions
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
}

#[test]
fn a_busy_machine_is_refused_and_names_the_share() {
    let mut series = Series::default();
    series.absorb(sample(100, true, 1, 0, 24_000_000), 0);
    series.absorb(sample(100, true, 2, 0, 36_000_000), 0);
    let complaint = series
        .conditions
        .complaint(&Limits::default(), None, false)
        .expect("half the machine went elsewhere");
    assert!(complaint.contains("50.0%"), "{complaint}");
    assert!(
        series
            .conditions
            .complaint(&Limits::OPEN, None, false)
            .is_none()
    );
}

/// Losing the front is recorded and does not spoil the run: a window
/// that is not in front is still composited, and this application
/// does not reliably take the focus off the shell that started it
/// (every tick of a healthy run can read `fg=0`).
#[test]
fn a_window_that_lost_the_front_is_still_a_reading() {
    let mut series = Series::default();
    for tick in 1..=10 {
        series.absorb(sample(100, tick > 3, tick, 0, tick * 24_000_000), 0);
    }
    assert!(
        series
            .conditions
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
    assert_eq!(series.conditions.foreground_share(), Some(0.7));
}

/// A locked session is the one the window cannot be seen through.
#[test]
fn a_locked_session_is_refused() {
    let mut series = Series::default();
    for tick in 1..=10 {
        let mut tick_sample = sample(100, false, tick, 0, tick * 24_000_000);
        tick_sample.interactive = tick < 8;
        series.absorb(tick_sample, 0);
    }
    let complaint = series
        .conditions
        .complaint(&Limits::default(), None, false)
        .expect("three ticks in a row with nobody able to look");
    assert!(
        complaint.contains("for 3 of 10 sampled ticks"),
        "{complaint}"
    );
    assert!(
        series
            .conditions
            .complaint(&Limits::OPEN, None, false)
            .is_none()
    );
}

/// The secure desktop flashing past — a consent prompt, a focus
/// change — is not a lock, and refusing on it costs a retake plus
/// the wait before it.
#[test]
fn a_blink_of_the_secure_desktop_is_not() {
    let mut series = Series::default();
    for tick in 1..=10 {
        let mut tick_sample = sample(100, true, tick, 0, tick * 24_000_000);
        tick_sample.interactive = tick != 4 && tick != 5;
        series.absorb(tick_sample, 0);
    }
    assert_eq!(series.conditions.longest_blind, 2);
    assert!(
        series
            .conditions
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
}

/// Two short blinks are not one long one: the gate reads the longest
/// unbroken stretch, not the total.
#[test]
fn scattered_blinks_do_not_add_up_to_a_lock() {
    let mut series = Series::default();
    for tick in 1..=12 {
        let mut tick_sample = sample(100, true, tick, 0, tick * 24_000_000);
        tick_sample.interactive = tick % 5 != 0;
        series.absorb(tick_sample, 0);
    }
    assert_eq!(series.conditions.interactive, 10);
    assert_eq!(series.conditions.longest_blind, 1);
    assert!(
        series
            .conditions
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
}

/// A window is mapped where the platform puts it and only then moved
/// onto the screen the run asked for, so where it was in the first
/// second is not held against it — but a move after that is the
/// window wandering off the screen whose refresh the frames are read
/// against.
#[test]
fn a_window_that_changed_screens_after_settling_is_refused() {
    let mut settling = Conditions::default();
    let arrived = sample(100, true, 1, 0, 24_000_000);
    let mut elsewhere = arrived.clone();
    elsewhere.display = "\\\\.\\DISPLAY1".into();
    settling.absorb(&elsewhere, None);
    for _ in 0..12 {
        settling.absorb(&arrived, None);
    }
    assert!(
        settling
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
    assert_eq!(settling.displays, ["\\\\.\\DISPLAY2"]);

    let mut wandered = settling.clone();
    wandered.absorb(&elsewhere, None);
    let complaint = wandered
        .complaint(&Limits::default(), None, false)
        .expect("a second screen once the window was placed");
    assert!(complaint.contains("moved between screens"), "{complaint}");
}

#[test]
fn a_minimised_window_is_refused_before_anything_else() {
    let mut conditions = Conditions::default();
    let mut down = sample(100, false, 1, 0, 24_000_000);
    down.minimized = true;
    conditions.absorb(&down, None);
    conditions.absorb(&down, None);
    assert!(
        conditions
            .complaint(&Limits::default(), None, false)
            .is_some_and(|c| c.contains("minimised"))
    );
}

/// The wake helper is the only thing holding the display timer off,
/// and a dark screen shows up on its own in nothing but the scroll
/// bench's frame count — so a run taken without one would publish
/// the numbers of a screen nobody could see.
#[test]
fn a_wake_helper_that_stopped_is_refused() {
    let alive = |away_ms| {
        let mut conditions = Conditions::default();
        for tick in 1..=10 {
            let mut tick_sample = sample(100, true, tick, 0, tick * 24_000_000);
            tick_sample.away_ms = away_ms;
            conditions.absorb(&tick_sample, None);
        }
        conditions
    };
    // One interval and a bit is what a running helper leaves behind.
    assert!(
        alive(WAKE_SECS * 1_000 + 500)
            .complaint(&Limits::default(), None, false)
            .is_none()
    );
    let stopped = alive(WAKE_SECS * 3_000);
    let complaint = stopped
        .complaint(&Limits::default(), None, false)
        .expect("nothing has injected an input for three intervals");
    assert!(complaint.contains("wake helper"), "{complaint}");
    assert!(stopped.complaint(&Limits::OPEN, None, false).is_none());
}

/// Pinning a window asks; it does not decide. The report divides the
/// frame rate by the refresh of the screen that was asked for, and
/// on this machine the screens are 180Hz and 100Hz.
#[test]
fn a_window_that_came_up_on_another_screen_is_refused() {
    let mut conditions = Conditions::default();
    // Past the settling window, or the window has not been seen on
    // any screen yet and there is nothing to compare.
    for tick in 1..=14 {
        conditions.absorb(&sample(100, true, tick, 0, tick * 24_000_000), None);
    }
    assert_eq!(conditions.displays, ["\\\\.\\DISPLAY2"]);
    assert!(
        conditions
            .complaint(&Limits::default(), Some("\\\\.\\DISPLAY2"), false)
            .is_none()
    );
    let complaint = conditions
        .complaint(&Limits::default(), Some("\\\\.\\DISPLAY1"), false)
        .expect("the window is on DISPLAY2 and the frames would be read against DISPLAY1");
    assert!(
        complaint.contains("pinned to \\\\.\\DISPLAY1"),
        "{complaint}"
    );
}

/// The job object counts the children: a process that does its work in
/// a child shows more time in `app` than in `own`. The process is
/// started suspended, as the app is, and does its work the moment the
/// sampler resumes it — which is after the join, or the child's time
/// would be its own.
#[cfg(windows)]
#[test]
fn the_children_of_the_watched_process_are_counted_as_its_own() {
    use std::os::windows::process::CommandExt;
    use std::time::Duration;
    let csv = std::env::temp_dir().join(format!("pg-sampler-{}.csv", std::process::id()));
    let file = std::fs::File::create(&csv).expect("a csv to write");
    let armed = super::arm(Duration::from_secs(30), file, false).expect("an armed sampler");
    let mut child = std::process::Command::new("cmd")
        .args([
            "/C",
            "powershell -NoProfile -Command \"$s=0; 1..300000 | ForEach-Object { $s += $_ }\" & \
             ping -n 2 127.0.0.1 >nul",
        ])
        .creation_flags(super::CREATE_SUSPENDED)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("a process with a child");
    let (sampler, _started) = armed
        .watch(child.id())
        .expect("the sampler joined and resumed the process");
    child.wait().expect("the process ends on its own");
    let series = sampler.finish().expect("the sampler ran to the end");
    let last = series.last.expect("at least one sample");
    let _ = std::fs::remove_file(csv);
    assert!(
        series.conditions.children_counted,
        "the process joined the job object"
    );
    assert!(
        last.app > last.own,
        "the child's time is counted as the process's: app={} own={}",
        last.app,
        last.own
    );
    // The power broadcast answers on registration, so every tick of a
    // live sampler knows whether the display is off.
    assert!(
        last.dark.is_some(),
        "the display's power state was read: {last:?}"
    );
}

/// The screen is a condition of a D3D run alone. One dark tick refuses
/// it: the display timer ran out under the run, and every frame after
/// that went nowhere — said before the wake helper's own interval would
/// say so. A software run's frames need no display, so the same ticks —
/// lit, dark, flipping, or never answered — refuse nothing and stay
/// evidence.
#[test]
fn the_screen_is_a_condition_of_a_d3d_run_alone() {
    let with = |dark: Option<bool>| {
        let mut tick_sample = sample(100, true, 1, 0, 24_000_000);
        tick_sample.dark = dark;
        tick_sample
    };
    let series = |ticks: &[Option<bool>]| {
        let mut series = Series::default();
        for dark in ticks {
            series.absorb(with(*dark), 0);
        }
        series
    };
    let d3d = false;
    let software = true;
    let lit = series(&[Some(false), Some(false)]);
    assert!(
        lit.conditions
            .complaint(&Limits::default(), None, d3d)
            .is_none()
    );
    assert!(
        lit.conditions
            .complaint(&Limits::default(), None, software)
            .is_none()
    );
    let dark = series(&[Some(true), Some(true)]);
    let refused = dark
        .conditions
        .complaint(&Limits::default(), None, d3d)
        .expect("a dark tick under a D3D run");
    assert!(refused.contains("display was off for 2 of 2"), "{refused}");
    assert!(refused.contains("--software"), "{refused}");
    assert!(
        dark.conditions
            .complaint(&Limits::default(), None, software)
            .is_none()
    );
    let flipping = series(&[Some(true), Some(false), Some(true)]);
    assert!(
        flipping
            .conditions
            .complaint(&Limits::default(), None, d3d)
            .is_some()
    );
    assert!(
        flipping
            .conditions
            .complaint(&Limits::default(), None, software)
            .is_none()
    );
    assert_eq!((flipping.conditions.dark, flipping.conditions.lit), (2, 1));
    let unread = series(&[None, None]);
    assert!(
        unread
            .conditions
            .complaint(&Limits::default(), None, d3d)
            .is_none()
    );
    assert!(
        unread
            .conditions
            .complaint(&Limits::default(), None, software)
            .is_none()
    );
    assert_eq!((unread.conditions.dark, unread.conditions.lit), (0, 0));
    // --allow-noisy opens this gate with every other.
    assert!(
        dark.conditions
            .complaint(&Limits::OPEN, None, d3d)
            .is_none()
    );
}

/// A software run injects no input, so the interval since the last
/// input is how long the person has been away — not a helper that
/// died, which is what the same interval means to a D3D run.
#[test]
fn a_software_run_is_not_refused_for_the_wake_helper_it_never_had() {
    let away = |dark: bool| {
        let mut series = Series::default();
        for _ in 0..2 {
            let mut tick_sample = sample(100, true, 1, 0, 24_000_000);
            tick_sample.dark = Some(dark);
            tick_sample.away_ms = WAKE_SECS * 10_000;
            series.absorb(tick_sample, 0);
        }
        series
    };
    assert!(
        away(true)
            .conditions
            .complaint(&Limits::default(), None, true)
            .is_none()
    );
    assert!(
        away(false)
            .conditions
            .complaint(&Limits::default(), None, true)
            .is_none()
    );
    let refused = away(false)
        .conditions
        .complaint(&Limits::default(), None, false)
        .expect("a stopped helper under a D3D run");
    assert!(refused.contains("wake helper"), "{refused}");
}

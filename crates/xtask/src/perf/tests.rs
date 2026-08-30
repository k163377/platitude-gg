use super::*;

#[test]
fn a_tracing_field_reads_up_to_the_next_space() {
    let line = "INFO first_chunk_ms=873 graph first chunk";
    assert_eq!(field(line, "first_chunk_ms="), Some("873"));
    assert_eq!(field(line, "missing="), None);
}

#[test]
fn the_bench_line_gives_its_fps() {
    let mut found = Reading::default();
    absorb("INFO report: scroll_bench fps=178.3 rows=2000", &mut found);
    assert_eq!(found.fps, Some(178.3));
}

#[test]
fn the_largest_heap_is_the_breakdown_that_is_kept() {
    let mut found = Reading::default();
    absorb("mem report rust_live=100 models=a", &mut found);
    absorb("mem report rust_live=900 models=b", &mut found);
    absorb("mem report rust_live=300 models=c", &mut found);
    assert_eq!(found.breakdown_live, 900);
    assert!(found.breakdown.is_some_and(|l| l.contains("models=b")));
}

#[test]
fn watchdog_is_the_outer_ceiling_and_quit_is_rejected() {
    let parsed = parse(&["--watchdog-ms".into(), "9000".into(), "--no-open".into()])
        .expect("watchdog should parse");
    assert_eq!(parsed.watchdog_ms, 9000);
    let err = match parse(&["--quit-ms".into(), "2500".into(), "--no-open".into()]) {
        Ok(_) => panic!("quit must not be accepted as a correctness clock"),
        Err(err) => err,
    };
    assert!(err.contains("--watchdog-ms"));
}

#[test]
fn perf_done_is_absorbed_and_required() {
    let mut found = Reading::default();
    absorb("INFO perf_done", &mut found);
    assert!(found.perf_done);
    let opts = parse(&["--no-open".into()]).expect("bare window options should parse");
    let err = missing(&Reading::default(), &opts).expect_err("done is mandatory");
    assert!(err.contains("perf_done"));
}

fn options(args: &[&str]) -> Options {
    parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).expect("valid scenario")
}

fn unselected_reading() -> Reading {
    let mut reading = Reading {
        peak_working_set: 100,
        peak_private: 100,
        startup_ms: Some(10),
        first_chunk_ms: Some(5),
        total_ms: Some(8),
        ..Reading::default()
    };
    for line in [
        "perf_selection mode=none oid=none",
        "perf_complete selection=none details=false diff=false graph=true scrolled=false",
        "perf_done",
    ] {
        absorb(line, &mut reading);
    }
    reading
}

#[test]
fn none_rejects_the_old_implicit_head_selection() {
    let opts = options(&["--repo", ".", "--no-select", "--no-scroll"]);
    let mut reading = unselected_reading();
    assert!(missing(&reading, &opts).is_ok());
    absorb("details request round trip elapsed_ms=50", &mut reading);
    assert!(
        missing(&reading, &opts)
            .expect_err("HEAD must not be read")
            .contains("unselected")
    );
}

#[test]
fn a_hidden_or_stationary_graph_cannot_produce_a_valid_scroll_reading() {
    let opts = options(&["--repo", ".", "--no-select"]);
    for evidence in [
        "visible=false moved=100 frame_count=60",
        "visible=true moved=0 frame_count=60",
        "visible=true moved=100 frame_count=0",
    ] {
        let mut reading = unselected_reading();
        absorb(
            "perf_complete selection=none details=false diff=false graph=true scrolled=true",
            &mut reading,
        );
        absorb(&format!("scroll_bench fps=60 {evidence}"), &mut reading);
        assert!(
            missing(&reading, &opts)
                .expect_err("no visible scroll")
                .contains("visible, moving")
        );
    }
}

#[test]
fn done_without_process_memory_is_never_a_measurement() {
    let opts = options(&["--no-open"]);
    let reading = Reading {
        perf_done: true,
        ..Reading::default()
    };
    assert!(
        missing(&reading, &opts)
            .expect_err("zero counters")
            .contains("memory")
    );
}

#[test]
fn data_arrival_does_not_stand_in_for_a_rendered_response() {
    let opts = options(&["--repo", ".", "--no-scroll", "--no-diff"]);
    let mut reading = unselected_reading();
    absorb("perf_selection mode=first oid=abc", &mut reading);
    absorb("details request round trip elapsed_ms=50", &mut reading);
    absorb(
        "perf_complete selection=first details=true diff=false graph=true scrolled=false",
        &mut reading,
    );
    assert!(
        missing(&reading, &opts)
            .expect_err("frame is missing")
            .contains("rendered frame")
    );
    absorb("perf_details_frame elapsed_ms=75", &mut reading);
    assert!(missing(&reading, &opts).is_ok());
}

#[test]
fn contradictory_or_empty_scenarios_are_rejected() {
    for args in [
        vec!["--no-open", "--runs", "0"],
        vec!["--repo", ".", "--no-select", "--file", "a.txt"],
        vec!["--repo", ".", "--no-diff", "--file", "a.txt"],
        vec!["--repo", ".", "--selection", "random"],
        vec!["--repo", ".", "--select-oid", "abbreviated"],
    ] {
        assert!(
            parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err(),
            "{args:?}"
        );
    }
}

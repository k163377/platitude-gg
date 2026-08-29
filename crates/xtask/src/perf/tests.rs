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

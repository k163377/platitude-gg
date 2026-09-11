//! Cache preparation, balanced A/B order and portable result files.
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{Options, Reading};

pub(super) fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn order(blocks: u32) -> Vec<&'static str> {
    (0..blocks).flat_map(|_| ["A", "B", "B", "A"]).collect()
}

pub(super) fn compare(mut opts: Options) -> Result<(), String> {
    let root = crate::tree::workspace_root();
    super::guard_the_window(&root)?;
    let path = crate::qt::path_with_qt()?;
    let (a, b) = {
        let _busy = crate::still::busy(&root, "perf A/B builds")?;
        let a = super::build(&root, &path, &opts)?;
        let mut other = opts.clone();
        other.at = opts.compare.clone();
        let b = super::build(&root, &path, &other)?;
        (a.commit, b.commit)
    };
    if opts.open {
        opts.corpus = super::corpus::describe(&opts.repo)?.token;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = opts.output.clone().unwrap_or_else(|| {
        root.join("target/perf")
            .join(platform())
            .join(format!("compare-{stamp}"))
    });
    std::fs::create_dir_all(
        directory
            .parent()
            .ok_or("comparison output has no parent")?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    let mut schedule =
        std::fs::File::create(directory.join("order.tsv")).map_err(|e| e.to_string())?;
    writeln!(schedule, "step\tvariant\tcommit\tcache\tresult").map_err(|e| e.to_string())?;
    let blocks = opts.runs;
    opts.runs = 1;
    opts.build = false;
    // Both shelves exist before the first timed launch. Each warm sample
    // warms its own binary immediately before measurement; no shelf switch
    // can silently borrow the other binary's warm-cache claim.
    let mut expected = None;
    for (step, variant) in order(blocks).iter().enumerate() {
        opts.at = if *variant == "A" {
            a.clone()
        } else {
            b.clone()
        };
        let output = directory.join(format!("{:03}-{variant}", step + 1));
        opts.output = Some(output.clone());
        writeln!(
            schedule,
            "{}\t{variant}\t{}\t{}\t{}",
            step + 1,
            opts.at,
            opts.cache,
            output.display()
        )
        .map_err(|e| e.to_string())?;
        schedule.flush().map_err(|e| e.to_string())?;
        super::run_options(opts.clone())?;
        let signature = comparison_identity(&output)?;
        if expected.as_ref().is_some_and(|before| before != &signature) {
            return Err(format!(
                "A/B environment or corpus changed; evidence remains in {}",
                directory.display()
            ));
        }
        expected = Some(signature);
    }
    println!(
        "A/B complete: {} (ABBA, {} samples per variant; compare matching cache/platform/case rows, not pooled values)",
        directory.display(),
        blocks * 2
    );
    Ok(())
}

fn comparison_identity(output: &Path) -> Result<String, String> {
    let manifest =
        std::fs::read_to_string(output.join("manifest.txt")).map_err(|e| e.to_string())?;
    let mut identity = manifest
        .lines()
        .filter(|l| {
            [
                "os=",
                "arch=",
                "features=",
                "repo=",
                "screen=",
                "screen_hz=",
                "window=",
                "corpus=",
                "software=",
                "cache=",
            ]
            .iter()
            .any(|k| l.starts_with(k))
        })
        .collect::<Vec<_>>()
        .join("\n");
    identity.push_str(
        &std::fs::read_to_string(output.join("graphics.txt")).map_err(|e| e.to_string())?,
    );
    Ok(identity)
}

pub(super) fn prepare_cold(opts: &Options, exe: &Path, output: &Path) -> Result<(), String> {
    if opts.cache != "cold" {
        return Ok(());
    }
    // The operator supplies an OS-specific executable, never a shell string.
    // It must finish successfully after preparing the named repo and binary.
    // No repository inspection or calibration follows it before app launch.
    let log = std::fs::File::create(output.join("cold-prepare.log")).map_err(|e| e.to_string())?;
    let mut child = Command::new(&opts.cold_prepare)
        .arg(&opts.repo)
        .arg(exe)
        .stdin(Stdio::null())
        .stderr(log.try_clone().map_err(|e| e.to_string())?)
        .stdout(log)
        .spawn()
        .map_err(|e| format!("cold preparation: {e}"))?;
    let mut wait = crate::wait::Wait::new(
        "cold preparation",
        crate::wait::Budget::whole(Duration::from_millis(opts.watchdog_ms)),
        crate::wait::LOOK_AGAIN,
    );
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("cold preparation failed: {status}"))
            };
        }
        if let Err(expired) = wait.look_again("preparing repository and executable caches") {
            child.kill().map_err(|e| e.to_string())?;
            child.wait().map_err(|e| e.to_string())?;
            return Err(expired.to_string());
        }
    }
}

pub(super) fn save(output: &Path, opts: &Options, kept: &[Reading]) -> Result<(), String> {
    let mut summary =
        std::fs::File::create(output.join("measurements.tsv")).map_err(|e| e.to_string())?;
    writeln!(
        summary,
        "platform\tcache\trun\tcase\toperation\tmetric\tvalue\tunit"
    )
    .map_err(|e| e.to_string())?;
    for (run, reading) in kept.iter().enumerate() {
        for (name, value, unit) in [
            (
                "sampled-resident-peak",
                Some(reading.peak_working_set as f64),
                "bytes",
            ),
            (
                "os-resident-peak",
                reading.os_peak_working_set.map(|v| v as f64),
                "bytes",
            ),
            ("startup", reading.startup_ms.map(|v| v as f64), "ms"),
            ("graph-fps", reading.fps, "fps"),
        ] {
            if let Some(value) = value {
                writeln!(
                    summary,
                    "{}\t{}\t{}\t-\t-\t{name}\t{value}\t{unit}",
                    platform(),
                    opts.cache,
                    run + 1
                )
                .map_err(|e| e.to_string())?;
            }
        }
        for e in &reading.events {
            if e.kind == "perf_diff_scroll_frame" {
                continue;
            }
            writeln!(
                summary,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\tms",
                platform(),
                opts.cache,
                run + 1,
                e.case,
                e.operation,
                e.kind,
                e.ms
            )
            .map_err(|e| e.to_string())?;
        }
        for line in &reading.diff_scrolls {
            let value = |key: &str| {
                super::reading::token(line, key)
                    .unwrap_or("")
                    .trim_matches('"')
            };
            for key in [
                "fps",
                "frame_p50_ms",
                "frame_p95_ms",
                "frame_p99_ms",
                "frame_max_ms",
                "over_16_ms",
                "over_100_ms",
            ] {
                let name = format!("{key}=");
                let unit = if key == "fps" {
                    "fps"
                } else if key.starts_with("over_") {
                    "frames"
                } else {
                    "ms"
                };
                writeln!(
                    summary,
                    "{}\t{}\t{}\t{}\t{}\tdiff-{key}\t{}\t{unit}",
                    platform(),
                    opts.cache,
                    run + 1,
                    value("case="),
                    value("operation="),
                    value(&name)
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    std::fs::write(
        output.join("graphics.txt"),
        kept.first()
            .map(|r| r.graphics.join("\n"))
            .unwrap_or_default(),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_balances_variant_and_position() {
        assert_eq!(super::order(2), ["A", "B", "B", "A", "A", "B", "B", "A"]);
    }

    #[test]
    fn saved_rows_keep_axes_and_comparison_rejects_environment_drift() {
        let output = std::env::temp_dir().join(format!(
            "pgg-perf-evidence-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&output).unwrap();
        let opts = super::super::options::parse(&["--repo", "x"].map(str::to_owned)).unwrap();
        let mut reading = Reading {
            os_peak_working_set: Some(900),
            graphics: vec!["Adapter test".into()],
            ..Reading::default()
        };
        reading.events.push(
            super::super::interactions::read(
                "perf_colour_frame case=large operation=2 oid=abc fingerprint=f elapsed_ms=15",
            )
            .unwrap(),
        );
        reading.diff_scrolls.push("scroll_bench surface=\"diff\" case=\"large\" operation=2 fps=60 frame_p50_ms=16 frame_p95_ms=17 frame_p99_ms=18 frame_max_ms=19 over_16_ms=2 over_100_ms=0".into());
        save(&output, &opts, &[reading]).unwrap();
        let rows = std::fs::read_to_string(output.join("measurements.tsv")).unwrap();
        assert!(rows.contains(&format!(
            "{}\twarm\t1\tlarge\t2\tperf_colour_frame\t15\tms",
            platform()
        )));
        assert!(rows.contains("\tlarge\t2\tdiff-frame_p99_ms\t18\tms"));
        assert!(rows.contains("\tos-resident-peak\t900\tbytes"));
        let manifest = output.join("manifest.txt");
        std::fs::write(
            &manifest,
            "commit=A\nos=windows\ncache=warm\ncorpus=fixed\n",
        )
        .unwrap();
        let original = comparison_identity(&output).unwrap();
        std::fs::write(
            &manifest,
            "commit=B\nos=windows\ncache=warm\ncorpus=fixed\n",
        )
        .unwrap();
        assert_eq!(comparison_identity(&output).unwrap(), original);
        std::fs::write(&manifest, "commit=B\nos=linux\ncache=warm\ncorpus=fixed\n").unwrap();
        assert_ne!(comparison_identity(&output).unwrap(), original);
        std::fs::remove_dir_all(output).unwrap();
    }
}

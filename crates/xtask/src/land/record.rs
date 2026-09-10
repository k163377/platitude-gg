//! The whole landing, including admission, kept beside the seat's gate records.

use std::path::{Path, PathBuf};
use std::time::Instant;

pub(super) struct Phases {
    started: Instant,
    last: Instant,
    marks: Vec<String>,
    dir: PathBuf,
    branch: String,
}

impl Phases {
    pub(super) fn start() -> Self {
        // waits(measured): the landing's entry, before preflight or the turn queue
        let now = Instant::now();
        Self {
            started: now,
            last: now,
            marks: Vec::new(),
            dir: crate::tree::workspace_root(),
            branch: String::new(),
        }
    }

    pub(super) fn target(&mut self, dir: &Path, branch: &str) {
        self.dir = dir.to_path_buf();
        self.branch = branch.to_string();
    }

    pub(super) fn mark(&mut self, what: &str) {
        // waits(measured): the end of a phase, never an admission condition
        self.mark_at(what, Instant::now());
    }

    fn mark_at(&mut self, what: &str, now: Instant) {
        self.marks.push(format!(
            "{what} {}",
            super::clock(now.duration_since(self.last))
        ));
        self.last = now;
    }

    fn report(&self, result: &Result<(), String>) -> String {
        let outcome = match result {
            Ok(()) => "PASS".to_string(),
            Err(why) => format!("FAIL: {why}"),
        };
        format!(
            "land: {} — {} in all\n  branch {} / {outcome}\n",
            self.marks.join(", "),
            super::clock(self.last.duration_since(self.started)),
            self.branch,
        )
    }

    pub(super) fn finish(&mut self, result: &Result<(), String>) {
        self.mark(if result.is_ok() {
            "finish"
        } else {
            "unfinished"
        });
        let report = self.report(result);
        print!("{report}");
        if let Err(why) = keep(&self.dir, &report) {
            eprintln!("land: could not keep the timing record: {why}");
        }
    }
}

/// Keep the newest 200 records, as gate does. A record failure cannot
/// turn a completed fast-forward into a failed landing.
fn keep(dir: &Path, report: &str) -> std::io::Result<()> {
    let records = dir.join("target/land-runs");
    std::fs::create_dir_all(&records)?;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = records.join(format!("{at}-{}.txt", std::process::id()));
    std::fs::write(path, report)?;
    let mut names = std::fs::read_dir(&records)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect::<Vec<_>>();
    names.sort();
    let over = names.len().saturating_sub(200);
    for stale in names.into_iter().take(over) {
        std::fs::remove_file(stale)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Phases;
    use std::time::Duration;

    #[test]
    fn the_total_includes_preflight_and_queue_on_success_and_failure() {
        let mut phases = Phases::start();
        let start = phases.started;
        phases.mark_at("preflight", start + Duration::from_secs(2));
        phases.mark_at("queue", start + Duration::from_secs(12));
        phases.mark_at("gate", start + Duration::from_secs(15));
        for result in [Ok(()), Err("gate refused".to_string())] {
            let text = phases.report(&result);
            assert!(
                text.contains("preflight 0m02s, queue 0m10s, gate 0m03s"),
                "{text}"
            );
            assert!(text.contains("0m15s in all"), "{text}");
            assert!(
                text.contains(if result.is_ok() {
                    "PASS"
                } else {
                    "FAIL: gate refused"
                }),
                "{text}"
            );
        }
    }
}

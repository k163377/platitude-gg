// Phase 0 spike for platitude-gg — throwaway code.
//
// Validates Qt Bridges (qtbridge) against the go/no-go criteria:
//   S1: expose Rust structs to QML, list-model / property updates reach the UI
//   S2: Japanese IME in a QML TextArea (manual check, page provided here)
//   S3: 100k-row commit-graph rendering at ~60 fps (items / canvas / shape renderers)
//   S4: worker thread -> UI notification via QmlMethodInvoker
//   S5: streaming `git log` of the JetBrains/kotlin repo, first chunk < 3s
//
// Headless-ish automation:
//   PGG_SPIKE_AUTOBENCH=items|canvas|shape|all  -> run scroll benchmark, print fps, quit
//   PGG_SPIKE_AUTOLOG=<repo path>               -> stream git log, print timings, quit
//   PGG_SPIKE_TOPO=0                            -> use default order instead of --topo-order

// HashMap must be in scope: the QModelItem derive expands to unhygienic code
// that names `HashMap` directly.
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use qtbridge::qtbridge_type_lib::QModelIndex;
use qtbridge::{
    QApp, QListModel, QListModelBase, QModelItem, QObjectHolder, invoke_method, qobject,
};

// ---------------------------------------------------------------------------
// SpikeConfig: singleton exposing env-driven automation flags + stdout reporter
// ---------------------------------------------------------------------------

pub struct SpikeConfig {
    auto_bench: String,
    auto_log: String,
    log_topo: bool,
    default_repo: String,
    start_tab: i32,
    stay: bool,
    shot_dir: String,
}

impl Default for SpikeConfig {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        Self {
            auto_bench: std::env::var("PGG_SPIKE_AUTOBENCH").unwrap_or_default(),
            auto_log: std::env::var("PGG_SPIKE_AUTOLOG").unwrap_or_default(),
            log_topo: std::env::var("PGG_SPIKE_TOPO")
                .map(|v| v != "0")
                .unwrap_or(true),
            default_repo: std::env::var("PGG_SPIKE_REPO")
                .unwrap_or_else(|_| format!("{}/IdeaProjects/kotlin", home.replace('\\', "/"))),
            start_tab: std::env::var("PGG_SPIKE_TAB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(-1),
            stay: std::env::var("PGG_SPIKE_STAY")
                .map(|v| v == "1")
                .unwrap_or(false),
            shot_dir: std::env::var("PGG_SPIKE_SHOTDIR")
                .unwrap_or_default()
                .replace('\\', "/"),
        }
    }
}

#[qobject(Singleton)]
impl SpikeConfig {
    qproperty!("autoBench", Member = auto_bench, Constant);
    qproperty!("autoLog", Member = auto_log, Constant);
    qproperty!("logTopo", Member = log_topo, Constant);
    qproperty!("defaultRepo", Member = default_repo, Constant);
    qproperty!("startTab", Member = start_tab, Constant);
    qproperty!("stay", Member = stay, Constant);
    qproperty!("shotDir", Member = shot_dir, Constant);

    #[qslot]
    fn report(&self, msg: String) {
        println!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// S1: DemoModel — QListModel of Strings + a plain property, mutated via slots.
// Uses ConvertToCamelCase to validate the naming option.
// ---------------------------------------------------------------------------

pub struct DemoModel {
    items: Vec<String>,
    counter: i32,
}

impl Default for DemoModel {
    fn default() -> Self {
        Self {
            items: vec!["alpha".into(), "bravo".into(), "charlie".into()],
            counter: 0,
        }
    }
}

impl QListModel for DemoModel {
    type Item = String;

    fn len(&self) -> usize {
        self.items.len()
    }
    fn get(&self, index: usize) -> Option<&String> {
        self.items.get(index)
    }
    fn set_unnotified(&mut self, index: usize, value: String) -> bool {
        match self.items.get_mut(index) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
    fn push_unnotified(&mut self, value: String) {
        self.items.push(value);
    }
    fn remove_unnotified(&mut self, index: usize) -> String {
        self.items.remove(index)
    }
    fn reset_unnotified(&mut self) {
        self.items.clear();
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase)]
impl DemoModel {
    qproperty!("counter", Member = counter, Notify = counter_changed);

    #[qsignal]
    fn counter_changed(&mut self);

    #[qslot]
    fn add_item(&mut self, value: String) {
        self.push(value);
    }

    #[qslot]
    fn update_item(&mut self, index: i32, value: String) {
        if index >= 0 {
            self.set(index as usize, value);
        }
    }

    #[qslot]
    fn remove_item(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            self.remove(index as usize);
        }
    }

    #[qslot]
    fn clear_items(&mut self) {
        self.reset();
    }

    #[qslot]
    fn bump_counter(&mut self) {
        self.counter += 1;
        self.counter_changed();
    }
}

// ---------------------------------------------------------------------------
// S4: WorkerBackend — a std::thread ticking at ~60Hz, marshalled to the UI
// thread through QmlMethodInvoker / invoke_method!.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkerBackend {
    progress: i32,
    running: bool,
    stop_flag: Arc<AtomicBool>,
}

#[qobject]
impl WorkerBackend {
    qproperty!("progress", Member = progress, Notify = progress_changed);
    qproperty!("running", Member = running, Notify = running_changed);

    #[qsignal]
    fn progress_changed(&mut self);
    #[qsignal]
    fn running_changed(&mut self);

    #[qslot]
    fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        self.running_changed();
        self.stop_flag.store(false, Ordering::Relaxed);
        let stop = Arc::clone(&self.stop_flag);
        let invoker = self.get_qml_method_invoker();
        std::thread::spawn(move || {
            for i in 0..=300i32 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Queued call onto the Qt main thread.
                invoke_method!(invoker, "on_tick", i);
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            invoke_method!(invoker, "on_worker_done");
        });
    }

    #[qslot]
    fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    #[qslot]
    fn on_tick(&mut self, value: i32) {
        self.progress = value;
        self.progress_changed();
    }

    #[qslot]
    fn on_worker_done(&mut self) {
        self.running = false;
        self.running_changed();
    }
}

// ---------------------------------------------------------------------------
// S3: GraphModel — 100k synthetic commit-graph rows, lane data precomputed in
// Rust (bit masks), QML only draws.
// ---------------------------------------------------------------------------

const LANES: i32 = 8;

#[derive(QModelItem, Default, Clone)]
pub struct GraphRow {
    sha: String,
    subject: String,
    node_lane: i32,
    through_mask: i32,
    conn_from: i32,
    color_idx: i32,
}

fn gen_rows(n: usize) -> Vec<GraphRow> {
    let mut rows = Vec::with_capacity(n);
    let mut seed: u32 = 0x5eed_1234;
    let mut lane: i32 = 0;
    let mut active: i32 = 0b1111_1111;
    for i in 0..n {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let r = (seed >> 4) as i32 & 0x7fff_ffff;
        if r % 5 == 0 {
            lane = (lane + (r / 5) % 3 - 1).rem_euclid(LANES);
        }
        if r % 13 == 0 {
            active ^= 1 << ((r / 13) % LANES);
            active |= (1 << lane) | 1;
        }
        let conn_from = if r % 6 == 0 {
            (lane + 1 + (r / 6) % (LANES - 1)) % LANES
        } else {
            -1
        };
        rows.push(GraphRow {
            sha: format!("{seed:08x}"),
            subject: format!("synthetic commit #{i} — scroll benchmark row"),
            node_lane: lane,
            through_mask: active & !(1 << lane),
            conn_from,
            color_idx: (lane % 10).abs(),
        });
    }
    rows
}

pub struct GraphModel {
    rows: Vec<GraphRow>,
    pending: usize,
}

impl Default for GraphModel {
    fn default() -> Self {
        Self {
            rows: gen_rows(100_000),
            pending: 100_000,
        }
    }
}

impl QListModel for GraphModel {
    type Item = GraphRow;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&GraphRow> {
        self.rows.get(index)
    }
    fn reset_unnotified(&mut self) {
        self.rows = gen_rows(self.pending);
    }
}

#[qobject(Base = QListModel)]
impl GraphModel {
    #[qslot]
    fn regenerate(&mut self, count: i32) {
        self.pending = count.max(1) as usize;
        self.reset();
    }
}

// ---------------------------------------------------------------------------
// S5: LogModel — streams `git log` of a real repository from a worker thread,
// batches rows into the model on the UI thread.
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct LogRow {
    sha: String,
    author: String,
    subject: String,
}

#[derive(Default)]
pub struct LogModel {
    rows: Vec<LogRow>,
    staging: Arc<Mutex<Vec<LogRow>>>,
    started: Option<Instant>,
    generation: Arc<AtomicBool>, // true while a load is running (poor man's cancel)
    first_ms: i32,
    total_ms: i32,
    row_total: i32,
    loading: bool,
    status: String,
}

impl QListModel for LogModel {
    type Item = LogRow;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&LogRow> {
        self.rows.get(index)
    }
    fn reset_unnotified(&mut self) {
        self.rows.clear();
    }
}

#[qobject(Base = QListModel)]
impl LogModel {
    qproperty!("firstChunkMs", Member = first_ms, Notify = stats_changed);
    qproperty!("totalMs", Member = total_ms, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!("loading", Member = loading, Notify = stats_changed);
    qproperty!("status", Member = status, Notify = stats_changed);

    #[qsignal]
    fn stats_changed(&mut self);
    #[qsignal]
    fn finished(&mut self);

    #[qslot]
    fn load(&mut self, repo_path: String, topo: bool) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.first_ms = -1;
        self.total_ms = -1;
        self.row_total = 0;
        self.status = format!(
            "spawning git log ({}) ...",
            if topo {
                "--topo-order"
            } else {
                "default order"
            }
        );
        self.stats_changed();
        self.reset();
        self.staging.lock().map(|mut g| g.clear()).ok();
        self.started = Some(Instant::now());

        let staging = Arc::clone(&self.staging);
        let invoker = self.get_qml_method_invoker();
        std::thread::spawn(move || run_git_log(repo_path, topo, staging, invoker));
    }

    #[qslot]
    fn drain(&mut self) {
        let batch = match self.staging.lock() {
            Ok(mut g) => std::mem::take(&mut *g),
            Err(_) => return,
        };
        if batch.is_empty() {
            return;
        }
        if self.first_ms < 0 {
            if let Some(t0) = self.started {
                self.first_ms = t0.elapsed().as_millis() as i32;
            }
        }
        self.extend_notified(batch);
        self.row_total = self.rows.len() as i32;
        self.status = "streaming ...".into();
        self.stats_changed();
    }

    #[qslot]
    fn on_stream_done(&mut self, error: String) {
        self.drain();
        if let Some(t0) = self.started {
            self.total_ms = t0.elapsed().as_millis() as i32;
        }
        self.loading = false;
        self.row_total = self.rows.len() as i32;
        self.status = if error.is_empty() {
            "done".into()
        } else {
            format!("error: {error}")
        };
        self.stats_changed();
        println!(
            "PGG_SPIKE_LOG first_chunk_ms={} total_ms={} rows={} error={:?}",
            self.first_ms, self.total_ms, self.row_total, error
        );
        self.finished();
    }
}

impl LogModel {
    /// Batch append with a single begin/endInsertRows pair. qtbridge's
    /// QListModelBase only exposes single-row push/insert, so this mirrors its
    /// implementation using the public proxy API.
    fn extend_notified(&mut self, batch: Vec<LogRow>) {
        let Some(proxy) = self.try_get_rust_proxy_ptr() else {
            self.rows.extend(batch);
            return;
        };
        let first = self.rows.len() as i32;
        let last = first + batch.len() as i32 - 1;
        // SAFETY: same pattern as QListModelBase::push — the proxy pointer is
        // valid while the QObject side of this model is attached.
        unsafe { &mut *proxy }.base_begin_insert_rows(
            &mut *self,
            &QModelIndex::default(),
            first,
            last,
        );
        self.rows.extend(batch);
        // SAFETY: see above.
        unsafe { &mut *proxy }.base_end_insert_rows(&mut *self);
    }
}

fn run_git_log(
    repo: String,
    topo: bool,
    staging: Arc<Mutex<Vec<LogRow>>>,
    invoker: qtbridge::QmlMethodInvoker,
) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(&repo).args([
        "-c",
        "color.ui=false",
        "-c",
        "core.quotepath=false",
        "--no-optional-locks",
        "log",
    ]);
    if topo {
        cmd.arg("--topo-order");
    }
    cmd.arg("--format=%H%x1f%an%x1f%s");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            invoke_method!(invoker, "on_stream_done", format!("spawn failed: {e}"));
            return;
        }
    };
    let Some(stdout) = child.stdout.take() else {
        invoke_method!(invoker, "on_stream_done", "no stdout".to_string());
        return;
    };

    let reader = BufReader::with_capacity(1 << 16, stdout);
    let mut local: Vec<LogRow> = Vec::with_capacity(4096);
    let mut first_sent = false;
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let mut parts = line.splitn(3, '\u{1f}');
        let sha: String = parts.next().unwrap_or("").chars().take(8).collect();
        let author = parts.next().unwrap_or("").to_string();
        let subject = parts.next().unwrap_or("").to_string();
        local.push(LogRow {
            sha,
            author,
            subject,
        });

        let limit = if first_sent { 4000 } else { 500 };
        if local.len() >= limit {
            flush(&staging, &mut local, &invoker);
            first_sent = true;
        }
    }
    flush(&staging, &mut local, &invoker);
    let _ = child.wait();
    invoke_method!(invoker, "on_stream_done", String::new());
}

fn flush(
    staging: &Arc<Mutex<Vec<LogRow>>>,
    local: &mut Vec<LogRow>,
    invoker: &qtbridge::QmlMethodInvoker,
) {
    if local.is_empty() {
        return;
    }
    let was_empty = match staging.lock() {
        Ok(mut g) => {
            let was_empty = g.is_empty();
            g.append(local);
            was_empty
        }
        Err(_) => return,
    };
    if was_empty {
        // Edge-triggered: the UI-thread drain empties the staging buffer, so
        // one queued invocation per burst is enough.
        invoke_method!(invoker, "drain");
    }
}

// ---------------------------------------------------------------------------

fn main() {
    QApp::new()
        .register::<SpikeConfig>()
        .register::<DemoModel>()
        .register::<WorkerBackend>()
        .register::<GraphModel>()
        .register::<LogModel>()
        .load_qml(include_bytes!("Main.qml"))
        .run();
}

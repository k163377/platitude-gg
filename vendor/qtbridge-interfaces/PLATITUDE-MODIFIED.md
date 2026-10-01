# Modified copy of qtbridge-interfaces 0.3.0

This directory is `qtbridge-interfaces` 0.3.0 as published on crates.io
(upstream commit d9a89bc1767a444f280e0548edddbd5b2023104c, `crates/qtbridge-interfaces`
of https://code.qt.io/cgit/qt/qtbridge-rust.git), licensed
`LicenseRef-Qt-Commercial OR LGPL-3.0-only`; platitude-gg uses it under LGPL-3.0-only
(`LGPL-3.0-only.txt`).

**Modified by platitude-gg on 2026-10-01.** The only change is
`platitude-proxy-shared.diff` (8 files under `src/`, apply with `patch -p1` to the
published crate); every other file of the published crate is byte for byte as published.
Added beside them: `LGPL-3.0-only.txt` (from the upstream repository's `LICENSES/`), this
file, the diff, and `UPSTREAM-REPORT.md`.

What the change does: the Rust proxy of a QObject (`Q*ProxyRust`) is re-entered by Qt
through its own pointer while a call into Qt is running, so it is reached through
shared references only. Every `base_*` call, `set_data` / `remove_rows` /
`remove_columns`, and `class_begin` / `component_complete` take `&self` instead of
`&mut self` (in the Rust impls and in the CXX `extern "Rust"` declarations), and the
callers reach the proxy through `&*proxy` instead of `&mut *proxy`. The user model's own
`&mut self` and its `RefCell` checks are unchanged.

Why: with `&mut self`, an optimized build drops the borrow state the proxy stores for
Qt's re-entrant reads (`try_store_handle_and_call_cpp_mut`), and the first row notified
from a mutable slot aborts in `rowCount` with `BorrowError`
(internal-docs/P3-確認事項.md, the qtbridge entry; the report for upstream is `UPSTREAM-REPORT.md`).

Back to the published crate when a qtbridge release takes the proxies by `&self`: remove
this directory and the `[patch.crates-io]` entry in the workspace `Cargo.toml`.

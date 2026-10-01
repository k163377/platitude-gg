# Qt Bridges (Rust) — the report

Posted to the Qt Forum's Qt Bridges category: https://forum.qt.io/topic/165133 (Qt's Jira,
QTBRIDGES, was not open to us; GitHub `qt/qtbridge-rust` has issues closed).

Versions: qtbridge 0.3.0 (crates.io), unchanged on `dev` at f7fe523 when posted. Rust 1.98.1 /
Qt 6.12.0 and Rust 1.97.1 / Qt 6.10.3, Windows 11 / MSVC 14.44, `QT_QPA_PLATFORM=offscreen`.

| # | Title |
|---|---|
| 1 | `QListModelBase::push` from a mutable slot aborts in release builds (`BorrowError` in `row_count`) |
| 2 | A property with no setter is registered writable, so a QML write is silently ignored |
| 3 | A property value of a custom type that does not convert panics in `write_property` (related to QTBRIDGES-332) |

## 1. `QListModelBase::push` from a mutable slot aborts in release builds

```toml
[package]
name = "qtbridge_push"
edition = "2024"
[dependencies]
qtbridge = "=0.3.0"
```

```rust
use qtbridge::{QApp, QModelItem, qobject};

#[derive(Clone, Debug, Default, QModelItem)]
pub struct Row { value: i32 }

#[qobject(Base = QListModel, ConvertToCamelCase)]
mod backend {
    use qtbridge::{QListModel, QListModelBase};
    #[derive(Default)]
    pub struct PushModel { rows: Vec<crate::Row> }
    impl PushModel {
        #[qslot] fn push_one(&mut self) { let value = self.rows.len() as i32; self.push(crate::Row { value }); }
        #[qslot] fn count(&self) -> i32 { self.rows.len() as i32 }
    }
    impl QListModel for PushModel {
        type Item = crate::Row;
        fn len(&self) -> usize { self.rows.len() }
        fn get(&self, index: usize) -> Option<&Self::Item> { self.rows.get(index) }
        fn push_unnotified(&mut self, value: Self::Item) { self.rows.push(value); }
    }
}
pub use backend::PushModel;

fn main() {
    QApp::new().register::<PushModel>().load_qml(br#"
        import QtQuick
        import qtbridge_push
        Item {
            PushModel { id: model }
            ListView { model: model; width: 100; height: 100; delegate: Item { width: 10; height: 10 } }
            Component.onCompleted: { model.pushOne(); model.pushOne(); model.pushOne();
                                     console.log("count=" + model.count()); Qt.quit() }
        }"#).run();
}
```

- **Expected**: `qml: count=3`, exit 0.
- **Release**: aborts every run (10/10 on 1.97.1 / 6.10.3, 5/5 on 1.98.1 / 6.12.0), exit 0xC0000409 —
  `Failed to borrow for row_count: BorrowError` (`qlist_model/proxy_rust.rs:393`), then
  `panic in ffi function … QListModelProxyRust::row_count, aborting`.
- **Dev**: passes (3/3).

**Cause.** `QListModelBase::push` calls `unsafe { &mut *proxy }.base_begin_insert_rows(...)`, which
stores the slot's `RefMut` handle (`BorrowState::Mutable`) in the proxy and calls C++. Qt re-enters the
same proxy (`rowCount`) through its own pointer and reads that handle. The proxy is an `&mut self`
(`noalias`) across the call, and the optimised IR keeps no store of the handle before the C++ call,
so `row_count` finds no handle, `try_borrow` fails against the slot's `RefMut`, and the `expect`
aborts across FFI. Same class as 88105239 ("Fix aliasing of proxy references in mutable dispatch"),
which changed `invoke_slot_mut` / `write_property` only.

**Candidate fix** (`platitude-proxy-shared.diff` beside this file, 8 files): every `base_*` of
`QListModelProxyRust` / `QAbstractItemModelProxyRust` / `QTableModelProxyRust`, their `set_data` /
`remove_rows` / `remove_columns`, and `QParserStatusProxyRust::class_begin` / `component_complete`
take `&self` (with their CXX `extern "Rust"` declarations), and the model bases call the proxy
through `unsafe { &*proxy }`. The user model's own `&mut self` and `RefCell` checks are unchanged.
With it the same IR stores the handle before the call, and the repro prints `count=3` in every run
(10/10 and 5/5 on the two toolchains, the lockfile otherwise the same).

Each case from a mutable slot with a view attached, release, 3 runs each (1.98.1 / 6.12.0; dev
builds of 0.3.0 pass all but the last row):

| case | 0.3.0 | candidate (MSVC / GCC) |
|---|---|---|
| `QListModel` push, `QTableModel` push_row, `QAbstractItemModel` insert | abort in `row_count` | pass / pass |
| `QAbstractItemModel` reset, `setData`, `removeRows` | abort in `role_names` | pass / pass |
| `QParserStatus` (`componentComplete` → signal → mutable slot) | pass | pass / pass |
| a mutable slot running the event loop into another mutable slot of the same object | `BorrowMutError` | `BorrowMutError` |

The last row is the contract still enforced: an exclusive borrow nothing handed over is refused.

**Not QTBRIDGES-332.** 332 proposes how each call path treats an expired object or a failed borrow;
for the model virtuals it keeps the panic on a borrow error. Here a correct call fails its borrow
because the hand-over is lost, so 332 as proposed cannot be said to remove this abort.

## 2. A property with no setter is registered writable

`qtbridge-runtime` 0.3.0 registers every property but a `Constant` one as writable
(`dynamicmetaobjectbuilder.cpp`, `registerProperty`: `writable = !isConstant`). A
`qproperty!("x", Read = getter, Notify = ...)` has no setter, and a QML write to it reaches the
generated write arm, which does nothing: the value does not change and QML raises no error.

- **Expected**: registered read-only, so QML says `Cannot assign to read-only property "x"`, as for
  a C++ property without `WRITE`.

## 3. A custom type that does not convert panics in `write_property` (related to QTBRIDGES-332)

A related panic path that 332's description does not list (it names an expired object and borrow
errors). Offered as a case for 332's scope, not as covered by it.

- For Qt's own types (`int`, `QString`, `QStringList`) the QML engine converts or refuses the value
  (`Cannot assign …`) before Rust sees it; nothing aborts.
- For a type of one's own carried as `QVariantList` / `QVariantMap` (`QVariantConvertible`) in a
  `Member` property, a QML write whose element or field the type cannot read reaches the generated
  `write_property`, which panics inside `extern "C"` (`qtbridge-gen` `qproperty_info.rs`:
  `Failed to convert QVariant for qproperty …`) and the process aborts. 0.2 does the same.
- Upstream's tests cover the invalid-property-id panic, not a conversion failure.

# QListModelBase::push from a mutable slot aborts in optimized builds (BorrowError in row_count)

Project: QTBRIDGES / component Rust. Affects: qtbridge 0.3.0 (crates.io), `dev` at f7fe523 unchanged.

## Environment

| | |
|---|---|
| qtbridge / qtbridge-interfaces / -runtime / -gen / -type-lib | 0.3.0 (crates.io) |
| cxx / cxx-qt / cxx-qt-lib | 1.0.202 / 0.10.0 / 0.10.0 |
| Rust | 1.97.1 (8bab26f4f 2026-07-14); also 1.98.1 (48a229cea 2026-09-01) |
| Qt | 6.10.3 msvc2022_64; also 6.12.0 msvc2022_64 |
| OS / compiler | Windows 11, MSVC 14.44 (VS 2022 Build Tools), x86_64-pc-windows-msvc |
| Profile | `--release` (opt-level 3, defaults). `dev` does not reproduce. |

## Reproduction (public API only)

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

Run with `QT_QPA_PLATFORM=offscreen`.

- **Expected**: `qml: count=3`, exit 0.
- **Actual (release)**: every run aborts (10 of 10 on Rust 1.97.1 / Qt 6.10.3, 5 of 5 on
  1.98.1 / 6.12.0), exit 0xC0000409:
  ```
  thread 'main' panicked at qtbridge-interfaces-0.3.0/src/qlist_model/proxy_rust.rs:393:9:
  Failed to borrow for row_count: BorrowError(BorrowError)
  thread 'main' panicked at cxx-1.0.202/src/unwind.rs:37:9:
  panic in ffi function qtbridge_interfaces::qlist_model::proxy_rust_bridge::ffi::QListModelProxyRust::row_count, aborting.
  ```
- **Actual (dev)**: 3 of 3 runs print `count=3`.

## Borrow flow

1. QML calls `pushOne` → `QBaseProxy::invokeSlotMut` → `QListModelProxyRust::invoke_slot_mut(&self)` →
   `RustObjAccess::try_call_rust_with_handle_mut`: the borrow cell is `None`, so it takes
   `RefCell::try_borrow_mut` and runs the slot with that `RefMut` alive.
2. The slot calls `QListModelBase::push` → `unsafe { &mut *proxy }.base_begin_insert_rows(&mut *self, ..)`.
   `base_begin_insert_rows(&mut self, mut_ref, ..)` → `call_cpp_impl!(mut ..)` →
   `try_store_handle_and_call_cpp_mut`: writes `BorrowState::Mutable(mut_ref)` into
   `self.rust_obj.borrow`, calls C++, and writes the previous state back.
3. `QAbstractItemModel::beginInsertRows` → `QAbstractItemModelPrivate::rowsAboutToBeInserted` →
   `rowCount()` → `QListModelProxyRust::row_count(&self)` → `try_call_rust_with_handle` is meant to find
   `Mutable` in the cell; it finds `None`, calls `RefCell::try_borrow`, which fails against the
   `RefMut` of step 1, and the `expect` aborts across the FFI boundary.

## The reference contract problem (from the source)

`QListModelProxyRust::base_*` (and the `QAbstractItemModelProxyRust` / `QTableModelProxyRust`
equivalents, plus the `set_data` / `remove_rows` / `remove_columns` / `class_begin` /
`component_complete` entry points in the `*_proxy_rust_bridge.rs` declarations) take
`&mut self`, and `QListModelBase` and the adapters call them through `unsafe { &mut *proxy }`.
While that `&mut GenericRustProxy` is live, the same proxy is

- borrowed shared by the dispatch frame of step 1 (`invoke_slot_mut(&self)`), and
- re-entered by C++ through `m_rustProxy` (step 3), which reads `rust_obj.borrow`.

A `&mut` asserts that nothing else reads or writes the referent for its lifetime, so the
re-entrant read of `rust_obj.borrow` through another pointer violates it. This is the same class
of problem 88105239 ("Fix aliasing of proxy references in mutable dispatch") fixed for
`invoke_slot_mut` and `write_property`; the `base_*` receivers and the remaining bridge entry
points were not changed there.

## What the generated code shows

`--emit=llvm-ir` of qtbridge-interfaces 0.3.0, release, for
`GenericRustProxy<QListModelProxyCpp, dyn QListModelAdapter>::base_begin_insert_rows`:

- `%self` is `ptr noalias noundef align 8`.
- Between reading the old borrow state and the call to
  `rust$bridge$cxxbridge1$202$QListModelProxyCpp$base_begin_insert_rows` there is **no store to
  `%self`**: the `Mutable(mut_ref)` write of step 2 is gone; only the write-back of the old state
  after the call remains. With `noalias` on `%self` and `%self` not passed to the C++ call, the
  optimizer may treat that store as dead.

With the receivers changed to `&self` (below), the same function's `%self` is
`ptr noundef nonnull align 8 captures(none)` (no `noalias`), and the IR stores the discriminant
(`store i64 2`) and both halves of `mut_ref` before the call.

## Candidate fix

In `qtbridge-interfaces` (`platitude-proxy-shared.diff` beside this file, 8 files):

- every `pub fn base_*` of `QListModelProxyRust`, `QAbstractItemModelProxyRust` and
  `QTableModelProxyRust`, and their `set_data` / `remove_rows` / `remove_columns`, take `&self`
  (three of them are declared over several lines: `QAbstractItemModelProxyRust::base_begin_move_columns`,
  `base_begin_move_rows`, `QTableModelProxyRust::base_begin_move_columns`);
- `QParserStatusProxyRust::class_begin` / `component_complete` take `&self`;
- the CXX `extern "Rust"` declarations of those entry points take `&self`;
- `QListModelBase`, `QTableModelBase`, `QAbstractItemModelBase` and the adapters call the proxy
  through `unsafe { &*proxy }`.

Unchanged on purpose: the user model's own `&mut self` (the `QListModel` / adapter traits and
`RefCell` borrow checks), and `call_cpp_impl!`'s `Pin<&mut CppProxy>` on the C++ object (an opaque,
`!Unpin` CXX type whose memory Rust never touches).

## Verification

Same toolchain, Qt, lockfile (only the `qtbridge-interfaces` source differs) and inputs:

| build | Rust / Qt | runs | result |
|---|---|---|---|
| 0.3.0 release | 1.97.1 / 6.10.3 | 10 | 10 × abort, `Failed to borrow for row_count` |
| candidate release | 1.97.1 / 6.10.3 | 10 | 10 × `count=3`, exit 0 |
| 0.3.0 release | 1.98.1 / 6.12.0 | 5 | 5 × abort, `Failed to borrow for row_count` |
| candidate release | 1.98.1 / 6.12.0 | 5 | 5 × `count=3`, exit 0 |
| 0.3.0 dev | 1.97.1 / 6.10.3 | 3 | 3 × `count=3` |

No diagnostic output was added to either build that was compared.

Across the model kinds and entry points, each from a mutable slot with a view attached (Rust 1.98.1,
Qt 6.12.0, offscreen, 3 runs each):

| case | 0.3.0 release (MSVC) | 0.3.0 dev (MSVC) | candidate release (MSVC / GCC) |
|---|---|---|---|
| `QListModel` push | abort, `row_count` | pass | pass / pass |
| `QAbstractItemModel` insert | abort, `row_count` | pass | pass / pass |
| `QAbstractItemModel` reset, `setData`, `removeRows` | abort, `role_names` | pass | pass / pass |
| `QTableModel` push_row | abort, `row_count` | pass | pass / pass |
| `QParserStatus` (`componentComplete` → signal → mutable slot) | pass | pass | pass / pass |
| a mutable slot that runs the event loop, which runs another mutable slot of the same object | `BorrowMutError` | `BorrowMutError` | `BorrowMutError` / `BorrowMutError` |

The last row is the contract still enforced: a second exclusive borrow of the user object that
nothing handed over is refused with the candidate as without it.

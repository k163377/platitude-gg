//! How a typed Rust value reaches QML as a JS value, and comes back.
//!
//! One record is a `QVariantMap` of named fields — a JS object on the
//! other side — and a list of them is a `QVariantList`, a JS array. The
//! map is built and read in exactly one place per type (its [`Record`]
//! impl), so a field added, renamed or retyped is a compile error here
//! and a named key there.
//!
//! **The Qt value is built when it is read**, not held beside the Rust
//! value: a read is bounded by the delegates on screen, while holding it
//! stands the whole window's lanes and chips on Qt's heap, past the
//! memory budget (ci/baseline/code-costs-windows-x64.md §メモリの形).
//!
//! Three wrappers — [`Listed<T>`], [`One<T>`], [`Optional<T>`] — each a
//! model role (`QVariantConvertible`) and a `#[qslot]` argument or answer
//! (`QMetaTypeCompatible`). qtbridge implements those two for its own
//! containers only, and a property (`QPropertyMember`) comes with them
//! for any type that is also `PartialEq` — which is why [`Optional`] is
//! not.

use qtbridge::qtbridge_runtime::QMetaTypeGet;
use qtbridge::qtbridge_type_lib::{
    QMetaType, QMetaTypeType, QString, QVariant, QVariantList, QVariantMap,
};
use qtbridge::{QMetaTypeCompatible, QVariantConvertible};

/// A record with a JS object's shape: named fields, each read by QML as
/// its own type.
pub trait Record: Sized {
    fn to_map(&self) -> QVariantMap;
    /// The inverse. A missing or mistyped field is `Err`.
    fn from_map(map: &QVariantMap) -> Result<Self, ()>;
}

/// What one field of a record may hold, as QML reads it.
pub(crate) trait FieldValue {
    fn variant(&self) -> QVariant;
}

/// The inverse of [`FieldValue`]: a value of another type that does not
/// convert is `Err` (`QVariant::canConvert`, then the value).
pub(crate) trait FieldRead: Sized {
    fn read(variant: &QVariant) -> Result<Self, ()>;
}

/// Every type the bridge itself carries by value, and the three wrappers.
macro_rules! field_by_bridge {
    ($([$($generic:tt)*] $ty:ty),* $(,)?) => {
        $(impl<$($generic)*> FieldValue for $ty {
            fn variant(&self) -> QVariant {
                QVariantConvertible::to_qvariant(self)
            }
        }

        impl<$($generic)*> FieldRead for $ty {
            fn read(variant: &QVariant) -> Result<Self, ()> {
                QVariantConvertible::try_from_qvariant(variant)
            }
        })*
    };
}
field_by_bridge!(
    [] bool,
    [] i32,
    [] u32,
    [] i64,
    [] u64,
    [] f64,
    [] Vec<String>,
    [] Vec<i32>,
    [T: Record] Listed<T>,
    [T: Record] One<T>,
    [T: Record] Optional<T>,
);

impl FieldValue for String {
    fn variant(&self) -> QVariant {
        QVariantConvertible::to_qvariant(self)
    }
}

/// Read by [`text_of`], not the bridge's own conversion: records come back
/// from QML a field at a time (a graph row's chips, on every delegate).
impl FieldRead for String {
    fn read(variant: &QVariant) -> Result<Self, ()> {
        variant
            .value::<QString>()
            .map(|text| text_of(&text))
            .ok_or(())
    }
}

/// A `QString` as a `String`, allocated once at its UTF-8 length. The
/// bridge's conversion (`String::from(&QString)` = `from_utf16_lossy`)
/// grows its buffer as it decodes, from a guess of half the UTF-16 length:
/// more than one allocation for most strings, several for text that is not
/// ASCII (ci/baseline/code-costs-windows-x64.md §橋の値の組み立て). An
/// unpaired surrogate reads as U+FFFD, as there.
fn text_of(text: &QString) -> String {
    let units = text.as_slice();
    let decoded = || {
        char::decode_utf16(units.iter().copied())
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
    };
    let mut out = String::with_capacity(decoded().map(char::len_utf8).sum());
    out.extend(decoded());
    out
}

impl FieldValue for str {
    fn variant(&self) -> QVariant {
        QVariant::from(&QString::from(self))
    }
}

/// A record nested in a record (`encode::Marks`).
impl FieldValue for QVariantMap {
    fn variant(&self) -> QVariant {
        QVariant::from(self)
    }
}

impl FieldRead for QVariantMap {
    fn read(variant: &QVariant) -> Result<Self, ()> {
        variant.value::<QVariantMap>().ok_or(())
    }
}

pub(crate) struct Fields(QVariantMap);

impl Fields {
    pub(crate) fn new() -> Self {
        Self(QVariantMap::default())
    }

    pub(crate) fn put<V: FieldValue + ?Sized>(mut self, key: &'static str, value: &V) -> Self {
        // The value first: a nested record looks its own keys up.
        let value = value.variant();
        with_key(key, |key| self.0.insert_clone(key, &value));
        self
    }

    pub(crate) fn done(self) -> QVariantMap {
        self.0
    }
}

/// One named field read back. An absent one is `Err`.
pub(crate) fn field<T: FieldRead>(map: &QVariantMap, key: &'static str) -> Result<T, ()> {
    // Read once the key is let go of: a nested record looks its own up.
    let value = with_key(key, |key| map.get(key)).ok_or(())?;
    T::read(&value)
}

/// Whether the record has a field `key` holding something — what a field
/// that may be left out is told apart by.
pub(crate) fn has_field(map: &QVariantMap, key: &'static str) -> bool {
    with_key(key, |key| map.get(key)).is_some_and(|value| value.is_valid())
}

/// `use_key` handed a field's name as Qt keys it, made once per name and
/// thread and lent after. Records are built on every read, from the same
/// few dozen names, and turning each to UTF-16 on every build is a visible
/// share of a row's (ci/baseline/code-costs-windows-x64.md §橋の値の組み立て).
/// The name is lent, not copied: Qt's map takes its own copy, so a copy
/// here is one more pair of calls across the bridge per field. Lent for
/// `use_key` alone, which must not come back here — a nested record
/// builds and reads its fields outside it ([`Fields::put`], [`field`]).
fn with_key<R>(name: &'static str, use_key: impl FnOnce(&QString) -> R) -> R {
    thread_local! {
        static KEYS: std::cell::RefCell<std::collections::HashMap<(usize, usize), QString>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    // By where the literal lives and how long it is, never by hashing its
    // text: a name is one string in the image, and a name stored twice
    // merely takes two entries.
    KEYS.with_borrow_mut(|keys| {
        use_key(
            keys.entry((name.as_ptr().addr(), name.len()))
                .or_insert_with(|| QString::from(name)),
        )
    })
}

// ---------------------------------------------------------------------------

/// The C++ value a `#[qslot]` takes or answers for a wrapper, as the
/// bridge reads and writes it in the call's argument array: laid out as
/// the Qt type itself (`repr(transparent)` over cxx-qt-lib's layout of
/// it), under the metatype Qt knows that type by.
macro_rules! slot_wire {
    ($(#[$doc:meta])* $name:ident($inner:ty) = $metatype:ident) => {
        $(#[$doc])*
        #[repr(transparent)]
        pub struct $name($inner);

        impl QMetaTypeGet for $name {
            fn get_qmetatype() -> QMetaType {
                QMetaType::new(i32::from(QMetaTypeType::$metatype))
            }
        }
    };
}

slot_wire!(
    /// [`Listed`] in a slot: a `QVariantList`.
    ListWire(QVariantList) = QVariantList
);
slot_wire!(
    /// [`One`] in a slot: a `QVariantMap`.
    MapWire(QVariantMap) = QVariantMap
);
slot_wire!(
    /// [`Optional`] in a slot: a `QVariant`, invalid for nothing.
    VariantWire(QVariant) = QVariant
);

// ---------------------------------------------------------------------------

/// A list of records: a JS array of objects on the other side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed<T> {
    items: Vec<T>,
}

impl<T> Default for Listed<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Listed<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }
}

impl<T: Record> Listed<T> {
    /// Built for this read (see the module).
    fn to_list(&self) -> QVariantList {
        let mut list = QVariantList::default();
        list.reserve(isize::try_from(self.items.len()).unwrap_or(isize::MAX));
        for record in &self.items {
            list.append(QVariant::from(&record.to_map()));
        }
        list
    }

    /// A missing or mistyped field in any element is `Err`.
    fn read(list: &QVariantList) -> Result<Vec<T>, ()> {
        let mut out = Vec::with_capacity(usize::try_from(list.len()).unwrap_or(0));
        for element in list {
            out.push(T::from_map(&element.value::<QVariantMap>().ok_or(())?)?);
        }
        Ok(out)
    }
}

impl<T> std::ops::Deref for Listed<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.items
    }
}

impl<T: Record> QVariantConvertible for Listed<T> {
    fn to_qvariant(&self) -> QVariant {
        QVariant::from(&self.to_list())
    }

    fn try_from_qvariant(value: &QVariant) -> Result<Self, ()> {
        Ok(Self::new(Self::read(
            &value.value::<QVariantList>().ok_or(())?,
        )?))
    }
}

impl<T: Record> QMetaTypeCompatible for Listed<T> {
    type CompatibleType = ListWire;

    fn to_compatible(&self) -> ListWire {
        ListWire(self.to_list())
    }

    /// An element that does not read is no shape this side hands out: it
    /// is logged and left out rather than taking the call down.
    fn from_compatible(wire: &ListWire) -> Self {
        let mut out = Vec::with_capacity(usize::try_from(wire.0.len()).unwrap_or(0));
        for (i, element) in wire.0.iter().enumerate() {
            match element
                .value::<QVariantMap>()
                .and_then(|map| T::from_map(&map).ok())
            {
                Some(record) => out.push(record),
                None => tracing::warn!(
                    index = i,
                    record = std::any::type_name::<T>(),
                    "a record handed back from QML did not read"
                ),
            }
        }
        Self::new(out)
    }
}

impl<T: platitude_core::mem::Footprint> platitude_core::mem::Footprint for Listed<T> {
    fn heap_bytes(&self) -> usize {
        self.items.heap_bytes()
    }
}

// ---------------------------------------------------------------------------

/// One record, always there: a JS object on the other side.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct One<T> {
    value: T,
}

impl<T> One<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> std::ops::Deref for One<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: Record> QVariantConvertible for One<T> {
    fn to_qvariant(&self) -> QVariant {
        QVariant::from(&self.value.to_map())
    }

    fn try_from_qvariant(value: &QVariant) -> Result<Self, ()> {
        Ok(Self::new(T::from_map(
            &value.value::<QVariantMap>().ok_or(())?,
        )?))
    }
}

impl<T: Record + Default> QMetaTypeCompatible for One<T> {
    type CompatibleType = MapWire;

    fn to_compatible(&self) -> MapWire {
        MapWire(self.value.to_map())
    }

    /// A record that does not read is logged and answered as the default
    /// (as for [`Listed`]).
    fn from_compatible(wire: &MapWire) -> Self {
        T::from_map(&wire.0).map_or_else(
            |()| {
                tracing::warn!(
                    record = std::any::type_name::<T>(),
                    "a record handed back from QML did not read"
                );
                Self::default()
            },
            Self::new,
        )
    }
}

impl<T: platitude_core::mem::Footprint> platitude_core::mem::Footprint for One<T> {
    fn heap_bytes(&self) -> usize {
        self.value.heap_bytes()
    }
}

// ---------------------------------------------------------------------------

/// One record or nothing: the JS object, or `undefined` (an invalid
/// `QVariant`). The reader tests it for truth and nothing else.
///
/// **A role or a slot's answer, never a property**: a property read
/// converts to its declared metatype (`handleMetaCallReadProperty`), and
/// an invalid value ends the process with "Property type mismatch". The
/// bridge makes a property of any `PartialEq` type that crosses by value,
/// so this one is not `PartialEq` — compare what it holds (`*a == *b`).
/// A property that may hold nothing is a revision property beside a slot
/// answering this (`RepoTab.remoteBranchRevision` / `remoteBranchAsked()`).
#[derive(Debug, Clone)]
pub struct Optional<T> {
    value: Option<T>,
}

impl<T> Optional<T> {
    pub fn new(value: Option<T>) -> Self {
        Self { value }
    }

    pub fn some(value: T) -> Self {
        Self::new(Some(value))
    }

    pub fn none() -> Self {
        Self::new(None)
    }
}

impl<T> std::ops::Deref for Optional<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Option<T> {
        &self.value
    }
}

impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self::none()
    }
}

impl<T: Record> QVariantConvertible for Optional<T> {
    fn to_qvariant(&self) -> QVariant {
        self.value
            .as_ref()
            .map_or_else(QVariant::default, |v| QVariant::from(&v.to_map()))
    }

    /// An invalid value is "nothing", not a refusal.
    fn try_from_qvariant(value: &QVariant) -> Result<Self, ()> {
        if !value.is_valid() {
            return Ok(Self::none());
        }
        Ok(Self::some(T::from_map(
            &value.value::<QVariantMap>().ok_or(())?,
        )?))
    }
}

impl<T: Record> QMetaTypeCompatible for Optional<T> {
    type CompatibleType = VariantWire;

    fn to_compatible(&self) -> VariantWire {
        VariantWire(QVariantConvertible::to_qvariant(self))
    }

    fn from_compatible(wire: &VariantWire) -> Self {
        QVariantConvertible::try_from_qvariant(&wire.0).unwrap_or_else(|()| {
            tracing::warn!(
                record = std::any::type_name::<T>(),
                "a record handed back from QML did not read"
            );
            Self::none()
        })
    }
}

impl<T: platitude_core::mem::Footprint> platitude_core::mem::Footprint for Optional<T> {
    fn heap_bytes(&self) -> usize {
        self.value.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct Probe {
        word: String,
        count: i32,
        on: bool,
    }

    impl Record for Probe {
        fn to_map(&self) -> QVariantMap {
            Fields::new()
                .put("word", &self.word)
                .put("count", &self.count)
                .put("on", &self.on)
                .done()
        }

        fn from_map(map: &QVariantMap) -> Result<Self, ()> {
            Ok(Self {
                word: field(map, "word")?,
                count: field(map, "count")?,
                on: field(map, "on")?,
            })
        }
    }

    fn probes() -> Vec<Probe> {
        vec![
            Probe {
                word: "main".into(),
                count: 3,
                on: true,
            },
            Probe {
                word: String::new(),
                count: -1,
                on: false,
            },
        ]
    }

    /// What the bridge does with a slot's argument: the C++ value in
    /// place, read through the layout of [`ListWire`] and friends.
    fn through_a_slot<W: QMetaTypeCompatible>(value: &W) -> W {
        W::from_compatible(&value.to_compatible())
    }

    /// By the same conversions the bridge runs for a role, a property and a slot.
    #[test]
    fn a_list_of_records_round_trips_through_qt_containers() {
        let listed = Listed::new(probes());
        let variant = listed.to_qvariant();
        assert_eq!(
            Listed::<Probe>::try_from_qvariant(&variant),
            Ok(listed.clone())
        );
        assert_eq!(through_a_slot(&listed), listed);
        assert_eq!(
            Listed::<Probe>::try_from_qvariant(&Listed::<Probe>::default().to_qvariant()),
            Ok(Listed::default())
        );
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].word, "main");
    }

    /// A copy of a `QList<QVariant>` is the list's own copy constructor —
    /// not brace-initialisation, which GCC resolves to the
    /// `initializer_list` one: a one-element list holding the original.
    /// Run under GCC too (`cargo xtask linux test -p platitude-app`).
    #[test]
    fn a_copied_list_of_records_keeps_every_element() {
        let list = Listed::new(probes()).to_list();
        let copied = list.clone();
        assert_eq!(copied.len(), 2);
        let first = copied
            .get(0)
            .and_then(|element| element.value::<QVariantMap>())
            .map(|map| Probe::from_map(&map));
        assert_eq!(first, Some(Ok(probes().remove(0))));
        assert_eq!(Listed::<Probe>::read(&copied), Ok(probes()));
    }

    #[test]
    fn one_record_and_an_optional_one_round_trip() {
        let one = One::new(probes().remove(0));
        assert_eq!(
            One::<Probe>::try_from_qvariant(&one.to_qvariant()),
            Ok(one.clone())
        );
        assert_eq!(through_a_slot(&one), one);
        assert_eq!(one.word, "main");

        let some = Optional::some(probes().remove(1));
        let back = Optional::<Probe>::try_from_qvariant(&some.to_qvariant()).map(|o| o.value);
        assert_eq!(back, Ok(Some(probes().remove(1))));
        assert_eq!(*through_a_slot(&some), *some);
        assert!(some.is_some());
        let none = Optional::<Probe>::none();
        assert!(!none.to_qvariant().is_valid());
        let back = Optional::<Probe>::try_from_qvariant(&none.to_qvariant()).map(|o| o.value);
        assert_eq!(back, Ok(None));
        assert_eq!(
            *Optional::<Probe>::from_compatible(&VariantWire(QVariant::default())),
            None
        );
    }

    #[test]
    fn a_missing_field_is_a_refusal_and_not_a_default() {
        let short = Fields::new().put("word", "x").done();
        assert_eq!(Probe::from_map(&short), Err(()));
        let mut list = QVariantList::default();
        list.append(QVariant::from(&short));
        assert_eq!(Listed::<Probe>::read(&list), Err(()));
        // The slot road leaves such an element out instead.
        assert_eq!(Listed::<Probe>::from_compatible(&ListWire(list)).len(), 0);
        assert_eq!(
            One::<Probe>::from_compatible(&MapWire(short.clone())),
            One::default()
        );
        assert_eq!(
            *Optional::<Probe>::from_compatible(&VariantWire(QVariant::from(&short))),
            None
        );
    }

    /// A property can be any type the bridge carries by value that is also
    /// `PartialEq` (qtbridge's blanket `QPropertyMember`); `Optional` must
    /// stay out, since an invalid value read as a property ends the
    /// process. A `PartialEq` added to it stops the test build.
    #[test]
    fn of_the_three_only_optional_is_no_property() {
        struct Is<T>(std::marker::PhantomData<T>);
        trait NoProperty {
            const PROPERTY: bool = false;
        }
        impl<T> NoProperty for Is<T> {}
        impl<T: qtbridge::QPropertyMember> Is<T> {
            const PROPERTY: bool = true;
        }
        const {
            assert!(Is::<Listed<Probe>>::PROPERTY);
            assert!(Is::<One<Probe>>::PROPERTY);
            assert!(!Is::<Optional<Probe>>::PROPERTY);
        }
    }

    /// Records inside a record: built and read back while the outer one is
    /// mid-way through its own fields, so no key may still be lent.
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Nest {
        name: String,
        inner: Listed<Probe>,
        one: One<Probe>,
    }

    impl Record for Nest {
        fn to_map(&self) -> QVariantMap {
            Fields::new()
                .put("name", &self.name)
                .put("inner", &self.inner)
                .put("one", &self.one)
                .done()
        }

        fn from_map(map: &QVariantMap) -> Result<Self, ()> {
            Ok(Self {
                name: field(map, "name")?,
                inner: field(map, "inner")?,
                one: field(map, "one")?,
            })
        }
    }

    #[test]
    fn a_record_inside_a_record_builds_and_reads_back() {
        let nest = Nest {
            name: "outer".into(),
            inner: Listed::new(probes()),
            one: One::new(probes().remove(0)),
        };
        assert_eq!(Nest::from_map(&nest.to_map()), Ok(nest.clone()));
        let nested = Listed::new(vec![nest.clone(), Nest::default()]);
        assert_eq!(through_a_slot(&nested), nested);
    }

    /// What the bridge's own conversion answers, for every kind of text:
    /// one allocation is the only difference.
    #[test]
    fn a_string_field_reads_as_the_bridge_would_read_it() {
        for text in [
            QString::from(""),
            QString::from("main"),
            QString::from("修正: レーンを保つ"),
            QString::from("emoji 🙂 and ü"),
        ] {
            let read = String::read(&QVariant::from(&text));
            assert_eq!(read, Ok(String::from(&text)));
            assert_eq!(read.map(|s| s.capacity() == s.len()), Ok(true), "{text}");
        }
        assert_eq!(String::read(&QVariant::default()), Err(()));
    }

    /// The slot wires are read and written in place of the Qt value, so
    /// each must be exactly its size and carry the metatype Qt gives it.
    #[test]
    fn each_slot_wire_is_its_qt_type() {
        use std::mem::size_of;
        assert_eq!(size_of::<ListWire>(), size_of::<QVariantList>());
        assert_eq!(size_of::<MapWire>(), size_of::<QVariantMap>());
        assert_eq!(size_of::<VariantWire>(), size_of::<QVariant>());
        assert_eq!(ListWire::get_qmetatype().name(), "QVariantList");
        assert_eq!(MapWire::get_qmetatype().name(), "QVariantMap");
        assert_eq!(VariantWire::get_qmetatype().name(), "QVariant");
    }
}

//! How a typed Rust value reaches QML as a JS value, and comes back.
//!
//! One record is a `QVariantMap` of named fields — a JS object on the
//! other side — and a list of them is a `QVariantList`, a JS array. The
//! struct stays the struct on this side; the map is built and read in
//! exactly one place per type (its [`Record`] impl), so a field that is
//! added, renamed or retyped is a compile error here and a named key
//! there, never a seat counted by hand.
//!
//! **The Qt value is built when it is read**, not held beside the Rust
//! value. A read is bounded by the delegates on screen — a row's roles
//! are asked for when its delegate is made or reused, not per frame —
//! so what building costs is a few maps per visible row. Holding the Qt
//! value instead was measured and refused: on the benchmark corpus it
//! stood the whole window's lanes and chips on Qt's heap, past the
//! memory budget, for no frame gained
//! (ci/baseline/code-costs-windows-x64.md §メモリの形).
//!
//! Three wrappers, one for each shape a value takes, and each of them is
//! a model role (`QVariantConvertible`) and a `#[qslot]` argument or
//! answer (`QMetaCallArg`); the two that are always something are a
//! `qproperty!` member too (`QPropertyMember`) — qtbridge implements
//! those three for its own containers only:
//! - [`Listed<T>`]: a list of records, a JS array of objects.
//! - [`One<T>`]: one record, a JS object.
//! - [`Optional<T>`]: one record or nothing — the JS object, or
//!   `undefined`. **Nothing is falsy** on the other side: a reader asks
//!   `if (!x)` and never compares against `""` or `null`.
//!
//! Plain lists of strings and numbers need none of this: `Vec<String>`
//! and `Vec<i32>` are qtbridge's own and arrive as JS arrays.
//!
//! No bare `QList<QVariant>` is held or copied on this side: the type
//! lib's copy of one (`QList_Clone`, `return { src }`) comes back under
//! GCC as a list of one element holding the whole list — measured in the
//! Linux container, where every chip read as `undefined`; MSVC copies it.
//! A list is built, handed to Qt, and dropped.

use qtbridge::qtbridge_type_lib::{
    QMetaType, QMetaTypeGet, QString, QVariant, QVariantList, QVariantMap,
};
use qtbridge::{QMetaCallArg, QObjectHolder, QPropertyMember};

/// A record with a JS object's shape: named fields, each a value QML
/// reads as its own type (`string` / `int` / `bool` / a nested record or
/// list).
pub trait Record: Sized {
    fn to_map(&self) -> QVariantMap;
    /// The inverse, for a value QML hands back (a slot argument) and for
    /// the round-trip tests. A missing or mistyped field is `Err`.
    fn from_map(map: &QVariantMap) -> Result<Self, ()>;
}

/// The fields of one record, built in the order they are named.
pub(crate) struct Fields(QVariantMap);

impl Fields {
    pub(crate) fn new() -> Self {
        Self(QVariantMap::default())
    }

    /// One named field. `str`, `String`, the integers, `bool`, a
    /// `Vec<String>`, and the three wrappers all convert; a type that
    /// does not is a compile error at the call.
    pub(crate) fn put<V: ?Sized>(mut self, key: &str, value: &V) -> Self
    where
        for<'a> QVariant: From<&'a V>,
    {
        self.0.insert(&QString::from(key), &QVariant::from(value));
        self
    }

    pub(crate) fn done(self) -> QVariantMap {
        self.0
    }
}

/// One named field read back, as the type the reader names. A field that
/// is absent reads as an invalid `QVariant`, which no conversion accepts.
pub(crate) fn field<T>(map: &QVariantMap, key: &str) -> Result<T, ()>
where
    for<'a> T: TryFrom<&'a QVariant, Error = ()>,
{
    T::try_from(&map.value(&QString::from(key)))
}

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
    /// The list as Qt takes it, built for this read (see the module).
    fn to_list(&self) -> QVariantList {
        let mut list = QVariantList::default();
        list.reserve(self.items.len());
        for record in &self.items {
            list.push_back(QVariant::from(&record.to_map()));
        }
        list
    }

    /// The items read back out of a list QML handed over. A missing or
    /// mistyped field in any element is `Err`.
    fn read(list: &QVariantList) -> Result<Vec<T>, ()> {
        let mut out = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            out.push(T::from_map(&QVariantMap::try_from(&list[i])?)?);
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

impl<T: Record> From<&Listed<T>> for QVariant {
    fn from(value: &Listed<T>) -> Self {
        QVariant::from(&value.to_list())
    }
}

impl<T: Record> TryFrom<&QVariant> for Listed<T> {
    type Error = ();

    fn try_from(value: &QVariant) -> Result<Self, ()> {
        Ok(Self::new(Self::read(&QVariantList::try_from(value)?)?))
    }
}

impl<T: Record + PartialEq> QPropertyMember for Listed<T> {
    fn qmetatype() -> QMetaType {
        <QVariantList as QMetaTypeGet>::get_qmetatype()
    }

    fn to_qvariant<Owner: QObjectHolder>(&self, _owner: &Owner) -> QVariant {
        QVariant::from(self)
    }

    fn from_qvariant(value: &QVariant) -> Result<Self, ()> {
        Self::try_from(value)
    }

    fn property_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: Record> QMetaCallArg for Listed<T> {
    type WireType = QVariantList;

    fn to_wire(&self) -> QVariantList {
        self.to_list()
    }

    /// A list QML hands back is one this side handed out, so an element
    /// that does not read is not a shape this program produces; it is
    /// left out rather than taking the call down with it, and said out
    /// loud so a chip that went missing has a line in the log.
    fn from_wire(wire: &QVariantList) -> Self {
        let mut out = Vec::with_capacity(wire.len());
        for i in 0..wire.len() {
            match QVariantMap::try_from(&wire[i])
                .ok()
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

    fn wire_metatype() -> QMetaType {
        <QVariantList as QMetaTypeGet>::get_qmetatype()
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

impl<T: Record> From<&One<T>> for QVariant {
    fn from(value: &One<T>) -> Self {
        QVariant::from(&value.to_map())
    }
}

impl<T: Record> TryFrom<&QVariant> for One<T> {
    type Error = ();

    fn try_from(value: &QVariant) -> Result<Self, ()> {
        Ok(Self::new(T::from_map(&QVariantMap::try_from(value)?)?))
    }
}

impl<T: Record + PartialEq> QPropertyMember for One<T> {
    fn qmetatype() -> QMetaType {
        <QVariantMap as QMetaTypeGet>::get_qmetatype()
    }

    fn to_qvariant<Owner: QObjectHolder>(&self, _owner: &Owner) -> QVariant {
        QVariant::from(self)
    }

    fn from_qvariant(value: &QVariant) -> Result<Self, ()> {
        Self::try_from(value)
    }

    fn property_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: Record + Default> QMetaCallArg for One<T> {
    type WireType = QVariantMap;

    fn to_wire(&self) -> QVariantMap {
        self.value.to_map()
    }

    /// A record QML hands back is one this side handed out (the note on
    /// [`Listed`]): one that does not read is logged and answered as the
    /// record's default.
    fn from_wire(wire: &QVariantMap) -> Self {
        T::from_map(wire).map_or_else(
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

    fn wire_metatype() -> QMetaType {
        <QVariantMap as QMetaTypeGet>::get_qmetatype()
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
/// **A role or a slot's answer, never a property.** Both hand the
/// `QVariant` to Qt as it is, and an invalid one reads as `undefined`. A
/// property is read through `QMetaType::convert` against the metatype it
/// was declared with (`dynamicmetaobjectdata.cpp`,
/// `handleMetaCallReadProperty`), and an invalid value converts to
/// nothing at all — the process ends with "Property type mismatch". So
/// this type has no [`QPropertyMember`], and a property that may hold
/// nothing is a revision property beside a slot that answers with this
/// (`RepoTab.remoteBranchRevision` / `remoteBranchAsked()`).
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl<T: Record> From<&Optional<T>> for QVariant {
    fn from(value: &Optional<T>) -> Self {
        value
            .value
            .as_ref()
            .map_or_else(QVariant::default, |v| QVariant::from(&v.to_map()))
    }
}

impl<T: Record> TryFrom<&QVariant> for Optional<T> {
    type Error = ();

    /// An invalid value is "nothing", not a refusal.
    fn try_from(value: &QVariant) -> Result<Self, ()> {
        if !value.is_valid() {
            return Ok(Self::none());
        }
        Ok(Self::some(T::from_map(&QVariantMap::try_from(value)?)?))
    }
}

impl<T: Record> QMetaCallArg for Optional<T> {
    type WireType = QVariant;

    fn to_wire(&self) -> QVariant {
        QVariant::from(self)
    }

    fn from_wire(wire: &QVariant) -> Self {
        Self::try_from(wire).unwrap_or_else(|()| {
            tracing::warn!(
                record = std::any::type_name::<T>(),
                "a record handed back from QML did not read"
            );
            Self::none()
        })
    }

    fn wire_metatype() -> QMetaType {
        <QVariant as QMetaTypeGet>::get_qmetatype()
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

    /// The witness outside the bookkeeping: what went into Qt's own
    /// containers comes back out of them equal, through the same
    /// conversions the bridge runs for a role, a property and a slot.
    #[test]
    fn a_list_of_records_round_trips_through_qt_containers() {
        let listed = Listed::new(probes());
        let variant = QVariant::from(&listed);
        assert_eq!(Listed::<Probe>::try_from(&variant), Ok(listed.clone()));
        assert_eq!(
            <Listed<Probe> as QMetaCallArg>::from_wire(&listed.to_wire()),
            listed
        );
        assert_eq!(
            Listed::<Probe>::try_from(&QVariant::from(&Listed::<Probe>::default())),
            Ok(Listed::default())
        );
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].word, "main");
    }

    #[test]
    fn one_record_and_an_optional_one_round_trip() {
        let one = One::new(probes().remove(0));
        assert_eq!(
            One::<Probe>::try_from(&QVariant::from(&one)),
            Ok(one.clone())
        );
        assert_eq!(<One<Probe> as QMetaCallArg>::from_wire(&one.to_wire()), one);
        assert_eq!(one.word, "main");

        let some = Optional::some(probes().remove(1));
        assert_eq!(
            Optional::<Probe>::try_from(&QVariant::from(&some)),
            Ok(some.clone())
        );
        assert!(some.is_some());
        let none = Optional::<Probe>::none();
        // Nothing goes over as an invalid variant — `undefined` on the
        // other side — and comes back as nothing.
        assert!(!QVariant::from(&none).is_valid());
        assert_eq!(
            Optional::<Probe>::try_from(&QVariant::from(&none)),
            Ok(none)
        );
        assert_eq!(
            <Optional<Probe> as QMetaCallArg>::from_wire(&QVariant::default()),
            Optional::none()
        );
    }

    #[test]
    fn a_missing_field_is_a_refusal_and_not_a_default() {
        let short = Fields::new().put("word", "x").done();
        assert_eq!(Probe::from_map(&short), Err(()));
        let mut list = QVariantList::default();
        list.push_back(QVariant::from(&short));
        assert_eq!(Listed::<Probe>::read(&list), Err(()));
        // The slot road leaves such an element out instead.
        assert_eq!(<Listed<Probe> as QMetaCallArg>::from_wire(&list).len(), 0);
        assert_eq!(
            <One<Probe> as QMetaCallArg>::from_wire(&short),
            One::default()
        );
        assert_eq!(
            <Optional<Probe> as QMetaCallArg>::from_wire(&QVariant::from(&short)),
            Optional::none()
        );
    }
}

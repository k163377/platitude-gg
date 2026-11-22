//! The metatypes of the Qt types a slot wire is laid out as
//! (`wire::ListWire` and its two siblings), asked of Qt (`metatype.h`):
//! qtbridge names Qt's own types only for its own containers, and makes no
//! `QMetaType` out of a type's id.

#![expect(
    unsafe_code,
    reason = "the CXX bridge to QMetaType::fromType is unsafe code of its own"
)]

pub(super) use ffi::{variant, variant_list, variant_map};

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("platitude-app/src/encode/metatype.h");
        type QMetaType = qtbridge::qtbridge_type_lib::QMetaType;
    }

    #[namespace = "platitude::wire"]
    unsafe extern "C++" {
        #[rust_name = "variant_list"]
        fn variantListMetaType() -> QMetaType;
        #[rust_name = "variant_map"]
        fn variantMapMetaType() -> QMetaType;
        #[rust_name = "variant"]
        fn variantMetaType() -> QMetaType;
    }
}

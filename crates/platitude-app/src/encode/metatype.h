// What `encode::metatype` asks of Qt: the metatypes of the Qt types a slot
// wire is laid out as (`encode::wire`).
#pragma once

#include <QtCore/QMetaType>
#include <QtCore/QVariant>

#include <type_traits>

#include "rust/cxx.h"

// The same specialisation qtbridge-type-lib's `qmetatype.h` declares for the
// `QMetaType` it hands Rust by value. Its header is not on this crate's
// include path, and a specialisation has to be seen in every translation unit
// that uses it.
namespace rust {
template <>
struct IsRelocatable<::QMetaType> : ::std::true_type
{
};
} // namespace rust

namespace platitude::wire {

inline QMetaType variantListMetaType()
{
    return QMetaType::fromType<QVariantList>();
}

inline QMetaType variantMapMetaType()
{
    return QMetaType::fromType<QVariantMap>();
}

inline QMetaType variantMetaType()
{
    return QMetaType::fromType<QVariant>();
}

} // namespace platitude::wire

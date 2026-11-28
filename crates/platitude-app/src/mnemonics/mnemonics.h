// What `mnemonics` asks of Qt: that an `&` in a control's text be no key.
#pragma once

#include <QtGui/qtguiglobal.h>

QT_BEGIN_NAMESPACE
// Qt Gui exports it and documents it beside QKeySequence; qkeysequence.h
// declares it for the docs alone.
Q_GUI_EXPORT void qt_set_sequence_auto_mnemonic(bool b);
QT_END_NAMESPACE

namespace platitude::mnemonics {

inline void turnOff()
{
    QT_PREPEND_NAMESPACE(qt_set_sequence_auto_mnemonic)(false);
}

} // namespace platitude::mnemonics

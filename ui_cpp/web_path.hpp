#pragma once

#include <QString>
#include <QStringList>
#include <functional>

namespace melyar {

/// The candidate paths for the web bundle, in priority order.
///
/// The list is split out from the resolution so the order can be tested
/// without creating any files on disk.
QStringList webPathCandidates(const QString &currentDir, const QString &appDir);

/// Resolve the URL of the web bundle.
///
/// `devUrl` wins when it is not empty. Otherwise the first candidate for
/// which `exists` returns true is returned as a file URL. When none exists,
/// the first candidate is returned as a file URL so the caller still has a
/// deterministic path to report.
QString resolveWebUrl(
    const QString &devUrl,
    const QStringList &candidates,
    const std::function<bool(const QString &)> &exists);

} // namespace melyar

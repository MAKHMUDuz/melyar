#include "web_path.hpp"

#include <QFileInfo>
#include <QUrl>

namespace melyar {

QStringList webPathCandidates(const QString &currentDir, const QString &appDir) {
    return {
        currentDir + "/ui_web/dist/index.html",
        appDir + "/../ui_web/dist/index.html",
        appDir + "/ui_web/dist/index.html",
        appDir + "/../share/melyar/web/index.html",
        "/usr/share/melyar/web/index.html",
        "/usr/local/share/melyar/web/index.html",
    };
}

QString resolveWebUrl(
    const QString &devUrl,
    const QStringList &candidates,
    const std::function<bool(const QString &)> &exists) {
    if (!devUrl.isEmpty()) {
        return devUrl;
    }
    if (candidates.isEmpty()) {
        return {};
    }
    for (const QString &candidate : candidates) {
        if (!exists(candidate)) {
            continue;
        }
        const QString canonical = QFileInfo(candidate).canonicalFilePath();
        return QUrl::fromLocalFile(canonical.isEmpty() ? candidate : canonical).toString();
    }
    return QUrl::fromLocalFile(candidates.first()).toString();
}

} // namespace melyar

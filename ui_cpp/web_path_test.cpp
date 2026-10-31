#include "web_path.hpp"

#include <QUrl>
#include <cstdlib>
#include <iostream>

namespace {

int failures = 0;

void check(const char *name, const QString &actual, const QString &expected) {
    if (actual == expected) {
        return;
    }
    std::cerr << "FAIL " << name << ": got " << actual.toStdString()
              << ", expected " << expected.toStdString() << '\n';
    ++failures;
}

} // namespace

int main() {
    const QStringList candidates = {
        "/current/ui_web/dist/index.html",
        "/app/../ui_web/dist/index.html",
        "/app/ui_web/dist/index.html",
        "/app/../share/melyar/web/index.html",
        "/usr/share/melyar/web/index.html",
        "/usr/local/share/melyar/web/index.html",
    };

    check("dev url wins",
          melyar::resolveWebUrl(
              "http://localhost:5173",
              candidates,
              [](const QString &) { return true; }),
          "http://localhost:5173");

    check("first existing candidate wins",
          melyar::resolveWebUrl(
              "",
              candidates,
              [&candidates](const QString &path) { return path == candidates[1]; }),
          QUrl::fromLocalFile(candidates[1]).toString());

    check("fallback is the first candidate",
          melyar::resolveWebUrl("", candidates, [](const QString &) { return false; }),
          QUrl::fromLocalFile(candidates[0]).toString());

    check("empty candidate list",
          melyar::resolveWebUrl("", {}, [](const QString &) { return false; }),
          QString());

    // Candidate order: current dir, app dir, app dir/../share, system dirs.
    const QStringList generated = melyar::webPathCandidates("/work", "/app");
    check("candidate count", QString::number(generated.size()), "6");
    check("first candidate", generated[0], "/work/ui_web/dist/index.html");
    check("second candidate", generated[1], "/app/../ui_web/dist/index.html");
    check("third candidate", generated[2], "/app/ui_web/dist/index.html");
    check("fourth candidate", generated[3], "/app/../share/melyar/web/index.html");
    check("fifth candidate", generated[4], "/usr/share/melyar/web/index.html");
    check("sixth candidate", generated[5], "/usr/local/share/melyar/web/index.html");

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return EXIT_FAILURE;
    }
    std::cout << "All web path tests passed\n";
    return EXIT_SUCCESS;
}

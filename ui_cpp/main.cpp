#include <chrono>
#include <clocale>
#include <cstdio>
#include <cstring>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QIcon>
#include <QDir>
#include <QFile>
#include <QUrl>
#include <QUrlQuery>
#include <iostream>
#include <QWebChannel>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>
#include <QTimer>
#include <QImage>
#include <QJSValue>

#include "bridge/bridge.hpp"
#include "web_path.hpp"

int main(int argc, char *argv[]) {
    // A single clock for the whole start up, so the log shows where the
    // time goes.
    const auto started = std::chrono::steady_clock::now();
    auto elapsed = [&started]() -> long {
        return std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - started)
            .count();
    };

    std::setlocale(LC_NUMERIC, "C");

    // Prevent GPU buffer sharing segfault on Wayland with hybrid GPU (NVIDIA + AMD/Intel)
    // A music player draws no video, so the whole GPU stack is dead weight: it
    // costs roughly 50 MB of resident memory and buys nothing. Turning it off
    // also keeps the software rasteriser from being pulled in as a fallback.
    if (!qEnvironmentVariableIsSet("QTWEBENGINE_CHROMIUM_FLAGS")) {
        qputenv("QTWEBENGINE_CHROMIUM_FLAGS",
                "--disable-gpu --disable-gpu-compositing --disable-software-rasterizer "
                "--disable-dev-shm-usage");
    }
    if (!qEnvironmentVariableIsSet("QT_QUICK_CONTROLS_STYLE")) {
        qputenv("QT_QUICK_CONTROLS_STYLE", "Basic");
    }

    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    std::setlocale(LC_NUMERIC, "C");
    qInstallMessageHandler([](QtMsgType, const QMessageLogContext &, const QString &msg) {
        std::cout << "[Qt/QML] " << msg.toStdString() << std::endl;
    });
    app.setApplicationName("Melyar");
    app.setApplicationDisplayName("Melyar");
    app.setOrganizationName("Melyar");
    app.setOrganizationDomain("melyar.org");
    // The desktop file calls itself melyar and declares StartupWMClass=melyar.
    // Without this the session cannot match the window to the entry on Wayland,
    // so the launcher gets a generic icon and the window cannot be pinned.
    app.setDesktopFileName(QStringLiteral("melyar"));
    QIcon appIcon;
    appIcon.addFile(":/melyar/resources/melyar.svg");
    app.setWindowIcon(appIcon);

    QQmlApplicationEngine engine;
    engine.addImportPath(":/melyar");
    engine.addImportPath("qrc:/melyar");

    // C++/Rust bridge exposed to QML (for global media shortcuts)
    auto bridge = std::make_unique<MelyarBridge>();
    MelyarBridge *bridgePtr = bridge.get();
    engine.rootContext()->setContextProperty("bridge", bridgePtr);

    // Resolve web UI path BEFORE loading QML. The candidate order and the
    // fallback live in web_path.cpp so they can be tested without a real
    // filesystem layout.
    const QStringList webCandidates = melyar::webPathCandidates(
        QDir::currentPath(), QCoreApplication::applicationDirPath());
    QString webUrlStr = melyar::resolveWebUrl(
        qEnvironmentVariable("MELYAR_DEV_URL"),
        webCandidates,
        [](const QString &path) { return QFile::exists(path); });

    // MELYAR_LANG forces the interface language, overriding the browser
    // locale detection. The web layer reads it from the ?lang= parameter.
    const QString forcedLang = qEnvironmentVariable("MELYAR_LANG").trimmed().toLower();
    if (!forcedLang.isEmpty()) {
        QUrl url(webUrlStr);
        QUrlQuery query(url);
        query.removeAllQueryItems("lang");
        query.addQueryItem("lang", forcedLang);
        url.setQuery(query);
        webUrlStr = url.toString();
    }

    std::cout << "Loading Web UI from: " << webUrlStr.toStdString() << std::endl;

    // Connected before load, so the very first - and most likely to fail -
    // parse of Main.qml is reported instead of being dropped silently.
    QObject::connect(&engine, &QQmlApplicationEngine::warnings,
                     [](const QList<QQmlError> &warnings) {
        if (!warnings.isEmpty()) {
            std::cerr << "QML Warnings:" << std::endl;
            for (const auto &warning : warnings) {
                std::cerr << "  " << warning.toString().toStdString() << std::endl;
            }
        }
    });

    // Race-free WebChannel protocol:
    // 1. QML creates the WebChannel + WebEngineView WITHOUT any url bound.
    // 2. Here (after engine.load) we register the bridge onto that channel.
    // 3. Only then do we set the view url -> the page can never init its
    //    QWebChannel client before the "bridge" object is registered.
    engine.load(QUrl(QStringLiteral("qrc:/melyar/ui_cpp/qml/Main.qml")));

    QObject *rootObject = engine.rootObjects().isEmpty() ? nullptr : engine.rootObjects().first();
    if (!rootObject) {
        std::cerr << "FATAL: QML root object failed to load" << std::endl;
        return -1;
    }

    // The window size can be set from the outside, so the layout can be
    // checked at the sizes a user actually has. MELYAR_WINDOW_GEOMETRY takes
    // "widthxheight", for example 1000x640.
    const QString geometry = qEnvironmentVariable("MELYAR_WINDOW_GEOMETRY").trimmed();
    if (!geometry.isEmpty()) {
        const QStringList parts = geometry.split(QLatin1Char('x'));
        if (parts.size() == 2) {
            const int width = parts[0].trimmed().toInt();
            const int height = parts[1].trimmed().toInt();
            if (width > 0) rootObject->setProperty("width", width);
            if (height > 0) rootObject->setProperty("height", height);
        }
    }

    QObject *channelObj = rootObject->findChild<QObject *>(QStringLiteral("appChannel"));
    if (!channelObj) {
        std::cerr << "FATAL: appChannel (QML WebChannel) not found" << std::endl;
        return -1;
    }
    auto *webChannel = qobject_cast<QWebChannel *>(channelObj);
    if (!webChannel) {
        std::cerr << "FATAL: appChannel is not a QWebChannel" << std::endl;
        return -1;
    }
    webChannel->registerObject(QStringLiteral("bridge"), bridgePtr);
    std::cout << "[C++] bridge registered onto QML appChannel at " << elapsed() << " ms" << std::endl;

    QObject *webViewObj = rootObject->findChild<QObject *>(QStringLiteral("webView"));
    if (!webViewObj) {
        std::cerr << "FATAL: webView (QML WebEngineView) not found" << std::endl;
        return -1;
    }
    webViewObj->setProperty("url", webUrlStr);
    std::cout << "[C++] webView url set at " << elapsed() << " ms: " << webUrlStr.toStdString() << std::endl;

    // A melyar:// link passed on the command line, for example
    // "melyar://settings" or "melyar://track/<id>". The desktop entry
    // declares the handler, so the application has to honour it.
    QString startupUri;
    for (const QString &arg : QCoreApplication::arguments()) {
        if (arg.startsWith("melyar://", Qt::CaseInsensitive)) {
            startupUri = arg;
            break;
        }
    }

    QObject::connect(&engine, &QQmlApplicationEngine::warnings,
                     [](const QList<QQmlError> &warnings) {
        if (!warnings.isEmpty()) {
            std::cerr << "QML Warnings:" << std::endl;
            for (const auto &warning : warnings) {
                std::cerr << "  " << warning.toString().toStdString() << std::endl;
            }
        }
    });

    // One-shot screenshot: MELYAR_SCREENSHOT=/path/to.png renders the window,    // writes it and exits. Handy for attaching a visual to a bug report.
    // MELYAR_SCREENSHOT_DELAY (ms) controls how long to wait before grabbing,
    // which gives the web interface time to paint.
    const QString shotPath = qEnvironmentVariable("MELYAR_SCREENSHOT");
    if (!shotPath.isEmpty()) {
        auto *window = qobject_cast<QQuickWindow *>(rootObject);
        if (!window) {
            std::cerr << "MELYAR_SCREENSHOT: the QML root object is not a window" << std::endl;
            return -1;
        }
        int delay = qEnvironmentVariableIntValue("MELYAR_SCREENSHOT_DELAY");
        if (delay <= 0) {
            delay = 6000;
        }
        QTimer::singleShot(delay, window, [window, webViewObj, shotPath]() {
            // The web view may have scrolled itself; reset it so the capture
            // starts at the top of the interface. runJavaScript is a public
            // slot on QQuickWebEngineView, reached through the meta object so
            // that the private header is not needed.
            const QJSValue undefined;
            QMetaObject::invokeMethod(webViewObj, "runJavaScript", Qt::DirectConnection,
                                      Q_ARG(QString, QStringLiteral("window.scrollTo(0, 0)")),
                                      Q_ARG(QJSValue, undefined));

            QTimer::singleShot(600, window, [window, shotPath]() {
                const QImage image = window->grabWindow();
                if (image.isNull()) {
                    std::cerr << "MELYAR_SCREENSHOT: the window grab returned nothing" << std::endl;
                    qApp->exit(1);
                    return;
                }
                if (!image.save(shotPath)) {
                    std::cerr << "MELYAR_SCREENSHOT: could not write " << shotPath.toStdString() << std::endl;
                    qApp->exit(1);
                    return;
                }
                std::cout << "[C++] screenshot written to " << shotPath.toStdString() << std::endl;

                qApp->exit(0);
            });
        });
    }

    // MELYAR_EVAL_JS runs a script inside the interface once it has booted.
    // It is a debugging aid: it lets you drive the interface from the
    // command line, for example to reproduce a state or to check that a
    // setting is written. MELYAR_EVAL_FILE reads the script from a file,
    // which is the easier way to run anything longer than one line.
    const QString evalJs = qEnvironmentVariable("MELYAR_EVAL_JS").trimmed();
    const QString evalFile = qEnvironmentVariable("MELYAR_EVAL_FILE").trimmed();
    if (!evalJs.isEmpty() || !evalFile.isEmpty()) {
        // The script needs the interface to be up and the bridge connected,
        // so the default delay is generous.
        int evalDelay = qEnvironmentVariableIntValue("MELYAR_EVAL_DELAY");
        if (evalDelay <= 0) {
            evalDelay = 2500;
        }
        QTimer::singleShot(evalDelay, webViewObj, [webViewObj, evalJs, evalFile]() {
            QString script = evalJs;
            if (!evalFile.isEmpty()) {
                QFile file(evalFile);
                if (file.open(QIODevice::ReadOnly | QIODevice::Text)) {
                    script = QString::fromUtf8(file.readAll());
                } else {
                    // Keep the inline script instead of running nothing.
                    std::cerr << "Could not read " << evalFile.toStdString()
                              << "; falling back to MELYAR_EVAL_JS" << std::endl;
                }
            }
            if (script.isEmpty()) return;
            const QJSValue undefined;
            QMetaObject::invokeMethod(webViewObj, "runJavaScript", Qt::DirectConnection,
                                      Q_ARG(QString, script),
                                      Q_ARG(QJSValue, undefined));
            // The path was being logged here, not the script, so a forty
            // character file name reported a forty byte program.
            std::cout << "[C++] evaluated " << script.toUtf8().size() << " byte script"
                      << std::endl;
        });
    }

    // Hand the startup link to the interface once it has booted, and keep
    // listening for links that arrive while the application is running.
    if (!startupUri.isEmpty()) {
        QTimer::singleShot(1500, bridgePtr, [bridgePtr, startupUri]() {
            bridgePtr->openUri(startupUri);
        });
    }

    return app.exec();
}

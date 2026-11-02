import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Dialogs
import QtWebEngine
import QtWebChannel

Window {
    id: window
    width: 1360
    height: 860
    minimumWidth: 1000
    minimumHeight: 640
    visible: true
    title: "Melyar"
    color: "#0d0d0f"

    // The single WebChannel instance. The page url is set from C++ AFTER
    // the bridge object is registered here (race-free handshake).
    WebChannel {
        id: appChannel
        objectName: "appChannel"
    }

    // A profile with a name of its own. Without one the view falls back to the
    // shared off-the-record profile, where nothing is ever written to disk: the
    // language the user chose lives in localStorage and reset on every launch,
    // and no session could reuse its cache.
    WebEngineProfile {
        id: appProfile
        storageName: "melyar"
        persistentCookiesPolicy: WebEngineProfile.AllowPersistentCookies
    }

    // Set when the bundle cannot be opened, so the window says so rather than
    // sitting there black with nothing to tell the user why.
    property bool webUiFailed: false

    WebEngineView {
        id: webView
        objectName: "webView"
        anchors.fill: parent
        focus: true
        webChannel: appChannel
        profile: appProfile
        backgroundColor: "#0d0d0f"

        // Remote artwork and API calls are the only thing the page needs.
        //
        // file:// access cannot simply be turned off, and it was worth finding
        // out why rather than assuming: the bundle is served from the local
        // filesystem and pulls its qwebchannel.js from alongside itself, so
        // with this off the channel script is blocked, the bridge never
        // connects, and the application is a still picture of itself - no
        // playback, no queue, no settings, nothing. The flag stays on until the
        // page is served from somewhere that does not need it.
        settings.localContentCanAccessRemoteUrls: true
        settings.localContentCanAccessFileUrls: true
        settings.webGLEnabled: true
        settings.javascriptEnabled: true
        settings.playbackRequiresUserGesture: false
        settings.showScrollBars: false

        onJavaScriptConsoleMessage: function(level, message, lineNumber, sourceID) {
            console.log("[Web " + level + "] " + message + " (" + sourceID + ":" + lineNumber + ")")
        }

        onLoadingChanged: function(loadRequest) {
            if (loadRequest.status === WebEngineView.LoadFailedStatus) {
                window.webUiFailed = true
                console.error("Failed to load web UI: " + loadRequest.errorString)
            } else if (loadRequest.status === WebEngineView.LoadStatus) {
                window.webUiFailed = false
            }
        }
    }

    // The bundle is missing or broken. The page itself cannot report that, so
    // the shell does.
    Text {
        anchors.centerIn: parent
        visible: window.webUiFailed
        text: qsTr("The interface could not be loaded.")
        color: "#e8e8ea"
        font.pixelSize: 16
    }

    // Window-scoped shortcuts (default Qt.WindowShortcut), not system-wide grabs.
    Shortcut {
        sequence: StandardKey.PlayPause
        onActivated: bridge.togglePlay()
    }
    Shortcut {
        sequence: "Ctrl+Space"
        onActivated: bridge.togglePlay()
    }
    Shortcut {
        sequence: "Ctrl+Right"
        onActivated: bridge.nextTrack()
    }
    Shortcut {
        sequence: "Ctrl+Left"
        onActivated: bridge.previousTrack()
    }
    Shortcut {
        sequence: "Ctrl+Up"
        onActivated: bridge.setVolume(Math.min(100, bridge.volume + 5))
    }
    Shortcut {
        sequence: "Ctrl+Down"
        onActivated: bridge.setVolume(Math.max(0, bridge.volume - 5))
    }

    // The native file dialog lives here, because the web interface cannot
    // reach the file system. The bridge asks for it and receives the result.
    FileDialog {
        id: libraryFileDialog
        title: qsTr("Import a library export")
        fileMode: FileDialog.OpenFile
        nameFilters: [qsTr("Library exports (*.json *.csv)"), qsTr("All files (*)")]
        onAccepted: bridge.setPickedFile(selectedFile.toString())
        // The dialog must answer on cancel too. Nothing was emitted here, so the
        // web side waited the full five minutes on a promise that could never
        // settle, with its handler still connected for the next dialog to wake.
        onRejected: bridge.setPickedFile("")
    }

    Connections {
        target: bridge
        function onFilePickRequested() {
            libraryFileDialog.open()
        }
    }
}

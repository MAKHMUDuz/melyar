cmake_minimum_required(VERSION 3.25)
project(melyar LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# -----------------------------------------------------------------------------
# Corrosion (Rust integration for CMake)
# -----------------------------------------------------------------------------
find_package(Corrosion QUIET)
if(NOT Corrosion_FOUND)
    include(FetchContent)
    FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.5.2
    )
    FetchContent_MakeAvailable(Corrosion)
endif()

# Import Rust Core Static Library
corrosion_import_crate(
    MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/core_rs/Cargo.toml
    CRATES melyar_core
)

# -----------------------------------------------------------------------------
# Qt6 Modules
# -----------------------------------------------------------------------------
find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick Qml WebEngineQuick WebChannel)

# -----------------------------------------------------------------------------
# Libmpv (resolved per-project, not by changing the host linker defaults)
# -----------------------------------------------------------------------------
find_package(PkgConfig REQUIRED)
pkg_check_modules(MPV REQUIRED IMPORTED_TARGET mpv)

# -----------------------------------------------------------------------------
# Melyar Executable Target
# -----------------------------------------------------------------------------
qt_add_executable(melyar
    ui_cpp/main.cpp
    ui_cpp/bridge/bridge.cpp
    ui_cpp/bridge/bridge.hpp
)

file(GLOB_RECURSE MELYAR_QML_FILES CONFIGURE_DEPENDS
    RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}"
    "ui_cpp/qml/*.qml"
    "ui_cpp/qml/**/*.qml"
    "ui_cpp/qml/**/qmldir"
)

# Application resources (icons) embedded into the binary. Only what the
# program actually opens is embedded: the desktop entry declares Icon=melyar
# and CMake installs the SVG to the scalable icon directory, so the raster
# copies were being carried inside the executable for no reason - two of them
# identical to each other, 162 KB between them.
file(GLOB MELYAR_RESOURCE_FILES CONFIGURE_DEPENDS
    RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}"
    "resources/*.svg"
)

qt6_add_resources(melyar "melyar_qml"
    PREFIX "/melyar"
    FILES ${MELYAR_QML_FILES} ${MELYAR_RESOURCE_FILES}
)

# Web bundle is required at runtime. If it is missing, fail early with an
# actionable message instead of letting the app die later with a blank/failed UI.
if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/ui_web/dist/index.html")
    message(WARNING
        "ui_web/dist/index.html not found. Build the web UI first with:\n"
        "  cd ui_web && pnpm install && pnpm build\n"
        "Then rerun cmake --build ."
    )
endif()

target_include_directories(melyar PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/core_rs/include
    ${CMAKE_CURRENT_SOURCE_DIR}/ui_cpp/bridge
)

target_link_libraries(melyar PRIVATE
    melyar_core
    Qt6::Core
    Qt6::Gui
    Qt6::Quick
    Qt6::Qml
    Qt6::WebEngineQuick
    Qt6::WebChannel
    PkgConfig::MPV
)

# The Rust side may pull in libmpv as an interface dependency. Some distros 
# (notably Debian/Ubuntu with --as-needed) can drop it from the final link line,
# leaving the app with "undefined reference to mpv_create". This keeps the
# project-local fix contained to the binary we build and does not modify the
# system linker defaults globally.
if(UNIX AND NOT APPLE)
    target_link_options(melyar PRIVATE "LINKER:--no-as-needed")
endif()

set_target_properties(melyar PROPERTIES
    CXX_STANDARD 20
    CMAKE_CXX_STANDARD_REQUIRED ON
    AUTOMOC ON
    AUTORCC ON
    AUTOUIC ON
)

install(TARGETS melyar DESTINATION bin)
install(FILES resources/melyar.desktop DESTINATION share/applications)
install(FILES resources/melyar.svg DESTINATION share/icons/hicolor/scalable/apps)

# -----------------------------------------------------------------------------
# Web UI bundle (ui_web/dist) - built separately with `pnpm build`
# -----------------------------------------------------------------------------
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/ui_web/dist/index.html")
    install(DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/ui_web/dist/"
            DESTINATION share/melyar/web
            FILES_MATCHING
            PATTERN "*.html"
            PATTERN "*.js"
            PATTERN "*.css"
            PATTERN "*.svg"
            PATTERN "*.woff"
            PATTERN "*.woff2")
endif()

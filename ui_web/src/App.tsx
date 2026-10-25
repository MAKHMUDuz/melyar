import { useEffect, useRef } from "react";
import { Sidebar } from "@/components/Sidebar";
import { Header, PlayerBar } from "@/components/Toolbar";
import { ContextMenu } from "@/components/ContextMenu";
import { SettingsPanel } from "@/components/SettingsPanel";
import { PluginsPanel } from "@/components/PluginsPanel";
import { ImportOverlay } from "@/components/ImportOverlay";
import { QueuePanel } from "@/components/QueuePanel";
import { LyricsOverlay } from "@/components/LyricsOverlay";
import { ExpandedPlayer } from "@/components/ExpandedPlayer";
import { HomeView } from "@/components/views/HomeView";
import { SearchView } from "@/components/views/SearchView";
import {
  AlbumDetailView,
  AlbumsView,
  ArtistDetailView,
  ArtistsView,
  HistoryView,
  LikedView,
  SongsView,
} from "@/components/views/LibraryViews";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/cn";

function Stage() {
  const view = useUIStore((s) => s.view);
  switch (view.name) {
    case "home":
      return <HomeView />;
    case "search":
      return <SearchView />;
    case "songs":
      return <SongsView />;
    case "albums":
      return <AlbumsView />;
    case "artists":
      return <ArtistsView />;
    case "liked":
      return <LikedView />;
    case "history":
      return <HistoryView />;
    case "album":
      return <AlbumDetailView albumKey={view.key} />;
    case "artist":
      return <ArtistDetailView artistKey={view.key} />;
    default:
      return <HomeView />;
  }
}

function Toast() {
  const toast = useUIStore((s) => s.toast);
  const action = useUIStore((s) => s.toastAction);
  if (!toast) return null;
  return (
    <div className="toast" role="status">
      <span>{toast}</span>
      {action ? (
        <button
          className="toast-action"
          onClick={() => {
            action.run();
          }}
        >
          {action.label}
        </button>
      ) : null}
    </div>
  );
}

// Overlay portal for proper stacking
function OverlayPortal({ children }: { children: React.ReactNode }) {
  return <div className="overlay-portal">{children}</div>;
}

/**
 * Elements that own their own keys. The window-level shortcuts below are for
 * when nothing in particular has focus.
 */
const OWNED_KEYS = [
  "a[href]",
  "button",
  "input",
  "select",
  "textarea",
  "[contenteditable]",
  '[role="button"]',
  '[role="switch"]',
  '[role="slider"]',
  '[role="menuitem"]',
  '[role="tab"]',
  '[role="option"]',
  '[role="dialog"]',
  '[role="menu"]',
  "[tabindex]",
].join(", ");

export default function App() {
  const view = useUIStore((s) => s.view);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const lyricsOpen = useUIStore((s) => s.lyricsOpen);
  const expandedPlayerOpen = useUIStore((s) => s.expandedPlayerOpen);

  // Global keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
        if (e.key === "Escape") (target as HTMLInputElement).blur();
        return;
      }
      // A focused control keeps its own keys. Space on a focused button has to
      // press that button: the preventDefault below used to eat the key-up that
      // fires it, so Space toggled playback while a button sat there ready to be
      // used. Arrow keys inside a slider, a menu or a dialog step and scroll
      // there, instead of seeking the song and changing the volume underneath.
      // Escape is still let through, so a dialog closes from within.
      if (target?.closest?.(OWNED_KEYS) && e.key !== "Escape") return;
      const ui = useUIStore.getState();
      const playback = usePlaybackStore.getState();
      switch (e.key) {
        case " ":
          e.preventDefault();
          playback.toggle();
          break;
        case "ArrowRight":
          if (e.ctrlKey || e.metaKey) playback.next();
          else playback.seekSec(playback.position + 5);
          break;
        case "ArrowLeft":
          if (e.ctrlKey || e.metaKey) playback.prev();
          else playback.seekSec(playback.position - 5);
          break;
        case "ArrowUp":
          e.preventDefault();
          playback.setVolume(Math.min(1, playback.volume + 0.05));
          break;
        case "ArrowDown":
          e.preventDefault();
          playback.setVolume(Math.max(0, playback.volume - 0.05));
          break;
        case "l":
        case "L":
          ui.setLyricsOpen(!ui.lyricsOpen);
          break;
        case "q":
        case "Q":
          ui.setQueueOpen(!ui.queueOpen);
          break;
        case "/":
          e.preventDefault();
          document.getElementById("melyar-search")?.focus();
          break;
        case "Escape":
          ui.closeMenu();
          if (useSettings.getState().open) useSettings.getState().setOpen(false);
          else if (ui.expandedPlayerOpen) ui.setExpandedPlayerOpen(false);
          else if (ui.lyricsOpen) ui.setLyricsOpen(false);
          else if (ui.queueOpen) ui.setQueueOpen(false);
          break;
      }
    };
    const onPointer = () => {
      if (useUIStore.getState().menu) useUIStore.getState().closeMenu();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer);
    };
  }, []);

  const viewKey = view.name === "album" || view.name === "artist" ? `${view.name}:${view.key}` : view.name;
  const isNarrow = typeof window !== "undefined" && window.innerWidth < 1200;

  return (
    <div className={cn("app-root", queueOpen && "queue-open")}>
      <Sidebar />
      <div className="main-col">
        <Header />
        <main className="content-scroll" key={viewKey}>
          <Stage />
        </main>
      </div>

      {/* The player sits along the bottom edge, full width, where every
          desktop music client puts it: the hand is already there, and it
          leaves the top of the window for position and history. */}
      <PlayerBar />

      {/* Docked Queue Panel - part of layout */}
      <QueuePanel />

      {/* Overlay Portals - managed centrally */}
      <OverlayPortal>
        {expandedPlayerOpen && <ExpandedPlayer />}
        {lyricsOpen && <LyricsOverlay />}
        <ContextMenu />
        <SettingsPanel />
        <PluginsPanel />
        <ImportOverlay />
        <Toast />
      </OverlayPortal>
    </div>
  );
}
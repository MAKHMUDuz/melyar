import { create } from "zustand";
import { register } from "@/lib/registry";
import { useSettings } from "@/lib/settings";
import type { TrackMeta, RepeatMode } from "@/lib/types";

type PlaybackState = {
  current: TrackMeta | null;
  playing: boolean;
  position: number;
  duration: number;
  volume: number;
  muted: boolean;
  shuffle: boolean;
  repeat: RepeatMode;
};

type PlaybackActions = {
  setCurrent: (track: TrackMeta | null) => void;
  setPlaying: (playing: boolean) => void;
  setPosition: (position: number) => void;
  setDuration: (duration: number) => void;
  setVolume: (volume: number) => void;
  setMuted: (muted: boolean) => void;
  setShuffle: (shuffle: boolean) => void;
  setRepeat: (repeat: RepeatMode) => void;
  togglePlay: () => void;
  toggle: () => void;
  next: () => void;
  prev: () => void;
  toggleMute: () => void;
  toggleShuffle: () => void;
  cycleRepeat: () => void;
  seekRatio: (ratio: number) => void;
  seekSec: (sec: number) => void;
  playTrack: (track: TrackMeta, context?: TrackMeta[]) => void;
  patch: (partial: Partial<PlaybackState>) => void;
};

let lastVolume = 0.8;

function bridge(): any | null {
  return (typeof window !== "undefined" && window.bridge) || null;
}

export const usePlaybackStore = create<PlaybackState & PlaybackActions>()((set, get) => ({
  current: null,
  playing: false,
  position: 0,
  duration: 0,
  volume: 0.8,
  muted: false,
  shuffle: false,
  repeat: 0,

  setCurrent: (current) => set({ current }),
  setPlaying: (playing) => set({ playing }),
  setPosition: (position) => set({ position }),
  setDuration: (duration) => set({ duration }),
  setVolume: (volume) => {
    const v = Math.max(0, Math.min(1, volume));
    if (v > 0) lastVolume = v;
    // The slider and the arrow keys both land here, so the bridge call has to
    // live in the store. Setting only the local state moved the thumb while
    // the audio stayed where it was.
    bridge()?.setVolume(v * 100);
    useSettings.getState().set("volume", v);
    set({ volume: v, muted: v === 0 });
  },
  setMuted: (muted) => set({ muted }),
  setShuffle: (shuffle) => set({ shuffle }),
  setRepeat: (repeat) => set({ repeat }),

  togglePlay: () => {
    const b = bridge();
    if (!b) return;
    b.togglePlay();
    const { current } = get();
    if (!current) return;
    set({ playing: !get().playing });
  },

  toggleMute: () => {
    const b = bridge();
    if (!b) return;
    const { muted, volume } = get();
    if (muted || volume === 0) {
      const restore = lastVolume > 0.01 ? lastVolume : 0.5;
      b.setVolume(restore * 100);
      set({ volume: restore, muted: false });
    } else {
      lastVolume = volume;
      b.setVolume(0);
      set({ muted: true });
    }
  },

  toggleShuffle: () => {
    const next = !get().shuffle;
    bridge()?.toggleShuffle();
    // Persisted, or the next launch would put the player back on the mode it
    // started with while the panel still showed the one the user chose.
    useSettings.getState().set("shuffle", next);
    set({ shuffle: next });
  },

  cycleRepeat: () => {
    const next = ((get().repeat + 1) % 3) as RepeatMode;
    bridge()?.cycleRepeat();
    useSettings.getState().set("repeat", next);
    set({ repeat: next });
  },

  seekRatio: (ratio) => {
    const b = bridge();
    if (!b) return;
    const dur = get().duration || 0;
    const sec = Math.max(0, Math.min(1, ratio)) * dur;
    b.seek(sec);
    set({ position: sec });
  },

  seekSec: (sec) => {
    const b = bridge();
    if (!b) return;
    b.seek(Math.max(0, sec));
    set({ position: Math.max(0, sec) });
  },

  toggle: () => get().togglePlay(),

  next: () => bridge()?.nextTrack(),

  prev: () => bridge()?.previousTrack(),

  playTrack: (track, context) => {
    const b = bridge();
    if (!b) return;
    const metas = context && context.length > 0 ? context : [track];
    const index = Math.max(0, metas.findIndex((t) => t.id === track.id));
    try {
      b.replaceQueue(
        metas.map((t) => ({
          id: t.id,
          title: t.title,
          artist: t.artist,
          album: t.album,
          duration: t.duration,
          thumbnail: t.thumbnail,
        })),
        index,
      );
    } catch (e) {
      console.error("replaceQueue failed", e);
    }
    b.playTrack(track.id);
    set({ current: register(track), playing: true, position: 0, duration: track.duration || 0 });
  },

  patch: (partial) => set((state) => ({ ...state, ...partial })),
}));

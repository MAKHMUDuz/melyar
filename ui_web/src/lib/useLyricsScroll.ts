import { useCallback, useEffect, useRef } from "react";
import { useUIStore } from "@/store";

/**
 * Keeps the active lyric line centred while the user is not scrolling it
 * themselves.
 *
 * Two things here used to misbehave, and both showed as a stutter:
 *
 *   `scrollIntoView` walks every scrollable ancestor on the way, and the
 *   scroll events it raises look exactly like a hand on the wheel. The
 *   listener that pauses auto-follow therefore fired right after every step
 *   of auto-follow itself, so the pane advanced one line, stopped for three
 *   seconds, advanced again. The scroller is now moved directly — nothing
 *   else on the page scrolls while a full-screen dialog is open — and the
 *   events that move causes are marked as ours and ignored.
 *
 *   The timer that decides when the user has let go was held in a plain
 *   local. That is recreated on every render, so the clean-up closed over a
 *   stale one and the timer outlived the render that set it. It is a ref now,
 *   along with the settle timer.
 *
 * `scrollToActive` recenters on demand and hands control back to auto-follow.
 */
const QUIET_MS = 3000;
// Long enough for a smooth scroll down a tall pane to have come to rest.
const SETTLE_MS = 500;

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function useLyricsScroll(enabled: boolean, active: number) {
  const autoFollow = useUIStore((s) => s.lyricsAutoFollow);
  const setAutoFollow = useUIStore((s) => s.setLyricsAutoFollow);
  const setUserScrolling = useUIStore((s) => s.setLyricsUserScrolling);

  const scrollerRef = useRef<HTMLDivElement>(null);
  const userScrollingRef = useRef(false);
  const oursRef = useRef(false);
  const settleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const centerActive = useCallback((behavior: ScrollBehavior) => {
    const el = scrollerRef.current;
    if (!el) return;
    const lineEl = el.querySelector<HTMLElement>('[data-active="true"]');
    if (!lineEl) return;

    // Measured rather than assumed. The pane carries its own padding, the
    // lines change size as they activate, and an activated line near the top
    // cannot always be centred — so the distance is taken from the boxes
    // that are actually on screen.
    const lineRect = lineEl.getBoundingClientRect();
    const delta =
      lineRect.top - el.getBoundingClientRect().top - (el.clientHeight - lineRect.height) / 2;

    oursRef.current = true;
    if (settleTimerRef.current) clearTimeout(settleTimerRef.current);
    settleTimerRef.current = setTimeout(() => {
      oursRef.current = false;
    }, SETTLE_MS);

    el.scrollTo({ top: Math.max(0, el.scrollTop + delta), behavior });
  }, []);

  const scrollToActive = useCallback(() => {
    userScrollingRef.current = false;
    setUserScrolling(false);
    setAutoFollow(true);
    centerActive(prefersReducedMotion() ? "auto" : "smooth");
  }, [centerActive, setAutoFollow, setUserScrolling]);

  // Mark our own scrolling so it is not mistaken for the user's hand.
  useEffect(() => {
    const el = scrollerRef.current;
    if (!el) return undefined;
    const onScroll = () => {
      if (oursRef.current) return;
      userScrollingRef.current = true;
      setUserScrolling(true);
      if (resetTimerRef.current) clearTimeout(resetTimerRef.current);
      resetTimerRef.current = setTimeout(() => {
        userScrollingRef.current = false;
        setUserScrolling(false);
      }, QUIET_MS);
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      el.removeEventListener("scroll", onScroll);
      if (resetTimerRef.current) clearTimeout(resetTimerRef.current);
      if (settleTimerRef.current) clearTimeout(settleTimerRef.current);
    };
  }, [setUserScrolling]);

  // Follow the line when playback advances, or when a line is tapped.
  useEffect(() => {
    if (!enabled || !autoFollow || active < 0 || userScrollingRef.current) return;
    if (!scrollerRef.current || scrollerRef.current.clientHeight === 0) return;
    // Smooth scrolling asked for in script is not cancelled by the page's
    // `scroll-behavior`, so reduced motion has to be honoured here too.
    centerActive(prefersReducedMotion() ? "auto" : "smooth");
  }, [enabled, autoFollow, active, centerActive]);

  return { ref: scrollerRef, scrollToActive };
}

/**
 * Shared surface tokens for the glass component family.
 *
 * The translucent surface treatment - background, border, inner
 * highlight, shadow - was spelled out per component, so changing the
 * look meant editing several places and the variants drifted apart. It
 * lives here once and the components compose it with `cn`.
 *
 * The values are exact: moving them must not change what renders.
 */

/** Accent tint plus the matching text colour: the on state of an active control. */
export const surfaceActive = "bg-accent/15 border-accent/30 text-accent-text";

/** The same tint without the text colour, for surfaces that colour their own text. */
export const surfaceActiveTint = "bg-accent/15 border-accent/30";

/** A control waiting on an in-flight action. */
export const surfaceLoading = "opacity-50 cursor-wait";

/** A control that cannot be interacted with. */
export const surfaceDisabled = "opacity-30 pointer-events-none";

/** The accent gradient behind primary actions and the playing play/pause state. */
export const surfacePrimaryGradient =
  "bg-gradient-to-br from-accent to-[#d61d33] text-on-accent border-none";

/** The shared body of a floating tooltip, before any reveal classes. */
export const surfaceTooltip =
  "absolute -bottom-8 left-1/2 -translate-x-1/2 px-2 py-1 text-xs font-medium bg-glass-strong text-white rounded-md shadow-lg whitespace-nowrap";

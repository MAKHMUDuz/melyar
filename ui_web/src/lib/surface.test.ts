import { test } from "vitest";
import {
  surfaceActive,
  surfaceActiveTint,
  surfaceDisabled,
  surfaceLoading,
  surfacePrimaryGradient,
  surfaceTooltip,
} from "./surface.ts";

function runSurfaceTests() {
  let failures = 0;
  function check(name: string, actual: unknown, expected: unknown) {
    const ok = JSON.stringify(actual) === JSON.stringify(expected);
    if (!ok) {
      failures++;
      console.error(`FAIL ${name}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
    } else {
      console.log(`ok ${name}`);
    }
  }

  // These literals are the contract with what already ships. Editing a token
  // here changes what renders, so the strings are pinned on purpose.
  check("active tint with text", surfaceActive, "bg-accent/15 border-accent/30 text-accent-text");
  check("active tint alone", surfaceActiveTint, "bg-accent/15 border-accent/30");
  check("loading", surfaceLoading, "opacity-50 cursor-wait");
  check("disabled", surfaceDisabled, "opacity-30 pointer-events-none");
  check(
    "primary gradient",
    surfacePrimaryGradient,
    "bg-gradient-to-br from-accent to-[#d61d33] text-on-accent border-none"
  );
  check(
    "tooltip body",
    surfaceTooltip,
    "absolute -bottom-8 left-1/2 -translate-x-1/2 px-2 py-1 text-xs font-medium " +
      "bg-glass-strong text-white rounded-md shadow-lg whitespace-nowrap"
  );

  // The active tint is the shared prefix of the active state: composing the
  // two must never drift into two different accent treatments.
  check(
    "active extends the tint",
    surfaceActive.startsWith(surfaceActiveTint),
    true
  );

  // clsx joins with a single space, so a token plus an extra class has to
  // land exactly where the old inline string had it.
  const composed = [surfaceTooltip, "opacity-0 group-hover:opacity-100"].join(" ");
  check(
    "tooltip composes in order",
    composed.endsWith("whitespace-nowrap opacity-0 group-hover:opacity-100"),
    true
  );

  if (failures > 0) {
    console.error(`\n${failures} test(s) failed`);
    throw new Error("surface tests failed");
  } else {
    console.log(`\nAll surface tests passed`);
  }
}

test("glass surface tokens", runSurfaceTests);

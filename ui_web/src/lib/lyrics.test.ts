import { test } from "vitest";
import { parseLyrics, activeLineIndex, type LyricLine } from "./lyrics.ts";

function runLyricsTests() {
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

  // --- parseLyrics ---

  check("empty input", parseLyrics(null), { synced: false, lines: [], plain: [] });
  check("whitespace only", parseLyrics("   \n  "), { synced: false, lines: [], plain: [] });

  const single = parseLyrics("[00:01.20]First line\n");
  check("single timestamped line", single.synced, true);
  check("single timestamped line count", single.lines.length, 1);
  check("single timestamped line time", single.lines[0]!.time, 1.2);
  check("single timestamped line text", single.lines[0]!.text, "First line");

  const hours = parseLyrics("[01:02.345]Second line\n");
  check("hour-minute-fraction time", hours.lines[0]!.time, 62.345);

  const plain = parseLyrics("Just plain text\nAnother line\n");
  check("plain synced flag", plain.synced, false);
  check("plain lines", plain.plain, ["Just plain text", "Another line"]);

  const multi = parseLyrics("[00:05.00]C\n[00:00.00]A\n[00:02.50]B\n");
  check("sorts by time", multi.lines.map((l) => l.text), ["A", "B", "C"]);
  check("sorts by time values", multi.lines.map((l) => l.time), [0, 2.5, 5]);

  const sameTs = parseLyrics("[00:00.00]A\n[00:00.00]B\n");
  check("same timestamp keeps both", sameTs.lines.map((l) => l.text), ["A", "B"]);

  // --- activeLineIndex ---

  const lines: LyricLine[] = [
    { time: 0, text: "First" },
    { time: 5, text: "Second" },
    { time: 10, text: "Third" },
  ];

  check("t=0 -> First", activeLineIndex(lines, 0), 0);
  check("t=4.99 -> First", activeLineIndex(lines, 4.99), 0);
  check("t=5.0 -> Second", activeLineIndex(lines, 5.0), 1);
  check("t=9.99 -> Second", activeLineIndex(lines, 9.99), 1);
  check("t=10.0 -> Third", activeLineIndex(lines, 10.0), 2);
  check("t=100 -> last", activeLineIndex(lines, 100), 2);
  check("t=-1 -> none", activeLineIndex(lines, -1), -1);
  check("empty lines -> -1", activeLineIndex([], 5), -1);

  // Backward seek: after being at Third, seeking back to 3 should go to First (line at 0).
  check("backward seek 10 -> 3", activeLineIndex(lines, 3), 0);
  check("backward seek 10 -> 6", activeLineIndex(lines, 6), 1);

  // Same timestamp: active is the last of the group.
  const sameTsLines: LyricLine[] = [
    { time: 0, text: "A" },
    { time: 0, text: "B" },
    { time: 5, text: "C" },
  ];
  check("same timestamp t=0 -> last of group", activeLineIndex(sameTsLines, 0), 1);

  if (failures > 0) {
    console.error(`\n${failures} test(s) failed`);
    throw new Error("lyrics tests failed");
  } else {
    console.log(`\nAll lyrics tests passed`);
  }
}

test("lyrics parser", runLyricsTests);

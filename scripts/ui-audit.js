/*
 * Interface audit.
 *
 * Drives the interface through every view and every overlay, then reports
 * what is wrong: a console error, an element that reaches outside the
 * window, a translation key that was never replaced, an interactive element
 * without an accessible name, a view that renders nothing.
 *
 * It also pushes deliberately awkward data through the interface, a title
 * long enough to break any layout and a track with no artwork, because
 * those are what a real library contains.
 *
 * Run it with:
 *
 *   MELYAR_EVAL_FILE=scripts/ui-audit.js ./build/melyar
 *
 * The application prints "DENETIM_SONUCU problems=0" when the interface is
 * clean, and the continuous integration fails when it is not.
 */
(function () {
  const problems = [];
  const note = (kind, detail) => problems.push(kind + ": " + detail);

  const originalError = console.error;
  console.error = function () {
    note("console.error", Array.prototype.map.call(arguments, String).join(" ").slice(0, 200));
    originalError.apply(console, arguments);
  };
  window.addEventListener("error", (e) => note("uncaught", String(e.message).slice(0, 200)));
  window.addEventListener("unhandledrejection", (e) => note("unhandled", String(e.reason).slice(0, 200)));

  // An element inside a horizontal scroller is allowed to reach past the
  // edge of the window, that is what a scroller is for.
  function inScroller(el) {
    let p = el.parentElement;
    while (p) {
      const s = getComputedStyle(p);
      if (s.overflowX === "auto" || s.overflowX === "scroll" || s.overflowX === "hidden") return true;
      p = p.parentElement;
    }
    return false;
  }

  function checkOverflow(tag) {
    const w = window.innerWidth;
    const bad = Array.prototype.slice.call(document.querySelectorAll("*")).filter((el) => {
      const r = el.getBoundingClientRect();
      if (r.width <= 0 || r.width >= w * 2) return false;
      if (r.right <= w + 2 && r.left >= -2) return false;
      return !inScroller(el);
    });
    if (bad.length) {
      const b = bad[0];
      note(
        "overflow",
        tag + ": " + bad.length + " elements reach outside the window, first <" +
          b.tagName + " class=" + String(b.className).slice(0, 70) + ">",
      );
    }
  }

  function checkText(tag) {
    const keys = Array.prototype.slice
      .call(document.querySelectorAll("*"))
      .filter((el) => el.children.length === 0)
      .map((el) => (el.textContent || "").trim())
      .filter((t) => /^[a-z]+(\.[a-zA-Z]+)+$/.test(t));
    if (keys.length) note("missing-translation", tag + ": " + Array.from(new Set(keys)).join(", "));
  }

  function checkAccessibleNames(tag) {
    const unnamed = Array.prototype.slice
      .call(document.querySelectorAll("button, a, input, [role=button]"))
      .filter((el) => {
        if (el.getAttribute("aria-hidden") === "true") return false;
        if (el.getAttribute("aria-label")) return false;
        if ((el.textContent || "").trim()) return false;
        if (el.getAttribute("title")) return false;
        if (el.getAttribute("placeholder")) return false;
        const labelled = el.getAttribute("aria-labelledby");
        if (labelled && document.getElementById(labelled)) return false;
        return true;
      });
    if (unnamed.length) {
      note(
        "no-accessible-name",
        tag + ": " + unnamed.length + " interactive elements have no accessible name, first class=" +
          String(unnamed[0].className).slice(0, 70),
      );
    }
  }

  // Every accessible label has to come out of the language that is actually
  // selected. This is the check the others cannot make: they only care that
  // a label exists, and a Turkish label in the English interface does exist.
  // It is also the one a screen-reader user cannot reconcile with the button
  // they are looking at, so it is worth catching by name rather than by
  // eye, once, rather than in every screenshot forever.
  //
  // i18n.ts publishes the active dictionary. A label counts as translated
  // when it matches a value exactly, or matches a template whose
  // {placeholders} have been filled in. Everything else is prose somebody
  // typed by hand — except strings carrying data no translation can
  // predict, such as a frequency or a percentage.
  function localeLabelPatterns() {
    const out = [];
    const dicts = [window.__melyarDict || {}, window.__melyarDictEn || {}];
    dicts.forEach((dict) => {
      Object.keys(dict).forEach((key) => {
        const value = dict[key];
        if (typeof value !== "string" || !value) return;
        if (value.indexOf("{") === -1) {
          out.push({ exact: value, key });
          return;
        }
        const pattern = value
          .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
          .replace(/\\\{[^}]*\\\}/g, "[\\s\\S]*");
        out.push({ re: new RegExp("^" + pattern + "$"), key });
      });
    });
    return out;
  }

  function checkLocaleLabels(tag) {
    if (!window.__melyarDict) {
      note(
        "no-locale-dictionary",
        tag + ": i18n did not publish its dictionary, so accessible labels cannot be checked",
      );
      return;
    }
    const patterns = localeLabelPatterns();
    const foreign = [];
    Array.prototype.slice.call(document.querySelectorAll("[aria-label]")).forEach((el) => {
      if (el.getAttribute("aria-hidden") === "true") return;
      const label = el.getAttribute("aria-label");
      if (!label) return;
      if (/\d/.test(label)) return; // "440 Hz", "100%" — data, not prose
      if (label.length < 3) return;
      const known = patterns.some((p) => (p.re ? p.re.test(label) : p.exact === label));
      if (!known) foreign.push(label + " <" + el.tagName.toLowerCase() + "." + String(el.className).slice(0, 40) + ">");
    });
    if (foreign.length) {
      note(
        "untranslated-label",
        tag + ": " + foreign.length + " accessible labels are not in the active dictionary: " +
          foreign.slice(0, 5).join(" | "),
      );
    }
  }

  // Reduced motion is a request the user makes to their operating system,
  // not to us. Answering it only where somebody remembered to soften an
  // animation means the next animation added is answered to nobody, so the
  // stylesheet needs one rule that swallows everything.
  //
  // Merely finding a `*` selector is not enough: this file used to carry
  // three copies of the suppression, and a check that accepted any wildcard
  // would have passed whether the rule reset the durations or described
  // them. So the declarations are read, and they have to actually stop the
  // motion.
  let reducedMotionChecked = false;
  function checkReducedMotion(tag) {
    if (reducedMotionChecked) return;
    reducedMotionChecked = true;
    let swallowsEverything = 0;
    const walk = (rules) => {
      Array.prototype.slice.call(rules || []).forEach((rule) => {
        if (rule.type !== 4 /* CSSRule.MEDIA_RULE */) return;
        const condition = rule.conditionText || (rule.media && rule.media.mediaText) || "";
        if (!/prefers-reduced-motion:\s*reduce/.test(condition)) return;
        Array.prototype.slice.call(rule.cssRules || []).forEach((inner) => {
          if (!inner.selectorText || !inner.style) return;
          if (inner.selectorText.indexOf("*") === -1) return;
          // A duration stops motion when it is gone, or stepped down to
          // where nobody can see it: the standard reset writes 0.01ms, and
          // a bare number is seconds.
          const stops = (property) => {
            const value = (inner.style.getPropertyValue(property) || "").trim();
            if (!value) return false;
            if (/none/.test(value)) return true;
            const tokens = value.split(/[\s,]+/).filter(Boolean);
            return (
              tokens.length > 0 &&
              tokens.every((token) => {
                const number = parseFloat(token);
                if (Number.isNaN(number)) return false;
                return number * (/ms$/.test(token) ? 1 : 1000) <= 1;
              })
            );
          };
          if (stops("transition-duration") || stops("animation-duration")) {
            swallowsEverything++;
          }
        });
        walk(rule.cssRules);
      });
    };
    Array.prototype.slice.call(document.styleSheets).forEach((sheet) => {
      try {
        walk(sheet.cssRules);
      } catch {
        // A cross-origin sheet cannot be read; ours are all local.
      }
    });
    if (!swallowsEverything) {
      note(
        "reduced-motion",
        tag + ": nothing stops every animation when the system asks for reduced motion",
      );
    }
  }

  // The contrast of the text against what is behind it. A light surface with
  // light text is invisible, and the browser engine does not always blur
  // what is behind a panel, so this has to be checked rather than assumed.
  //
  // The colours have to be read carefully, and they have to be read here
  // rather than through the engine. Tailwind writes its tints in oklab() and
  // color-mix(), and what the engine makes of them differs between versions:
  // this QtWebEngine hands oklab() back untouched through a canvas, while the
  // one in the build normalises it to a hex string and drops the alpha, which
  // would turn a twenty percent tint into a solid one. So the colour function
  // is read directly and nothing is asked of the engine.

  /** Convert a colour in oklab to sRGB, per the CSS colour specification. */
  function oklabToSrgb(L, a, b, alpha) {
    const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    const s_ = L - 0.0894841775 * a - 1.291485548 * b;
    const l = l_ * l_ * l_;
    const m = m_ * m_ * m_;
    const s = s_ * s_ * s_;
    const enc = (v) => {
      const c = Math.max(0, Math.min(1, v));
      return c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
    };
    return {
      r: enc(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s) * 255,
      g: enc(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s) * 255,
      b: enc(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s) * 255,
      a: alpha,
    };
  }

  function hslToRgb(h, s, l, alpha) {
    const f = (n) => {
      const k = (n + h / 30) % 12;
      const a = s * Math.min(l, 1 - l);
      return l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    };
    return { r: f(0) * 255, g: f(8) * 255, b: f(4) * 255, a: alpha };
  }

  function parseColor(value) {
    if (!value) return null;

    const hex = /^#([0-9a-f]{3,8})$/i.exec(value.trim());
    if (hex) {
      let digits = hex[1];
      if (digits.length === 3 || digits.length === 4) {
        digits = digits
          .split("")
          .map((d) => d + d)
          .join("");
      }
      return {
        r: parseInt(digits.slice(0, 2), 16),
        g: parseInt(digits.slice(2, 4), 16),
        b: parseInt(digits.slice(4, 6), 16),
        a: digits.length === 8 ? parseInt(digits.slice(6, 8), 16) / 255 : 1,
      };
    }

    if (value.indexOf("(") === -1) return null;

    // The modern syntax puts the alpha after a slash, rgba() as a fourth
    // component. It is taken off first so the components are unambiguous.
    let alpha = 1;
    let body = value;
    const slash = value.indexOf("/");
    if (slash !== -1) {
      body = value.slice(0, slash);
      const rest = value
        .slice(slash + 1)
        .replace(/[)\s]/g, "")
        .trim();
      const a = parseFloat(rest);
      if (!Number.isNaN(a)) alpha = rest.indexOf("%") !== -1 ? a / 100 : a;
    }

    const words = body.replace(/[()]/g, " ").split(/[\s,]+/).filter(Boolean);
    if (!words.length) return null;
    const name = words[0].toLowerCase();
    const rest = words.slice(name === "color" && words[1] === "srgb" ? 2 : 1);
    const nums = rest.map((v) => parseFloat(v)).filter((v) => !Number.isNaN(v));
    if (nums.length < 3) return null;
    // A fourth component is the alpha when no slash carried it.
    if (slash === -1 && nums.length > 3) alpha = nums[3];

    if (name === "rgb" || name === "rgba") {
      return { r: nums[0], g: nums[1], b: nums[2], a: alpha };
    }
    if (name === "color") {
      // srgb components are normalized to 1.
      if (words[1] && words[1].toLowerCase() !== "srgb") return null;
      return { r: nums[0] * 255, g: nums[1] * 255, b: nums[2] * 255, a: alpha };
    }
    if (name === "oklab") {
      return oklabToSrgb(nums[0], nums[1], nums[2], alpha);
    }
    if (name === "hsl" || name === "hsla") {
      return hslToRgb(nums[0], nums[1] / 100, nums[2] / 100, alpha);
    }
    return null;
  }

  function blend(top, bottom) {
    const a = top.a;
    return {
      r: top.r * a + bottom.r * (1 - a),
      g: top.g * a + bottom.g * (1 - a),
      b: top.b * a + bottom.b * (1 - a),
      a: 1,
    };
  }

  function luminance(c) {
    const f = (v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b);
  }

  function contrast(a, b) {
    const la = luminance(a);
    const lb = luminance(b);
    const lighter = Math.max(la, lb);
    const darker = Math.min(la, lb);
    return (lighter + 0.05) / (darker + 0.05);
  }

  /**
   * The colour behind an element: every layer between it and the first
   * opaque one, composited together. The layers come back as well, so a
   * measurement that looks wrong can be read rather than guessed at.
   */
  function backgroundBehind(el) {
    const layers = [];
    const labels = [];
    let node = el;
    let raw = "";
    while (node) {
      const declared = getComputedStyle(node).backgroundColor;
      if (!raw) raw = declared;
      const bg = parseColor(declared);
      if (bg && bg.a > 0) {
        layers.push(bg);
        labels.push(
          String(node.className || node.tagName).slice(0, 26) + " @" + bg.a.toFixed(2),
        );
        if (bg.a >= 0.99) break;
      } else if (declared && declared !== "rgba(0, 0, 0, 0)" && declared !== "transparent") {
        // A colour that could not be read is not silently passed over: it
        // changes what is behind the text, so it is named instead.
        labels.push("unread " + declared.slice(0, 46));
      }
      node = node.parentElement;
    }
    let result = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i -= 1) {
      result = blend(layers[i], result);
    }
    result.over = labels.join(" over ") || "the page default";
    result.raw = raw;
    return result;
  }

  function checkContrast(tag) {
    const texts = Array.prototype.slice
      .call(document.querySelectorAll("*"))
      .filter((el) => {
        if (el.children.length > 0) return false;
        if (!(el.textContent || "").trim()) return false;
        if (el.getAttribute("aria-hidden") === "true") return false;
        const rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
      });

    const bad = [];
    texts.forEach((el) => {
      const style = getComputedStyle(el);
      if (style.visibility === "hidden" || style.opacity === "0") return;
      const fg = parseColor(style.color);
      if (!fg || fg.a === 0) return;
      const bg = backgroundBehind(el);
      const ratio = contrast(fg.a < 1 ? blend(fg, bg) : fg, bg);
      // WCAG 2.1 AA, which is the floor this audit holds to: 4.5:1 for text
      // of this size, and 3:1 for large text — 24px, or 18.66px at a bold
      // weight. A little is asked of every measurement rather than exactly
      // the threshold, because the colours come from several rounded layers.
      const px = parseFloat(style.fontSize);
      const weight = parseInt(style.fontWeight, 10) || 400;
      const large = px >= 24 || (px >= 18.66 && weight >= 700);
      const floor = (large ? 3.0 : 4.5) + 0.1;
      if (ratio < floor) {
        bad.push(
          (el.textContent || "").trim().slice(0, 24) +
            " (" +
            px.toFixed(0) +
            "px " +
            style.fontWeight +
            ", ratio " +
            ratio.toFixed(2) +
            " < " +
            floor +
            ", color " +
            style.color +
            ", behind rgb(" +
            [bg.r, bg.g, bg.b].map((v) => Math.round(v)).join(", ") +
            ") from " +
            bg.over +
            ", declared " +
            bg.raw +
            ", class " +
            String(el.className).slice(0, 40) +
            ")",
        );
      }
    });

    if (bad.length) {
      // Every distinct piece of text is named, not only the first one: the
      // first is usually the same sentence on several pages, and it hides
      // what the rest of them are.
      Array.from(new Set(bad)).forEach((one) =>
        note("low-contrast", tag + ": " + one),
      );
    }
  }

  const steps = [];

  function view(name, needsHeading) {
    steps.push([
      "view:" + name,
      function () { window.__navigate(name); },
      function () {
        const cs = document.querySelector(".content-scroll");
        if (!cs) { note("no-view", name + ": the view did not render"); return; }
        if (needsHeading) {
          const h1 = cs.querySelector("h1");
          if (!h1 || !(h1.textContent || "").trim()) note("missing-heading", name + ": no heading");
        }
        if (cs.textContent.trim().length < 10) note("empty-view", name + ": nothing rendered");
      },
    ]);
  }

  view("home", true);
  view("songs", true);
  view("liked", true);
  view("history", true);
  view("albums", true);
  view("artists", true);
  view("search", false);

  // The panel shows one category at a time behind a tablist, so the audit
  // cannot expect every section to be on screen at once — it selects each
  // tab in turn and checks the section that category is supposed to render.
  // The category ids are the panel's own ids (tab-<id>, panel-<id>), and the
  // expected heading comes from the dictionary in whichever language is
  // active, for the same reason as everywhere else in this file.
  const SETTINGS_CATEGORIES = [
    "general", "appearance", "playback", "audio", "library",
    "lyrics", "providers", "advanced", "about",
  ];

  steps.push([
    "settings",
    function () {
      window.__testSetSetting("equalizerEnabled", "true");
      window.bridge.openUri("melyar://settings");
    },
    function () {
      const panel = document.querySelector(".settings-panel");
      if (!panel) { note("no-panel", "settings: the panel did not open"); return; }
      const missing = SETTINGS_CATEGORIES.filter(
        (id) => !document.getElementById("tab-" + id),
      );
      if (missing.length) {
        note("missing-tab", "settings: no tab for " + missing.join(", "));
      }
    },
  ]);

  SETTINGS_CATEGORIES.forEach((category) => {
    steps.push([
      "settings:" + category,
      function () {
        const tab = document.getElementById("tab-" + category);
        if (tab) tab.click();
      },
      function () {
        const panel = document.querySelector(".settings-panel");
        if (!panel) { note("no-panel", "settings:" + category + ": the panel closed"); return; }
        const content = document.getElementById("panel-" + category);
        if (!content) { note("no-category", "settings:" + category + ": its content did not render"); return; }
        if (content.textContent.trim().length < 10) {
          note("empty-category", "settings:" + category + ": nothing rendered");
        }
        const dict = window.__melyarDict || {};
        const want = dict["settings.section." + category] || category;
        const labels = Array.prototype.slice
          .call(content.querySelectorAll("h3"))
          .map((e) => (e.textContent || "").trim());
        if (labels.indexOf(want) === -1) {
          note("missing-section", "settings:" + category + ": no '" + want + "' section, found: " + labels.join(", "));
        }
        if (category === "advanced") {
          // The bands live under the advanced tab now, and the cache size
          // slider sits above them, so ten range inputs means the bands are
          // actually on screen rather than merely configured.
          const sliders = panel.querySelectorAll("input[type=range]").length;
          if (sliders < 10) note("missing-equalizer", "settings: the equaliser bands are missing (" + sliders + ")");
        }
      },
    ]);
  });

  steps.push([
    "context-menu",
    function () {
      // The menu needs a track to open on, and the library is empty in a
      // clean profile.
      window.__testSetState({
        results: [{
          id: "audit-menu", title: "A Track For The Menu", artist: "Someone",
          album: "An Album", duration: 200, thumbnail: "",
        }],
      });
      window.__openMenuOnFirstTrack();
    },
    function () {
      if (!window.__testState().menuOpen) note("no-menu", "context-menu: it did not open");
    },
  ]);
  steps.push([
    "expanded",
    function () { window.__openExpandedPlayer(); },
    function () {
      if (!window.__testState().expandedPlayerOpen) note("no-overlay", "expanded: it did not open");
    },
  ]);
  steps.push([
    "lyrics",
    function () { window.__openLyrics(); },
    function () {
      if (!window.__testState().lyricsOpen) note("no-overlay", "lyrics: it did not open");
    },
  ]);
  steps.push([
    "queue",
    function () { window.__openQueue(); },
    function () {
      if (!window.__testState().queueOpen) note("no-overlay", "queue: it did not open");
    },
  ]);

  // Awkward data. A title this long has to be truncated rather than push
  // the layout apart, and a track with no artwork has to fall back to the
  // placeholder instead of leaving a hole.
  const awkward = {
    id: "audit-edge-case",
    title:
      "An Extremely Long Track Title That Has To Be Truncated Rather Than Pushing The Layout Apart In Every Single Row Of The Table",
    artist: "A Collaborating Artist With A Really Unreasonably Long Name That Goes On And On And On",
    album: "Also An Unusually Long Album Name Here To Make Sure That One Is Handled As Well",
    duration: 245,
    thumbnail: "",
  };
  steps.push([
    "awkward-data",
    function () {
      window.__testSetState({
        results: [awkward],
        liked: [awkward],
        history: [awkward],
        queue: [awkward],
        current: awkward,
      });
      window.__navigate("songs");
    },
    null,
  ]);
  steps.push([
    "search-with-results",
    function () { window.__testSearch("daft punk"); },
    function () {
      setTimeout(function () {
        const s = window.__testState();
        if (!s.results || s.results.length === 0) {
          note("no-results", "the search returned nothing");
        } else {
          const bad = s.results.filter((t) => !t.title || !t.artist);
          if (bad.length) note("incomplete-track", bad.length + " results have no title or artist");
        }
      }, 3000);
    },
  ]);

  let index = 0;
  let theme = "dark";
  function next() {
    if (index >= steps.length) {
      // The light theme is walked through step for step as well, not just
      // glanced at: the floating surfaces were drawn for the dark one, and
      // the light one has its own set of them, over its own colours.
      if (theme === "dark" && window.__setTheme) {
        console.log("  acik tema denetimi basliyor");
        theme = "light";
        window.__setTheme("light");
        index = 0;
        setTimeout(next, 1200);
        return;
      }
      finish();
      return;
    }
    const step = steps[index++];
    const tag = theme === "dark" ? step[0] : step[0] + " @acik-tema";
    try {
      step[1]();
    } catch (e) {
      note("threw", tag + ": " + String(e).slice(0, 140));
    }
    setTimeout(function () {
      try {
        checkOverflow(tag);
        checkText(tag);
        checkAccessibleNames(tag);
        checkLocaleLabels(tag);
        checkReducedMotion(tag);
        checkContrast(tag);
        if (step[2]) step[2]();
      } catch (e) {
        note("check", tag + ": " + String(e).slice(0, 110));
      }
      next();
    }, 900);
  }

  function finish() {
    setTimeout(function () {
      const unique = Array.from(new Set(problems));
      console.log("DENETIM_SONUCU problems=" + unique.length);
      unique.forEach((p) => console.log("  SORUN: " + p));
      if (unique.length === 0) console.log("  SORUN YOK");
      window.bridge.quit();
    }, 1200);
  }

  setTimeout(next, 1200);
})();

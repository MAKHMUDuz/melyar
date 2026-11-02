import { renderToStaticMarkup } from "react-dom/server";
import { expect, test, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";
import { t } from "@/lib/i18n";

vi.mock("@/lib/client", () => ({ getBridge: () => null }));
vi.mock("@/lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/settings")>();
  return {
    ...actual,
    useSettings: (selector: (state: unknown) => unknown) => selector({
      settings: actual.DEFAULT_SETTINGS,
      open: true,
      set: vi.fn(),
      reset: vi.fn(),
      setOpen: vi.fn(),
    }),
  };
});

test("does not offer notifications before a notification consumer exists", () => {
  const html = renderToStaticMarkup(<SettingsPanel />);
  expect(html).not.toContain(`aria-label="${t("settings.showNotifications")}"`);
  // Guard against passing by accidentally rendering a closed/empty panel.
  expect(html).toContain('role="dialog"');
  expect(html).toContain(`aria-label="${t("settings.showAlbumArt")}"`);
  expect(html).toContain(`aria-label="${t("settings.showLyricsAutomatically")}"`);
});

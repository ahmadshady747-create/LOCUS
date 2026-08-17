export type MonoFont = "JetBrains Mono" | "Fira Code" | "Geist Mono" | "Cascadia Code";

export interface TypographySettings {
  monoFont: MonoFont;
  codeFontSize: number; // 12 to 18
  chatFontSize: number; // 12 to 18
  lineHeight: "tight" | "normal" | "relaxed";
  ligatures: boolean;
}

export const DEFAULT_TYPOGRAPHY: TypographySettings = {
  monoFont: "JetBrains Mono",
  codeFontSize: 13,
  chatFontSize: 14,
  lineHeight: "normal",
  ligatures: true,
};

export const AVAILABLE_FONTS: Array<{
  id: MonoFont;
  name: string;
  family: string;
  description: string;
  recommendedFor: string;
  preview: string;
}> = [
  {
    id: "JetBrains Mono",
    name: "JetBrains Mono",
    family: "'JetBrains Mono', monospace",
    description: "Tailored specifically for developers with high x-height and clear distinct characters.",
    recommendedFor: "Rust, TypeScript, Python",
    preview: "fn match_pattern(val: &Option<T>) -> Result<u64> { 0x42 }",
  },
  {
    id: "Fira Code",
    name: "Fira Code",
    family: "'Fira Code', monospace",
    description: "Iconic monospaced font with rich programming ligatures (=>, !=, ===, <=).",
    recommendedFor: "Functional & Modern Web",
    preview: "const handleDiff = async (id: string) => { if (a !== b) return; }",
  },
  {
    id: "Geist Mono",
    name: "Geist Mono",
    family: "'Geist Mono', monospace",
    description: "Vercel's ultra-clean, minimal modernist typeface engineered for clarity.",
    recommendedFor: "Modern UI & Full-Stack",
    preview: "export async function generateContext(req: Request): Promise<AST>",
  },
  {
    id: "Cascadia Code",
    name: "Cascadia Code",
    family: "'Cascadia Code', Consolas, monospace",
    description: "Microsoft's modern terminal font with cursive italics and crisp rendering.",
    recommendedFor: "C++, Systems & Terminal",
    preview: "#include <iostream>\nauto main() -> int { return 0; }",
  },
];

const STORAGE_KEY = "locus_typography_settings";

/**
 * Loads typography settings from LocalStorage or returns defaults
 */
export function getTypographySettings(): TypographySettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        ...DEFAULT_TYPOGRAPHY,
        ...parsed,
      };
    }
  } catch (e) {
    console.warn("Failed to read typography settings from localStorage", e);
  }
  return DEFAULT_TYPOGRAPHY;
}

/**
 * Saves and immediately applies typography settings to the document
 */
export function saveTypographySettings(settings: TypographySettings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (e) {
    console.warn("Failed to write typography settings to localStorage", e);
  }
  applyTypographySettings(settings);
}

/**
 * Applies CSS variables directly to :root without page reload
 */
export function applyTypographySettings(settings: TypographySettings): void {
  const root = document.documentElement;
  const fontMeta = AVAILABLE_FONTS.find((f) => f.id === settings.monoFont) || AVAILABLE_FONTS[0];

  root.style.setProperty("--locus-mono-font", fontMeta.family);
  root.style.setProperty("--locus-code-size", `${settings.codeFontSize}px`);
  root.style.setProperty("--locus-chat-size", `${settings.chatFontSize}px`);

  const lineHeightVal =
    settings.lineHeight === "tight"
      ? "1.35"
      : settings.lineHeight === "relaxed"
      ? "1.7"
      : "1.5";
  root.style.setProperty("--locus-line-height", lineHeightVal);

  root.style.setProperty(
    "--locus-font-features",
    settings.ligatures ? '"liga" 1, "calt" 1' : '"liga" 0, "calt" 0'
  );
}

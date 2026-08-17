import React, { createContext, useContext, useEffect, useState } from "react";
import { ar } from "./locales/ar";
import { en, type TranslationDictionary } from "./locales/en";
import { i18nApi } from "../lib/api";

export type Locale = "en" | "ar";
export type Direction = "ltr" | "rtl";

interface I18nContextType {
  locale: Locale;
  setLocale: (nextLocale: Locale) => void;
  dir: Direction;
  isRTL: boolean;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const DICTIONARIES: Record<Locale, TranslationDictionary> = {
  en,
  ar,
};

// Synchronous FOLS (Flash of Layout Shift) Prevention: initialize directly from localStorage
function getInitialLocale(): Locale {
  try {
    const saved = localStorage.getItem("locus_locale");
    if (saved === "ar" || saved === "en") {
      return saved;
    }
  } catch {
    // Fallback if localStorage unavailable
  }
  return "en";
}

const I18nContext = createContext<I18nContextType | null>(null);

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);

  const dir: Direction = locale === "ar" ? "rtl" : "ltr";
  const isRTL = dir === "rtl";

  // Apply direction to HTML root immediately to eliminate FOUL/FOLS
  useEffect(() => {
    document.documentElement.dir = dir;
    document.documentElement.lang = locale;
    try {
      localStorage.setItem("locus_locale", locale);
    } catch {}

    // Async background sync with Tauri IPC
    i18nApi.setLocale(locale).catch(() => {});
  }, [locale, dir]);

  // Initial sync with backend IPC if available
  useEffect(() => {
    i18nApi
      .getLocale()
      .then((backendLocale) => {
        if (backendLocale === "ar" || backendLocale === "en") {
          if (backendLocale !== locale) {
            setLocaleState(backendLocale as Locale);
          }
        }
      })
      .catch(() => {});
  }, []);

  const setLocale = (nextLocale: Locale) => {
    setLocaleState(nextLocale);
  };

  // Translation lookup with nested keys ("diff.apply_selected") and Unicode BiDi isolation
  const t = (key: string, params?: Record<string, string | number>): string => {
    const dict = DICTIONARIES[locale] || DICTIONARIES.en;
    const parts = key.split(".");

    let current: any = dict;
    for (const part of parts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        // Fallback to English
        let fb: any = DICTIONARIES.en;
        for (const p of parts) {
          if (fb && typeof fb === "object" && p in fb) {
            fb = fb[p];
          } else {
            return key;
          }
        }
        current = fb;
        break;
      }
    }

    if (typeof current !== "string") {
      return key;
    }

    let result = current;

    if (params) {
      for (const [pKey, pVal] of Object.entries(params)) {
        const rawVal = String(pVal);

        // Unicode BiDi Isolation: Wrap numbers, paths, and ratios in Left-to-Right marks (\u200E)
        // when rendering in RTL context to prevent bidirectional text distortion.
        const safeVal =
          isRTL && (/[\d/:.\-_]/.test(rawVal) || rawVal.includes("@"))
            ? `\u200E${rawVal}\u200E`
            : rawVal;

        result = result.replace(new RegExp(`\\{${pKey}\\}`, "g"), safeVal);
      }
    }

    return result;
  };

  return (
    <I18nContext.Provider value={{ locale, setLocale, dir, isRTL, t }}>
      {children}
    </I18nContext.Provider>
  );
};

export const useTranslation = (): I18nContextType => {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error("useTranslation must be used within an I18nProvider");
  }
  return ctx;
};

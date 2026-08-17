import React, { useEffect, useState, useCallback } from "react";
import { OmniSearchBar } from "./OmniSearchBar";
import { HUDTelemetryBar } from "./HUDTelemetryBar";
import { OmniResultCard } from "./OmniResultCard";
import { overlayApi } from "../lib/api";
import type {
  OmniIntent,
  OmniSearchResult,
  QuickVerifyReport,
  AmbientActionResult,
  AmbientTelemetry,
} from "../types";
import { useTranslation } from "../i18n";

export default function SpotlightHUD() {
  const { t } = useTranslation();

  const [query, setQuery] = useState("");
  const [intent, setIntent] = useState<OmniIntent | null>(null);
  const [results, setResults] = useState<OmniSearchResult[]>([]);
  const [formalReport, setFormalReport] = useState<QuickVerifyReport | null>(null);
  const [agentResult, setAgentResult] = useState<AmbientActionResult | null>(null);
  const [telemetry, setTelemetry] = useState<AmbientTelemetry | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [clipboardCode, setClipboardCode] = useState<string | null>(null);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  // Inspect clipboard on mount
  useEffect(() => {
    const fetchClipboard = async () => {
      try {
        if (navigator.clipboard && navigator.clipboard.readText) {
          const text = await navigator.clipboard.readText();
          if (text && text.trim().length > 0) {
            setClipboardCode(text.trim());
          }
        }
      } catch {
        // Safe fallback
      }
    };
    fetchClipboard();
  }, []);

  // Fetch telemetry periodic updates
  useEffect(() => {
    const fetchTelemetry = async () => {
      try {
        const tel = await overlayApi.getAmbientTelemetry();
        setTelemetry(tel);
      } catch {
        // Safe fallback
      }
    };

    fetchTelemetry();
    const interval = setInterval(fetchTelemetry, 3000);
    return () => clearInterval(interval);
  }, []);

  // Handle Query Changes and Intent Dispatch
  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      setIntent(null);
      setResults([]);
      setFormalReport(null);
      setAgentResult(null);
      return;
    }

    let isMounted = true;
    const executeQuery = async () => {
      setLoading(true);
      try {
        // 1. Parse Intent instantly (<0.1ms)
        const parsedIntent = await overlayApi.parseOmnibarInput(trimmed, clipboardCode);
        if (!isMounted) return;
        setIntent(parsedIntent);

        // 2. Dispatch based on intent type
        if (parsedIntent.type === "LocalSearch") {
          const searchHits = await overlayApi.queryOmniSearch(parsedIntent.data.query);
          if (isMounted) {
            setResults(searchHits);
            setFormalReport(null);
            setAgentResult(null);
            setSelectedIndex(0);
          }
        } else if (parsedIntent.type === "ChatMemory") {
          const chatHits = await overlayApi.searchChatMemory(parsedIntent.data.description);
          if (isMounted) {
            const mapped: OmniSearchResult[] = chatHits.map((h) => ({
              title: `${h.entry.role.toUpperCase()}: ${h.snippet}`,
              subtitle: h.entry.content.slice(0, 120),
              category: "Memory",
              score: h.score,
            }));
            setResults(mapped);
            setFormalReport(null);
            setAgentResult(null);
            setSelectedIndex(0);
          }
        } else if (parsedIntent.type === "FormalVerify") {
          const report = await overlayApi.runQuickFormalVerify(
            parsedIntent.data.target,
            clipboardCode
          );
          if (isMounted) {
            setFormalReport(report);
            setResults([]);
            setAgentResult(null);
          }
        } else if (parsedIntent.type === "AgentAction") {
          const actResult = await overlayApi.executeAmbientAgent(
            parsedIntent.data.prompt,
            clipboardCode
          );
          if (isMounted) {
            setAgentResult(actResult);
            setResults([]);
            setFormalReport(null);
          }
        } else {
          setResults([]);
          setFormalReport(null);
          setAgentResult(null);
        }
      } catch {
        // Safe fallback
      } finally {
        if (isMounted) setLoading(false);
      }
    };

    const debounceTimer = setTimeout(executeQuery, 80);
    return () => {
      isMounted = false;
      clearTimeout(debounceTimer);
    };
  }, [query, clipboardCode]);

  // Handle Safe Text Injection
  const handleInject = useCallback(
    async (textToInject: string) => {
      try {
        const report = await overlayApi.injectTextToActive(textToInject, true);
        setToastMessage(
          t("spotlight.injected_success", {
            bytes: report.bytes_injected,
            time: report.elapsed_ms.toFixed(1),
          })
        );
        setTimeout(async () => {
          await overlayApi.dismiss();
        }, 600);
      } catch {
        setToastMessage("Injection failed");
      }
    },
    [t]
  );

  // Global Keyboard Navigation
  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Escape") {
        e.preventDefault();
        await overlayApi.dismiss();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((prev) => (results.length > 0 ? (prev + 1) % results.length : 0));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((prev) =>
          results.length > 0 ? (prev - 1 + results.length) % results.length : 0
        );
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (agentResult?.generated_patch) {
          handleInject(agentResult.generated_patch);
        } else if (results.length > 0 && results[selectedIndex]) {
          const hit = results[selectedIndex];
          handleInject(hit.subtitle);
        }
      }
    },
    [results, selectedIndex, agentResult, handleInject]
  );

  return (
    <div className="flex items-center justify-center w-full h-screen p-3 bg-transparent select-none">
      {/* Glassmorphic Floating Container */}
      <div className="relative flex flex-col w-full max-w-[720px] bg-neutral-950/85 backdrop-blur-xl border border-neutral-800/80 rounded-2xl shadow-2xl overflow-hidden transition-all duration-200">
        {/* OmniBar Header */}
        <OmniSearchBar
          query={query}
          onChange={setQuery}
          onKeyDown={handleKeyDown}
          intent={intent}
          loading={loading}
        />

        {/* Results / Verifier / Action Body */}
        <div className="flex flex-col gap-2 p-3 max-h-[300px] overflow-y-auto custom-scrollbar">
          {/* Toast Notification */}
          {toastMessage && (
            <div className="p-2.5 bg-emerald-500/20 border border-emerald-500/40 rounded-xl text-center text-xs font-semibold text-emerald-300 animate-fade-in">
              ⚡ {toastMessage}
            </div>
          )}

          {/* Formal Verification Report */}
          {formalReport && (
            <OmniResultCard
              formalReport={formalReport}
              selected={true}
              onSelect={() => {}}
              onInject={handleInject}
            />
          )}

          {/* Ambient Agent Action Result */}
          {agentResult && (
            <OmniResultCard
              agentResult={agentResult}
              selected={true}
              onSelect={() => {}}
              onInject={handleInject}
            />
          )}

          {/* Local Search / Memory Results List */}
          {results.map((item, idx) => (
            <OmniResultCard
              key={`${item.title}-${idx}`}
              result={item}
              selected={idx === selectedIndex}
              onSelect={() => setSelectedIndex(idx)}
              onInject={handleInject}
            />
          ))}

          {/* Empty State */}
          {!loading &&
            !formalReport &&
            !agentResult &&
            results.length === 0 &&
            query.trim().length > 0 && (
              <div className="py-8 text-center text-xs text-neutral-500 font-mono">
                {t("spotlight.no_results")}
              </div>
            )}
        </div>

        {/* Live Telemetry Footer Bar */}
        <HUDTelemetryBar telemetry={telemetry} />
      </div>
    </div>
  );
}

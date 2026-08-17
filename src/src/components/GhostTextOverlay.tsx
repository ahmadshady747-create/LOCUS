import React, { useEffect, useRef, useState } from "react";
import { fim } from "../lib/api";
import { sounds } from "../lib/sound";
import type { FimCompletionResponse } from "../types";

interface GhostTextOverlayProps {
  content: string;
  cursorLine: number;
  cursorCol: number;
  filePath: string;
  onAcceptCompletion: (completionText: string) => void;
  disabled?: boolean;
}

let monotonicRequestId = 0;

export const GhostTextOverlay: React.FC<GhostTextOverlayProps> = ({
  content,
  cursorLine,
  cursorCol,
  filePath,
  onAcceptCompletion,
  disabled = false,
}) => {
  const [suggestion, setSuggestion] = useState<string>("");
  const [latency, setLatency] = useState<number>(0);
  const debounceTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (disabled || !content || content.trim().length === 0) {
      setSuggestion("");
      return;
    }

    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    // 150ms debounce before requesting FIM inline completion
    debounceTimerRef.current = window.setTimeout(async () => {
      const currentReqId = ++monotonicRequestId;

      try {
        const res: FimCompletionResponse = await fim.requestInlineCompletion({
          request_id: currentReqId,
          file_path: filePath,
          prefix: content,
          suffix: "",
          cursor_line: cursorLine,
          cursor_col: cursorCol,
          max_tokens: 32,
        });

        // Stale drop: only display if request is still active
        if (res.request_id === monotonicRequestId && res.suggested_text) {
          setSuggestion(res.suggested_text);
          setLatency(res.latency_ms);
        } else {
          setSuggestion("");
        }
      } catch {
        setSuggestion("");
      }
    }, 150);

    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [content, cursorLine, cursorCol, filePath, disabled]);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!suggestion) return;

    if (e.key === "Tab") {
      e.preventDefault();
      sounds.playSuccess();
      onAcceptCompletion(suggestion);
      setSuggestion("");
    } else if (e.key === "Escape") {
      e.preventDefault();
      sounds.playClick();
      setSuggestion("");
    }
  };

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [suggestion]);

  if (!suggestion) return null;

  return (
    <div className="flex items-center gap-2 px-3 py-1 bg-black/60 border border-teal-500/30 rounded-lg text-xs font-mono shadow-lg animate-fade-in">
      <span className="text-zinc-500 text-[10px]">Ghost FIM ({latency}ms):</span>
      <span className="text-teal-300 italic opacity-80">{suggestion}</span>
      <span className="text-[9px] px-1.5 py-0.2 rounded bg-white/10 text-zinc-400 font-sans">
        [Tab] to accept • [Esc] to dismiss
      </span>
    </div>
  );
};

export default GhostTextOverlay;

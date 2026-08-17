import React, { useRef, useState } from "react";
import { ergonomics } from "../lib/api";
import { sounds } from "../lib/sound";
import type { MentionCandidate } from "../types";

interface PromptPillTokenizerProps {
  value: string;
  onChange: (newValue: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  disabled?: boolean;
  workspaceRoot?: string;
}

// In-memory frontend LRU cache for 0ms instantaneous autocompletion
const MENTION_CACHE = new Map<string, MentionCandidate[]>();
const MAX_CACHE_SIZE = 50;

export const PromptPillTokenizer: React.FC<PromptPillTokenizerProps> = ({
  value,
  onChange,
  onSubmit,
  placeholder = "Ask LOCUS or use @file, @symbol, /fix, /test...",
  disabled = false,
  workspaceRoot,
}) => {
  const [suggestions, setSuggestions] = useState<MentionCandidate[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showDropdown, setShowDropdown] = useState(false);
  const [triggerQuery, setTriggerQuery] = useState("");
  const [triggerPos, setTriggerPos] = useState<number | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const debounceTimerRef = useRef<number | null>(null);

  // Detect active mention or slash command trigger on cursor position
  const checkForTriggers = (text: string, cursorPos: number) => {
    const textBeforeCursor = text.slice(0, cursorPos);
    const words = textBeforeCursor.split(/\s+/);
    const lastWord = words[words.length - 1] || "";

    if (lastWord.startsWith("@") || lastWord.startsWith("/")) {
      setTriggerQuery(lastWord);
      setTriggerPos(cursorPos - lastWord.length);
      fetchSuggestions(lastWord);
    } else {
      setShowDropdown(false);
      setSuggestions([]);
    }
  };

  const fetchSuggestions = (query: string) => {
    // 1. Check LRU cache for 0ms response
    if (MENTION_CACHE.has(query)) {
      const cached = MENTION_CACHE.get(query)!;
      setSuggestions(cached);
      setSelectedIndex(0);
      setShowDropdown(cached.length > 0);
      return;
    }

    // 2. Debounce IPC query by 40ms to avoid flooding
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    debounceTimerRef.current = window.setTimeout(async () => {
      try {
        const results = await ergonomics.queryMentions(query, workspaceRoot);

        // Store in cache
        if (MENTION_CACHE.size >= MAX_CACHE_SIZE) {
          const firstKey = MENTION_CACHE.keys().next().value;
          if (firstKey) MENTION_CACHE.delete(firstKey);
        }
        MENTION_CACHE.set(query, results);

        setSuggestions(results);
        setSelectedIndex(0);
        setShowDropdown(results.length > 0);
      } catch {
        setShowDropdown(false);
      }
    }, 40);
  };

  const handleSelectCandidate = (candidate: MentionCandidate) => {
    sounds.playClick();
    if (triggerPos === null) return;

    const before = value.slice(0, triggerPos);
    const after = value.slice(triggerPos + triggerQuery.length);
    const inserted = `${candidate.value} `;
    const updated = before + inserted + after;

    onChange(updated);
    setShowDropdown(false);

    // Reposition cursor after inserted pill
    setTimeout(() => {
      if (textareaRef.current) {
        const newCursorPos = before.length + inserted.length;
        textareaRef.current.focus();
        textareaRef.current.setSelectionRange(newCursorPos, newCursorPos);
      }
    }, 10);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (showDropdown && suggestions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % suggestions.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : suggestions.length - 1));
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        handleSelectCandidate(suggestions[selectedIndex]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setShowDropdown(false);
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onSubmit();
    }
  };

  // Render visual pill badges from typed mentions
  const extractedPills = value
    .split(/\s+/)
    .filter((w) => w.startsWith("@") || (w.startsWith("/") && w.length > 1));

  return (
    <div className="relative w-full flex flex-col space-y-1.5">
      {/* Visual Pill Capsule Bar */}
      {extractedPills.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5 px-2 py-1 bg-black/40 rounded-lg border border-white/5 text-[11px] font-mono animate-fade-in">
          <span className="text-zinc-500 text-[10px]">Context Pills:</span>
          {extractedPills.map((pill, idx) => {
            const isCommand = pill.startsWith("/");
            return (
              <span
                key={`${pill}-${idx}`}
                className={`px-2 py-0.5 rounded-full border flex items-center gap-1 shadow-sm ${
                  isCommand
                    ? "bg-amber-500/15 text-amber-300 border-amber-500/30"
                    : "bg-teal-500/15 text-teal-300 border-teal-500/30"
                }`}
              >
                <span>{isCommand ? "⚡" : "📌"}</span>
                <span>{pill}</span>
              </span>
            );
          })}
        </div>
      )}

      {/* Input Area */}
      <div className="relative">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            checkForTriggers(e.target.value, e.target.selectionStart || 0);
          }}
          onKeyDown={handleKeyDown}
          onClick={(e) => {
            const target = e.target as HTMLTextAreaElement;
            checkForTriggers(value, target.selectionStart || 0);
          }}
          placeholder={placeholder}
          disabled={disabled}
          rows={3}
          className="w-full bg-[#0A0D14] border border-white/10 focus:border-teal-500/60 rounded-xl p-3 text-xs text-white placeholder:text-zinc-600 focus:outline-none resize-none font-sans leading-relaxed shadow-inner"
        />

        {/* Mention Suggestions Dropdown */}
        {showDropdown && suggestions.length > 0 && (
          <div className="absolute bottom-full left-0 mb-1 w-full max-h-56 overflow-y-auto bg-[#0E131F] border border-teal-500/30 rounded-xl shadow-2xl z-50 p-1.5 space-y-1 font-mono text-xs animate-scale-up">
            <div className="px-2 py-1 text-[10px] text-zinc-500 font-bold uppercase tracking-wider flex items-center justify-between border-b border-white/5">
              <span>Suggestions ({triggerQuery})</span>
              <span className="text-zinc-600">↑↓ to navigate • ↵ to select</span>
            </div>

            {suggestions.map((item, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={`${item.value}-${idx}`}
                  onClick={() => handleSelectCandidate(item)}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  className={`px-2.5 py-1.5 rounded-lg cursor-pointer flex items-center justify-between transition-all ${
                    isSelected
                      ? "bg-teal-500/20 text-teal-200 border border-teal-500/40"
                      : "hover:bg-white/5 text-zinc-300"
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="text-sm shrink-0">{item.icon}</span>
                    <div className="truncate">
                      <span className="font-bold text-white mr-1.5">{item.label}</span>
                      <span className="text-zinc-400 text-[11px]">{item.description}</span>
                    </div>
                  </div>
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-white/5 text-zinc-400 font-mono shrink-0 ml-2">
                    {item.mention_type}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

export default PromptPillTokenizer;

import { useCallback, useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AppState, ChatMessage, Template } from "../types";
import { context, llm, templates as templateApi } from "../lib/api";
import { sounds } from "../lib/sound";

interface ChatProps {
  state: AppState;
}

const QUICK_PROMPTS = [
  { label: "⚡ Optimize Code", prompt: "Review and optimize this code for maximum performance and minimum allocations:" },
  { label: "🧪 Generate Unit Tests", prompt: "Write comprehensive unit tests with edge cases for the following function:" },
  { label: "🐛 Find & Fix Bugs", prompt: "Analyze this code for potential concurrency issues, memory leaks, or logical bugs:" },
  { label: "✨ Refactor Module", prompt: "Refactor this code to follow clean architecture principles, idiomatic Rust/TypeScript patterns:" },
  { label: "🔍 Explain Logic", prompt: "Explain step-by-step how this implementation works and its time/space complexity:" },
];

export const ARCHITECTURAL_MODES = [
  {
    command: "/grill",
    label: "🔥 /grill",
    title: "Strict Architecture Critique",
    description: "Strict architectural critique and vulnerability probe",
    prefix: "/grill Analyze the architecture, performance bottlenecks, concurrency hazards, and edge cases in the following code with maximum rigor:\n\n",
    badgeClass: "bg-rose-500/15 text-rose-300 border-rose-500/30 hover:bg-rose-500/25",
    activeClass: "bg-rose-600 text-white border-rose-400 shadow-sm font-semibold",
  },
  {
    command: "/plan",
    label: "📐 /plan",
    title: "Architecture Planner",
    description: "Production-grade technical implementation plan",
    prefix: "/plan Create a comprehensive, production-grade technical implementation plan with architecture breakdown, data structures, and step-by-step verification:\n\n",
    badgeClass: "bg-violet-500/15 text-violet-300 border-violet-500/30 hover:bg-violet-500/25",
    activeClass: "bg-violet-600 text-white border-violet-400 shadow-sm font-semibold",
  },
  {
    command: "/spec",
    label: "📋 /spec",
    title: "Formal Tech Specs",
    description: "Formal technical specifications and contracts",
    prefix: "/spec Generate formal technical specifications, API schemas, data contracts, and architectural invariants for:\n\n",
    badgeClass: "bg-cyan-500/15 text-cyan-300 border-cyan-500/30 hover:bg-cyan-500/25",
    activeClass: "bg-cyan-600 text-white border-cyan-400 shadow-sm font-semibold",
  },
];

function Chat({ state }: ChatProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      role: "assistant",
      content:
        "👋 **Welcome to LOCUS** — your local-first AI coding companion.\n\nEverything runs directly on your machine without cloud dependency. Ask me to generate code, refactor functions, write tests, or analyze errors in your workspace.",
      timestamp: Date.now(),
      status: "done",
    },
  ]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [selectedTemplates, setSelectedTemplates] = useState<Template[]>([]);
  const [showTemplatePicker, setShowTemplatePicker] = useState(false);
  const [templateResults, setTemplateResults] = useState<Template[]>([]);
  const [templateQuery, setTemplateQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [validationRunning, setValidationRunning] = useState<string | null>(null);

  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streaming]);

  useEffect(() => {
    if (templateQuery.trim()) {
      templateApi.search(templateQuery).then(setTemplateResults).catch(() => {});
    } else {
      templateApi.list().then((t) => setTemplateResults(t)).catch(() => {});
    }
  }, [templateQuery, showTemplatePicker]);

  const assemblePrompt = useCallback(
    async (request: string, tpls: Template[]) => {
      try {
        const result = await context.assemble({
          user_request: request,
          templates: tpls,
        });
        return result.full_prompt;
      } catch (e) {
        console.error("Context assembly failed, using raw request", e);
        return request;
      }
    },
    [],
  );

  const handleSend = useCallback(async (customText?: string) => {
    const text = (customText ?? input).trim();
    if (!text || streaming) return;

    sounds.playSend();
    setInput("");
    setError(null);

    const userMessage: ChatMessage = {
      role: "user",
      content: text,
      timestamp: Date.now(),
      status: "done",
    };
    setMessages((m) => [...m, userMessage]);
    setMessages((m) => [
      ...m,
      {
        role: "assistant",
        content: "",
        timestamp: Date.now(),
        status: "pending",
      },
    ]);
    setStreaming(true);

    const prompt = await assemblePrompt(text, selectedTemplates);
    const model = state.selectedModel ?? undefined;

    try {
      const result = await llm.generate({
        prompt,
        model,
        temperature: 0.7,
        max_tokens: 4096,
      });

      sounds.playReceive();
      setMessages((m) => {
        const next = [...m];
        const resObj = result as {
          response: string;
          model: string;
          backend: string;
          provider_stamp?: string;
          latency_ms?: number;
          was_fallback?: boolean;
          fallback_reason?: string;
        };

        next[next.length - 1] = {
          ...next[next.length - 1],
          content: resObj.response,
          status: "done",
          provider_stamp: resObj.provider_stamp ?? (model ? `🔒 Local (${model})` : "🔒 Local (Ollama)"),
          model_used: resObj.model,
          latency_ms: resObj.latency_ms,
          was_fallback: resObj.was_fallback,
          fallback_reason: resObj.fallback_reason,
        };
        return next;
      });

      if (result.response.includes("```")) {
        void runValidationAgent(result.response);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setMessages((m) => {
        const next = [...m];
        next[next.length - 1] = {
          ...next[next.length - 1],
          content: "⚠️ Generation encountered an error. Check details below or verify your local model backend.",
          status: "error",
        };
        return next;
      });
    } finally {
      setStreaming(false);
    }
  }, [input, streaming, selectedTemplates, assemblePrompt, state.selectedModel]);

  const runValidationAgent = async (code: string) => {
    try {
      const fenced = /```(\w+)\n([\s\S]*?)```/.exec(code);
      if (!fenced) return;
      const lang = fenced[1];
      const body = fenced[2];
      setValidationRunning(lang);

      let testCmd: string | undefined;
      switch (lang) {
        case "python":
          testCmd = `python3 -m py_compile /tmp/locus_agent.py && echo "SYNTAX OK"`;
          break;
        case "javascript":
        case "typescript":
          testCmd = `node --check /tmp/locus_agent.js && echo "SYNTAX OK"`;
          break;
        case "rust":
          testCmd = `rustc --edition 2021 /tmp/locus_agent.rs 2>&1 && echo "COMPILE OK"`;
          break;
        default:
          setValidationRunning(null);
          return;
      }

      const result = await import("../lib/api").then((m) =>
        m.agents.executeTask({
          context: body,
          language: lang,
          test_command: testCmd,
          timeout_seconds: 30,
          max_memory_mb: 256,
        }),
      );

      if (result?.success) {
        sounds.playSuccess();
      }

      if (result) {
        setMessages((m) => [
          ...m,
          {
            role: "assistant",
            content:
              result.success
                ? `🛡️ **Agent Sandbox Verification**: Passed in **${result.duration_ms}ms** (Language: \`${lang}\`, Sandbox memory OK)`
                : `⚠️ **Agent Sandbox Verification Alert**: Execution reported issues in **${result.duration_ms}ms**:\n\n\`\`\`\n${result.errors?.join("\n") || "Unknown error"}\n\`\`\``,
            timestamp: Date.now(),
            status: "done",
          },
        ]);
      }
    } catch (e) {
      console.error("Validation agent failed", e);
    } finally {
      setValidationRunning(null);
    }
  };

  const toggleTemplate = (t: Template) => {
    sounds.playClick();
    setSelectedTemplates((prev) =>
      prev.some((x) => x.id === t.id)
        ? prev.filter((x) => x.id !== t.id)
        : [...prev, t],
    );
  };

  const clearChat = () => {
    sounds.playClick();
    setMessages([
      {
        role: "assistant",
        content: "Chat cleared. What would you like to build or inspect next?",
        timestamp: Date.now(),
        status: "done",
      },
    ]);
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-locus-bg">
      {/* Header bar */}
      <div className="h-11 px-4 flex items-center justify-between border-b border-locus-border/80 glass-panel shrink-0">
        <div className="flex items-center gap-2.5">
          <span className="text-xs font-semibold tracking-wide text-locus-text">AI ASSISTANT</span>
          <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-locus-accent/15 border border-locus-accent/30 text-locus-accent text-[10px] font-medium shadow-glow-emerald">
            <span className="status-dot-online" />
            {state.privacyMode === "local" ? "Air-Gapped Local" : "Mesh Network"}
          </span>
          {state.selectedModel && (
            <span className="text-[10px] text-locus-muted px-2 py-0.5 bg-white/5 border border-white/10 rounded-md font-mono">
              {state.selectedModel}
            </span>
          )}
          {validationRunning && (
            <span className="inline-flex items-center gap-1 text-[10px] text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded-md border border-amber-500/20 animate-pulse">
              <span className="status-dot-busy" />
              Testing {validationRunning} in Sandbox…
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {selectedTemplates.length > 0 && (
            <span className="text-[10px] text-locus-violet font-mono px-2 py-0.5 bg-locus-violet/10 rounded-md border border-locus-violet/20">
              {selectedTemplates.length} context template(s)
            </span>
          )}
          <button
            onClick={clearChat}
            className="btn-ghost"
            title="Clear conversation"
          >
            <TrashIcon />
            Clear
          </button>
        </div>
      </div>

      {/* Messages area */}
      <div className="flex-1 overflow-y-auto px-4 py-5 space-y-4">
        <div className="max-w-3xl mx-auto space-y-4 w-full">
          {messages.map((msg, i) => (
            <MessageBubble
              key={i}
              msg={msg}
              streaming={streaming && i === messages.length - 1}
              onRetry={() => handleSend(messages[i - 1]?.content)}
            />
          ))}

          {error && (
            <div className="p-3.5 rounded-xl bg-red-500/10 border border-red-500/20 text-red-300 text-xs flex items-start gap-2.5 animate-fade-in">
              <span className="text-red-400 text-sm font-bold">✕</span>
              <div className="flex-1">
                <div className="font-semibold mb-0.5">Execution Warning</div>
                <div className="font-mono text-[11px] opacity-90">{error}</div>
              </div>
            </div>
          )}

          <div ref={bottomRef} />
        </div>
      </div>

      {/* Quick Prompts Bar (when idle) */}
      {!streaming && messages.length <= 2 && (
        <div className="px-4 py-2 bg-locus-bg shrink-0">
          <div className="max-w-3xl mx-auto flex items-center gap-1.5 overflow-x-auto pb-1">
            {QUICK_PROMPTS.map((qp) => (
              <button
                key={qp.label}
                onClick={() => {
                  sounds.playClick();
                  setInput(qp.prompt);
                  textareaRef.current?.focus();
                }}
                className="btn-secondary text-[11px] py-1 px-2.5 whitespace-nowrap bg-locus-card/80 border-locus-border/60 hover:border-locus-violet/40"
              >
                {qp.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Template picker tray */}
      {showTemplatePicker && (
        <div className="border-t border-locus-border/80 glass-panel px-4 py-3 shrink-0 animate-fade-in">
          <div className="max-w-3xl mx-auto space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-locus-text">Context Templates</span>
              <button
                className="text-xs text-locus-muted hover:text-white"
                onClick={() => setShowTemplatePicker(false)}
              >
                Close ✕
              </button>
            </div>
            <input
              className="input-dark input-compact"
              placeholder="Filter templates (e.g. auth, api, react, database)…"
              value={templateQuery}
              onChange={(e) => setTemplateQuery(e.target.value)}
            />
            <div className="flex gap-1.5 flex-wrap max-h-28 overflow-y-auto pt-1">
              {templateResults.map((t) => {
                const active = selectedTemplates.some((x) => x.id === t.id);
                return (
                  <button
                    key={t.id}
                    onClick={() => toggleTemplate(t)}
                    className={`px-2.5 py-1 rounded-md text-[11px] border transition-all duration-150 ${
                      active
                        ? "tag-active"
                        : "tag hover:border-locus-violet/40 hover:text-locus-text"
                    }`}
                  >
                    {active ? "✓ " : "+ "}
                    <span className="opacity-70">{t.category}/</span>
                    {t.name}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* Input area */}
      <div className="border-t border-locus-border/80 glass-panel px-4 py-3 shrink-0">
        <div className="max-w-3xl mx-auto space-y-2">
          {/* Architectural Modes Quick Bar */}
          <div className="flex items-center justify-between pb-0.5">
            <div className="flex items-center gap-1.5 overflow-x-auto">
              <span className="text-[10px] uppercase font-bold text-zinc-500 tracking-wider select-none shrink-0 mr-1">
                Modes:
              </span>
              {ARCHITECTURAL_MODES.map((mode) => {
                const isActive = input.startsWith(mode.command);
                return (
                  <button
                    key={mode.command}
                    type="button"
                    onClick={() => {
                      sounds.playClick();
                      if (isActive) {
                        setInput((prev) => prev.replace(new RegExp(`^${mode.command}\\s*`, "i"), ""));
                      } else {
                        setInput((prev) => {
                          const cleaned = prev.replace(/^\/(grill|plan|spec)\s*/i, "");
                          return `${mode.command} ${cleaned || mode.prefix.replace(`${mode.command} `, "")}`;
                        });
                      }
                      textareaRef.current?.focus();
                    }}
                    className={`text-[11px] font-mono px-2.5 py-0.5 rounded-md border transition-all duration-150 flex items-center gap-1 shrink-0 ${
                      isActive
                        ? mode.activeClass
                        : `${mode.badgeClass} text-zinc-300`
                    }`}
                    title={mode.description}
                  >
                    <span>{mode.label}</span>
                    {isActive && <span className="text-[9px]">●</span>}
                  </button>
                );
              })}
            </div>

            {input.match(/^\/(grill|plan|spec)/i) && (
              <span className="text-[10px] font-mono text-violet-400 animate-pulse hidden sm:inline-block">
                ⚡ System Prompt Injected
              </span>
            )}
          </div>

          <div className="flex items-end gap-2">
            <button
              className={`p-2.5 rounded-xl border text-xs transition-all duration-150 shrink-0 ${
                showTemplatePicker || selectedTemplates.length > 0
                  ? "bg-locus-violet/15 border-locus-violet/40 text-locus-violet shadow-glow-violet"
                  : "bg-locus-card border-locus-border/80 text-locus-muted hover:text-locus-text hover:border-locus-border"
              }`}
              title="Add Context Templates"
              onClick={() => {
                sounds.playClick();
                setShowTemplatePicker((v) => !v);
              }}
            >
              <TemplateIcon />
            </button>

            <textarea
              ref={textareaRef}
              className="flex-1 bg-[#090b10] border border-locus-border/80 rounded-xl px-3.5 py-2.5 text-sm text-locus-text placeholder:text-locus-muted/40 focus:outline-none focus:border-locus-violet/70 focus:ring-1 focus:ring-locus-violet/40 resize-none max-h-36 transition-all"
              placeholder="Ask LOCUS to write, refactor, or debug code… (Enter to send, Shift+Enter for newline)"
              rows={1}
              value={input}
              onChange={(e) => {
                setInput(e.target.value);
                const el = e.target;
                el.style.height = "auto";
                el.style.height = Math.min(el.scrollHeight, 150) + "px";
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSend();
                }
              }}
            />

            <button
              className="btn-primary py-2.5 px-4 rounded-xl shrink-0"
              onClick={() => handleSend()}
              disabled={streaming || !input.trim()}
              title="Send message"
              aria-label="Send message"
            >
              {streaming ? (
                <LoaderIcon className="w-4 h-4 animate-spin text-white" />
              ) : (
                <SendIcon />
              )}
            </button>
          </div>

          <div className="flex items-center justify-between text-[10px] text-locus-muted font-mono px-1">
            <span>
              {streaming
                ? "⚡ Synthesizing response locally…"
                : `${state.privacyMode === "local" ? "🔒 100% On-Device" : "🌐 Mesh Connected"} · ${selectedTemplates.length} template(s)`}
            </span>
            <span className="opacity-60">Ctrl+Enter / Enter to send</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({
  msg,
  streaming,
  onRetry,
}: {
  msg: ChatMessage;
  streaming: boolean;
  onRetry?: () => void;
}) {
  const isUser = msg.role === "user";
  const [copied, setCopied] = useState(false);

  const copyContent = async () => {
    sounds.playClick();
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(msg.content);
      }
    } catch {
      // ignore clipboard permission failures
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  if (isUser) {
    return (
      <div className="flex justify-end animate-fade-in">
        <div className="max-w-[85%] px-4 py-2.5 rounded-2xl rounded-tr-sm bg-gradient-to-r from-locus-violet/20 to-locus-violet/10 border border-locus-violet/30 text-locus-text text-sm whitespace-pre-wrap shadow-glow-violet font-sans leading-relaxed">
          {msg.content}
        </div>
      </div>
    );
  }

  return (
    <div className="flex justify-start animate-fade-in">
      <div className="max-w-[95%] w-full">
        <div className="flex items-center justify-between mb-1.5 px-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-[11px] font-bold tracking-wider text-locus-violet flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-locus-violet animate-pulse" />
              LOCUS
            </span>
            {msg.provider_stamp && (
              <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-violet-500/15 text-violet-200 border border-violet-500/30 font-medium">
                {msg.provider_stamp}
              </span>
            )}
            {msg.latency_ms !== undefined && msg.latency_ms > 0 && (
              <span className="text-[10px] font-mono text-zinc-400 px-1.5 py-0.2 bg-white/5 rounded border border-white/5">
                ⏱️ {msg.latency_ms}ms
              </span>
            )}
            {streaming && (
              <span className="text-[10px] text-locus-muted animate-pulse-subtle font-mono">
                generating…
              </span>
            )}
            {msg.status === "error" && (
              <span className="text-[10px] text-red-400 font-semibold">error</span>
            )}
          </div>

          {!streaming && msg.content && (
            <div className="flex items-center gap-1">
              <button
                onClick={copyContent}
                className="btn-ghost py-0.5 px-1.5 text-[10px] opacity-70 hover:opacity-100 font-mono"
                title="Copy response"
              >
                {copied ? "✓ Copied" : "Copy"}
              </button>
              {onRetry && (
                <button
                  onClick={() => {
                    sounds.playClick();
                    onRetry();
                  }}
                  className="btn-ghost py-0.5 px-1.5 text-[10px] opacity-70 hover:opacity-100 font-mono"
                  title="Regenerate"
                >
                  ↻ Retry
                </button>
              )}
            </div>
          )}
        </div>

        <div className="panel p-4 text-sm text-locus-text shadow-sm relative group">
          {/* Fallback Notice Banner */}
          {msg.was_fallback && (
            <div className="mb-3 p-2.5 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-200 text-xs font-mono flex items-start gap-2 animate-fade-in">
              <span className="text-sm">🔄</span>
              <div>
                <div className="font-bold text-amber-300">تم التحويل تلقائياً للمزود البديل (Auto-Failover)</div>
                <div className="text-[11px] text-amber-200/80 mt-0.5">
                  {msg.fallback_reason
                    ? `انتقل التوجيه بسبب انشغال المزود الأول أو تجاوزه للحدود (${msg.fallback_reason})`
                    : "تم تحويل الطلب تلقائياً لضمان عدم انقطاع الخدمة."}
                </div>
              </div>
            </div>
          )}

          {msg.content ? (
            <div className="prose prose-invert prose-sm max-w-none [&_pre]:bg-[#06080c] [&_pre]:border [&_pre]:border-white/5 [&_pre]:rounded-xl [&_pre]:p-3.5 [&_code]:text-violet-300 [&_code]:bg-white/5 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded-md [&_code]:font-mono [&_p]:leading-relaxed">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>
                {msg.content}
              </ReactMarkdown>
            </div>
          ) : streaming ? (
            <div className="flex items-center gap-2 py-2 text-locus-muted">
              <Dot delay="0ms" />
              <Dot delay="150ms" />
              <Dot delay="300ms" />
              <span className="text-xs font-mono ml-2 opacity-70">Synthesizing response…</span>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function Dot({ delay }: { delay: string }) {
  return (
    <span
      className="w-2 h-2 rounded-full bg-locus-violet animate-bounce shadow-glow-violet"
      style={{ animationDelay: delay }}
    />
  );
}

function LoaderIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeDasharray="24" strokeDashoffset="8">
        <animateTransform attributeName="transform" type="rotate" from="0 8 8" to="360 8 8" dur="0.9s" repeatCount="indefinite" />
      </circle>
    </svg>
  );
}

function SendIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <path d="M13.5 2.5L2.5 7.5L7.5 9.5L9.5 14.5L13.5 2.5Z" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
      <path d="M7.5 9.5L13.5 2.5" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function TemplateIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <rect x="2" y="3" width="12" height="10" rx="2" stroke="currentColor" strokeWidth={1.5} />
      <path d="M5 7h6M5 10h4" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  );
}

function TrashIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" className={className}>
      <path d="M3 4h10M6 4V2.5a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5V4M4.5 4v9a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V4" stroke="currentColor" strokeWidth={1.4} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export default Chat;
import { useEffect, useState, useCallback, useRef } from "react";
import {
  BookOpen,
  ChevronDown,
  ChevronRight,
  Clock,
  Compass,
  CreditCard,
  Plus,
  Rocket,
  Send,
  ShoppingBag,
  Sparkles,
  Swords,
  Trash2,
  User,
  Users,
  Wrench,
} from "lucide-react";
import {
  api,
  AuthRequiredError,
  AuthUser,
  Campaign,
  CampaignSnapshot,
  formatCr,
  formatImperialDate,
  MessageRecord,
  TurnEvent,
} from "./lib/api";
import { MarkdownMessage } from "./components/MarkdownMessage";
import { StarMapPanel } from "./components/StarMapPanel";
import { CharactersPanel } from "./components/CharactersPanel";
import { ShipsPanel } from "./components/ShipsPanel";
import { TradeShopPanel } from "./components/TradeShopPanel";
import { CombatPanel } from "./components/CombatPanel";
import { FinancesCrewPanel } from "./components/FinancesCrewPanel";
import { ModulesPanel } from "./components/ModulesPanel";

type Tab = "map" | "characters" | "ships" | "trade" | "combat" | "finances" | "modules";

interface LiveToolCard {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  is_error?: boolean;
}

const QUICK_PROMPTS = [
  "Survey the starport, local news, and ships in system.",
  "Check the starport lounge for patron jobs or freight leads.",
  "Run a readiness check on our ship, fuel, and crew.",
];

export default function App() {
  const [authUser, setAuthUser] = useState<AuthUser | null>(null);
  const [authLoginUrl, setAuthLoginUrl] = useState<string | null>(null);
  const [campaigns, setCampaigns] = useState<Campaign[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<CampaignSnapshot | null>(null);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [tab, setTab] = useState<Tab>("characters");
  const [charPanelMode, setCharPanelMode] = useState<"roster" | "lifepath">("roster");

  // Chat state
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [statusChip, setStatusChip] = useState<string | null>(null);
  const [streamText, setStreamText] = useState("");
  const [liveTools, setLiveTools] = useState<LiveToolCard[]>([]);
  const [expandedToolIds, setExpandedToolIds] = useState<Record<string, boolean>>({});
  const [errorBanner, setErrorBanner] = useState<string | null>(null);

  // Campaign creation
  const [newCampName, setNewCampName] = useState("Spinward Marches Free Trader");

  const chatEndRef = useRef<HTMLDivElement>(null);

  const refreshCampaigns = useCallback(async () => {
    try {
      const [{ user }, { campaigns: list }] = await Promise.all([
        api.authMe(),
        api.listCampaigns(),
      ]);
      setAuthUser(user);
      setAuthLoginUrl(null);
      setCampaigns(list);
      if (!activeId && list.length > 0) {
        setActiveId(list[0].id);
      } else if (activeId && !list.some((c) => c.id === activeId)) {
        setActiveId(list[0]?.id ?? null);
      }
    } catch (e) {
      if (e instanceof AuthRequiredError) {
        setAuthLoginUrl(e.loginUrl);
      } else {
        setErrorBanner((e as Error).message);
      }
    }
  }, [activeId]);

  const refreshActive = useCallback(async (id: string) => {
    try {
      const [snap, msgs] = await Promise.all([api.getSnapshot(id), api.listMessages(id)]);
      setSnapshot(snap);
      setMessages(msgs.messages);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }, []);

  useEffect(() => {
    refreshCampaigns();
  }, [refreshCampaigns]);

  useEffect(() => {
    if (activeId) refreshActive(activeId);
    else {
      setSnapshot(null);
      setMessages([]);
    }
  }, [activeId, refreshActive]);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamText, liveTools]);

  async function handleCreateCampaign() {
    if (!newCampName.trim()) return;
    setErrorBanner(null);
    try {
      const { campaign } = await api.createCampaign(newCampName.trim());
      await refreshCampaigns();
      setActiveId(campaign.id);
      setTab("characters");
      setCharPanelMode("lifepath");
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleDeleteActiveCampaign() {
    if (!activeId || !snapshot) return;
    if (!confirm(`Delete game "${snapshot.campaign.name}" permanently?`)) return;
    try {
      await api.deleteCampaign(activeId);
      setActiveId(null);
      await refreshCampaigns();
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function sendChatText(text: string) {
    if (!activeId || !text.trim() || streaming) return;
    const clean = text.trim();
    setStreaming(true);
    setStreamText("");
    setLiveTools([]);
    setStatusChip("Sending to Referee…");
    setErrorBanner(null);

    setMessages((prev) => [
      ...prev,
      {
        id: Date.now(),
        role: "user",
        content: clean,
        superseded: false,
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        created_at: new Date().toISOString(),
      },
    ]);

    try {
      await api.streamChat(activeId, clean, (ev: TurnEvent) => {
        if (ev.type === "status") setStatusChip(ev.message);
        else if (ev.type === "clear_draft") setStreamText("");
        else if (ev.type === "delta") setStreamText((prev) => prev + ev.text);
        else if (ev.type === "tool_call") {
          setLiveTools((prev) => [...prev, { id: ev.id, name: ev.name, input: ev.input }]);
        } else if (ev.type === "tool_result") {
          setLiveTools((prev) =>
            prev.map((t) =>
              t.id === ev.id ? { ...t, result: ev.result, is_error: ev.is_error } : t
            )
          );
        } else if (ev.type === "state_changed") {
          refreshActive(activeId);
        } else if (ev.type === "error") {
          setErrorBanner(ev.message);
        } else if (ev.type === "done") {
          setStatusChip(null);
        }
      });
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    } finally {
      setStreaming(false);
      setStreamText("");
      setStatusChip(null);
    }
  }

  async function handleSend() {
    if (!input.trim()) return;
    const text = input;
    setInput("");
    await sendChatText(text);
  }

  async function handleAdvanceTime(opts: Record<string, unknown>) {
    if (!activeId || streaming) return;
    try {
      await api.advanceTime(activeId, opts);
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  if (authLoginUrl) {
    return (
      <div className="min-h-screen bg-slate-950 text-slate-100 flex items-center justify-center p-6">
        <div className="max-w-md w-full bg-slate-900 border border-slate-800 rounded-xl p-6 text-center space-y-4 shadow-2xl">
          <div className="text-xl font-bold text-cyan-400">TravellerLLM</div>
          <p className="text-sm text-slate-400">
            Sign in via Authentik to access your starship bridge.
          </p>
          <a
            href={authLoginUrl}
            className="inline-block bg-cyan-600 hover:bg-cyan-500 text-white font-semibold px-5 py-2 rounded-lg transition-colors"
          >
            Sign In with Authentik
          </a>
        </div>
      </div>
    );
  }

  const playerChar = snapshot?.characters.find((r) => r.character.kind === "player")?.character;
  const companyAcc = snapshot?.accounts.find((a) => a.kind === "company");
  const hasActiveCombat = Boolean(snapshot?.active_space_combat || snapshot?.active_personal_combat);

  return (
    <div className="h-screen flex flex-col bg-slate-950 text-slate-100 overflow-hidden">
      {/* Top Imperial Command Bar */}
      <header className="h-13 shrink-0 border-b border-slate-800/90 bg-slate-900/95 px-3 flex items-center justify-between gap-2 shadow-md">
        <div className="flex items-center gap-2 min-w-0">
          <div className="flex items-center gap-1.5 font-bold tracking-tight text-cyan-400 pr-1">
            <Sparkles className="w-4 h-4" />
            <span>TravellerLLM</span>
          </div>
          <select
            className="bg-slate-950 border border-slate-700 hover:border-slate-600 rounded-md px-2 py-1 text-xs max-w-[180px] transition-colors"
            value={activeId ?? ""}
            onChange={(e) => {
              setActiveId(e.target.value);
              setCharPanelMode("roster");
            }}
          >
            {campaigns.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <button
            disabled={!activeId}
            onClick={handleDeleteActiveCampaign}
            title="Delete current game"
            className="p-1.5 rounded text-slate-500 hover:text-red-400 hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
          <div className="hidden lg:flex items-center gap-1 pl-1 border-l border-slate-800">
            <input
              className="bg-slate-950 border border-slate-800 rounded-md px-2 py-1 text-xs w-40 focus:outline-none focus:border-cyan-600"
              placeholder="New game name…"
              value={newCampName}
              onChange={(e) => setNewCampName(e.target.value)}
            />
            <button
              disabled={!newCampName.trim()}
              onClick={handleCreateCampaign}
              className="bg-cyan-700 hover:bg-cyan-600 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded-md px-2.5 py-1 text-xs flex items-center gap-1 font-medium transition-colors"
            >
              <Plus className="w-3.5 h-3.5" /> New Game
            </button>
          </div>
        </div>

        {/* Status Telemetry & Quick Time Controls */}
        {snapshot && (
          <div className="flex items-center gap-2 text-xs">
            <div className="hidden xl:flex items-center gap-2 bg-slate-950/90 border border-slate-800 rounded-md px-2.5 py-1 font-mono text-[11px]">
              <span className="flex items-center gap-1 text-amber-300">
                <Clock className="w-3.5 h-3.5 text-amber-400" />
                {formatImperialDate(snapshot.campaign.date)}
              </span>
              <span className="text-slate-700">|</span>
              <span className="text-cyan-300">
                {snapshot.campaign.current_system_name} (Port{" "}
                {snapshot.current_system_info?.starport ?? "?"})
              </span>
              <span className="text-slate-700">|</span>
              <span className="text-emerald-300">{snapshot.system_ship_count ?? 0} ships</span>
              {companyAcc && (
                <>
                  <span className="text-slate-700">|</span>
                  <span className={companyAcc.balance_cr >= 0 ? "text-emerald-400" : "text-red-400"}>
                    {formatCr(companyAcc.balance_cr)}
                  </span>
                </>
              )}
              {playerChar && (
                <>
                  <span className="text-slate-700">|</span>
                  <span className="flex items-center gap-1 text-slate-200">
                    <User className="w-3 h-3 text-cyan-400" /> {playerChar.name}
                  </span>
                </>
              )}
            </div>

            <div className="flex items-center gap-1">
              <button
                disabled={streaming}
                onClick={() => handleAdvanceTime({ hours: 1 })}
                className="px-2 py-1 bg-slate-800 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed rounded text-[11px] transition-colors"
                title="Advance Imperial Clock by 1 hour"
              >
                +1h
              </button>
              <button
                disabled={streaming}
                onClick={() => handleAdvanceTime({ days: 1 })}
                className="px-2 py-1 bg-slate-800 hover:bg-slate-700 disabled:opacity-40 disabled:cursor-not-allowed rounded text-[11px] transition-colors"
                title="Advance Imperial Clock by 1 day"
              >
                +1d
              </button>
              <button
                disabled={streaming}
                onClick={() => handleAdvanceTime({ days: 28, pay_maintenance_periods: true })}
                className="px-2 py-1 bg-amber-900/70 hover:bg-amber-800 disabled:opacity-40 disabled:cursor-not-allowed text-amber-200 rounded text-[11px] transition-colors"
                title="Advance 28 days and pay Maintenance Period bills"
              >
                +28d
              </button>
            </div>
            {authUser && (
              <div className="hidden sm:flex items-center gap-1.5 pl-2 border-l border-slate-800 text-[11px] text-slate-400">
                <span>{authUser.display_name || authUser.username}</span>
                {authUser.logout_url && (
                  <a href={authUser.logout_url} className="text-cyan-400 hover:underline">
                    Sign out
                  </a>
                )}
              </div>
            )}
          </div>
        )}
      </header>

      {errorBanner && (
        <div className="bg-red-950/95 border-b border-red-800 px-4 py-2 text-xs text-red-200 flex items-center justify-between">
          <span>{errorBanner}</span>
          <button
            onClick={() => setErrorBanner(null)}
            className="text-red-300 hover:text-white font-semibold px-2 py-0.5 rounded bg-red-900/60"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Main Split Command Deck */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Left: Live Referee Narrative Console */}
        <section className="flex-1 flex flex-col min-w-0 border-r border-slate-800 bg-gradient-to-b from-slate-950 to-slate-950/95">
          {!activeId ? (
            <div className="flex-1 flex items-center justify-center p-6">
              <div className="max-w-md text-center space-y-4 bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-xl">
                <div className="text-lg font-bold text-cyan-400">Welcome to TravellerLLM</div>
                <p className="text-xs text-slate-400 leading-relaxed">
                  Run multiple independent Mongoose Traveller 2e campaigns — each with its own
                  Player Character, starship fleet, Hindsight regional memory bank, adventure
                  modules, and live starship encounters.
                </p>
                <div className="flex gap-2">
                  <input
                    className="flex-1 bg-slate-950 border border-slate-700 rounded-lg px-3 py-1.5 text-xs"
                    value={newCampName}
                    onChange={(e) => setNewCampName(e.target.value)}
                  />
                  <button
                    disabled={!newCampName.trim()}
                    onClick={handleCreateCampaign}
                    className="bg-cyan-600 hover:bg-cyan-500 disabled:opacity-40 disabled:cursor-not-allowed text-white font-semibold px-4 py-1.5 rounded-lg text-xs"
                  >
                    Create Game
                  </button>
                </div>
              </div>
            </div>
          ) : (
            <>
              <div className="flex-1 overflow-y-auto p-4 space-y-3.5">
                {messages.length === 0 && (
                  <div className="max-w-xl mx-auto text-center text-slate-400 text-xs py-10 space-y-4">
                    <div className="text-slate-200 font-semibold text-base">
                      Bridge Online · {snapshot?.campaign.current_system_name}{" "}
                      <span className="font-mono text-cyan-400">
                        [{snapshot?.campaign.current_system_uwp}]
                      </span>
                    </div>
                    <p className="text-slate-400 leading-relaxed">
                      {snapshot?.system_ship_count ?? 0} vessel(s) detected on long-range sensors.
                      Roll your Player Character in <strong>Crew</strong>, load an adventure module
                      in <strong>Modules</strong>, or pick an opening action below:
                    </p>
                    <div className="flex flex-wrap justify-center gap-2 pt-1">
                      {QUICK_PROMPTS.map((qp) => (
                        <button
                          key={qp}
                          disabled={streaming}
                          onClick={() => sendChatText(qp)}
                          className="bg-slate-900 hover:bg-slate-800 border border-slate-700 hover:border-cyan-600/70 text-slate-200 rounded-lg px-3 py-1.5 text-xs transition-colors"
                        >
                          {qp}
                        </button>
                      ))}
                    </div>
                  </div>
                )}

                {messages.map((m) => (
                  <div
                    key={m.id}
                    className={`rounded-xl p-3.5 shadow-sm ${
                      m.role === "user"
                        ? "bg-cyan-950/35 border border-cyan-800/60 ml-12 text-cyan-50"
                        : "bg-slate-900/90 border border-slate-800/90 mr-6 text-slate-200"
                    }`}
                  >
                    <div className="text-[10px] uppercase tracking-wider text-slate-500 mb-1.5 flex justify-between items-center">
                      <span className={m.role === "user" ? "text-cyan-400 font-semibold" : "text-amber-400 font-semibold"}>
                        {m.role === "user" ? playerChar?.name || "Captain" : "Referee"}
                      </span>
                      {m.role === "assistant" && m.input_tokens > 0 && (
                        <span className="font-mono text-slate-500">
                          in {m.input_tokens.toLocaleString()} (cache{" "}
                          {m.cache_read_tokens.toLocaleString()}) · out{" "}
                          {m.output_tokens.toLocaleString()}
                        </span>
                      )}
                    </div>
                    {m.role === "user" ? (
                      <div className="text-sm whitespace-pre-wrap leading-relaxed">{m.content}</div>
                    ) : (
                      <MarkdownMessage content={m.content} />
                    )}
                  </div>
                ))}

                {/* Collapsible Live Tool Calls */}
                {liveTools.length > 0 && (
                  <div className="space-y-1.5 mr-6">
                    {liveTools.map((t) => {
                      const open = Boolean(expandedToolIds[t.id]);
                      return (
                        <div
                          key={t.id}
                          className="bg-slate-900/80 border border-slate-800 rounded-lg text-[11px] font-mono overflow-hidden"
                        >
                          <button
                            onClick={() =>
                              setExpandedToolIds((p) => ({ ...p, [t.id]: !p[t.id] }))
                            }
                            className="w-full px-2.5 py-1.5 flex items-center justify-between hover:bg-slate-800/60 text-left"
                          >
                            <span className="flex items-center gap-1.5 text-amber-300">
                              {open ? (
                                <ChevronDown className="w-3.5 h-3.5" />
                              ) : (
                                <ChevronRight className="w-3.5 h-3.5" />
                              )}
                              <Wrench className="w-3 h-3" />
                              <span>{t.name}</span>
                            </span>
                            <span className={t.is_error ? "text-red-400" : "text-emerald-400"}>
                              {t.result ? (t.is_error ? "error" : "done") : "running…"}
                            </span>
                          </button>
                          {open && t.result && (
                            <pre className="px-2.5 py-2 border-t border-slate-800 text-[10px] text-slate-300 overflow-x-auto max-h-40">
                              {t.result}
                            </pre>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}

                {streamText && (
                  <div className="rounded-xl p-3.5 bg-slate-900 border border-cyan-700/60 mr-6 text-slate-100 shadow-sm">
                    <div className="text-[10px] uppercase tracking-wider text-amber-400 font-semibold mb-1.5">
                      Referee
                    </div>
                    <MarkdownMessage content={streamText} />
                  </div>
                )}

                {statusChip && (
                  <div className="text-xs text-cyan-400 animate-pulse px-1">{statusChip}</div>
                )}
                <div ref={chatEndRef} />
              </div>

              {/* Input bar */}
              <div className="p-3 border-t border-slate-800 bg-slate-900/80">
                <div className="flex gap-2">
                  <textarea
                    rows={2}
                    disabled={streaming}
                    className="flex-1 bg-slate-950 border border-slate-700 focus:border-cyan-500 rounded-lg px-3 py-2 text-sm resize-none focus:outline-none disabled:opacity-50"
                    placeholder="Declare an action, hail an in-system ship, investigate module leads, or roll a task…"
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        handleSend();
                      }
                    }}
                  />
                  <button
                    disabled={streaming || !input.trim()}
                    onClick={handleSend}
                    className="bg-cyan-600 hover:bg-cyan-500 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded-lg px-4 flex items-center gap-1.5 font-semibold text-sm transition-colors"
                  >
                    <Send className="w-4 h-4" /> Send
                  </button>
                </div>
                {snapshot && (
                  <div className="flex justify-between text-[10px] text-slate-500 mt-1.5 font-mono">
                    <span>
                      Referee: {snapshot.campaign.referee_model} · Worker: {snapshot.campaign.worker_model}
                    </span>
                    <span>
                      Session Cost: ${snapshot.llm_costs.total_cost_usd.toFixed(4)} · Cache hits:{" "}
                      {snapshot.llm_costs.cache_read_tokens.toLocaleString()} tok
                    </span>
                  </div>
                )}
              </div>
            </>
          )}
        </section>

        {/* Right: Modular Traveller Command Deck */}
        {snapshot && activeId && (
          <aside className="w-[480px] xl:w-[540px] shrink-0 flex flex-col bg-slate-950/95 min-h-0">
            <nav className="grid grid-cols-7 border-b border-slate-800 bg-slate-900 text-[10px]">
              {(
                [
                  ["map", "Map", Compass, false],
                  ["characters", "Crew", Users, false],
                  ["ships", "Ships", Rocket, false],
                  ["trade", "Trade", ShoppingBag, false],
                  ["combat", "Combat", Swords, hasActiveCombat],
                  ["finances", "Credits", CreditCard, false],
                  ["modules", "Modules", BookOpen, false],
                ] as const
              ).map(([id, label, Icon, alertDot]) => (
                <button
                  key={id}
                  onClick={() => setTab(id)}
                  className={`relative py-2.5 flex flex-col items-center gap-0.5 border-b-2 transition-colors ${
                    tab === id
                      ? "border-cyan-400 text-cyan-300 bg-slate-950/70 font-semibold"
                      : "border-transparent text-slate-400 hover:text-slate-200 hover:bg-slate-800/40"
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{label}</span>
                  {alertDot && (
                    <span className="absolute top-1.5 right-2.5 w-2 h-2 rounded-full bg-red-500 animate-pulse" />
                  )}
                </button>
              ))}
            </nav>

            <div className="flex-1 overflow-y-auto p-3">
              {tab === "map" && (
                <StarMapPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                  onSendChat={sendChatText}
                  onSwitchToCombat={() => setTab("combat")}
                />
              )}
              {tab === "characters" && (
                <CharactersPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  initialMode={charPanelMode}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
              {tab === "ships" && (
                <ShipsPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
              {tab === "trade" && (
                <TradeShopPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
              {tab === "combat" && (
                <CombatPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
              {tab === "finances" && (
                <FinancesCrewPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
              {tab === "modules" && (
                <ModulesPanel
                  campaignId={activeId}
                  snapshot={snapshot}
                  onRefresh={() => refreshActive(activeId)}
                  onError={setErrorBanner}
                />
              )}
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}

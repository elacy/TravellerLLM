import { useEffect, useState, useCallback, useRef } from "react";
import {
  BookOpen,
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
  Users,
} from "lucide-react";
import {
  api,
  AuthRequiredError,
  AuthUser,
  Campaign,
  CampaignSnapshot,
  formatImperialDate,
  MessageRecord,
  TurnEvent,
} from "./lib/api";
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
      // Jump straight into Lifepath Character Creation so the new game gets its own unique Player Character!
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
    if (!activeId) return;
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
        <div className="max-w-md w-full bg-slate-900 border border-slate-800 rounded-xl p-6 text-center space-y-4">
          <div className="text-xl font-bold text-cyan-400">TravellerLLM</div>
          <p className="text-sm text-slate-400">
            Sign in via Authentik to access your starship bridge.
          </p>
          <a
            href={authLoginUrl}
            className="inline-block bg-cyan-600 hover:bg-cyan-500 text-white font-semibold px-5 py-2 rounded-lg"
          >
            Sign In with Authentik
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-slate-950 text-slate-100 overflow-hidden">
      {/* Top Imperial Command Bar */}
      <header className="h-12 shrink-0 border-b border-slate-800 bg-slate-900/90 px-3 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <div className="flex items-center gap-1.5 font-bold tracking-tight text-cyan-400">
            <Sparkles className="w-4 h-4" />
            <span>TravellerLLM</span>
          </div>
          <select
            className="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs max-w-[190px]"
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
          {activeId && (
            <button
              onClick={handleDeleteActiveCampaign}
              title="Delete current game"
              className="p-1 text-slate-500 hover:text-red-400"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
          <div className="hidden md:flex items-center gap-1">
            <input
              className="bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs w-40"
              placeholder="New game name…"
              value={newCampName}
              onChange={(e) => setNewCampName(e.target.value)}
            />
            <button
              onClick={handleCreateCampaign}
              className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2 py-1 text-xs flex items-center gap-1"
            >
              <Plus className="w-3.5 h-3.5" /> New Game
            </button>
          </div>
        </div>

        {/* Imperial Calendar & Quick Time Controls */}
        {snapshot && (
          <div className="flex items-center gap-2 text-xs">
            <div className="hidden lg:flex items-center gap-1.5 bg-slate-950 border border-slate-800 rounded px-2 py-1 font-mono text-amber-300">
              <Clock className="w-3.5 h-3.5 text-amber-400" />
              <span>{formatImperialDate(snapshot.campaign.date)}</span>
              <span className="text-slate-500">|</span>
              <span className="text-cyan-300">{snapshot.campaign.current_system_name}</span>
              <span className="text-slate-500">|</span>
              <span className="text-emerald-300">{snapshot.system_ship_count ?? 0} ships</span>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={() => handleAdvanceTime({ hours: 1 })}
                className="px-1.5 py-0.5 bg-slate-800 hover:bg-slate-700 rounded text-[11px]"
              >
                +1h
              </button>
              <button
                onClick={() => handleAdvanceTime({ days: 1 })}
                className="px-1.5 py-0.5 bg-slate-800 hover:bg-slate-700 rounded text-[11px]"
              >
                +1d
              </button>
              <button
                onClick={() => handleAdvanceTime({ execute_jump: true })}
                className="px-1.5 py-0.5 bg-cyan-900 hover:bg-cyan-800 text-cyan-200 rounded text-[11px]"
              >
                Jump
              </button>
              <button
                onClick={() => handleAdvanceTime({ days: 28, pay_maintenance_periods: true })}
                className="px-1.5 py-0.5 bg-amber-900/70 hover:bg-amber-800 text-amber-200 rounded text-[11px]"
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
        <div className="bg-red-950 border-b border-red-800 px-4 py-1.5 text-xs text-red-200 flex items-center justify-between">
          <span>{errorBanner}</span>
          <button onClick={() => setErrorBanner(null)} className="text-red-400 hover:text-white">
            Dismiss
          </button>
        </div>
      )}

      {/* Main Split Command Deck */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Left: Live Referee Narrative Console */}
        <section className="flex-1 flex flex-col min-w-0 border-r border-slate-800">
          {!activeId ? (
            <div className="flex-1 flex items-center justify-center p-6">
              <div className="max-w-md text-center space-y-3 bg-slate-900 border border-slate-800 rounded-xl p-6">
                <div className="text-lg font-bold text-cyan-400">Welcome to TravellerLLM</div>
                <p className="text-xs text-slate-400">
                  Run multiple independent Mongoose Traveller 2e campaigns — each with its own
                  Player Character, starship, Hindsight memory bank, loaded adventure modules, and
                  live in-system starship encounters.
                </p>
                <div className="flex gap-2">
                  <input
                    className="flex-1 bg-slate-950 border border-slate-700 rounded px-3 py-1.5 text-xs"
                    value={newCampName}
                    onChange={(e) => setNewCampName(e.target.value)}
                  />
                  <button
                    onClick={handleCreateCampaign}
                    className="bg-cyan-600 hover:bg-cyan-500 text-white font-semibold px-4 py-1.5 rounded text-xs"
                  >
                    Create Game
                  </button>
                </div>
              </div>
            </div>
          ) : (
            <>
              <div className="flex-1 overflow-y-auto p-4 space-y-3">
                {messages.length === 0 && (
                  <div className="text-center text-slate-500 text-xs py-12 space-y-2">
                    <div className="text-slate-300 font-semibold text-sm">
                      Bridge online at {snapshot?.campaign.current_system_name} [
                      {snapshot?.campaign.current_system_uwp}] — {snapshot?.system_ship_count ?? 0}{" "}
                      ship(s) on sensors
                    </div>
                    <div>
                      Roll your unique Player Character in <strong>Crew → New Character</strong>,
                      load an adventure module into Hindsight in <strong>Modules</strong>, or hail a
                      vessel in <strong>Map</strong>!
                    </div>
                  </div>
                )}

                {messages.map((m) => (
                  <div
                    key={m.id}
                    className={`rounded-lg p-3 text-sm whitespace-pre-wrap leading-relaxed ${
                      m.role === "user"
                        ? "bg-cyan-950/40 border border-cyan-900/60 ml-10 text-cyan-50"
                        : "bg-slate-900/90 border border-slate-800 mr-6 text-slate-200"
                    }`}
                  >
                    <div className="text-[10px] uppercase tracking-wider text-slate-500 mb-1 flex justify-between">
                      <span>{m.role === "user" ? "Captain" : "Referee"}</span>
                      {m.role === "assistant" && m.input_tokens > 0 && (
                        <span className="font-mono">
                          in {m.input_tokens} (cache {m.cache_read_tokens}) · out {m.output_tokens}
                        </span>
                      )}
                    </div>
                    {m.content}
                  </div>
                ))}

                {liveTools.length > 0 && (
                  <div className="space-y-1.5 mr-6">
                    {liveTools.map((t) => (
                      <div
                        key={t.id}
                        className="bg-slate-900/70 border border-slate-800 rounded p-2 text-[11px] font-mono"
                      >
                        <div className="text-amber-300">⚙ {t.name}</div>
                        {t.result && (
                          <div className="text-slate-400 truncate mt-0.5">
                            {t.result.slice(0, 220)}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}

                {streamText && (
                  <div className="rounded-lg p-3 text-sm whitespace-pre-wrap leading-relaxed bg-slate-900 border border-cyan-800/60 mr-6 text-slate-100">
                    {streamText}
                  </div>
                )}

                {statusChip && <div className="text-xs text-cyan-400 animate-pulse">{statusChip}</div>}
                <div ref={chatEndRef} />
              </div>

              {/* Input bar */}
              <div className="p-3 border-t border-slate-800 bg-slate-900/70">
                <div className="flex gap-2">
                  <textarea
                    rows={2}
                    className="flex-1 bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm resize-none focus:outline-none focus:border-cyan-500"
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
                    className="bg-cyan-600 hover:bg-cyan-500 disabled:opacity-40 text-white rounded-lg px-4 flex items-center gap-1.5 font-semibold text-sm"
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
          <aside className="w-[480px] xl:w-[540px] shrink-0 flex flex-col bg-slate-950/90 min-h-0">
            <nav className="grid grid-cols-7 border-b border-slate-800 bg-slate-900 text-[10px]">
              {(
                [
                  ["map", "Map", Compass],
                  ["characters", "Crew", Users],
                  ["ships", "Ships", Rocket],
                  ["trade", "Trade", ShoppingBag],
                  ["combat", "Combat", Swords],
                  ["finances", "Credits", CreditCard],
                  ["modules", "Modules", BookOpen],
                ] as const
              ).map(([id, label, Icon]) => (
                <button
                  key={id}
                  onClick={() => setTab(id)}
                  className={`py-2.5 flex flex-col items-center gap-0.5 border-b-2 transition-colors ${
                    tab === id
                      ? "border-cyan-400 text-cyan-300 bg-slate-950/60"
                      : "border-transparent text-slate-400 hover:text-slate-200"
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{label}</span>
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

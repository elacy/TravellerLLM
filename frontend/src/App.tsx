import { useEffect, useState, useCallback, useRef } from "react";
import {
  Clock,
  Cpu,
  CreditCard,
  Dice5,
  Plus,
  Radar,
  Rocket,
  Send,
  ShieldCheck,
  ShieldAlert,
  Sparkles,
  Users,
} from "lucide-react";
import {
  api,
  Campaign,
  CampaignSnapshot,
  CharacterRow,
  formatCr,
  formatImperialDate,
  MessageRecord,
  skillUpgradeCost,
  TurnEvent,
} from "./lib/api";

type Tab = "characters" | "ships" | "sensors" | "finances";

interface LiveToolCard {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  is_error?: boolean;
}

export default function App() {
  const [campaigns, setCampaigns] = useState<Campaign[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<CampaignSnapshot | null>(null);
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [tab, setTab] = useState<Tab>("characters");

  // Chat state
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [statusChip, setStatusChip] = useState<string | null>(null);
  const [streamText, setStreamText] = useState("");
  const [liveTools, setLiveTools] = useState<LiveToolCard[]>([]);
  const [errorBanner, setErrorBanner] = useState<string | null>(null);

  // Campaign creation
  const [newCampName, setNewCampName] = useState("Spinward Marches Free Trader");

  // Quick character creation
  const [newCharName, setNewCharName] = useState("");
  const [newCharKind, setNewCharKind] = useState<"player" | "crew" | "npc">("player");

  // Sensor calculator state
  const [sensorGrade, setSensorGrade] = useState("military");
  const [observerTl, setObserverTl] = useState(12);
  const [targetTl, setTargetTl] = useState(12);
  const [targetThrust, setTargetThrust] = useState(2);
  const [targetStealth, setTargetStealth] = useState(-4);
  const [targetTransponder, setTargetTransponder] = useState(false);
  const [sensorResult, setSensorResult] = useState<Record<string, unknown> | null>(null);

  const chatEndRef = useRef<HTMLDivElement>(null);

  const refreshCampaigns = useCallback(async () => {
    try {
      const { campaigns: list } = await api.listCampaigns();
      setCampaigns(list);
      if (!activeId && list.length > 0) {
        setActiveId(list[0].id);
      }
    } catch (e) {
      setErrorBanner((e as Error).message);
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
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleSend() {
    if (!activeId || !input.trim() || streaming) return;
    const text = input.trim();
    setInput("");
    setStreaming(true);
    setStreamText("");
    setLiveTools([]);
    setStatusChip("Sending to Referee…");
    setErrorBanner(null);

    // Optimistic user message
    setMessages((prev) => [
      ...prev,
      {
        id: Date.now(),
        role: "user",
        content: text,
        superseded: false,
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        created_at: new Date().toISOString(),
      },
    ]);

    try {
      await api.streamChat(activeId, text, (ev: TurnEvent) => {
        if (ev.type === "status") setStatusChip(ev.message);
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

  async function handleAdvanceTime(opts: Record<string, unknown>) {
    if (!activeId) return;
    try {
      await api.advanceTime(activeId, opts);
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleCreateCharacter() {
    if (!activeId || !newCharName.trim()) return;
    try {
      await api.createCharacter(activeId, {
        kind: newCharKind,
        name: newCharName.trim(),
        species: "Human",
        age: 30,
        upp: { str_val: 7, dex_val: 8, end_val: 7, int_val: 9, edu_val: 8, soc_val: 7 },
        skills: { "Pilot (Spacecraft)": 1, Astrogation: 1, "Electronics (sensors)": 1 },
        augments: [
          {
            kind: "neural_jack",
            tl: 13,
            computer_rating: 2,
            storage_bandwidth: 8,
            loaded_expert: [{ skill: "Broker", rating: 2, active: true }],
          },
        ],
        salary_cr: newCharKind === "crew" ? 4000 : 0,
      });
      setNewCharName("");
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleSpendXp(charId: string, skill: string) {
    if (!activeId) return;
    try {
      await api.spendXp(activeId, charId, skill);
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleQuickEditStat(row: CharacterRow, key: keyof CharacterRow["character"]["upp_current"], delta: number) {
    if (!activeId) return;
    const updated = structuredClone(row.character);
    const cur = (updated.upp_current[key] as number) ?? 7;
    (updated.upp_current[key] as number) = Math.max(0, cur + delta);
    try {
      await api.updateCharacter(activeId, updated.id, updated, row.equipment);
      await refreshActive(activeId);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  async function handleSensorCheck() {
    if (!activeId) return;
    try {
      const res = await api.sensorCheck(activeId, {
        context: "space_initial_detection",
        operator_edu_dm: 1,
        operator_sensors_skill: 1,
        augment_or_software_dm: 0,
        observer_sensor_grade: sensorGrade,
        observer_tl: observerTl,
        observer_extended_array: false,
        observer_active_sensors: true,
        target_tl: targetTl,
        target_active_sensors: false,
        target_thrust_applied: targetThrust,
        target_power_plant_active: true,
        target_transponder_or_comms: targetTransponder,
        target_stealth_armor_dm: targetStealth,
      });
      setSensorResult(res.sensor_check);
    } catch (e) {
      setErrorBanner((e as Error).message);
    }
  }

  const companyAcc = snapshot?.accounts.find((a) => a.kind === "company");
  const personalAcc = snapshot?.accounts.find((a) => a.kind === "personal");

  return (
    <div className="flex h-screen flex-col bg-slate-950 text-slate-100">
      {/* Top Starship HUD */}
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 bg-slate-900/90 px-4 py-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 font-mono text-sm font-bold tracking-wider text-amber-400">
            <Rocket className="h-4 w-4" />
            TRAVELLER<span className="text-slate-400">LLM</span>
          </div>
          <select
            className="rounded border border-slate-700 bg-slate-950 px-2.5 py-1 text-xs text-slate-200"
            value={activeId ?? ""}
            onChange={(e) => setActiveId(e.target.value)}
          >
            {campaigns.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <div className="flex items-center gap-1">
            <input
              value={newCampName}
              onChange={(e) => setNewCampName(e.target.value)}
              placeholder="New campaign..."
              className="w-44 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs"
            />
            <button
              onClick={handleCreateCampaign}
              className="flex items-center gap-1 rounded bg-amber-500/20 px-2 py-1 text-xs font-medium text-amber-300 hover:bg-amber-500/30"
            >
              <Plus className="h-3.5 w-3.5" /> New
            </button>
          </div>
        </div>

        {snapshot && (
          <div className="flex flex-wrap items-center gap-4 font-mono text-xs">
            <span className="flex items-center gap-1.5 text-cyan-300">
              <Clock className="h-3.5 w-3.5" />
              {formatImperialDate(snapshot.campaign.date)}
            </span>
            <span className="rounded bg-slate-800 px-2 py-0.5 text-slate-200">
              {snapshot.campaign.current_system_name}{" "}
              <span className="text-amber-300">{snapshot.campaign.current_system_uwp}</span>
            </span>
            {companyAcc && (
              <span className="text-emerald-400">Co: {formatCr(companyAcc.balance_cr)}</span>
            )}
            {personalAcc && (
              <span className="text-sky-400">Pers: {formatCr(personalAcc.balance_cr)}</span>
            )}
            <div className="flex items-center gap-1">
              <button
                onClick={() => handleAdvanceTime({ hours: 1 })}
                className="rounded border border-slate-700 px-1.5 py-0.5 text-[11px] hover:bg-slate-800"
              >
                +1h
              </button>
              <button
                onClick={() => handleAdvanceTime({ days: 1 })}
                className="rounded border border-slate-700 px-1.5 py-0.5 text-[11px] hover:bg-slate-800"
              >
                +1d
              </button>
              <button
                onClick={() => handleAdvanceTime({ execute_jump: true })}
                className="rounded border border-amber-600/50 bg-amber-500/10 px-1.5 py-0.5 text-[11px] text-amber-300 hover:bg-amber-500/20"
              >
                Jump (1wk)
              </button>
            </div>
          </div>
        )}
      </header>

      {errorBanner && (
        <div className="flex items-center justify-between bg-rose-950/80 px-4 py-1.5 text-xs text-rose-200">
          <span>{errorBanner}</span>
          <button onClick={() => setErrorBanner(null)} className="underline">
            dismiss
          </button>
        </div>
      )}

      {/* Main Split Command Deck */}
      <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-12">
        {/* Left: Referee Narrative Stream */}
        <section className="flex min-h-0 flex-col border-r border-slate-800 lg:col-span-7">
          <div className="flex-1 space-y-4 overflow-y-auto p-4">
            {messages.length === 0 && !streaming && (
              <div className="rounded-lg border border-dashed border-slate-800 p-6 text-center text-sm text-slate-400">
                <Sparkles className="mx-auto mb-2 h-6 w-6 text-amber-400" />
                <p className="font-medium text-slate-200">Starship Command Deck Online</p>
                <p className="mt-1 text-xs">
                  Create a character on the right or send your opening action to the Referee.
                </p>
              </div>
            )}

            {messages.map((m) => (
              <div
                key={m.id}
                className={`rounded-lg px-4 py-3 text-sm leading-relaxed ${
                  m.role === "user"
                    ? "ml-8 border border-amber-500/20 bg-amber-500/10 text-amber-50"
                    : "mr-4 border border-slate-800 bg-slate-900/70 text-slate-100"
                }`}
              >
                <div className="mb-1 flex items-center justify-between text-[11px] font-mono uppercase tracking-wider text-slate-400">
                  <span>{m.role === "user" ? "Traveller" : `Referee · ${m.model ?? ""}`}</span>
                  {m.cache_read_tokens > 0 && (
                    <span className="text-emerald-400">
                      cache hit: {m.cache_read_tokens.toLocaleString()} tok
                    </span>
                  )}
                </div>
                <div className="whitespace-pre-wrap">{m.content}</div>
              </div>
            ))}

            {liveTools.map((t) => (
              <div
                key={t.id}
                className="rounded border border-cyan-500/30 bg-cyan-950/30 px-3 py-2 font-mono text-xs"
              >
                <div className="flex items-center gap-2 text-cyan-300">
                  <Dice5 className="h-3.5 w-3.5" />
                  <span className="font-semibold">{t.name}</span>
                  <span className="truncate text-slate-400">{JSON.stringify(t.input)}</span>
                </div>
                {t.result && (
                  <pre className="mt-1.5 max-h-36 overflow-auto rounded bg-slate-950/80 p-2 text-[11px] text-slate-300">
                    {t.result}
                  </pre>
                )}
              </div>
            ))}

            {streamText && (
              <div className="mr-4 rounded-lg border border-slate-700 bg-slate-900/90 px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap">
                {streamText}
              </div>
            )}

            {statusChip && (
              <div className="inline-flex items-center gap-2 rounded-full border border-amber-500/30 bg-amber-500/10 px-3 py-1 font-mono text-xs text-amber-300">
                <span className="h-2 w-2 animate-pulse rounded-full bg-amber-400" />
                {statusChip}
              </div>
            )}
            <div ref={chatEndRef} />
          </div>

          <div className="border-t border-slate-800 bg-slate-900/80 p-3">
            <div className="flex gap-2">
              <textarea
                rows={2}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                  }
                }}
                placeholder="State your action, ask the Referee to roll, hail the starport, or plot a jump…"
                className="flex-1 resize-none rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm focus:border-amber-500 focus:outline-none"
              />
              <button
                onClick={handleSend}
                disabled={streaming || !activeId}
                className="flex items-center gap-1.5 rounded-lg bg-amber-500 px-4 py-2 text-sm font-semibold text-slate-950 hover:bg-amber-400 disabled:opacity-50"
              >
                <Send className="h-4 w-4" /> Send
              </button>
            </div>
          </div>
        </section>

        {/* Right: Authoritative Game State Workspace */}
        <aside className="flex min-h-0 flex-col bg-slate-950 lg:col-span-5">
          <nav className="grid grid-cols-4 border-b border-slate-800 bg-slate-900/60 text-xs font-medium">
            {(
              [
                ["characters", "Roster", Users],
                ["ships", "Ships", Rocket],
                ["sensors", "Sensors", Radar],
                ["finances", "Ledger", CreditCard],
              ] as const
            ).map(([key, label, Icon]) => (
              <button
                key={key}
                onClick={() => setTab(key)}
                className={`flex items-center justify-center gap-1.5 py-2.5 transition ${
                  tab === key
                    ? "border-b-2 border-amber-400 bg-slate-900 text-amber-300"
                    : "text-slate-400 hover:text-slate-200"
                }`}
              >
                <Icon className="h-3.5 w-3.5" /> {label}
              </button>
            ))}
          </nav>

          <div className="flex-1 space-y-4 overflow-y-auto p-4">
            {/* CHARACTERS TAB */}
            {tab === "characters" && snapshot && (
              <>
                <div className="flex gap-2 rounded-lg border border-slate-800 bg-slate-900/60 p-2.5">
                  <select
                    value={newCharKind}
                    onChange={(e) => setNewCharKind(e.target.value as "player" | "crew" | "npc")}
                    className="rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs"
                  >
                    <option value="player">Player</option>
                    <option value="crew">Crew</option>
                    <option value="npc">NPC</option>
                  </select>
                  <input
                    value={newCharName}
                    onChange={(e) => setNewCharName(e.target.value)}
                    placeholder="Add character name…"
                    className="flex-1 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs"
                  />
                  <button
                    onClick={handleCreateCharacter}
                    className="rounded bg-emerald-500/20 px-2.5 py-1 text-xs font-medium text-emerald-300 hover:bg-emerald-500/30"
                  >
                    + Add
                  </button>
                </div>

                {snapshot.characters.map((row) => {
                  const c = row.character;
                  return (
                    <div
                      key={c.id}
                      className="rounded-lg border border-slate-800 bg-slate-900/70 p-3.5 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <span className="rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[10px] uppercase text-amber-300">
                            {c.kind}
                          </span>{" "}
                          <span className="text-sm font-semibold text-white">{c.name}</span>
                          <span className="ml-2 text-slate-400">
                            {c.species} · Age {c.age}
                          </span>
                        </div>
                        <span className="font-mono text-emerald-400">
                          GenXP: {c.general_xp}
                        </span>
                      </div>

                      {/* Editable UPP */}
                      <div className="mt-3 grid grid-cols-6 gap-1.5 text-center font-mono">
                        {(
                          [
                            ["STR", "str_val"],
                            ["DEX", "dex_val"],
                            ["END", "end_val"],
                            ["INT", "int_val"],
                            ["EDU", "edu_val"],
                            ["SOC", "soc_val"],
                          ] as const
                        ).map(([lbl, field]) => (
                          <div
                            key={lbl}
                            className="rounded border border-slate-800 bg-slate-950 p-1.5"
                          >
                            <div className="text-[10px] text-slate-400">{lbl}</div>
                            <div className="text-sm font-bold text-amber-300">
                              {c.upp_current[field]}/{c.upp_max[field]}
                            </div>
                            <div className="mt-0.5 flex justify-center gap-1">
                              <button
                                onClick={() => handleQuickEditStat(row, field, -1)}
                                className="rounded bg-slate-800 px-1 text-[10px] hover:bg-slate-700"
                              >
                                -
                              </button>
                              <button
                                onClick={() => handleQuickEditStat(row, field, 1)}
                                className="rounded bg-slate-800 px-1 text-[10px] hover:bg-slate-700"
                              >
                                +
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>

                      {/* Skills + Per-Skill XP */}
                      <div className="mt-3">
                        <div className="mb-1 font-mono text-[10px] uppercase tracking-wider text-slate-400">
                          Skills & Dedicated Activity XP
                        </div>
                        <div className="flex flex-wrap gap-1.5">
                          {Object.entries(c.skills).map(([sk, lvl]) => {
                            const xp = c.skill_xp[sk] ?? 0;
                            const cost = skillUpgradeCost(lvl + 1);
                            const canUpgrade = xp + c.general_xp >= cost;
                            return (
                              <div
                                key={sk}
                                className="flex items-center gap-1.5 rounded border border-slate-800 bg-slate-950 px-2 py-1"
                              >
                                <span className="font-medium text-slate-200">
                                  {sk}-{lvl}
                                </span>
                                <span className="font-mono text-[10px] text-cyan-400">
                                  ({xp}/{cost} XP)
                                </span>
                                {canUpgrade && (
                                  <button
                                    onClick={() => handleSpendXp(c.id, sk)}
                                    className="rounded bg-amber-500/20 px-1 text-[10px] text-amber-300 hover:bg-amber-500/40"
                                  >
                                    ↑
                                  </button>
                                )}
                              </div>
                            );
                          })}
                        </div>
                      </div>

                      {/* Augments / Neural Jack */}
                      {c.augments.length > 0 && (
                        <div className="mt-2.5 flex flex-wrap gap-1.5">
                          {c.augments.map((aug, i) => (
                            <span
                              key={i}
                              className="inline-flex items-center gap-1 rounded border border-purple-500/30 bg-purple-950/40 px-2 py-0.5 font-mono text-[11px] text-purple-200"
                            >
                              <Cpu className="h-3 w-3" />
                              {aug.kind === "neural_jack" || aug.kind === "wafer_jack"
                                ? `${aug.kind} TL${aug.tl} [${aug.loaded_expert.map((e) => `Expert/${e.rating} ${e.skill}`).join(", ") || "empty"}]`
                                : aug.kind}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}
              </>
            )}

            {/* SHIPS TAB */}
            {tab === "ships" &&
              snapshot &&
              snapshot.ships.map((s) => {
                const maintCr = Math.round((s.spec.purchase_price_cr * 0.001) / 12);
                return (
                  <div
                    key={s.id}
                    className="space-y-3 rounded-lg border border-slate-800 bg-slate-900/70 p-4 text-xs"
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-sm font-bold text-white">{s.name}</h3>
                        <p className="text-slate-400">
                          {s.class_name} · {s.spec.tonnage}t · TL{s.spec.tl} · J-{s.spec.jump_rating} / M-
                          {s.spec.thrust}
                        </p>
                      </div>
                      <span
                        className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 font-mono text-[11px] ${
                          s.validation.valid
                            ? "bg-emerald-500/20 text-emerald-300"
                            : "bg-rose-500/20 text-rose-300"
                        }`}
                      >
                        {s.validation.valid ? (
                          <ShieldCheck className="h-3.5 w-3.5" />
                        ) : (
                          <ShieldAlert className="h-3.5 w-3.5" />
                        )}
                        {s.validation.valid ? "HIGH GUARD VALID" : "INVALID SPEC"}
                      </span>
                    </div>

                    <div className="grid grid-cols-3 gap-2 font-mono">
                      <div className="rounded bg-slate-950 p-2">
                        <div className="text-[10px] text-slate-400">HULL</div>
                        <div className="text-sm font-bold text-amber-300">
                          {s.hull_current}/{s.spec.hull_points_max}
                        </div>
                      </div>
                      <div className="rounded bg-slate-950 p-2">
                        <div className="text-[10px] text-slate-400">FUEL (REF/UNREF)</div>
                        <div className="text-sm font-bold text-cyan-300">
                          {s.fuel_refined.toFixed(1)} / {s.fuel_unrefined.toFixed(1)}t
                        </div>
                      </div>
                      <div className="rounded bg-slate-950 p-2">
                        <div className="text-[10px] text-slate-400">POWER (PEAK)</div>
                        <div className="text-sm font-bold text-emerald-300">
                          {s.spec.power_plant_output} / {s.validation.power_total_peak_combat}
                        </div>
                      </div>
                    </div>

                    <div className="rounded border border-slate-800 bg-slate-950 p-2.5 font-mono text-[11px]">
                      <div className="text-slate-400">
                        Tonnage: {s.validation.used_tonnage.toFixed(1)} / {s.spec.tonnage}t (Free:{" "}
                        {s.validation.free_tonnage.toFixed(1)}t) · Hardpoints:{" "}
                        {s.validation.hardpoints_used}/{s.validation.hardpoints_max}
                      </div>
                      <div className="mt-1 text-slate-300">
                        4-Week Maintenance: <span className="text-amber-300">{formatCr(maintCr)}</span> ·
                        Skipped Periods: {s.skipped_maintenance_periods}
                      </div>
                      {s.mortgage && (
                        <div className="mt-1 text-rose-300">
                          Mortgage: {formatCr(s.mortgage.monthly_payment_cr)}/4wk ({s.mortgage.periods_paid}/
                          {s.mortgage.periods_total} paid · Arrears: {formatCr(s.mortgage.arrears_cr)})
                        </div>
                      )}
                    </div>

                    {s.validation.issues.length > 0 && (
                      <ul className="space-y-1 text-[11px]">
                        {s.validation.issues.map((iss, idx) => (
                          <li
                            key={idx}
                            className={iss.severity === "error" ? "text-rose-400" : "text-amber-300"}
                          >
                            [{iss.severity.toUpperCase()}] {iss.message}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                );
              })}

            {/* SENSORS TAB */}
            {tab === "sensors" && (
              <div className="space-y-3 rounded-lg border border-slate-800 bg-slate-900/70 p-4 text-xs">
                <h3 className="font-mono text-sm font-bold text-cyan-300">
                  High Guard 2024 · Sensor Detection Calculator
                </h3>
                <div className="grid grid-cols-2 gap-2.5">
                  <label className="space-y-1">
                    <span className="text-slate-400">Observer Sensor Grade</span>
                    <select
                      value={sensorGrade}
                      onChange={(e) => setSensorGrade(e.target.value)}
                      className="w-full rounded border border-slate-700 bg-slate-950 p-1.5"
                    >
                      <option value="basic">Basic (-4)</option>
                      <option value="civilian">Civilian (-2)</option>
                      <option value="military">Military (0)</option>
                      <option value="improved">Improved (+1)</option>
                      <option value="advanced">Advanced (+2)</option>
                    </select>
                  </label>
                  <label className="space-y-1">
                    <span className="text-slate-400">Observer / Target TL</span>
                    <div className="flex gap-1">
                      <input
                        type="number"
                        value={observerTl}
                        onChange={(e) => setObserverTl(Number(e.target.value))}
                        className="w-1/2 rounded border border-slate-700 bg-slate-950 p-1.5"
                      />
                      <input
                        type="number"
                        value={targetTl}
                        onChange={(e) => setTargetTl(Number(e.target.value))}
                        className="w-1/2 rounded border border-slate-700 bg-slate-950 p-1.5"
                      />
                    </div>
                  </label>
                  <label className="space-y-1">
                    <span className="text-slate-400">Target Thrust Applied</span>
                    <input
                      type="number"
                      value={targetThrust}
                      onChange={(e) => setTargetThrust(Number(e.target.value))}
                      className="w-full rounded border border-slate-700 bg-slate-950 p-1.5"
                    />
                  </label>
                  <label className="space-y-1">
                    <span className="text-slate-400">Target Stealth / Armor DM</span>
                    <input
                      type="number"
                      value={targetStealth}
                      onChange={(e) => setTargetStealth(Number(e.target.value))}
                      className="w-full rounded border border-slate-700 bg-slate-950 p-1.5"
                    />
                  </label>
                </div>
                <label className="flex items-center gap-2 text-slate-300">
                  <input
                    type="checkbox"
                    checked={targetTransponder}
                    onChange={(e) => setTargetTransponder(e.target.checked)}
                  />
                  Target transponder or radio comms active (+6 DM)
                </label>
                <button
                  onClick={handleSensorCheck}
                  className="w-full rounded bg-cyan-500/20 py-2 font-mono font-semibold text-cyan-200 hover:bg-cyan-500/30"
                >
                  Run Sensor Sweep (8+ Electronics/sensors)
                </button>
                {sensorResult && (
                  <pre className="max-h-64 overflow-auto rounded bg-slate-950 p-3 font-mono text-[11px] text-cyan-100">
                    {JSON.stringify(sensorResult, null, 2)}
                  </pre>
                )}
              </div>
            )}

            {/* FINANCES TAB */}
            {tab === "finances" && snapshot && (
              <div className="space-y-3 text-xs">
                <div className="grid grid-cols-2 gap-2.5">
                  {snapshot.accounts.map((acc) => (
                    <div
                      key={acc.id}
                      className="rounded-lg border border-slate-800 bg-slate-900/70 p-3"
                    >
                      <div className="font-mono text-[10px] uppercase text-slate-400">
                        {acc.kind} account
                      </div>
                      <div className="font-medium text-slate-200">{acc.name}</div>
                      <div className="mt-1 font-mono text-base font-bold text-emerald-400">
                        {formatCr(acc.balance_cr)}
                      </div>
                    </div>
                  ))}
                </div>

                <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3">
                  <div className="mb-2 font-mono text-[10px] uppercase tracking-wider text-slate-400">
                    Recent Ledger Transactions
                  </div>
                  {snapshot.transactions.length === 0 ? (
                    <p className="text-slate-500">No transactions posted yet.</p>
                  ) : (
                    <div className="space-y-1.5">
                      {snapshot.transactions.map((t) => (
                        <div
                          key={t.id}
                          className="flex items-center justify-between border-b border-slate-800/60 pb-1.5 font-mono text-[11px]"
                        >
                          <div>
                            <span className="text-slate-400">{t.imperial_date}</span>{" "}
                            <span className="text-slate-200">{t.description}</span>
                          </div>
                          <span
                            className={t.amount_cr >= 0 ? "text-emerald-400" : "text-rose-400"}
                          >
                            {t.amount_cr >= 0 ? "+" : ""}
                            {formatCr(t.amount_cr)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}

import { useState } from "react";
import { Cpu, Dice5, Edit3, Plus, Save, Sparkles, Trash2, UserPlus, X } from "lucide-react";
import {
  api,
  Augment,
  CampaignSnapshot,
  CharacterRow,
  formatCr,
  LifepathSession,
  skillUpgradeCost,
} from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

const CAREERS = [
  "agent",
  "army",
  "citizen",
  "drifter",
  "entertainer",
  "marine",
  "merchant",
  "navy",
  "noble",
  "rogue",
  "scholar",
  "scout",
];

export function CharactersPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [mode, setMode] = useState<"roster" | "lifepath">("roster");

  // Quick add
  const [newName, setNewName] = useState("");
  const [newKind, setNewKind] = useState<"player" | "crew" | "npc">("player");

  // Inline sheet editor state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [newSkillName, setNewSkillName] = useState("");
  const [newSkillLevel, setNewSkillLevel] = useState(1);
  const [newAugKind, setNewAugKind] = useState<"neural_jack" | "wafer_jack" | "neural_link" | "skill_augmentation">("neural_jack");
  const [newAugSkill, setNewAugSkill] = useState("Pilot (Spacecraft)");
  const [newAugRating, setNewAugRating] = useState(2);

  // Quick Task Roller
  const [rollResult, setRollResult] = useState<string | null>(null);

  // Lifepath Wizard state
  const [lpName, setLpName] = useState("Capt. Morgan Vance");
  const [lpSpecies, setLpSpecies] = useState("Human");
  const [lpSession, setLpSession] = useState<LifepathSession | null>(null);
  const [lpCareer, setLpCareer] = useState("merchant");
  const [lpSpecialist, setLpSpecialist] = useState(true);
  const [lpCashRolls, setLpCashRolls] = useState(2);
  const [lpCommitKind, setLpCommitKind] = useState<"player" | "crew" | "npc">("player");

  async function handleQuickCreate() {
    if (!newName.trim()) return;
    try {
      await api.createCharacter(campaignId, {
        kind: newKind,
        name: newName.trim(),
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
        salary_cr: newKind === "crew" ? 4000 : 0,
      });
      setNewName("");
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function saveRow(row: CharacterRow, updatedChar: CharacterRow["character"]) {
    try {
      await api.updateCharacter(campaignId, updatedChar.id, updatedChar, row.equipment);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleStatDelta(
    row: CharacterRow,
    key: keyof CharacterRow["character"]["upp_current"],
    delta: number,
    alsoMax = false
  ) {
    const ch = structuredClone(row.character);
    const cur = (ch.upp_current[key] as number) ?? 7;
    (ch.upp_current[key] as number) = Math.max(0, Math.min(18, cur + delta));
    if (alsoMax) {
      const mx = (ch.upp_max[key] as number) ?? 7;
      (ch.upp_max[key] as number) = Math.max(1, Math.min(18, mx + delta));
    }
    await saveRow(row, ch);
  }

  async function handleAddSkill(row: CharacterRow) {
    if (!newSkillName.trim()) return;
    const ch = structuredClone(row.character);
    ch.skills[newSkillName.trim()] = newSkillLevel;
    setNewSkillName("");
    await saveRow(row, ch);
  }

  async function handleRemoveSkill(row: CharacterRow, sk: string) {
    const ch = structuredClone(row.character);
    delete ch.skills[sk];
    await saveRow(row, ch);
  }

  async function handleAddAugment(row: CharacterRow) {
    const ch = structuredClone(row.character);
    let aug: Augment;
    if (newAugKind === "neural_jack") {
      aug = {
        kind: "neural_jack",
        tl: 13,
        computer_rating: 2,
        storage_bandwidth: 8,
        loaded_expert: [{ skill: newAugSkill, rating: newAugRating, active: true }],
      };
    } else if (newAugKind === "wafer_jack") {
      aug = {
        kind: "wafer_jack",
        tl: 12,
        computer_rating: 2,
        storage_bandwidth: 4,
        loaded_expert: [{ skill: newAugSkill, rating: newAugRating, active: true }],
      };
    } else if (newAugKind === "neural_link") {
      aug = { kind: "neural_link", tl: 13 };
    } else {
      aug = { kind: "skill_augmentation", skill: newAugSkill };
    }
    ch.augments.push(aug);
    await saveRow(row, ch);
  }

  async function handleRemoveAugment(row: CharacterRow, idx: number) {
    const ch = structuredClone(row.character);
    ch.augments.splice(idx, 1);
    await saveRow(row, ch);
  }

  async function handleSpendXp(charId: string, skill: string) {
    try {
      await api.spendXp(campaignId, charId, skill);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleQuickRoll(row: CharacterRow, skill: string) {
    try {
      const res = await api.rollTask(campaignId, {
        character_id: row.character.id,
        skill,
        characteristic: "INT",
        difficulty: "average",
        award_activity_xp: true,
      });
      const r = res.result as Record<string, unknown>;
      setRollResult(
        `${row.character.name} (${skill}): rolled ${r.total} vs ${r.target} (Effect ${r.effect}) — ${r.success ? "SUCCESS" : "FAILURE"} (+1 XP)`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  // Lifepath Handlers
  async function startLp() {
    try {
      const { session } = await api.chargenStart(lpName.trim() || "Traveller", lpSpecies.trim() || "Human");
      setLpSession(session);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function runTerm() {
    if (!lpSession) return;
    try {
      const { session } = await api.chargenTerm(lpSession, lpCareer, lpSpecialist);
      setLpSession(session);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function doMuster() {
    if (!lpSession) return;
    try {
      const { session } = await api.chargenMuster(lpSession, lpCashRolls);
      setLpSession(session);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function commitLp() {
    if (!lpSession) return;
    const personalAcc = snapshot.accounts.find((a) => a.kind === "personal");
    try {
      await api.chargenCommit(
        campaignId,
        lpSession,
        lpCommitKind,
        lpCommitKind === "crew" ? 4000 : 0,
        lpCommitKind === "player" ? "Captain" : "Crew",
        personalAcc?.id
      );
      setLpSession(null);
      setMode("roster");
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  return (
    <div className="space-y-3 text-xs">
      <div className="flex gap-1.5">
        <button
          onClick={() => setMode("roster")}
          className={`flex-1 py-1.5 rounded font-medium border ${
            mode === "roster"
              ? "bg-cyan-950 border-cyan-600 text-cyan-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          Character Sheets ({snapshot.characters.length})
        </button>
        <button
          onClick={() => setMode("lifepath")}
          className={`flex-1 py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            mode === "lifepath"
              ? "bg-amber-950 border-amber-600 text-amber-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <Sparkles className="w-3.5 h-3.5" /> Lifepath Chargen
        </button>
      </div>

      {rollResult && (
        <div className="p-2 rounded bg-cyan-950/70 border border-cyan-700 text-cyan-100 flex items-center justify-between">
          <span>{rollResult}</span>
          <button onClick={() => setRollResult(null)} className="text-cyan-400 hover:text-white">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {mode === "lifepath" ? (
        <div className="bg-slate-900 border border-slate-800 rounded p-3 space-y-3">
          <div className="font-semibold text-amber-300 text-sm">
            MgT2e Core 2022 Lifepath Character Generator
          </div>
          {!lpSession ? (
            <div className="space-y-2">
              <div className="grid grid-cols-2 gap-2">
                <input
                  className="bg-slate-950 border border-slate-700 rounded px-2 py-1.5"
                  placeholder="Character Name"
                  value={lpName}
                  onChange={(e) => setLpName(e.target.value)}
                />
                <input
                  className="bg-slate-950 border border-slate-700 rounded px-2 py-1.5"
                  placeholder="Species"
                  value={lpSpecies}
                  onChange={(e) => setLpSpecies(e.target.value)}
                />
              </div>
              <button
                onClick={startLp}
                className="w-full bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded py-1.5 flex items-center justify-center gap-1.5"
              >
                <Dice5 className="w-4 h-4" /> Roll 2D6 Characteristics & Begin Age 18
              </button>
            </div>
          ) : (
            <div className="space-y-2.5">
              <div className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded p-2">
                <div>
                  <div className="font-bold text-slate-100">
                    {lpSession.name} ({lpSession.species})
                  </div>
                  <div className="text-[11px] text-slate-400">
                    Age {lpSession.age} · {lpSession.terms_completed} term(s) · Rank {lpSession.current_rank} ·{" "}
                    {lpSession.benefit_rolls_earned} benefit roll(s)
                  </div>
                </div>
                <button
                  onClick={() => setLpSession(null)}
                  className="text-slate-400 hover:text-red-400 text-[11px]"
                >
                  Reset
                </button>
              </div>

              <div className="grid grid-cols-6 gap-1 text-center font-mono">
                {(
                  [
                    ["STR", lpSession.upp.str_val],
                    ["DEX", lpSession.upp.dex_val],
                    ["END", lpSession.upp.end_val],
                    ["INT", lpSession.upp.int_val],
                    ["EDU", lpSession.upp.edu_val],
                    ["SOC", lpSession.upp.soc_val],
                  ] as const
                ).map(([lbl, val]) => (
                  <div key={lbl} className="bg-slate-950 border border-slate-800 rounded py-1">
                    <div className="text-[9px] text-slate-400">{lbl}</div>
                    <div className="font-bold text-cyan-300">{val}</div>
                  </div>
                ))}
              </div>

              <div className="flex flex-wrap gap-1">
                {Object.entries(lpSession.skills).map(([sk, lvl]) => (
                  <span
                    key={sk}
                    className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-700 text-slate-200 text-[11px]"
                  >
                    {sk}-{lvl}
                  </span>
                ))}
              </div>

              {!lpSession.mustered_out ? (
                <div className="space-y-2 border-t border-slate-800 pt-2">
                  <div className="flex gap-1.5 items-center">
                    <select
                      className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 capitalize"
                      value={lpCareer}
                      onChange={(e) => setLpCareer(e.target.value)}
                    >
                      {CAREERS.map((c) => (
                        <option key={c} value={c}>
                          {c}
                        </option>
                      ))}
                    </select>
                    <label className="flex items-center gap-1 text-[11px] text-slate-300">
                      <input
                        type="checkbox"
                        checked={lpSpecialist}
                        onChange={(e) => setLpSpecialist(e.target.checked)}
                      />
                      Specialist
                    </label>
                    <button
                      onClick={runTerm}
                      className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-3 py-1 font-medium"
                    >
                      Run 4-Yr Term
                    </button>
                  </div>
                  {lpSession.terms_completed > 0 && (
                    <div className="flex gap-2 items-center">
                      <label className="text-[11px] text-slate-400">
                        Cash rolls (max 3):
                        <input
                          type="number"
                          min={0}
                          max={3}
                          value={lpCashRolls}
                          onChange={(e) => setLpCashRolls(Number(e.target.value))}
                          className="w-12 ml-1 bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
                        />
                      </label>
                      <button
                        onClick={doMuster}
                        className="flex-1 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded py-1"
                      >
                        Muster Out & Roll Benefits
                      </button>
                    </div>
                  )}
                </div>
              ) : (
                <div className="bg-emerald-950/40 border border-emerald-700 rounded p-2 space-y-2">
                  <div className="text-emerald-200 font-semibold">
                    Mustered Out! Cash: {formatCr(lpSession.cash_cr)} · Ship Shares: {lpSession.ship_shares} ·
                    Pension: {formatCr(lpSession.pension_cr)}/yr
                  </div>
                  <div className="flex gap-2">
                    <select
                      className="bg-slate-950 border border-slate-700 rounded px-2 py-1"
                      value={lpCommitKind}
                      onChange={(e) => setLpCommitKind(e.target.value as "player" | "crew" | "npc")}
                    >
                      <option value="player">Add as Player</option>
                      <option value="crew">Add as Hired Crew</option>
                      <option value="npc">Add as NPC</option>
                    </select>
                    <button
                      onClick={commitLp}
                      className="flex-1 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded py-1 flex items-center justify-center gap-1"
                    >
                      <Save className="w-3.5 h-3.5" /> Save to Campaign & Deposit Cash
                    </button>
                  </div>
                </div>
              )}

              {lpSession.log.length > 0 && (
                <div className="max-h-40 overflow-y-auto space-y-1 border-t border-slate-800 pt-2">
                  {lpSession.log.map((e) => (
                    <div key={e.term} className="bg-slate-950 rounded p-1.5 text-[11px] border border-slate-800">
                      <div className="font-semibold text-cyan-300">
                        Term {e.term} (Age {e.age}) — {e.career} {!e.qualified && "[Drafted/Drifter]"}{" "}
                        {e.advanced && `★ Rank ${e.rank}`}
                      </div>
                      <div className="text-slate-300">{e.event}</div>
                      {e.skills_gained.length > 0 && (
                        <div className="text-slate-400">Skills: {e.skills_gained.join(", ")}</div>
                      )}
                      {e.aging_note && <div className="text-amber-300">{e.aging_note}</div>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      ) : (
        <>
          {/* Quick Character Creator */}
          <div className="flex gap-1.5">
            <input
              className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
              placeholder="Quick add character name…"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
            <select
              className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-xs"
              value={newKind}
              onChange={(e) => setNewKind(e.target.value as "player" | "crew" | "npc")}
            >
              <option value="player">Player</option>
              <option value="crew">Crew</option>
              <option value="npc">NPC</option>
            </select>
            <button
              onClick={handleQuickCreate}
              className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2.5 py-1 flex items-center gap-1"
            >
              <UserPlus className="w-3.5 h-3.5" /> Add
            </button>
          </div>

          {snapshot.characters.map((row) => {
            const c = row.character;
            const isEditing = editingId === c.id;
            return (
              <div key={c.id} className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <span className="font-bold text-slate-100 text-sm">{c.name}</span>{" "}
                    <span className="text-[10px] uppercase px-1.5 py-0.5 rounded bg-slate-800 text-cyan-300">
                      {c.kind}
                    </span>
                    {c.assigned_station && (
                      <span className="ml-1 text-[10px] text-amber-300">[{c.assigned_station}]</span>
                    )}
                    <div className="text-[10px] text-slate-400">
                      {c.species} · Age {c.age} · Gen XP: {c.general_xp}
                      {c.salary_cr > 0 && ` · Salary: ${formatCr(c.salary_cr)}/4wk`}
                    </div>
                  </div>
                  <button
                    onClick={() => setEditingId(isEditing ? null : c.id)}
                    className={`px-2 py-1 rounded text-[11px] flex items-center gap-1 border ${
                      isEditing
                        ? "bg-amber-950 border-amber-600 text-amber-200"
                        : "bg-slate-800 border-slate-700 text-slate-300 hover:text-white"
                    }`}
                  >
                    <Edit3 className="w-3 h-3" /> {isEditing ? "Done" : "Edit"}
                  </button>
                </div>

                {/* UPP Row */}
                <div className="grid grid-cols-6 gap-1 text-center font-mono">
                  {(
                    [
                      ["STR", "str_val"],
                      ["DEX", "dex_val"],
                      ["END", "end_val"],
                      ["INT", "int_val"],
                      ["EDU", "edu_val"],
                      ["SOC", "soc_val"],
                    ] as const
                  ).map(([lbl, key]) => {
                    const cur = c.upp_current[key] ?? 7;
                    const max = c.upp_max[key] ?? 7;
                    return (
                      <div key={lbl} className="bg-slate-950 border border-slate-800 rounded p-1">
                        <div className="text-[9px] text-slate-400">{lbl}</div>
                        <div className={cur < max ? "text-red-400 font-bold" : "text-cyan-300 font-bold"}>
                          {cur}/{max}
                        </div>
                        <div className="flex justify-center gap-1 mt-0.5">
                          <button
                            onClick={() => handleStatDelta(row, key, -1, isEditing)}
                            className="px-1 bg-slate-800 hover:bg-slate-700 rounded text-[10px]"
                          >
                            -
                          </button>
                          <button
                            onClick={() => handleStatDelta(row, key, 1, isEditing)}
                            className="px-1 bg-slate-800 hover:bg-slate-700 rounded text-[10px]"
                          >
                            +
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>

                {/* Skills + Per-Skill Activity XP */}
                <div className="space-y-1">
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider">
                    Skills & Per-Skill Activity XP (1/1/2/4/8/16/32)
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {Object.entries(c.skills).map(([sk, lvl]) => {
                      const xp = c.skill_xp[sk] ?? 0;
                      const cost = skillUpgradeCost(lvl + 1);
                      const canUpgrade = xp + c.general_xp >= cost;
                      return (
                        <div
                          key={sk}
                          className="flex items-center gap-1 bg-slate-950 border border-slate-800 rounded px-1.5 py-0.5 text-[11px]"
                        >
                          <button
                            title={`Roll ${sk} (+1 Activity XP)`}
                            onClick={() => handleQuickRoll(row, sk)}
                            className="hover:text-cyan-300 flex items-center gap-0.5"
                          >
                            <Dice5 className="w-3 h-3 text-slate-500" />
                            <span>
                              {sk}-{lvl}
                            </span>
                          </button>
                          <span className="text-[10px] text-amber-300">
                            ({xp}/{cost}xp)
                          </span>
                          {canUpgrade && (
                            <button
                              onClick={() => handleSpendXp(c.id, sk)}
                              className="px-1 bg-emerald-700 hover:bg-emerald-600 text-white rounded text-[9px]"
                            >
                              ↑
                            </button>
                          )}
                          {isEditing && (
                            <button
                              onClick={() => handleRemoveSkill(row, sk)}
                              className="text-red-400 hover:text-red-300"
                            >
                              <X className="w-3 h-3" />
                            </button>
                          )}
                        </div>
                      );
                    })}
                  </div>

                  {isEditing && (
                    <div className="flex gap-1 pt-1">
                      <input
                        className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-0.5 text-[11px]"
                        placeholder="Add skill (e.g. Gun Combat (Slug))"
                        value={newSkillName}
                        onChange={(e) => setNewSkillName(e.target.value)}
                      />
                      <input
                        type="number"
                        min={0}
                        max={6}
                        className="w-12 bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center text-[11px]"
                        value={newSkillLevel}
                        onChange={(e) => setNewSkillLevel(Number(e.target.value))}
                      />
                      <button
                        onClick={() => handleAddSkill(row)}
                        className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2 py-0.5 text-[11px] flex items-center gap-0.5"
                      >
                        <Plus className="w-3 h-3" /> Skill
                      </button>
                    </div>
                  )}
                </div>

                {/* Neural Jack / Wafer Jack / Augments */}
                <div className="space-y-1">
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider flex items-center gap-1">
                    <Cpu className="w-3 h-3 text-purple-400" /> Augments & Neural/Wafer Jacks
                  </div>
                  {c.augments.length === 0 ? (
                    <div className="text-[11px] text-slate-500">No cybernetic augments installed.</div>
                  ) : (
                    <div className="space-y-1">
                      {c.augments.map((aug, i) => (
                        <div
                          key={i}
                          className="flex items-center justify-between bg-purple-950/30 border border-purple-800/60 rounded px-2 py-1 text-[11px]"
                        >
                          <div className="text-purple-200">
                            <span className="font-semibold capitalize">{aug.kind.replace(/_/g, " ")}</span>
                            {"loaded_expert" in aug && aug.loaded_expert.length > 0 && (
                              <span className="text-purple-300 ml-1">
                                [
                                {aug.loaded_expert
                                  .map((e) => `Expert ${e.skill}/${e.rating}${e.active ? "" : " (off)"}`)
                                  .join(", ")}
                                ]
                              </span>
                            )}
                            {"skill" in aug && <span className="ml-1 text-purple-300">({aug.skill} +1)</span>}
                          </div>
                          {isEditing && (
                            <button
                              onClick={() => handleRemoveAugment(row, i)}
                              className="text-red-400 hover:text-red-300"
                            >
                              <Trash2 className="w-3 h-3" />
                            </button>
                          )}
                        </div>
                      ))}
                    </div>
                  )}

                  {isEditing && (
                    <div className="flex flex-wrap gap-1 pt-1">
                      <select
                        className="bg-slate-950 border border-slate-700 rounded px-1.5 py-0.5 text-[11px]"
                        value={newAugKind}
                        onChange={(e) => setNewAugKind(e.target.value as typeof newAugKind)}
                      >
                        <option value="neural_jack">Neural Jack (TL13)</option>
                        <option value="wafer_jack">Wafer Jack (TL12)</option>
                        <option value="neural_link">Neural Link (TL13)</option>
                        <option value="skill_augmentation">Skill Augment (+1)</option>
                      </select>
                      <input
                        className="flex-1 min-w-[110px] bg-slate-950 border border-slate-700 rounded px-1.5 py-0.5 text-[11px]"
                        placeholder="Expert/Aug skill"
                        value={newAugSkill}
                        onChange={(e) => setNewAugSkill(e.target.value)}
                      />
                      <input
                        type="number"
                        min={1}
                        max={3}
                        className="w-10 bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center text-[11px]"
                        value={newAugRating}
                        onChange={(e) => setNewAugRating(Number(e.target.value))}
                      />
                      <button
                        onClick={() => handleAddAugment(row)}
                        className="bg-purple-700 hover:bg-purple-600 text-white rounded px-2 py-0.5 text-[11px]"
                      >
                        + Install
                      </button>
                    </div>
                  )}
                </div>

                {/* Equipment Inventory */}
                {row.equipment.length > 0 && (
                  <div className="border-t border-slate-800 pt-1.5">
                    <div className="text-[10px] text-slate-400 uppercase tracking-wider">Inventory</div>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {row.equipment.map((eq, idx) => (
                        <span
                          key={idx}
                          className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-[10px] text-slate-300"
                        >
                          {String(eq.name ?? "Item")} x{String(eq.quantity ?? 1)} (TL{String(eq.tl ?? "?")})
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

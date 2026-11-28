import { useEffect, useState } from "react";
import { ArrowUpCircle, Cpu, Dice5, Edit3, Plus, Save, Sparkles, Trash2, X } from "lucide-react";
import {
  api,
  CampaignSnapshot,
  CharacterRow,
  formatCr,
  skillUpgradeCost,
} from "../lib/api";
import { CharGenStudio } from "./CharGenStudio";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  initialMode?: "roster" | "lifepath";
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

const LEARNABLE_SKILLS = [
  "Admin",
  "Advocate",
  "Animals (Handling)",
  "Astrogation",
  "Athletics (Dexterity)",
  "Athletics (Endurance)",
  "Broker",
  "Carouse",
  "Deception",
  "Diplomat",
  "Drive (Wheel)",
  "Electronics (comms)",
  "Electronics (computers)",
  "Electronics (sensors)",
  "Engineer (J-Drive)",
  "Engineer (M-Drive)",
  "Engineer (Power)",
  "Flyer (Grav)",
  "Gambler",
  "Gun Combat (Energy)",
  "Gun Combat (Slug)",
  "Gunner (Turret)",
  "Investigate",
  "Leadership",
  "Mechanic",
  "Medic",
  "Melee (Blade)",
  "Melee (Unarmed)",
  "Navigation",
  "Persuade",
  "Pilot (Small Craft)",
  "Pilot (Spacecraft)",
  "Recon",
  "Science (Physics)",
  "Stealth",
  "Steward",
  "Streetwise",
  "Survival",
  "Tactics (Military)",
  "Tactics (Naval)",
  "Vacc Suit",
];

function charXpCost(stat: "STR" | "DEX" | "END" | "INT" | "EDU", newLevel: number): number {
  const lvl = Math.max(1, newLevel);
  return stat === "INT" || stat === "EDU" ? lvl * 2 : lvl;
}

export function CharactersPanel({
  campaignId,
  snapshot,
  initialMode = "roster",
  onRefresh,
  onError,
}: Props) {
  const existingPlayer = snapshot.characters.find((r) => r.character.kind === "player");
  const [mode, setMode] = useState<"roster" | "lifepath">(initialMode);

  useEffect(() => {
    setMode(initialMode);
  }, [initialMode, campaignId]);

  // Safe non-cheat metadata editor (Name, Station, Notes only — stats/skills require XP!)
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editStation, setEditStation] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [learnSkillMap, setLearnSkillMap] = useState<Record<string, string>>({});
  const [rollResult, setRollResult] = useState<string | null>(null);

  function startMetaEdit(row: CharacterRow) {
    setEditingId(row.character.id);
    setEditName(row.character.name);
    setEditStation(row.character.assigned_station ?? "");
    setEditNotes(row.character.notes ?? "");
  }

  async function saveMetaEdit(row: CharacterRow) {
    const ch = structuredClone(row.character);
    if (editName.trim()) ch.name = editName.trim();
    ch.assigned_station = editStation.trim() || null;
    ch.notes = editNotes;
    try {
      await api.updateCharacter(campaignId, ch.id, ch, row.equipment);
      setEditingId(null);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleDeleteChar(charId: string, name: string) {
    if (!confirm(`Remove ${name} from this campaign?`)) return;
    try {
      await api.deleteCharacter(campaignId, charId);
      if (editingId === charId) setEditingId(null);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleSpendSkillXp(charId: string, skill: string) {
    try {
      await api.spendXp(campaignId, charId, skill);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleSpendCharXp(
    charId: string,
    stat: "STR" | "DEX" | "END" | "INT" | "EDU"
  ) {
    try {
      await api.spendCharXp(campaignId, charId, stat);
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
        award_activity_xp: false,
      });
      const r = res.result as Record<string, unknown>;
      setRollResult(
        `${row.character.name} (${skill}): rolled ${r.total} vs ${r.target} (Effect ${r.effect}) — ${
          r.success ? "SUCCESS" : "FAILURE"
        }`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const sortedChars = [...snapshot.characters].sort((a, b) => {
    const rank = (k: string) => (k === "player" ? 0 : k === "crew" ? 1 : 2);
    return rank(a.character.kind) - rank(b.character.kind);
  });

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
          <Sparkles className="w-3.5 h-3.5" /> New Character (Lifepath)
        </button>
      </div>

      {!existingPlayer && mode === "roster" && (
        <div className="bg-amber-950/40 border border-amber-700/80 rounded p-2.5 flex items-center justify-between gap-2">
          <div>
            <div className="font-semibold text-amber-200">No Player Character in This Game Yet</div>
            <div className="text-[10px] text-amber-300/80">
              Each game has strictly 1 Player Character. Roll yours with MgT2e Lifepath chargen!
            </div>
          </div>
          <button
            onClick={() => setMode("lifepath")}
            className="bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded px-2.5 py-1.5 shrink-0 flex items-center gap-1"
          >
            <Dice5 className="w-3.5 h-3.5" /> Roll PC
          </button>
        </div>
      )}

      {rollResult && (
        <div className="p-2 rounded bg-cyan-950/70 border border-cyan-700 text-cyan-100 flex items-center justify-between">
          <span>{rollResult}</span>
          <button onClick={() => setRollResult(null)} className="text-cyan-400 hover:text-white">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {mode === "lifepath" ? (
        <CharGenStudio
          campaignId={campaignId}
          snapshot={snapshot}
          onComplete={async () => {
            setMode("roster");
            await onRefresh();
          }}
          onError={onError}
        />
      ) : (
        <div className="space-y-2.5">
          <div className="text-[10px] text-slate-400 bg-slate-900/70 border border-slate-800 rounded px-2.5 py-1.5">
            Progression is strictly XP-gated (Companion 2024): earn Activity XP from skill rolls &amp;
            General XP from milestones to upgrade skills or characteristics. Hire crew in{" "}
            <strong>Credits → Hiring Hall</strong> or buy cybernetics in <strong>Trade → Item Market</strong>.
          </div>

          {sortedChars.map((row) => {
            const c = row.character;
            const isPlayer = c.kind === "player";
            const isEditing = editingId === c.id;
            const unlearned = LEARNABLE_SKILLS.filter((sk) => !(sk in c.skills));
            const chosenNewSkill = learnSkillMap[c.id] || unlearned[0] || "";
            const canLearnNew = c.general_xp >= 1 && Boolean(chosenNewSkill);

            return (
              <div
                key={c.id}
                className={`rounded-lg p-2.5 space-y-2.5 border ${
                  isPlayer
                    ? "bg-cyan-950/25 border-cyan-700/80"
                    : "bg-slate-900 border-slate-800"
                }`}
              >
                <div className="flex items-start justify-between gap-2">
                  <div>
                    <span className="font-bold text-slate-100 text-sm">{c.name}</span>{" "}
                    <span
                      className={`text-[10px] uppercase px-1.5 py-0.5 rounded font-semibold ${
                        isPlayer
                          ? "bg-cyan-800 text-cyan-100"
                          : c.kind === "crew"
                          ? "bg-emerald-900/70 text-emerald-200"
                          : "bg-slate-800 text-slate-300"
                      }`}
                    >
                      {isPlayer ? "PLAYER PC" : c.kind}
                    </span>
                    {c.assigned_station && (
                      <span className="ml-1 text-[10px] text-amber-300">[{c.assigned_station}]</span>
                    )}
                    <div className="text-[10px] text-slate-400">
                      {c.species} · Age {c.age} ·{" "}
                      <span className="text-amber-300 font-semibold">Gen XP: {c.general_xp}</span>
                      {c.salary_cr > 0 && ` · Salary: ${formatCr(c.salary_cr)}/4wk`}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => (isEditing ? setEditingId(null) : startMetaEdit(row))}
                      title="Edit name, ship station, or RP notes (stats/skills require XP)"
                      className={`px-2 py-1 rounded text-[11px] flex items-center gap-1 border ${
                        isEditing
                          ? "bg-amber-950 border-amber-600 text-amber-200"
                          : "bg-slate-800 border-slate-700 text-slate-300 hover:text-white"
                      }`}
                    >
                      <Edit3 className="w-3 h-3" /> {isEditing ? "Cancel" : "Notes"}
                    </button>
                    {!isPlayer && (
                      <button
                        onClick={() => handleDeleteChar(c.id, c.name)}
                        title="Dismiss / remove Crew or NPC"
                        className="p-1 rounded bg-red-950/70 border border-red-800 text-red-300 hover:bg-red-900"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                  </div>
                </div>

                {/* Safe Metadata Editor (no stat/skill/augment cheats!) */}
                {isEditing && (
                  <div className="bg-slate-950 border border-amber-800/70 rounded p-2 space-y-1.5">
                    <div className="grid grid-cols-2 gap-1.5">
                      <input
                        className="bg-slate-900 border border-slate-700 rounded px-2 py-1 text-[11px]"
                        placeholder="Character Name"
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                      />
                      <input
                        className="bg-slate-900 border border-slate-700 rounded px-2 py-1 text-[11px]"
                        placeholder="Ship Station (e.g. Captain, Pilot)"
                        value={editStation}
                        onChange={(e) => setEditStation(e.target.value)}
                      />
                    </div>
                    <textarea
                      rows={2}
                      className="w-full bg-slate-900 border border-slate-700 rounded px-2 py-1 text-[11px]"
                      placeholder="Character notes, contacts, personal goals…"
                      value={editNotes}
                      onChange={(e) => setEditNotes(e.target.value)}
                    />
                    <button
                      onClick={() => saveMetaEdit(row)}
                      className="bg-emerald-700 hover:bg-emerald-600 text-white rounded px-3 py-1 text-[11px] font-medium flex items-center gap-1"
                    >
                      <Save className="w-3 h-3" /> Save Notes
                    </button>
                  </div>
                )}

                {/* UPP Characteristics Row (STRICTLY XP-GATED UPGRADES ONLY!) */}
                <div className="grid grid-cols-6 gap-1 text-center font-mono">
                  {(
                    [
                      ["STR", "str_val", true],
                      ["DEX", "dex_val", true],
                      ["END", "end_val", true],
                      ["INT", "int_val", true],
                      ["EDU", "edu_val", true],
                      ["SOC", "soc_val", false],
                    ] as const
                  ).map(([lbl, key, trainable]) => {
                    const cur = c.upp_current[key] ?? 7;
                    const max = c.upp_max[key] ?? 7;
                    const cost = trainable ? charXpCost(lbl, max + 1) : 0;
                    const dedicated = c.characteristic_xp[lbl] ?? 0;
                    const avail = dedicated + c.general_xp;
                    const canTrain = trainable && avail >= cost && max < 15;
                    return (
                      <div key={lbl} className="bg-slate-950 border border-slate-800 rounded p-1 space-y-0.5">
                        <div className="text-[9px] text-slate-400">{lbl}</div>
                        <div className={cur < max ? "text-red-400 font-bold" : "text-cyan-300 font-bold"}>
                          {cur}/{max}
                        </div>
                        {trainable ? (
                          <button
                            disabled={!canTrain}
                            title={
                              max >= 15
                                ? `${lbl} is already at maximum (15)`
                                : canTrain
                                ? `Spend ${cost} XP to permanently raise ${lbl} to ${max + 1} (have ${avail} XP)`
                                : `Need ${cost} XP to raise ${lbl} to ${max + 1} (have ${avail} XP)`
                            }
                            onClick={() => handleSpendCharXp(c.id, lbl)}
                            className="w-full py-0.5 rounded text-[9px] bg-emerald-800/80 hover:bg-emerald-700 disabled:bg-slate-900 disabled:text-slate-600 disabled:opacity-50 disabled:cursor-not-allowed text-emerald-100 flex items-center justify-center gap-0.5"
                          >
                            <ArrowUpCircle className="w-2.5 h-2.5" /> {cost}xp
                          </button>
                        ) : (
                          <div
                            title="SOC cannot be trained with XP (Companion 2024 p. 43)"
                            className="text-[8px] text-slate-600 py-0.5"
                          >
                            Fixed
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>

                {/* Skills + Per-Skill Activity XP Upgrades */}
                <div className="space-y-1.5">
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider">
                    Skills &amp; Per-Skill XP (Costs: 1 / 1 / 2 / 4 / 8 / 16 / 32 XP)
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {Object.entries(c.skills).map(([sk, lvl]) => {
                      const xp = c.skill_xp[sk] ?? 0;
                      const cost = skillUpgradeCost(lvl + 1);
                      const avail = xp + c.general_xp;
                      const canUpgrade = avail >= cost;
                      return (
                        <div
                          key={sk}
                          className="flex items-center gap-1 bg-slate-950 border border-slate-800 rounded px-1.5 py-0.5 text-[11px]"
                        >
                          <button
                            title={`Test roll ${sk} (earn XP through Referee play & combat)`}
                            onClick={() => handleQuickRoll(row, sk)}
                            className="hover:text-cyan-300 flex items-center gap-0.5"
                          >
                            <Dice5 className="w-3 h-3 text-slate-500" />
                            <span className={lvl >= 1 ? "text-slate-100 font-medium" : "text-slate-300"}>
                              {sk}-{lvl}
                            </span>
                          </button>
                          <span className="text-[10px] text-amber-300 font-mono">
                            ({xp}/{cost})
                          </span>
                          <button
                            disabled={!canUpgrade}
                            title={
                              canUpgrade
                                ? `Spend ${cost} XP to raise ${sk} to ${lvl + 1} (${xp} skill + ${c.general_xp} general XP)`
                                : `Need ${cost} XP to upgrade ${sk} to ${lvl + 1} (have ${avail} XP)`
                            }
                            onClick={() => handleSpendSkillXp(c.id, sk)}
                            className="px-1 bg-emerald-700 hover:bg-emerald-600 disabled:bg-slate-800 disabled:text-slate-600 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded text-[9px]"
                          >
                            ↑
                          </button>
                        </div>
                      );
                    })}
                  </div>

                  {/* Learn a brand-new Skill at Level 0 (strictly requires 1 General XP!) */}
                  {unlearned.length > 0 && (
                    <div className="flex gap-1 pt-1">
                      <select
                        className="flex-1 bg-slate-950 border border-slate-800 rounded px-2 py-1 text-[11px]"
                        value={chosenNewSkill}
                        onChange={(e) =>
                          setLearnSkillMap((prev) => ({ ...prev, [c.id]: e.target.value }))
                        }
                      >
                        {unlearned.map((sk) => (
                          <option key={sk} value={sk}>
                            Learn {sk}-0 (1 Gen XP)
                          </option>
                        ))}
                      </select>
                      <button
                        disabled={!canLearnNew}
                        title={
                          canLearnNew
                            ? `Spend 1 General XP to learn ${chosenNewSkill}-0`
                            : "Requires at least 1 General XP (earned from play/milestones)"
                        }
                        onClick={() => handleSpendSkillXp(c.id, chosenNewSkill)}
                        className="bg-emerald-700 hover:bg-emerald-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2.5 py-1 text-[10px] font-medium flex items-center gap-1 shrink-0"
                      >
                        <Plus className="w-3 h-3" /> Learn (1 XP)
                      </button>
                    </div>
                  )}
                </div>

                {/* Neural Jack / Wafer Jack / Augments (Read-Only — bought in Shop!) */}
                <div className="space-y-1">
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider flex items-center gap-1">
                    <Cpu className="w-3 h-3 text-purple-400" /> Augments &amp; Neural/Wafer Jacks
                  </div>
                  {c.augments.length === 0 ? (
                    <div className="text-[11px] text-slate-500">
                      No cybernetics installed (purchase augments in Trade → Item Market).
                    </div>
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
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {c.notes && (
                  <div className="text-[11px] text-slate-400 border-t border-slate-800/80 pt-1">
                    {c.notes}
                  </div>
                )}

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
        </div>
      )}
    </div>
  );
}

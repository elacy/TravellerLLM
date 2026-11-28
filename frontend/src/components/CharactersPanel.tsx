import { useEffect, useState } from "react";
import { Cpu, Dice5, Edit3, Plus, Save, Sparkles, Trash2, UserPlus, X } from "lucide-react";
import {
  api,
  Augment,
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

  // Quick add (defaults to Crew/NPC once Player Character exists — strictly 1 Player per game)
  const [newName, setNewName] = useState("");
  const [newKind, setNewKind] = useState<"player" | "crew" | "npc">(
    existingPlayer ? "crew" : "player"
  );

  useEffect(() => {
    if (existingPlayer && newKind === "player") {
      setNewKind("crew");
    }
  }, [existingPlayer, newKind]);

  // Inline sheet editor state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [newSkillName, setNewSkillName] = useState("");
  const [newSkillLevel, setNewSkillLevel] = useState(1);
  const [newAugKind, setNewAugKind] = useState<
    "neural_jack" | "wafer_jack" | "neural_link" | "skill_augmentation"
  >("neural_jack");
  const [newAugSkill, setNewAugSkill] = useState("Pilot (Spacecraft)");
  const [newAugRating, setNewAugRating] = useState(2);

  // Quick Task Roller
  const [rollResult, setRollResult] = useState<string | null>(null);

  async function handleQuickCreate() {
    if (!newName.trim()) return;
    try {
      await api.createCharacter(campaignId, {
        kind: existingPlayer && newKind === "player" ? "crew" : newKind,
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
        `${row.character.name} (${skill}): rolled ${r.total} vs ${r.target} (Effect ${r.effect}) — ${
          r.success ? "SUCCESS" : "FAILURE"
        } (+1 XP)`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  // Sort strictly so the single Player Character is always rendered first
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
        <>
          {/* Quick Character Creator (strictly enforces 1 Player Character) */}
          <div className="flex gap-1.5">
            <input
              className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
              placeholder={existingPlayer ? "Quick add Crew or NPC…" : "Quick add Player / Crew / NPC…"}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
            <select
              className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-xs"
              value={newKind}
              onChange={(e) => setNewKind(e.target.value as "player" | "crew" | "npc")}
            >
              {!existingPlayer && <option value="player">Player (1 max)</option>}
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

          {sortedChars.map((row) => {
            const c = row.character;
            const isPlayer = c.kind === "player";
            const isEditing = editingId === c.id;
            return (
              <div
                key={c.id}
                className={`rounded p-2.5 space-y-2 border ${
                  isPlayer
                    ? "bg-cyan-950/25 border-cyan-700/80"
                    : "bg-slate-900 border-slate-800"
                }`}
              >
                <div className="flex items-center justify-between">
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
                      {c.species} · Age {c.age} · Gen XP: {c.general_xp}
                      {c.salary_cr > 0 && ` · Salary: ${formatCr(c.salary_cr)}/4wk`}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
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
                    {isEditing && (
                      <button
                        onClick={() => handleDeleteChar(c.id, c.name)}
                        title="Delete character"
                        className="p-1 rounded bg-red-950/70 border border-red-800 text-red-300 hover:bg-red-900"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                  </div>
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

                {c.notes && (
                  <div className="text-[11px] text-slate-400 border-t border-slate-800/80 pt-1">
                    {c.notes}
                  </div>
                )}

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

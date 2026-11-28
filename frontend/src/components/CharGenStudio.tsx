import { useEffect, useState } from "react";
import {
  CheckCircle2,
  Dice5,
  GraduationCap,
  RotateCcw,
  Save,
  Shield,
  Sparkles,
  Trash2,
} from "lucide-react";
import {
  api,
  Campaign,
  CampaignSnapshot,
  CareerOption,
  ChargenOptions,
  formatCr,
  LifepathSession,
  SpeciesOption,
} from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  fullscreen?: boolean;
  campaigns?: Campaign[];
  onSelectCampaign?: (id: string) => void;
  onDeleteCampaign?: () => void;
  onComplete: () => Promise<void>;
  onError: (msg: string) => void;
}

const FALLBACK_SPECIES: SpeciesOption[] = [
  {
    id: "Human (Imperial)",
    name: "Human (Imperial)",
    str_mod: 0,
    dex_mod: 0,
    end_mod: 0,
    int_mod: 0,
    edu_mod: 0,
    soc_mod: 0,
    psi_capable: false,
    ignores_aging: false,
    traits: "Versatile Third Imperium Humaniti; balanced characteristics.",
  },
];

const FALLBACK_CAREERS: CareerOption[] = [
  {
    career: "merchant",
    name: "Merchant",
    assignments: ["Merchant Marine", "Free Trader", "Broker"],
    qual_stat: "Int",
    qual_target: 4,
    surv_stat: "Int",
    surv_target: 5,
    adv_stat: "Edu",
    adv_target: 6,
    service_skills: [],
    specialist_skills: [],
    personal_dev: [],
    advanced_edu_skills: [],
    has_ship_benefit: true,
  },
];

function fmtMod(label: string, val: number): string | null {
  if (val === 0) return null;
  return `${label} ${val > 0 ? `+${val}` : val}`;
}

export function CharGenStudio({
  campaignId,
  snapshot,
  fullscreen = false,
  campaigns = [],
  onSelectCampaign,
  onDeleteCampaign,
  onComplete,
  onError,
}: Props) {
  const existingPlayer = snapshot.characters.find((r) => r.character.kind === "player");
  const [options, setOptions] = useState<ChargenOptions | null>(null);
  const [name, setName] = useState("Capt. Morgan Vance");
  const [species, setSpecies] = useState("Human (Imperial)");
  const [session, setSession] = useState<LifepathSession | null>(null);
  const [selectedBg, setSelectedBg] = useState<string[]>([]);
  const [career, setCareer] = useState("merchant");
  const [assignment, setAssignment] = useState("Free Trader");
  const [skillTable, setSkillTable] = useState<"service" | "specialist" | "personal" | "advanced_edu">("specialist");
  const [cashRolls, setCashRolls] = useState(2);
  const [commitKind, setCommitKind] = useState<"player" | "crew" | "npc">(
    fullscreen || !existingPlayer ? "player" : "crew"
  );
  const [replaceExisting, setReplaceExisting] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api
      .chargenOptions()
      .then((opts) => {
        setOptions(opts);
        if (opts.species.length > 0 && !opts.species.some((s) => s.id === species)) {
          setSpecies(opts.species[0].id);
        }
      })
      .catch((e) => onError((e as Error).message));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const speciesList = options?.species ?? FALLBACK_SPECIES;
  const careerList = options?.careers ?? FALLBACK_CAREERS;
  const bgSkillList = options?.background_skills ?? [];

  const activeSpecies = speciesList.find((s) => s.id === species || s.name === species) ?? speciesList[0];
  const activeCareer = careerList.find((c) => c.career === career) ?? careerList[0];

  useEffect(() => {
    if (activeCareer && activeCareer.assignments.length > 0) {
      if (!activeCareer.assignments.includes(assignment)) {
        setAssignment(activeCareer.assignments[0]);
      }
    }
  }, [activeCareer, assignment]);

  const bgSlots = session?.background_skill_slots ?? 3;
  const canPickBg = Boolean(session && session.terms_completed === 0 && !session.pre_career_done);
  const canDoPreCareer = Boolean(session && session.terms_completed === 0 && !session.pre_career_done);
  const canAdvancedEdu = Boolean(session && session.upp.edu_val >= 8);

  useEffect(() => {
    if (skillTable === "advanced_edu" && !canAdvancedEdu) {
      setSkillTable("specialist");
    }
  }, [skillTable, canAdvancedEdu]);

  async function handleStart() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const { session: s } = await api.chargenStart(name.trim(), species, []);
      setSession(s);
      setSelectedBg(Object.keys(s.skills));
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleToggleBg(sk: string) {
    if (!session || !canPickBg) return;
    const exists = selectedBg.includes(sk);
    let next: string[];
    if (exists) {
      next = selectedBg.filter((x) => x !== sk);
    } else {
      if (selectedBg.length >= bgSlots) return;
      next = [...selectedBg, sk];
    }
    setSelectedBg(next);
    try {
      const { session: updated } = await api.chargenBackground(session, next);
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handlePreCareer(choice: "none" | "university" | "military_academy") {
    if (!session || !canDoPreCareer) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenPreCareer(session, choice);
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleRunTerm() {
    if (!session || session.mustered_out) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenTerm(
        session,
        career,
        skillTable === "specialist",
        assignment,
        skillTable
      );
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleMuster() {
    if (!session || session.mustered_out || session.terms_completed < 1) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenMuster(session, cashRolls);
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleCommit() {
    if (!session) return;
    setBusy(true);
    const personalAcc = snapshot.accounts.find((a) => a.kind === "personal");
    const kindToSave = fullscreen ? "player" : commitKind;
    try {
      await api.chargenCommit(
        campaignId,
        session,
        kindToSave,
        kindToSave === "crew" ? 4000 : 0,
        kindToSave === "player" ? "Captain" : "Crew",
        personalAcc?.id,
        kindToSave === "player" && Boolean(existingPlayer) ? replaceExisting : false
      );
      setSession(null);
      setReplaceExisting(false);
      await onComplete();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  const spMods = activeSpecies
    ? [
        fmtMod("STR", activeSpecies.str_mod),
        fmtMod("DEX", activeSpecies.dex_mod),
        fmtMod("END", activeSpecies.end_mod),
        fmtMod("INT", activeSpecies.int_mod),
        fmtMod("EDU", activeSpecies.edu_mod),
        fmtMod("SOC", activeSpecies.soc_mod),
      ].filter(Boolean)
    : [];

  const maxCashRolls = Math.min(3, Math.max(1, session?.benefit_rolls_earned ?? 1));

  const body = (
    <div className={fullscreen ? "max-w-4xl mx-auto w-full space-y-5" : "space-y-3 text-xs"}>
      {/* Intro / Title */}
      <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-1.5 shadow-lg">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 font-bold text-amber-300 text-base">
            <Sparkles className="w-5 h-5" />
            <span>
              {fullscreen
                ? `Create Your Traveller — ${snapshot.campaign.name}`
                : "MgT2e Core 2022 Lifepath Character Generator"}
            </span>
          </div>
          {session && (
            <button
              onClick={() => setSession(null)}
              className="text-xs text-slate-400 hover:text-red-400 flex items-center gap-1 px-2 py-1 rounded bg-slate-950 border border-slate-800"
            >
              <RotateCcw className="w-3.5 h-3.5" /> Start Over
            </button>
          )}
        </div>
        <p className="text-xs text-slate-400">
          {fullscreen
            ? "Every new Traveller campaign begins with creating your Player Character through the Mongoose Traveller 2e Lifepath system. Roll characteristics, choose background skills, optionally attend University or Military Academy, serve career terms, and muster out to launch onto the Starship Bridge."
            : "Roll a complete Mongoose Traveller 2e character with authentic species modifiers, background skills, pre-career education, career assignments, aging, and mustering-out benefits."}
        </p>
      </div>

      {/* STEP 1: Identity & Species Dropdown */}
      {!session ? (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-5 space-y-4 shadow-lg">
          <div className="text-sm font-semibold text-cyan-300">
            Step 1 · Traveller Identity &amp; Species
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label className="block space-y-1">
              <span className="text-xs text-slate-300 font-medium">Character Name</span>
              <input
                className="w-full bg-slate-950 border border-slate-700 focus:border-cyan-500 rounded-lg px-3 py-2 text-sm text-slate-100 focus:outline-none"
                placeholder="e.g. Capt. Morgan Vance"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="block space-y-1">
              <span className="text-xs text-slate-300 font-medium">Species (Charted Space)</span>
              <select
                className="w-full bg-slate-950 border border-slate-700 focus:border-cyan-500 rounded-lg px-3 py-2 text-sm text-slate-100 focus:outline-none"
                value={species}
                onChange={(e) => setSpecies(e.target.value)}
              >
                {speciesList.map((sp) => (
                  <option key={sp.id} value={sp.id}>
                    {sp.name}
                  </option>
                ))}
              </select>
            </label>
          </div>

          {activeSpecies && (
            <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-1 text-xs">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-semibold text-cyan-300">{activeSpecies.name}</span>
                {spMods.length > 0 ? (
                  <span className="font-mono text-[11px] px-2 py-0.5 rounded bg-amber-950/70 border border-amber-800/70 text-amber-200">
                    {spMods.join(" · ")}
                  </span>
                ) : (
                  <span className="font-mono text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-300">
                    Balanced (+0 all)
                  </span>
                )}
                {activeSpecies.psi_capable && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-950 border border-purple-700 text-purple-200">
                    PSI Potential
                  </span>
                )}
                {activeSpecies.ignores_aging && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950 border border-cyan-700 text-cyan-200">
                    Ageless
                  </span>
                )}
              </div>
              <div className="text-slate-400">{activeSpecies.traits}</div>
            </div>
          )}

          <button
            disabled={busy || !name.trim()}
            onClick={handleStart}
            className="w-full bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-bold rounded-lg py-2.5 text-sm flex items-center justify-center gap-2 transition-colors"
          >
            <Dice5 className="w-4 h-4" /> Roll 2D6 Characteristics &amp; Begin at Age 18
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Live Character Sheet Header */}
          <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-base font-bold text-slate-100">
                  {session.name}{" "}
                  <span className="text-cyan-400 font-normal text-sm">({session.species})</span>
                </div>
                <div className="text-xs text-slate-400">
                  Age {session.age} · {session.terms_completed} term(s) · Rank {session.current_rank} ·{" "}
                  {session.benefit_rolls_earned} benefit roll(s) · Ship Shares: {session.ship_shares}
                  {session.qualification_bonus_dm ? ` · +${session.qualification_bonus_dm} Qual DM` : ""}
                </div>
                {session.species_traits && (
                  <div className="text-[11px] text-slate-500 mt-0.5">{session.species_traits}</div>
                )}
              </div>
            </div>

            {/* UPP Grid */}
            <div className={`grid gap-2 text-center font-mono ${session.upp.psi_val != null ? "grid-cols-7" : "grid-cols-6"}`}>
              {(
                [
                  ["STR", session.upp.str_val],
                  ["DEX", session.upp.dex_val],
                  ["END", session.upp.end_val],
                  ["INT", session.upp.int_val],
                  ["EDU", session.upp.edu_val],
                  ["SOC", session.upp.soc_val],
                  ...(session.upp.psi_val != null ? ([["PSI", session.upp.psi_val]] as const) : []),
                ] as const
              ).map(([lbl, val]) => {
                const dm = val === 0 ? -3 : Math.floor(val / 3) - 2;
                return (
                  <div key={lbl} className="bg-slate-950 border border-slate-800 rounded-lg py-2">
                    <div className="text-[10px] text-slate-400">{lbl}</div>
                    <div className="text-lg font-bold text-cyan-300">{val}</div>
                    <div className="text-[10px] text-amber-300">{dm >= 0 ? `+${dm}` : dm}</div>
                  </div>
                );
              })}
            </div>

            {/* Skills Chips */}
            <div className="space-y-1">
              <div className="text-[11px] font-semibold text-slate-400 uppercase tracking-wide">
                Skills ({Object.keys(session.skills).length})
              </div>
              <div className="flex flex-wrap gap-1.5">
                {Object.entries(session.skills).map(([sk, lvl]) => (
                  <span
                    key={sk}
                    className="px-2 py-0.5 rounded bg-slate-950 border border-slate-700 text-slate-200 text-xs"
                  >
                    {sk}-{lvl}
                  </span>
                ))}
              </div>
            </div>
          </div>

          {/* STEP 2: Background Skills Picker (editable before Term 1 / Pre-Career) */}
          {canPickBg && bgSkillList.length > 0 && (
            <div className="bg-slate-900 border border-cyan-900/60 rounded-xl p-4 space-y-2.5">
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-cyan-300">
                  Step 2 · Choose Background Skills (Level 0)
                </span>
                <span className="font-mono text-xs text-amber-300">
                  {selectedBg.length} / {bgSlots} selected (EDU DM + 3)
                </span>
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-1.5">
                {bgSkillList.map((sk) => {
                  const checked = selectedBg.includes(sk);
                  const disabled = !checked && selectedBg.length >= bgSlots;
                  return (
                    <button
                      key={sk}
                      type="button"
                      disabled={disabled}
                      onClick={() => handleToggleBg(sk)}
                      className={`text-left px-2 py-1.5 rounded border text-xs transition-colors disabled:opacity-35 disabled:cursor-not-allowed ${
                        checked
                          ? "bg-cyan-950 border-cyan-500 text-cyan-100 font-medium"
                          : "bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-600"
                      }`}
                    >
                      {checked ? "✓ " : "+ "}
                      {sk}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* STEP 3: Optional Pre-Career Education (University / Military Academy) */}
          {canDoPreCareer && (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-2.5">
              <div className="text-sm font-semibold text-amber-300 flex items-center gap-1.5">
                <GraduationCap className="w-4 h-4" /> Step 3 · Pre-Career Education (Optional, 4 Years)
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <button
                  disabled={busy}
                  onClick={() => handlePreCareer("university")}
                  className="bg-slate-950 hover:bg-slate-800 border border-slate-700 hover:border-cyan-600 rounded-lg p-2.5 text-left space-y-0.5 disabled:opacity-35"
                >
                  <div className="font-semibold text-cyan-300 text-xs">Apply to University</div>
                  <div className="text-[11px] text-slate-400">
                    Entry EDU 7+ · EDU +1..3, 2 skills, +1/+2 Career Qual DM
                  </div>
                </button>
                <button
                  disabled={busy}
                  onClick={() => handlePreCareer("military_academy")}
                  className="bg-slate-950 hover:bg-slate-800 border border-slate-700 hover:border-amber-600 rounded-lg p-2.5 text-left space-y-0.5 disabled:opacity-35"
                >
                  <div className="font-semibold text-amber-300 text-xs">Military Academy</div>
                  <div className="text-[11px] text-slate-400">
                    Entry END 8+ · Basic combat skills, Honours gives SOC +1 &amp; Leadership
                  </div>
                </button>
                <button
                  disabled={busy}
                  onClick={() => handlePreCareer("none")}
                  className="bg-slate-950 hover:bg-slate-800 border border-slate-700 rounded-lg p-2.5 text-left space-y-0.5 disabled:opacity-35"
                >
                  <div className="font-semibold text-slate-200 text-xs">Skip Straight to Career</div>
                  <div className="text-[11px] text-slate-400">
                    Start Term 1 immediately at Age 18
                  </div>
                </button>
              </div>
            </div>
          )}

          {session.pre_career_note && (
            <div className="bg-cyan-950/40 border border-cyan-800/70 rounded-lg px-3 py-2 text-xs text-cyan-200">
              <strong>Pre-Career:</strong> {session.pre_career_note}
            </div>
          )}

          {/* STEP 4: Career Terms & Mustering Out */}
          {!session.mustered_out ? (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-3">
              <div className="text-sm font-semibold text-cyan-300 flex items-center gap-1.5">
                <Shield className="w-4 h-4" /> Step 4 · Serve Career Terms (4 Years Each)
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <label className="block space-y-1">
                  <span className="text-[11px] text-slate-400">Career (All 12 Core 2022)</span>
                  <select
                    className="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs"
                    value={career}
                    onChange={(e) => setCareer(e.target.value)}
                  >
                    {careerList.map((c) => (
                      <option key={c.career} value={c.career}>
                        {c.name} (Qual {c.qual_stat} {c.qual_target}+)
                      </option>
                    ))}
                  </select>
                </label>

                <label className="block space-y-1">
                  <span className="text-[11px] text-slate-400">Assignment</span>
                  <select
                    className="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs"
                    value={assignment}
                    onChange={(e) => setAssignment(e.target.value)}
                  >
                    {(activeCareer?.assignments ?? ["Standard"]).map((a) => (
                      <option key={a} value={a}>
                        {a}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="block space-y-1">
                  <span className="text-[11px] text-slate-400">Skill Table Roll</span>
                  <select
                    className="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs"
                    value={skillTable}
                    onChange={(e) =>
                      setSkillTable(
                        e.target.value as "service" | "specialist" | "personal" | "advanced_edu"
                      )
                    }
                  >
                    <option value="service">Service Skills</option>
                    <option value="specialist">Specialist / Assignment</option>
                    <option value="personal">Personal Development (Stats)</option>
                    <option value="advanced_edu" disabled={!canAdvancedEdu}>
                      Advanced Education {canAdvancedEdu ? "(EDU 8+)" : "(Requires EDU 8+)"}
                    </option>
                  </select>
                </label>
              </div>

              {activeCareer && (
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-2.5 text-[11px] text-slate-400 flex flex-wrap gap-x-4 gap-y-1">
                  <span>
                    Qual: <strong className="text-slate-200">{activeCareer.qual_stat} {activeCareer.qual_target}+</strong>
                  </span>
                  <span>
                    Survival: <strong className="text-slate-200">{activeCareer.surv_stat} {activeCareer.surv_target}+</strong>
                  </span>
                  <span>
                    Advancement: <strong className="text-slate-200">{activeCareer.adv_stat} {activeCareer.adv_target}+</strong>
                  </span>
                  {activeCareer.has_ship_benefit && (
                    <span className="text-amber-300">★ Ship Benefit Eligible</span>
                  )}
                </div>
              )}

              <div className="flex flex-wrap gap-2">
                <button
                  disabled={busy}
                  onClick={handleRunTerm}
                  className="flex-1 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-35 text-white font-semibold rounded-lg py-2 px-4 text-xs transition-colors"
                >
                  Run 4-Year Term {session.terms_completed + 1} ({activeCareer?.name} — {assignment})
                </button>
              </div>

              {/* Muster Out controls once at least 1 term completed */}
              <div className="border-t border-slate-800 pt-3 flex flex-wrap items-center gap-3">
                <label className="text-xs text-slate-300 flex items-center gap-1.5">
                  Cash Rolls (max {maxCashRolls} of {Math.max(1, session.benefit_rolls_earned)} rolls):
                  <input
                    type="number"
                    min={0}
                    max={maxCashRolls}
                    disabled={session.terms_completed < 1}
                    value={Math.min(cashRolls, maxCashRolls)}
                    onChange={(e) =>
                      setCashRolls(Math.max(0, Math.min(maxCashRolls, Number(e.target.value) || 0)))
                    }
                    className="w-14 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-center font-mono disabled:opacity-35"
                  />
                </label>
                <button
                  disabled={busy || session.terms_completed < 1}
                  title={
                    session.terms_completed < 1
                      ? "Complete at least 1 career term before mustering out"
                      : "Roll mustering-out cash, benefits, ship shares & pension"
                  }
                  onClick={handleMuster}
                  className="flex-1 bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-bold rounded-lg py-2 px-4 text-xs transition-colors"
                >
                  Muster Out &amp; Roll Benefits
                </button>
              </div>
            </div>
          ) : (
            /* STEP 5: Finalize & Launch Campaign */
            <div className="bg-emerald-950/40 border border-emerald-700 rounded-xl p-4 space-y-3">
              <div className="text-emerald-200 font-bold text-sm flex items-center gap-2">
                <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                Mustered Out — Ready for Charted Space!
              </div>
              <div className="grid grid-cols-3 gap-2 text-center font-mono text-xs">
                <div className="bg-slate-950/80 border border-emerald-900 rounded-lg p-2">
                  <div className="text-[10px] text-slate-400">STARTING CASH</div>
                  <div className="text-emerald-300 font-bold">{formatCr(session.cash_cr)}</div>
                </div>
                <div className="bg-slate-950/80 border border-emerald-900 rounded-lg p-2">
                  <div className="text-[10px] text-slate-400">SHIP SHARES</div>
                  <div className="text-cyan-300 font-bold">{session.ship_shares}</div>
                </div>
                <div className="bg-slate-950/80 border border-emerald-900 rounded-lg p-2">
                  <div className="text-[10px] text-slate-400">PENSION / YR</div>
                  <div className="text-amber-300 font-bold">{formatCr(session.pension_cr)}</div>
                </div>
              </div>

              <div className="flex flex-wrap gap-2">
                {!fullscreen && (
                  <select
                    className="bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-2 text-xs"
                    value={commitKind}
                    onChange={(e) => setCommitKind(e.target.value as "player" | "crew" | "npc")}
                  >
                    <option value="player">
                      {existingPlayer ? "Player Character (Replace Existing)" : "Player Character"}
                    </option>
                    <option value="crew">Hired Crew</option>
                    <option value="npc">NPC</option>
                  </select>
                )}
                <button
                  disabled={
                    busy ||
                    (!fullscreen &&
                      commitKind === "player" &&
                      Boolean(existingPlayer) &&
                      !replaceExisting)
                  }
                  onClick={handleCommit}
                  className="flex-1 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-35 disabled:cursor-not-allowed text-white font-bold rounded-lg py-2.5 px-4 text-sm flex items-center justify-center gap-2 transition-colors"
                >
                  <Save className="w-4 h-4" />
                  {fullscreen
                    ? "Save Player Character & Enter Bridge"
                    : "Save to Game & Deposit Cash"}
                </button>
              </div>
              {!fullscreen && commitKind === "player" && existingPlayer && (
                <label className="flex items-center gap-1.5 text-xs text-amber-300">
                  <input
                    type="checkbox"
                    checked={replaceExisting}
                    onChange={(e) => setReplaceExisting(e.target.checked)}
                  />
                  Confirm replacing current Player Character ({existingPlayer.character.name}) — 1 Player max
                </label>
              )}
            </div>
          )}

          {/* Career Term History Log */}
          {session.log.length > 0 && (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-2">
              <div className="text-xs font-semibold text-slate-300 uppercase tracking-wide">
                Service Record ({session.log.length} Term{session.log.length === 1 ? "" : "s"})
              </div>
              <div className="max-h-52 overflow-y-auto space-y-1.5">
                {session.log.map((e) => (
                  <div
                    key={e.term}
                    className="bg-slate-950 rounded-lg p-2 text-xs border border-slate-800 space-y-0.5"
                  >
                    <div className="font-semibold text-cyan-300">
                      Term {e.term} (Age {e.age}) — {e.career}{" "}
                      {!e.qualified && (
                        <span className="text-amber-400">[Failed Qual → Drifter]</span>
                      )}{" "}
                      {!e.survived && <span className="text-red-400">[Mishap]</span>}{" "}
                      {e.advanced && <span className="text-emerald-300">★ Rank {e.rank}</span>}
                    </div>
                    <div className="text-slate-300">{e.event}</div>
                    {e.skills_gained.length > 0 && (
                      <div className="text-slate-400">Gained: {e.skills_gained.join(", ")}</div>
                    )}
                    {e.aging_note && <div className="text-amber-300">{e.aging_note}</div>}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );

  if (!fullscreen) return body;

  return (
    <div className="min-h-screen w-full bg-slate-950 text-slate-100 flex flex-col">
      <header className="h-12 shrink-0 border-b border-slate-800 bg-slate-900/95 px-4 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 font-bold text-cyan-400 text-sm">
          <Sparkles className="w-4 h-4" />
          <span>TravellerLLM</span>
          <span className="text-slate-600">·</span>
          <span className="text-slate-300 font-normal">New Game Character Creation</span>
        </div>
        <div className="flex items-center gap-2">
          {campaigns.length > 1 && onSelectCampaign && (
            <select
              className="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
              value={campaignId}
              onChange={(e) => onSelectCampaign(e.target.value)}
            >
              {campaigns.map((c) => (
                <option key={c.id} value={c.id}>
                  Switch game: {c.name}
                </option>
              ))}
            </select>
          )}
          {onDeleteCampaign && (
            <button
              onClick={onDeleteCampaign}
              title="Delete this unfinished game"
              className="p-1.5 rounded text-slate-400 hover:text-red-400 hover:bg-slate-800 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </header>
      <main className="flex-1 overflow-y-auto p-6">{body}</main>
    </div>
  );
}

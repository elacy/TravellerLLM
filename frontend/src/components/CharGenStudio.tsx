import { useEffect, useState } from "react";
import {
  ArrowLeftRight,
  CheckCircle2,
  Dice5,
  GraduationCap,
  Package,
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

const STAT_META = [
  { label: "STR", modKey: "str_mod" as const },
  { label: "DEX", modKey: "dex_mod" as const },
  { label: "END", modKey: "end_mod" as const },
  { label: "INT", modKey: "int_mod" as const },
  { label: "EDU", modKey: "edu_mod" as const },
  { label: "SOC", modKey: "soc_mod" as const },
];

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
    qual_stat: "INT",
    qual_target: 4,
    surv_stat: "INT",
    surv_target: 5,
    adv_stat: "EDU",
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

function baseSkillName(skill: string): string {
  return skill.split("(")[0].trim();
}

function statDm(score: number): number {
  if (score <= 0) return -3;
  if (score <= 2) return -2;
  if (score <= 5) return -1;
  if (score <= 8) return 0;
  if (score <= 11) return 1;
  if (score <= 14) return 2;
  return 3;
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

  // Phase 1: Swapping rolled 2D6 stats before locking
  const [swapIdx, setSwapIdx] = useState<number | null>(null);

  // Phase 2: Background Skills
  const [selectedBg, setSelectedBg] = useState<string[]>([]);

  // Phase 3: Pre-Career Education choices
  const [preMode, setPreMode] = useState<"university" | "military_academy" | "none">("university");
  const [uniSkill0, setUniSkill0] = useState("Admin");
  const [uniMinorGradSpec, setUniMinorGradSpec] = useState("");
  const [uniSkill1, setUniSkill1] = useState("Broker");
  const [acadBranch, setAcadBranch] = useState<"army" | "marine" | "navy">("navy");
  const [acadFocus, setAcadFocus] = useState<string[]>([]);

  // Phase 4: Adult Career choices
  const [career, setCareer] = useState("merchant");
  const [assignment, setAssignment] = useState("Free Trader");
  const [skillTable, setSkillTable] = useState<
    "service" | "specialist" | "personal" | "advanced_edu" | "officer"
  >("specialist");
  const [basicSkillPick, setBasicSkillPick] = useState("");
  const [attemptCommission, setAttemptCommission] = useState(false);
  const [fallbackOnFail, setFallbackOnFail] = useState<"draft" | "drifter">("drifter");
  const [cashRolls, setCashRolls] = useState(2);

  // Phase 6: Starting Skill Package
  const [pkgId, setPkgId] = useState("trader");
  const [pkgSkills, setPkgSkills] = useState<string[]>([]);

  // Commit
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
  const uniMinorList = options?.university_minor_skills ?? [
    "Admin",
    "Advocate",
    "Animals",
    "Art",
    "Astrogation",
    "Broker",
    "Electronics",
    "Engineer",
    "Language",
    "Medic",
    "Navigation",
    "Profession",
    "Science",
  ];
  const uniSkillList = options?.university_skills ?? [
    "Admin",
    "Broker",
    "Electronics (Computers)",
    "Medic",
    "Science (Physics)",
  ];
  const specMap = options?.skill_specializations ?? {};
  const uniMinorSpecs = specMap[uniSkill0] ?? [];
  const skillPackages = options?.skill_packages ?? [];

  const activeSpecies =
    speciesList.find((s) => s.id === species || s.name === species) ?? speciesList[0];
  const activeCareer = careerList.find((c) => c.career === career) ?? careerList[0];
  const activeBasicChoices = Array.from(
    new Set((activeCareer?.service_skills ?? []).map(baseSkillName))
  );
  const acadCareerObj = careerList.find((c) => c.career === acadBranch);
  const activePkg = skillPackages.find((p) => p.id === pkgId) ?? skillPackages[0];

  useEffect(() => {
    if (activeCareer && activeCareer.assignments.length > 0) {
      if (!activeCareer.assignments.includes(assignment)) {
        setAssignment(activeCareer.assignments[0]);
      }
      const baseChoices = Array.from(new Set(activeCareer.service_skills.map(baseSkillName)));
      if (baseChoices.length > 0 && !baseChoices.includes(basicSkillPick)) {
        setBasicSkillPick(baseChoices[0]);
      }
    }
  }, [activeCareer, assignment, basicSkillPick]);

  useEffect(() => {
    if (uniMinorSpecs.length > 0) {
      if (!uniMinorSpecs.includes(uniMinorGradSpec)) {
        setUniMinorGradSpec(uniMinorSpecs[0]);
      }
    } else if (uniMinorGradSpec !== "") {
      setUniMinorGradSpec("");
    }
  }, [uniSkill0, uniMinorSpecs, uniMinorGradSpec]);

  useEffect(() => {
    if (acadCareerObj && acadCareerObj.service_skills.length >= 3) {
      setAcadFocus(acadCareerObj.service_skills.slice(0, 3));
    }
  }, [acadBranch, acadCareerObj]);

  useEffect(() => {
    setPkgSkills([]);
  }, [pkgId]);

  const stage = session?.stage ?? "assign_stats";
  const bgSlots = session?.background_skill_slots ?? 3;
  const canAdvancedEdu = Boolean(session && session.upp.edu_val >= 8);
  const canOfficerTable = Boolean(session?.is_officer);
  const isContinuing = Boolean(
    session &&
      session.current_career &&
      !session.must_leave_career &&
      session.current_career === career
  );
  const isMilitaryCareer = ["army", "marine", "navy"].includes(career);
  const showBasicPick = Boolean(
    session && (session.careers_entered?.length ?? 0) > 0 && !isContinuing
  );

  useEffect(() => {
    if (skillTable === "advanced_edu" && !canAdvancedEdu) setSkillTable("specialist");
    if (skillTable === "officer" && !canOfficerTable) setSkillTable("specialist");
  }, [skillTable, canAdvancedEdu, canOfficerTable]);

  // If forced to stay in current career (nat 12 advancement) or continuing
  useEffect(() => {
    if (session?.must_stay_career && session.current_career) {
      setCareer(session.current_career);
    }
  }, [session?.must_stay_career, session?.current_career]);

  async function handleStart() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const { session: s } = await api.chargenStart(name.trim(), species, []);
      setSession(s);
      setSwapIdx(null);
      setSelectedBg([]);
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleSwapStat(idx: number) {
    if (!session || !session.rolled_pool) return;
    if (swapIdx === null) {
      setSwapIdx(idx);
      return;
    }
    if (swapIdx === idx) {
      setSwapIdx(null);
      return;
    }
    const nextPool = [...session.rolled_pool];
    const tmp = nextPool[swapIdx];
    nextPool[swapIdx] = nextPool[idx];
    nextPool[idx] = tmp;
    setSwapIdx(null);
    try {
      const { session: updated } = await api.chargenAssignStats(session, nextPool, false);
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleConfirmStats() {
    if (!session || !session.rolled_pool) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenAssignStats(session, session.rolled_pool, true);
      setSession(updated);
      setSelectedBg(Object.keys(updated.skills));
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  function handleToggleBg(sk: string) {
    if (selectedBg.includes(sk)) {
      setSelectedBg(selectedBg.filter((x) => x !== sk));
    } else if (selectedBg.length < bgSlots) {
      setSelectedBg([...selectedBg, sk]);
    }
  }

  async function handleConfirmBackground() {
    if (!session || selectedBg.length === 0) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenBackground(session, selectedBg);
      setSession(updated);
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  function toggleAcadFocus(sk: string) {
    if (acadFocus.includes(sk)) {
      setAcadFocus(acadFocus.filter((x) => x !== sk));
    } else if (acadFocus.length < 3) {
      setAcadFocus([...acadFocus, sk]);
    }
  }

  async function handlePreCareer() {
    if (!session) return;
    setBusy(true);
    try {
      const minorPayload =
        preMode === "university"
          ? uniMinorSpecs.length > 0 && uniMinorGradSpec
            ? `${uniSkill0} (${uniMinorGradSpec})`
            : uniSkill0
          : undefined;
      const { session: updated } = await api.chargenPreCareer(session, preMode, {
        uni_skill_0: minorPayload,
        uni_skill_1: preMode === "university" ? uniSkill1 : undefined,
        academy_branch: preMode === "military_academy" ? acadBranch : undefined,
        academy_focus_skills: preMode === "military_academy" ? acadFocus : undefined,
      });
      setSession(updated);
      if (updated.auto_qualify_career) {
        setCareer(updated.auto_qualify_career);
      }
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
        skillTable,
        {
          basic_training_skill: showBasicPick ? basicSkillPick : undefined,
          attempt_commission: isMilitaryCareer && !session.is_officer && attemptCommission,
          fallback_on_fail: !session.draft_used ? fallbackOnFail : "drifter",
        }
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

  function togglePkgSkill(sk: string) {
    if (pkgSkills.includes(sk)) {
      setPkgSkills(pkgSkills.filter((x) => x !== sk));
    } else if (pkgSkills.length < 2) {
      setPkgSkills([...pkgSkills, sk]);
    }
  }

  async function handleApplyPackage() {
    if (!session) return;
    setBusy(true);
    try {
      const { session: updated } = await api.chargenSkillPackage(session, pkgSkills);
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
      {/* Header Banner */}
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
          Authentic Mongoose Traveller 2e (2022 Update) lifepath: choose Species, roll &amp; assign
          2D6 characteristics, pick Background Skills, optionally attend University or Military
          Academy, serve Career Terms (gaining Level 1+ skills), muster out, and select a Starting
          Skill Package.
        </p>
      </div>

      {/* PHASE 0: Identity & Species Dropdown */}
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
            <Dice5 className="w-4 h-4" /> Roll Six 2D6 Characteristics &amp; Begin Age 18
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Live Character Sheet Summary */}
          <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-base font-bold text-slate-100">
                  {session.name}{" "}
                  <span className="text-cyan-400 font-normal text-sm">({session.species})</span>
                </div>
                <div className="text-xs text-slate-400">
                  Age {session.age} · {session.terms_completed} term(s) · Rank{" "}
                  {session.is_officer ? `Officer ${session.current_rank}` : session.current_rank} ·{" "}
                  {session.benefit_rolls_earned} benefit roll(s) · Ship Shares: {session.ship_shares}
                  {session.qualification_bonus_dm
                    ? ` · +${session.qualification_bonus_dm} Qual DM`
                    : ""}
                </div>
              </div>
            </div>

            {/* UPP Grid */}
            <div
              className={`grid gap-2 text-center font-mono ${
                session.upp.psi_val != null ? "grid-cols-7" : "grid-cols-6"
              }`}
            >
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
              ).map(([lbl, val], idx) => {
                const dm = statDm(val);
                const isSwappable = stage === "assign_stats" && idx < 6;
                const isPicked = swapIdx === idx;
                return (
                  <button
                    key={lbl}
                    type="button"
                    disabled={!isSwappable}
                    onClick={() => isSwappable && handleSwapStat(idx)}
                    title={
                      isSwappable
                        ? "Click two characteristics to swap your rolled 2D6 values"
                        : undefined
                    }
                    className={`rounded-lg py-2 border transition-colors ${
                      isPicked
                        ? "bg-amber-950/70 border-amber-400 ring-1 ring-amber-400"
                        : isSwappable
                        ? "bg-slate-950 border-cyan-700/70 hover:border-cyan-400 cursor-pointer"
                        : "bg-slate-950 border-slate-800 cursor-default"
                    }`}
                  >
                    <div className="text-[10px] text-slate-400">{lbl}</div>
                    <div className="text-lg font-bold text-cyan-300">{val}</div>
                    <div className="text-[10px] text-amber-300">{dm >= 0 ? `+${dm}` : dm}</div>
                    {isSwappable && session.rolled_pool && (
                      <div className="text-[9px] text-slate-500">
                        roll {session.rolled_pool[idx]}
                      </div>
                    )}
                  </button>
                );
              })}
            </div>

            {/* Skills List */}
            {Object.keys(session.skills).length > 0 && (
              <div className="space-y-1">
                <div className="text-[11px] font-semibold text-slate-400 uppercase tracking-wide">
                  Skills ({Object.keys(session.skills).length})
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {Object.entries(session.skills).map(([sk, lvl]) => (
                    <span
                      key={sk}
                      className={`px-2 py-0.5 rounded border text-xs ${
                        lvl >= 1
                          ? "bg-cyan-950/60 border-cyan-700 text-cyan-100 font-semibold"
                          : "bg-slate-950 border-slate-700 text-slate-300"
                      }`}
                    >
                      {sk}-{lvl}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* PHASE 1: Assign Rolled 2D6 Characteristics */}
          {stage === "assign_stats" && (
            <div className="bg-slate-900 border border-cyan-700/70 rounded-xl p-4 space-y-3">
              <div className="text-sm font-semibold text-cyan-300 flex items-center gap-1.5">
                <ArrowLeftRight className="w-4 h-4" /> Step 2 · Assign Your Six Rolled 2D6 Characteristics
              </div>
              <p className="text-xs text-slate-300">
                Per Core Rulebook 2022 p. 10, click any two characteristic boxes above (
                {STAT_META.map((m) => m.label).join(", ")}) to swap your rolled 2D6 scores into any
                order you like. Your species modifiers are applied automatically. Current EDU{" "}
                <strong>{session.upp.edu_val}</strong> grants{" "}
                <strong className="text-amber-300">{bgSlots} Background Skills</strong>.
              </p>
              <button
                disabled={busy}
                onClick={handleConfirmStats}
                className="w-full bg-cyan-600 hover:bg-cyan-500 text-white font-bold rounded-lg py-2 text-xs"
              >
                Lock Characteristics &amp; Choose Background Skills →
              </button>
            </div>
          )}

          {/* PHASE 2: Choose Background Skills (3 + EDU DM at Level 0) */}
          {stage === "background_skills" && (
            <div className="bg-slate-900 border border-cyan-700/70 rounded-xl p-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-semibold text-cyan-300">
                  Step 3 · Pick Your Homeworld &amp; Background Skills (Level 0 — No Specialisations)
                </span>
                <span className="font-mono text-xs text-amber-300">
                  {selectedBg.length} / {bgSlots} selected (3 + EDU DM)
                </span>
              </div>
              <p className="text-[11px] text-slate-400">
                Per Core 2022 p. 16 &amp; p. 62, Level 0 skills never carry a specialisation (e.g.{" "}
                <span className="text-slate-200">Electronics-0</span> covers all Electronics tasks
                at +0 until you specialise at Level 1+).
              </p>
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
              <button
                disabled={busy || selectedBg.length === 0}
                title={
                  selectedBg.length === 0
                    ? "Pick at least 1 background skill"
                    : "Confirm background skills and proceed to Pre-Career Education"
                }
                onClick={handleConfirmBackground}
                className="w-full bg-cyan-600 hover:bg-cyan-500 disabled:opacity-35 text-white font-bold rounded-lg py-2 text-xs"
              >
                Confirm Background Skills ({selectedBg.length}/{bgSlots}) &amp; Continue to Pre-Career Education →
              </button>
            </div>
          )}

          {/* PHASE 3: Pre-Career Education (University / Military Academy / Skip) */}
          {stage === "pre_career" && (
            <div className="bg-slate-900 border border-amber-700/70 rounded-xl p-4 space-y-3">
              <div className="text-sm font-semibold text-amber-300 flex items-center gap-1.5">
                <GraduationCap className="w-4 h-4" /> Step 4 · Pre-Career Education (Core 2022, Optional 4 Years)
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <button
                  type="button"
                  onClick={() => setPreMode("university")}
                  className={`rounded-lg p-2.5 text-left border transition-colors ${
                    preMode === "university"
                      ? "bg-cyan-950/60 border-cyan-500"
                      : "bg-slate-950 border-slate-800 hover:border-slate-600"
                  }`}
                >
                  <div className="font-semibold text-cyan-300 text-xs">University</div>
                  <div className="text-[11px] text-slate-400">
                    Entry EDU 7+ (+1 if SOC 9+) · Choose 2 study skills (Lvl 0 &amp; Lvl 1; both +1 on grad!) + EDU boost
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => setPreMode("military_academy")}
                  className={`rounded-lg p-2.5 text-left border transition-colors ${
                    preMode === "military_academy"
                      ? "bg-amber-950/60 border-amber-500"
                      : "bg-slate-950 border-slate-800 hover:border-slate-600"
                  }`}
                >
                  <div className="font-semibold text-amber-300 text-xs">Military Academy</div>
                  <div className="text-[11px] text-slate-400">
                    Army / Marine / Navy · All branch Basic Training + 3 Level-1 skills + auto-entry &amp; Honours Commission
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => setPreMode("none")}
                  className={`rounded-lg p-2.5 text-left border transition-colors ${
                    preMode === "none"
                      ? "bg-slate-800 border-slate-500"
                      : "bg-slate-950 border-slate-800 hover:border-slate-600"
                  }`}
                >
                  <div className="font-semibold text-slate-200 text-xs">Skip to Adult Career</div>
                  <div className="text-[11px] text-slate-400">
                    Begin Term 1 directly in a career at Age 18
                  </div>
                </button>
              </div>

              {preMode === "university" && (
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2.5">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <label className="block space-y-1">
                      <span className="text-[11px] text-slate-300">
                        Major Subject (Starts at Level 1 → Level 2 on Graduation)
                      </span>
                      <select
                        className="w-full bg-slate-900 border border-slate-700 rounded px-2 py-1.5 text-xs"
                        value={uniSkill1}
                        onChange={(e) => {
                          const next = e.target.value;
                          setUniSkill1(next);
                          if (baseSkillName(next) === uniSkill0) {
                            const alt = uniMinorList.find((s) => s !== baseSkillName(next));
                            if (alt) setUniSkill0(alt);
                          }
                        }}
                      >
                        {uniSkillList.map((sk) => (
                          <option key={sk} value={sk}>
                            {sk}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="block space-y-1">
                      <span className="text-[11px] text-slate-300">
                        Minor Subject (Starts at Level 0, Unspecialised → Level 1 on Grad)
                      </span>
                      <select
                        className="w-full bg-slate-900 border border-slate-700 rounded px-2 py-1.5 text-xs"
                        value={uniSkill0}
                        onChange={(e) => setUniSkill0(e.target.value)}
                      >
                        {uniMinorList
                          .filter((sk) => sk !== baseSkillName(uniSkill1))
                          .map((sk) => (
                            <option key={sk} value={sk}>
                              {sk}
                            </option>
                          ))}
                      </select>
                    </label>
                  </div>
                  {uniMinorSpecs.length > 0 && (
                    <label className="block space-y-1">
                      <span className="text-[11px] text-slate-400">
                        Minor Graduation Specialisation (only applied if you graduate to Level 1;
                        stays unspecialised <strong className="text-slate-200">{uniSkill0}-0</strong>{" "}
                        otherwise):
                      </span>
                      <select
                        className="w-full sm:w-64 bg-slate-900 border border-slate-700 rounded px-2 py-1 text-xs"
                        value={uniMinorGradSpec}
                        onChange={(e) => setUniMinorGradSpec(e.target.value)}
                      >
                        {uniMinorSpecs.map((sp) => (
                          <option key={sp} value={sp}>
                            {uniSkill0} ({sp})
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                </div>
              )}

              {preMode === "military_academy" && (
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2.5">
                  <div className="flex flex-wrap gap-2">
                    {(
                      [
                        ["army", "Army Academy (END 8+)"],
                        ["marine", "Marine Academy (END 9+)"],
                        ["navy", "Naval Academy (INT 9+)"],
                      ] as const
                    ).map(([b, lbl]) => (
                      <button
                        key={b}
                        type="button"
                        onClick={() => setAcadBranch(b)}
                        className={`px-3 py-1 rounded border text-xs ${
                          acadBranch === b
                            ? "bg-amber-900/60 border-amber-500 text-amber-100 font-semibold"
                            : "bg-slate-900 border-slate-700 text-slate-300"
                        }`}
                      >
                        {lbl}
                      </button>
                    ))}
                  </div>
                  {acadCareerObj && (
                    <div className="space-y-1">
                      <div className="text-[11px] text-slate-400">
                        Pick 3 Branch Service Skills to promote to Level 1 on graduation ({acadFocus.length}/3):
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {acadCareerObj.service_skills.map((sk) => {
                          const picked = acadFocus.includes(sk);
                          const dis = !picked && acadFocus.length >= 3;
                          return (
                            <button
                              key={sk}
                              type="button"
                              disabled={dis}
                              onClick={() => toggleAcadFocus(sk)}
                              className={`px-2 py-1 rounded border text-xs disabled:opacity-35 ${
                                picked
                                  ? "bg-amber-950 border-amber-500 text-amber-200"
                                  : "bg-slate-900 border-slate-700 text-slate-300"
                              }`}
                            >
                              {picked ? "✓ " : "+ "}
                              {sk}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              )}

              <button
                disabled={busy}
                onClick={handlePreCareer}
                className="w-full bg-amber-600 hover:bg-amber-500 disabled:opacity-35 text-slate-950 font-bold rounded-lg py-2 text-xs"
              >
                {preMode === "university"
                  ? "Apply to University (Roll Entry, Event & Graduation)"
                  : preMode === "military_academy"
                  ? `Apply to ${acadBranch.toUpperCase()} Academy`
                  : "Skip Pre-Career Education → Enter Adult Careers"}
              </button>
            </div>
          )}

          {session.pre_career_note && (
            <div className="bg-cyan-950/40 border border-cyan-800/70 rounded-lg px-3 py-2 text-xs text-cyan-200">
              <strong>Pre-Career Outcome:</strong> {session.pre_career_note}
            </div>
          )}

          {/* PHASE 4: Adult Career Terms */}
          {stage === "choose_career" && !session.mustered_out && (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <div className="text-sm font-semibold text-cyan-300 flex items-center gap-1.5">
                  <Shield className="w-4 h-4" /> Step 5 · Serve Career Term {session.terms_completed + 1} (4 Years)
                </div>
                {session.must_stay_career && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-amber-950 border border-amber-600 text-amber-200">
                    Natural 12 Advancement — Must serve another term in {session.current_career}
                  </span>
                )}
                {session.must_leave_career && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-red-950 border border-red-700 text-red-200">
                    Must leave previous career — choose a new career or Muster Out
                  </span>
                )}
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <label className="block space-y-1">
                  <span className="text-[11px] text-slate-400">Career (All 12 Core 2022)</span>
                  <select
                    disabled={Boolean(session.must_stay_career)}
                    className="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs disabled:opacity-50"
                    value={career}
                    onChange={(e) => setCareer(e.target.value)}
                  >
                    {careerList.map((c) => (
                      <option
                        key={c.career}
                        value={c.career}
                        disabled={Boolean(session.must_leave_career && session.current_career === c.career)}
                      >
                        {c.name}{" "}
                        {session.auto_qualify_career === c.career
                          ? "(AUTO-QUALIFY!)"
                          : session.current_career === c.career && !session.must_leave_career
                          ? "(Continuing)"
                          : `(Qual ${c.qual_stat} ${c.qual_target}+)`}
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
                  <span className="text-[11px] text-slate-400">Skill Table (Grants Level 1 / +1)</span>
                  <select
                    className="w-full bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs"
                    value={skillTable}
                    onChange={(e) =>
                      setSkillTable(
                        e.target.value as
                          | "service"
                          | "specialist"
                          | "personal"
                          | "advanced_edu"
                          | "officer"
                      )
                    }
                  >
                    <option value="service">Service Skills</option>
                    <option value="specialist">Specialist / Assignment</option>
                    <option value="personal">Personal Development (Stats)</option>
                    <option value="advanced_edu" disabled={!canAdvancedEdu}>
                      Advanced Education {canAdvancedEdu ? "(EDU 8+)" : "(Requires EDU 8+)"}
                    </option>
                    <option value="officer" disabled={!canOfficerTable}>
                      Officer Skills {canOfficerTable ? "(Commissioned)" : "(Officers Only)"}
                    </option>
                  </select>
                </label>
              </div>

              {/* Conditional player choices: Basic Training pick, Draft vs Drifter, Officer Commission */}
              <div className="bg-slate-950 border border-slate-800 rounded-lg p-2.5 flex flex-wrap items-center gap-3 text-[11px]">
                {!isContinuing && (
                  <label className="flex items-center gap-1.5 text-slate-300">
                    If Qualification Fails:
                    <select
                      className="bg-slate-900 border border-slate-700 rounded px-2 py-0.5 text-[11px]"
                      value={session.draft_used ? "drifter" : fallbackOnFail}
                      onChange={(e) => setFallbackOnFail(e.target.value as "draft" | "drifter")}
                    >
                      <option value="drifter">Enter Drifter</option>
                      <option value="draft" disabled={Boolean(session.draft_used)}>
                        Submit to Draft (1D6){session.draft_used ? " — Already Used" : ""}
                      </option>
                    </select>
                  </label>
                )}

                {showBasicPick && activeCareer && (
                  <label className="flex items-center gap-1.5 text-slate-300">
                    New Career Basic Training (Lvl 0, Unspecialised):
                    <select
                      className="bg-slate-900 border border-slate-700 rounded px-2 py-0.5 text-[11px]"
                      value={basicSkillPick}
                      onChange={(e) => setBasicSkillPick(e.target.value)}
                    >
                      {activeBasicChoices.map((sk) => (
                        <option key={sk} value={sk}>
                          {sk}
                        </option>
                      ))}
                    </select>
                  </label>
                )}

                {isMilitaryCareer && !session.is_officer && (
                  <label className="flex items-center gap-1.5 text-amber-200 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={attemptCommission}
                      onChange={(e) => setAttemptCommission(e.target.checked)}
                    />
                    Attempt Officer Commission (SOC 8+)
                  </label>
                )}

                {activeCareer && (
                  <span className="text-slate-400 ml-auto">
                    Surv: <strong>{activeCareer.surv_stat} {activeCareer.surv_target}+</strong> · Adv:{" "}
                    <strong>{activeCareer.adv_stat} {activeCareer.adv_target}+</strong>
                  </span>
                )}
              </div>

              <button
                disabled={busy}
                onClick={handleRunTerm}
                className="w-full bg-cyan-600 hover:bg-cyan-500 disabled:opacity-35 text-white font-semibold rounded-lg py-2 px-4 text-xs transition-colors"
              >
                {isContinuing ? "Continue" : "Enter"} {activeCareer?.name} ({assignment}) — Term{" "}
                {session.terms_completed + 1}
              </button>

              {/* Muster Out controls */}
              <div className="border-t border-slate-800 pt-3 flex flex-wrap items-center gap-3">
                <label className="text-xs text-slate-300 flex items-center gap-1.5">
                  Cash Rolls (max {maxCashRolls} of {Math.max(1, session.benefit_rolls_earned)} rolls):
                  <input
                    type="number"
                    min={0}
                    max={maxCashRolls}
                    disabled={session.terms_completed < 1 || Boolean(session.must_stay_career)}
                    value={Math.min(cashRolls, maxCashRolls)}
                    onChange={(e) =>
                      setCashRolls(Math.max(0, Math.min(maxCashRolls, Number(e.target.value) || 0)))
                    }
                    className="w-14 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-center font-mono disabled:opacity-35"
                  />
                </label>
                <button
                  disabled={busy || session.terms_completed < 1 || Boolean(session.must_stay_career)}
                  title={
                    session.must_stay_career
                      ? "Natural 12 advancement requires serving one more term first"
                      : session.terms_completed < 1
                      ? "Complete at least 1 term before mustering out"
                      : "Muster out and roll your Cash & Material Benefit tables"
                  }
                  onClick={handleMuster}
                  className="flex-1 bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-bold rounded-lg py-2 px-4 text-xs transition-colors"
                >
                  Muster Out &amp; Roll {Math.max(1, session.benefit_rolls_earned)} Benefit Roll(s)
                </button>
              </div>
            </div>
          )}

          {/* PHASE 5: Starting Skill Package (Core 2022 p. 51) */}
          {session.mustered_out && !session.skill_package_applied && skillPackages.length > 0 && (
            <div className="bg-slate-900 border border-cyan-700/70 rounded-xl p-4 space-y-3">
              <div className="text-sm font-semibold text-cyan-300 flex items-center gap-1.5">
                <Package className="w-4 h-4" /> Step 6 · Starting Campaign Skill Package (Pick 2 Skills at Level 1)
              </div>
              <div className="flex flex-wrap gap-1.5">
                {skillPackages.map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    onClick={() => setPkgId(p.id)}
                    className={`px-2.5 py-1 rounded border text-xs ${
                      pkgId === p.id
                        ? "bg-cyan-950 border-cyan-500 text-cyan-100 font-semibold"
                        : "bg-slate-950 border-slate-800 text-slate-400"
                    }`}
                  >
                    {p.name}
                  </button>
                ))}
              </div>
              {activePkg && (
                <div className="space-y-2">
                  <div className="text-[11px] text-slate-400">
                    {activePkg.description} Select 2 skills ({pkgSkills.length}/2):
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {activePkg.skills.map((sk) => {
                      const picked = pkgSkills.includes(sk);
                      const dis = !picked && pkgSkills.length >= 2;
                      return (
                        <button
                          key={sk}
                          type="button"
                          disabled={dis}
                          onClick={() => togglePkgSkill(sk)}
                          className={`px-2.5 py-1 rounded border text-xs disabled:opacity-35 ${
                            picked
                              ? "bg-emerald-950 border-emerald-500 text-emerald-200 font-semibold"
                              : "bg-slate-950 border-slate-700 text-slate-200"
                          }`}
                        >
                          {picked ? "✓ " : "+ "}
                          {sk} (Lvl 1)
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
              <button
                disabled={busy || pkgSkills.length < 2}
                onClick={handleApplyPackage}
                className="w-full bg-cyan-600 hover:bg-cyan-500 disabled:opacity-35 text-white font-bold rounded-lg py-2 text-xs"
              >
                Apply Skill Package ({pkgSkills.length}/2) → Finalize Traveller
              </button>
            </div>
          )}

          {/* PHASE 6: Final Review & Launch Campaign */}
          {session.mustered_out && (session.skill_package_applied || skillPackages.length === 0) && (
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
              {session.material_benefits && session.material_benefits.length > 0 && (
                <div className="text-[11px] text-emerald-200/90">
                  <strong>Material Benefits Rolled:</strong> {session.material_benefits.join(" · ")}
                </div>
              )}

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

          {/* Service Record Log */}
          {session.log.length > 0 && (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 space-y-2">
              <div className="text-xs font-semibold text-slate-300 uppercase tracking-wide">
                Service Record ({session.log.length} Term{session.log.length === 1 ? "" : "s"})
              </div>
              <div className="max-h-52 overflow-y-auto space-y-1.5">
                {session.log.map((e, i) => (
                  <div
                    key={`${e.term}-${i}`}
                    className="bg-slate-950 rounded-lg p-2 text-xs border border-slate-800 space-y-0.5"
                  >
                    <div className="font-semibold text-cyan-300">
                      Term {e.term} (Age {e.age}) — {e.career}{" "}
                      {!e.qualified && (
                        <span className="text-amber-400">[Failed Qual → Fallback]</span>
                      )}{" "}
                      {!e.survived && <span className="text-red-400">[Mishap / Did Not Grad]</span>}{" "}
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

import { useEffect, useState } from "react";
import { Crosshair, Flame, Radar, Shield, Swords, Target } from "lucide-react";
import { api, CampaignSnapshot } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function CombatPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [subTab, setSubTab] = useState<"space" | "personal" | "sensors">("space");
  const [combatLogMsg, setCombatLogMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Target selector from REAL ships currently in system (no phantom hardcoded pirate!)
  const inSystemTargets = (snapshot.system_ships ?? []).filter(
    (s) => s.status !== "destroyed" && s.status !== "departed"
  );
  const [selectedVesselId, setSelectedVesselId] = useState<string>(inSystemTargets[0]?.id ?? "");

  useEffect(() => {
    if (!selectedVesselId && inSystemTargets[0]) {
      setSelectedVesselId(inSystemTargets[0].id);
    } else if (selectedVesselId && !inSystemTargets.some((v) => v.id === selectedVesselId)) {
      setSelectedVesselId(inSystemTargets[0]?.id ?? "");
    }
  }, [inSystemTargets, selectedVesselId]);

  // Space combat inputs
  const [startRange, setStartRange] = useState("long");
  const [weaponName, setWeaponName] = useState("Pulse Laser");
  const [linkedCount, setLinkedCount] = useState(2);
  const [gunnerSkill, setGunnerSkill] = useState(2);
  const [sandcasters, setSandcasters] = useState(0);
  const [thrustRange, setThrustRange] = useState(0);
  const [thrustEvade, setThrustEvade] = useState(1);

  // Personal combat inputs
  const [personalEnemyName, setPersonalEnemyName] = useState("Armed Boarder");
  const [burstMode, setBurstMode] = useState(false);
  const [reaction, setReaction] = useState("none");
  const [autoAiTurn, setAutoAiTurn] = useState(true);

  // Sensor calculator state
  const [sensorGrade, setSensorGrade] = useState("military");
  const [observerTl, setObserverTl] = useState(12);
  const [targetTl, setTargetTl] = useState(12);
  const [targetThrust, setTargetThrust] = useState(2);
  const [targetStealth, setTargetStealth] = useState(-4);
  const [targetTransponder, setTargetTransponder] = useState(false);
  const [sensorResult, setSensorResult] = useState<Record<string, unknown> | null>(null);

  const spaceState = snapshot.active_space_combat?.state as
    | {
        round: number;
        phase: string;
        range_band: string;
        ships: Array<{
          ship_id: string;
          name: string;
          side: string;
          hull_current: number;
          hull_max: number;
          armor: number;
          thrust_remaining: number;
          thrust_max: number;
          evasive_dm: number;
          sensor_lock_targets: string[];
          crits: Record<string, number>;
          maneuver_used?: boolean;
          weapons_fired_this_round?: number;
          max_weapons_per_round?: number;
          sensor_action_used?: boolean;
          missiles_launched_this_round?: boolean;
        }>;
        active_missiles: Array<{
          id: string;
          count: number;
          rounds_to_impact: number;
          target_ship_id: string;
          ew_attempted_this_round?: boolean;
        }>;
        log: string[];
      }
    | undefined;

  const personalState = snapshot.active_personal_combat?.state as
    | {
        round: number;
        combatants: Array<{
          character_id: string;
          name: string;
          side: string;
          initiative: number;
          armor: number;
          range_band: string;
          conscious: boolean;
          alive: boolean;
          str_cur: number;
          dex_cur: number;
          end_cur: number;
          action_used_this_round?: boolean;
          weapon: { name: string; magazine_current: number; magazine_max: number };
        }>;
        log: string[];
      }
    | undefined;

  async function callSpace(body: Record<string, unknown>) {
    setBusy(true);
    setCombatLogMsg(null);
    try {
      const res = await api.spaceCombat(campaignId, body);
      const summary =
        typeof res.summary === "string"
          ? res.summary
          : typeof res.message === "string"
          ? res.message
          : null;
      if (summary) setCombatLogMsg(summary);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function callPersonal(body: Record<string, unknown>) {
    setBusy(true);
    setCombatLogMsg(null);
    try {
      const res = await api.personalCombat(campaignId, body);
      const summary = typeof res.summary === "string" ? res.summary : null;
      if (summary) setCombatLogMsg(summary);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleEngageSelectedVessel() {
    if (!selectedVesselId) return;
    setBusy(true);
    setCombatLogMsg(null);
    try {
      await api.interactVessel(campaignId, selectedVesselId, {
        action: "engage",
        range_band: startRange,
      });
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleStartPersonalEncounter() {
    const party = snapshot.characters
      .filter((r) => r.character.kind !== "npc")
      .slice(0, 4)
      .map((r) => ({
        character_id: r.character.id,
        name: r.character.name,
        side: "party",
        armor: 8,
        str_cur: r.character.upp_current.str_val,
        dex_cur: r.character.upp_current.dex_val,
        end_cur: r.character.upp_current.end_val,
        weapon: {
          name: "Gauss Rifle",
          skill: "Gun Combat (Slug)",
          range_m: 600,
          damage_dice: 4,
          damage_bonus: 0,
          ap: 5,
          auto_rating: 3,
          stun: false,
          magazine_current: 40,
          magazine_max: 40,
        },
      }));
    if (party.length === 0) {
      onError("Create a Player Character first before starting personal combat.");
      return;
    }
    const hostiles = [
      {
        name: personalEnemyName.trim() || "Hostile Combatant",
        side: "hostile",
        armor: 6,
        str_cur: 8,
        dex_cur: 8,
        end_cur: 8,
        weapon: {
          name: "Autopistol",
          skill: "Gun Combat (Slug)",
          range_m: 10,
          damage_dice: 3,
          damage_bonus: -3,
          ap: 0,
          auto_rating: 0,
          stun: false,
          magazine_current: 15,
          magazine_max: 15,
        },
      },
    ];
    await callPersonal({ action: "start", combatants: [...party, ...hostiles] });
  }

  async function runSensorCheck() {
    try {
      const res = await api.sensorCheck(campaignId, {
        context: "space_initial_detection",
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
        target_fired_weapons: false,
        target_crit_damage_severity: 0,
        operator_edu_dm: 1,
        operator_sensors_skill: 1,
        situational_dm: 0,
      });
      setSensorResult(res.sensor_check);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const partyShip = spaceState?.ships.find((s) => s.side === "party") ?? spaceState?.ships[0];
  const hostileShip = spaceState?.ships.find((s) => s.side !== "party" && s.hull_current > 0);
  const partyFighter = personalState?.combatants.find((c) => c.side === "party" && c.conscious);
  const hostileFighter = personalState?.combatants.find((c) => c.side !== "party" && c.alive);

  // Strict per-round Space Combat action economy flags
  const maxWeapons = Math.max(1, partyShip?.max_weapons_per_round ?? 1);
  const weaponsFired = partyShip?.weapons_fired_this_round ?? 0;
  const shotsRemaining = Math.max(0, maxWeapons - weaponsFired);
  const canManeuver =
    !busy &&
    Boolean(partyShip) &&
    partyShip!.hull_current > 0 &&
    !partyShip!.maneuver_used &&
    partyShip!.thrust_remaining >= Math.abs(thrustRange) + thrustEvade;
  const canFireTurret =
    !busy &&
    Boolean(partyShip && hostileShip) &&
    partyShip!.hull_current > 0 &&
    shotsRemaining > 0;
  const canSensorLock =
    !busy &&
    Boolean(partyShip && hostileShip) &&
    partyShip!.hull_current > 0 &&
    !partyShip!.sensor_action_used &&
    !partyShip!.sensor_lock_targets.includes(hostileShip!.ship_id);
  const canLaunchMissiles =
    !busy &&
    Boolean(partyShip && hostileShip) &&
    partyShip!.hull_current > 0 &&
    !partyShip!.missiles_launched_this_round;

  // Strict per-round Personal Combat action economy flag
  const canPersonalAttack =
    !busy &&
    Boolean(partyFighter && hostileFighter) &&
    !partyFighter!.action_used_this_round &&
    partyFighter!.weapon.magazine_current > 0;

  return (
    <div className="space-y-3 text-xs">
      <div className="grid grid-cols-3 gap-1">
        <button
          onClick={() => setSubTab("space")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 transition-colors ${
            subTab === "space"
              ? "bg-red-950 border-red-600 text-red-200"
              : "bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200"
          }`}
        >
          <Target className="w-3.5 h-3.5" /> Space
        </button>
        <button
          onClick={() => setSubTab("personal")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 transition-colors ${
            subTab === "personal"
              ? "bg-amber-950 border-amber-600 text-amber-200"
              : "bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200"
          }`}
        >
          <Swords className="w-3.5 h-3.5" /> Personal
        </button>
        <button
          onClick={() => setSubTab("sensors")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 transition-colors ${
            subTab === "sensors"
              ? "bg-cyan-950 border-cyan-600 text-cyan-200"
              : "bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200"
          }`}
        >
          <Radar className="w-3.5 h-3.5" /> Sensors
        </button>
      </div>

      {combatLogMsg && (
        <div className="p-2 rounded-lg bg-slate-900 border border-cyan-800/70 text-[11px] text-cyan-200">
          {combatLogMsg}
        </div>
      )}

      {subTab === "space" && (
        <div className="space-y-2.5">
          {!spaceState ? (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-3 space-y-2.5">
              <div className="font-semibold text-red-300">
                High Guard 2024 Spacecraft Combat Bridge
              </div>
              <p className="text-[11px] text-slate-400">
                No active space engagement. Choose a vessel currently in{" "}
                <strong className="text-slate-200">{snapshot.campaign.current_system_name}</strong>{" "}
                to engage (or click <em>Engage</em> on any ship in the <strong>Map</strong> tab).
              </p>
              {inSystemTargets.length === 0 ? (
                <div className="text-[11px] text-amber-300 bg-amber-950/30 border border-amber-800/60 rounded p-2">
                  No ships detected in {snapshot.campaign.current_system_name}. Sweep sensors in the
                  Map tab or jump to another system.
                </div>
              ) : (
                <div className="space-y-2">
                  <select
                    className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1.5 text-xs"
                    value={selectedVesselId}
                    onChange={(e) => setSelectedVesselId(e.target.value)}
                  >
                    {inSystemTargets.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.name} — {v.ship_class} ({v.tonnage}t, {v.disposition}, {v.orbit_location})
                      </option>
                    ))}
                  </select>
                  <div className="flex gap-2">
                    <select
                      className="bg-slate-950 border border-slate-700 rounded px-2 py-1"
                      value={startRange}
                      onChange={(e) => setStartRange(e.target.value)}
                    >
                      {["close", "short", "medium", "long", "very_long", "distant"].map((r) => (
                        <option key={r} value={r}>
                          Range: {r.replace("_", " ")}
                        </option>
                      ))}
                    </select>
                    <button
                      disabled={busy || !selectedVesselId || snapshot.ships.length === 0}
                      title={
                        snapshot.ships.length === 0
                          ? "No player ship in fleet"
                          : "Initiate High Guard space combat with selected vessel"
                      }
                      onClick={handleEngageSelectedVessel}
                      className="flex-1 bg-red-700 hover:bg-red-600 disabled:opacity-35 disabled:cursor-not-allowed text-white font-semibold rounded py-1.5 transition-colors"
                    >
                      Engage Selected Vessel
                    </button>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-2.5">
              <div className="flex items-center justify-between bg-red-950/40 border border-red-800 rounded-lg p-2.5">
                <div>
                  <span className="font-bold text-red-200">
                    Round {spaceState.round} · {spaceState.phase.toUpperCase()}
                  </span>
                  <div className="text-[11px] text-amber-300">
                    Range Band: <span className="font-mono uppercase">{spaceState.range_band}</span>
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    disabled={busy || !hostileShip}
                    title={
                      !hostileShip
                        ? "No surviving hostile vessels"
                        : "Let the Enemy AI decide maneuvers, EW, and return fire"
                    }
                    onClick={() =>
                      callSpace({
                        action: "ai_turn",
                        sandcaster_count: sandcasters,
                        gunner_skill: gunnerSkill,
                      })
                    }
                    className="bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded px-2 py-1 text-[11px] transition-colors"
                  >
                    AI Fight Back
                  </button>
                  <button
                    disabled={busy}
                    onClick={() =>
                      callSpace({
                        action: "next_round",
                        auto_ai_turn: autoAiTurn,
                        sandcaster_count: sandcasters,
                        gunner_skill: gunnerSkill,
                      })
                    }
                    className="bg-cyan-700 hover:bg-cyan-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2 py-1 text-[11px] font-medium transition-colors"
                  >
                    Next Round
                  </button>
                  <button
                    disabled={busy}
                    onClick={() => callSpace({ action: "end" })}
                    className="bg-slate-800 hover:bg-slate-700 text-slate-200 rounded px-2 py-1 text-[11px]"
                  >
                    End
                  </button>
                </div>
              </div>

              {/* Action Economy Strip */}
              {partyShip && (
                <div className="grid grid-cols-4 gap-1 text-center text-[10px] font-mono">
                  <div
                    className={`rounded py-1 border ${
                      partyShip.maneuver_used
                        ? "bg-slate-950 border-slate-800 text-slate-500"
                        : "bg-cyan-950/40 border-cyan-800 text-cyan-200"
                    }`}
                  >
                    Maneuver: {partyShip.maneuver_used ? "USED" : "READY"}
                  </div>
                  <div
                    className={`rounded py-1 border ${
                      shotsRemaining === 0
                        ? "bg-slate-950 border-slate-800 text-slate-500"
                        : "bg-red-950/40 border-red-800 text-red-200"
                    }`}
                  >
                    Weapons: {shotsRemaining}/{maxWeapons}
                  </div>
                  <div
                    className={`rounded py-1 border ${
                      partyShip.sensor_action_used
                        ? "bg-slate-950 border-slate-800 text-slate-500"
                        : "bg-cyan-950/40 border-cyan-800 text-cyan-200"
                    }`}
                  >
                    Sensors: {partyShip.sensor_action_used ? "USED" : "READY"}
                  </div>
                  <div
                    className={`rounded py-1 border ${
                      partyShip.missiles_launched_this_round
                        ? "bg-slate-950 border-slate-800 text-slate-500"
                        : "bg-amber-950/40 border-amber-800 text-amber-200"
                    }`}
                  >
                    Missiles: {partyShip.missiles_launched_this_round ? "USED" : "READY"}
                  </div>
                </div>
              )}

              <label className="flex items-center gap-1.5 text-[10px] text-amber-300 px-1">
                <input
                  type="checkbox"
                  checked={autoAiTurn}
                  onChange={(e) => setAutoAiTurn(e.target.checked)}
                />
                Enemy AI automatically decides tactics &amp; fights back on Next Round
              </label>

              {/* Ships Tactical Cards */}
              {spaceState.ships.map((s) => (
                <div
                  key={s.ship_id}
                  className={`rounded-lg p-2.5 border ${
                    s.side === "party"
                      ? "bg-slate-900 border-cyan-800/70"
                      : "bg-slate-900 border-red-800/70"
                  }`}
                >
                  <div className="flex justify-between font-semibold">
                    <span className={s.side === "party" ? "text-cyan-300" : "text-red-300"}>
                      {s.name} ({s.side}) {s.hull_current <= 0 && "☠ DISABLED"}
                    </span>
                    <span className="font-mono text-slate-200">
                      Hull {s.hull_current}/{s.hull_max} · Armor {s.armor}
                    </span>
                  </div>
                  <div className="text-[10px] text-slate-400 flex justify-between mt-0.5">
                    <span>
                      Thrust Pool: {s.thrust_remaining}/{s.thrust_max} · Evade DM-{s.evasive_dm}
                    </span>
                    <span>Sensor Locks: {s.sensor_lock_targets.length}</span>
                  </div>
                  {Object.keys(s.crits || {}).length > 0 && (
                    <div className="text-[10px] text-red-300 mt-1">
                      Crits:{" "}
                      {Object.entries(s.crits)
                        .map(([k, v]) => `${k} Sev ${v}`)
                        .join(", ")}
                    </div>
                  )}
                </div>
              ))}

              {/* Active Missile Salvos + EW Jam */}
              {spaceState.active_missiles.length > 0 && (
                <div className="bg-amber-950/40 border border-amber-700 rounded-lg p-2 space-y-1">
                  <div className="font-semibold text-amber-300 flex items-center gap-1">
                    <Flame className="w-3.5 h-3.5" /> Active Missile Salvos
                  </div>
                  {spaceState.active_missiles.map((m) => (
                    <div key={m.id} className="flex items-center justify-between text-[11px]">
                      <span>
                        {m.count}x Missiles — impact in {m.rounds_to_impact} rd(s)
                      </span>
                      <button
                        disabled={busy || Boolean(m.ew_attempted_this_round)}
                        title={
                          m.ew_attempted_this_round
                            ? "Already attempted EW jamming on this salvo this round"
                            : "Attempt Electronics (sensors) EW jamming"
                        }
                        onClick={() =>
                          callSpace({
                            action: "ew_jam",
                            salvo_id: m.id,
                            sensors_skill: 2,
                            edu_dm: 1,
                          })
                        }
                        className="bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded px-2 py-0.5"
                      >
                        {m.ew_attempted_this_round ? "EW Done" : "EW Jam"}
                      </button>
                    </div>
                  ))}
                </div>
              )}

              {/* Tactical Bridge Action Bar */}
              {partyShip && (
                <div className="bg-slate-900 border border-slate-800 rounded-lg p-2.5 space-y-2">
                  <div className="grid grid-cols-3 gap-1.5 text-[11px]">
                    <label>
                      <span className="text-[10px] text-slate-400">Thrust→Range</span>
                      <input
                        type="number"
                        className="w-full bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
                        value={thrustRange}
                        onChange={(e) => setThrustRange(Number(e.target.value))}
                      />
                    </label>
                    <label>
                      <span className="text-[10px] text-slate-400">Thrust→Evade</span>
                      <input
                        type="number"
                        min={0}
                        className="w-full bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
                        value={thrustEvade}
                        onChange={(e) => setThrustEvade(Number(e.target.value))}
                      />
                    </label>
                    <button
                      disabled={!canManeuver}
                      title={
                        partyShip.maneuver_used
                          ? "Maneuver already allocated this round — advance to Next Round"
                          : partyShip.thrust_remaining < Math.abs(thrustRange) + thrustEvade
                          ? `Not enough Thrust remaining (${partyShip.thrust_remaining})`
                          : "Allocate Maneuver Step Thrust Pool"
                      }
                      onClick={() =>
                        callSpace({
                          action: "maneuver",
                          ship_id: partyShip.ship_id,
                          thrust_to_range: thrustRange,
                          thrust_to_evade: thrustEvade,
                          pilot_skill: 2,
                          aid_gunners: false,
                        })
                      }
                      className="self-end bg-slate-800 hover:bg-slate-700 disabled:opacity-35 disabled:cursor-not-allowed border border-slate-600 rounded py-1 transition-colors"
                    >
                      {partyShip.maneuver_used ? "Maneuver Done" : "Maneuver"}
                    </button>
                  </div>

                  <div className="grid grid-cols-4 gap-1 text-[11px]">
                    <input
                      className="col-span-2 bg-slate-950 border border-slate-700 rounded px-1.5 py-0.5"
                      value={weaponName}
                      onChange={(e) => setWeaponName(e.target.value)}
                    />
                    <input
                      type="number"
                      min={1}
                      max={3}
                      title="Linked Turret Count"
                      className="bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
                      value={linkedCount}
                      onChange={(e) => setLinkedCount(Number(e.target.value) || 1)}
                    />
                    <input
                      type="number"
                      title="Gunner Skill"
                      className="bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
                      value={gunnerSkill}
                      onChange={(e) => setGunnerSkill(Number(e.target.value))}
                    />
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    <button
                      disabled={!canFireTurret}
                      title={
                        !hostileShip
                          ? "No surviving hostile target"
                          : shotsRemaining === 0
                          ? "All weapon mounts already fired this round — click Next Round"
                          : `Fire ${weaponName} (${shotsRemaining}/${maxWeapons} remaining this round)`
                      }
                      onClick={() =>
                        hostileShip &&
                        callSpace({
                          action: "fire",
                          ship_id: partyShip.ship_id,
                          target_ship_id: hostileShip.ship_id,
                          weapon_name: weaponName,
                          linked_count: linkedCount,
                          damage_dice: 2,
                          is_laser: weaponName.toLowerCase().includes("laser"),
                          gunner_skill: gunnerSkill,
                          gunner_dex_dm: 1,
                          fire_control_dm: 1,
                          sandcaster_count: sandcasters,
                        })
                      }
                      className="flex-1 bg-red-700 hover:bg-red-600 disabled:opacity-35 disabled:cursor-not-allowed text-white font-semibold rounded py-1 flex items-center justify-center gap-1 transition-colors"
                    >
                      <Crosshair className="w-3.5 h-3.5" />{" "}
                      {shotsRemaining === 0
                        ? "Weapons Spent"
                        : `Fire (${shotsRemaining}/${maxWeapons})`}
                    </button>
                    <button
                      disabled={!canSensorLock}
                      title={
                        !hostileShip
                          ? "No surviving target"
                          : partyShip.sensor_lock_targets.includes(hostileShip.ship_id)
                          ? "Sensor Lock already acquired on target (+2 DM active)"
                          : partyShip.sensor_action_used
                          ? "Sensor action already used this round — click Next Round"
                          : "Attempt Electronics (sensors) Lock (+2 DM)"
                      }
                      onClick={() =>
                        hostileShip &&
                        callSpace({
                          action: "sensor_lock",
                          ship_id: partyShip.ship_id,
                          target_ship_id: hostileShip.ship_id,
                          sensors_skill: 2,
                          edu_dm: 1,
                        })
                      }
                      className="bg-cyan-800 hover:bg-cyan-700 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2 py-1 transition-colors"
                    >
                      {hostileShip && partyShip.sensor_lock_targets.includes(hostileShip.ship_id)
                        ? "Locked (+2)"
                        : partyShip.sensor_action_used
                        ? "Sensors Used"
                        : "Sensor Lock"}
                    </button>
                    <button
                      disabled={!canLaunchMissiles}
                      title={
                        partyShip.missiles_launched_this_round
                          ? "Missile salvo already launched this round — click Next Round"
                          : "Launch 3x Smart TL12 Missiles"
                      }
                      onClick={() =>
                        hostileShip &&
                        callSpace({
                          action: "launch_missiles",
                          ship_id: partyShip.ship_id,
                          target_ship_id: hostileShip.ship_id,
                          missile_count: 3,
                          smart_tl: 12,
                        })
                      }
                      className="bg-amber-700 hover:bg-amber-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2 py-1 transition-colors"
                    >
                      {partyShip.missiles_launched_this_round ? "Salvo Spent" : "3x Missiles"}
                    </button>
                    <label className="flex items-center gap-1 text-[10px] text-slate-400">
                      <Shield className="w-3 h-3" /> Sand:
                      <input
                        type="number"
                        min={0}
                        max={3}
                        className="w-8 bg-slate-950 border border-slate-700 rounded text-center"
                        value={sandcasters}
                        onChange={(e) => setSandcasters(Number(e.target.value))}
                      />
                    </label>
                  </div>
                </div>
              )}

              {/* Combat Log */}
              <div className="bg-slate-950 border border-slate-800 rounded-lg p-2 max-h-36 overflow-y-auto text-[10px] space-y-0.5 font-mono text-slate-300">
                {spaceState.log.slice(-12).map((l, i) => (
                  <div key={i}>{l}</div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {subTab === "personal" && (
        <div className="space-y-2.5">
          {!personalState ? (
            <div className="bg-slate-900 border border-slate-800 rounded-xl p-3 space-y-2.5">
              <div className="font-semibold text-amber-300">
                Core 2022 Personal Combat (6s Rounds)
              </div>
              <p className="text-[11px] text-slate-400">
                No active firefight. Name an opponent to start a skirmish here, or let the Referee
                trigger one in narrative play.
              </p>
              <div className="flex gap-2">
                <input
                  className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
                  placeholder="Opponent name (e.g. Customs Enforcer, Armed Boarder)"
                  value={personalEnemyName}
                  onChange={(e) => setPersonalEnemyName(e.target.value)}
                />
                <button
                  disabled={busy || snapshot.characters.filter((r) => r.character.kind !== "npc").length === 0}
                  title={
                    snapshot.characters.filter((r) => r.character.kind !== "npc").length === 0
                      ? "Create a Player Character first"
                      : "Roll Initiative and start personal combat"
                  }
                  onClick={handleStartPersonalEncounter}
                  className="bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded px-3 py-1.5 transition-colors"
                >
                  Start Fight
                </button>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="flex items-center justify-between bg-amber-950/40 border border-amber-700 rounded-lg p-2">
                <span className="font-bold text-amber-200">
                  Personal Combat — Round {personalState.round}
                </span>
                <div className="flex gap-1">
                  <button
                    disabled={busy || !hostileFighter}
                    onClick={() =>
                      callPersonal({
                        action: "ai_turn",
                        reaction,
                        defender_reaction_dm: 1,
                      })
                    }
                    className="bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded px-2 py-1 text-[11px]"
                  >
                    AI Fight Back
                  </button>
                  <button
                    disabled={busy}
                    onClick={() =>
                      callPersonal({ action: "next_round", auto_ai_turn: autoAiTurn })
                    }
                    className="bg-cyan-700 hover:bg-cyan-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2 py-1 text-[11px]"
                  >
                    Next Round
                  </button>
                  <button
                    disabled={busy}
                    onClick={() => callPersonal({ action: "end" })}
                    className="bg-slate-800 hover:bg-slate-700 text-slate-200 rounded px-2 py-1 text-[11px]"
                  >
                    End
                  </button>
                </div>
              </div>

              {personalState.combatants.map((c) => (
                <div
                  key={c.character_id}
                  className={`p-2 rounded-lg border ${
                    c.side === "party"
                      ? "bg-slate-900 border-cyan-800/60"
                      : "bg-slate-900 border-red-800/60"
                  }`}
                >
                  <div className="flex justify-between font-semibold">
                    <span>
                      {c.name} (Init {c.initiative}){" "}
                      <span
                        className={`text-[9px] px-1 py-0.5 rounded ml-1 ${
                          c.action_used_this_round
                            ? "bg-slate-800 text-slate-500"
                            : "bg-emerald-950 text-emerald-300"
                        }`}
                      >
                        {c.action_used_this_round ? "ACTED" : "READY"}
                      </span>{" "}
                      {!c.alive && "☠ DEAD"} {c.alive && !c.conscious && "⚡ UNCONSCIOUS"}
                    </span>
                    <span className="font-mono text-cyan-300">
                      END {c.end_cur} · DEX {c.dex_cur} · STR {c.str_cur}
                    </span>
                  </div>
                  <div className="text-[10px] text-slate-400 flex justify-between mt-0.5">
                    <span>
                      {c.weapon.name} (Ammo {c.weapon.magazine_current}/{c.weapon.magazine_max}) ·
                      Armor {c.armor}
                    </span>
                    <span>Range: {c.range_band}</span>
                  </div>
                </div>
              ))}

              {partyFighter && (
                <div className="bg-slate-900 border border-slate-800 rounded-lg p-2 space-y-2">
                  <div className="flex items-center gap-2 text-[11px]">
                    <label className="flex items-center gap-1">
                      <input
                        type="checkbox"
                        checked={burstMode}
                        onChange={(e) => setBurstMode(e.target.checked)}
                      />
                      Auto Burst
                    </label>
                    <select
                      className="bg-slate-950 border border-slate-700 rounded px-1.5 py-0.5"
                      value={reaction}
                      onChange={(e) => setReaction(e.target.value)}
                    >
                      <option value="none">No Reaction</option>
                      <option value="dodge">Dodge (-1 Init)</option>
                      <option value="parry">Parry (-1 Init)</option>
                      <option value="dive_for_cover">Dive for Cover (-2 Init)</option>
                    </select>
                  </div>
                  <button
                    disabled={!canPersonalAttack}
                    title={
                      !hostileFighter
                        ? "No living hostile targets remaining"
                        : partyFighter.action_used_this_round
                        ? "Already took Significant Action this round — click Next Round"
                        : partyFighter.weapon.magazine_current <= 0
                        ? "Out of ammunition"
                        : "Fire weapon at hostile combatant"
                    }
                    onClick={() =>
                      hostileFighter &&
                      callPersonal({
                        action: "attack",
                        attacker_id: partyFighter.character_id,
                        defender_id: hostileFighter.character_id,
                        burst_mode: burstMode,
                        reaction,
                        defender_reaction_dm: 1,
                      })
                    }
                    className="w-full bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded py-1.5 transition-colors"
                  >
                    {!hostileFighter
                      ? "All Hostiles Neutralized"
                      : partyFighter.action_used_this_round
                      ? "Action Spent — Click Next Round"
                      : `${partyFighter.name} Attacks ${hostileFighter.name}`}
                  </button>
                </div>
              )}

              <div className="bg-slate-950 border border-slate-800 rounded-lg p-2 max-h-32 overflow-y-auto text-[10px] font-mono text-slate-300">
                {personalState.log.slice(-8).map((l, i) => (
                  <div key={i}>{l}</div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {subTab === "sensors" && (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-3 space-y-2.5">
          <div className="font-semibold text-cyan-300 flex items-center gap-1">
            <Radar className="w-4 h-4" /> High Guard 2024 Sensor Detection &amp; Stealth Calculator
          </div>
          <div className="grid grid-cols-2 gap-2 text-[11px]">
            <label className="block">
              <span className="text-slate-400">Observer Sensors</span>
              <select
                className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 mt-0.5"
                value={sensorGrade}
                onChange={(e) => setSensorGrade(e.target.value)}
              >
                {["basic", "civilian", "military", "improved", "advanced"].map((g) => (
                  <option key={g} value={g}>
                    {g}
                  </option>
                ))}
              </select>
            </label>
            <label className="block">
              <span className="text-slate-400">Target Stealth/Armor DM</span>
              <input
                type="number"
                className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 mt-0.5 font-mono"
                value={targetStealth}
                onChange={(e) => setTargetStealth(Number(e.target.value))}
              />
            </label>
            <label className="block">
              <span className="text-slate-400">Observer TL</span>
              <input
                type="number"
                className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 mt-0.5 font-mono"
                value={observerTl}
                onChange={(e) => setObserverTl(Number(e.target.value))}
              />
            </label>
            <label className="block">
              <span className="text-slate-400">Target TL</span>
              <input
                type="number"
                className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 mt-0.5 font-mono"
                value={targetTl}
                onChange={(e) => setTargetTl(Number(e.target.value))}
              />
            </label>
            <label className="block">
              <span className="text-slate-400">Target M-Drive Thrust</span>
              <input
                type="number"
                min={0}
                className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1 mt-0.5 font-mono"
                value={targetThrust}
                onChange={(e) => setTargetThrust(Number(e.target.value))}
              />
            </label>
            <label className="flex items-center gap-1.5 pt-4 text-slate-300">
              <input
                type="checkbox"
                checked={targetTransponder}
                onChange={(e) => setTargetTransponder(e.target.checked)}
              />
              Transponder / Comms On
            </label>
          </div>
          <button
            onClick={runSensorCheck}
            className="w-full bg-cyan-700 hover:bg-cyan-600 text-white font-semibold rounded py-1.5 transition-colors"
          >
            Run Sensor Detection Roll
          </button>
          {sensorResult && (
            <pre className="bg-slate-950 border border-slate-800 rounded p-2 text-[10px] overflow-x-auto text-cyan-200">
              {JSON.stringify(sensorResult, null, 2)}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

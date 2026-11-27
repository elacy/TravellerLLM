import { useState } from "react";
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

  // Space combat inputs
  const [startRange, setStartRange] = useState("medium");
  const [weaponName, setWeaponName] = useState("Pulse Laser");
  const [linkedCount, setLinkedCount] = useState(2);
  const [gunnerSkill, setGunnerSkill] = useState(2);
  const [sandcasters, setSandcasters] = useState(0);
  const [thrustRange, setThrustRange] = useState(0);
  const [thrustEvade, setThrustEvade] = useState(1);

  // Personal combat inputs
  const [burstMode, setBurstMode] = useState(false);
  const [reaction, setReaction] = useState("none");

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
        }>;
        active_missiles: Array<{ id: string; count: number; rounds_to_impact: number; target_ship_id: string }>;
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
          weapon: { name: string; magazine_current: number; magazine_max: number };
        }>;
        log: string[];
      }
    | undefined;

  async function callSpace(body: Record<string, unknown>) {
    setCombatLogMsg(null);
    try {
      const res = await api.spaceCombat(campaignId, body);
      setCombatLogMsg(JSON.stringify(res).slice(0, 260));
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function callPersonal(body: Record<string, unknown>) {
    setCombatLogMsg(null);
    try {
      const res = await api.personalCombat(campaignId, body);
      setCombatLogMsg(JSON.stringify(res).slice(0, 260));
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleStartSpaceEncounter() {
    const playerShip = snapshot.ships[0];
    const ships = [
      {
        ship_id: playerShip?.id,
        name: playerShip?.name ?? "ISS Beowulf",
        side: "party",
        tonnage: playerShip?.spec.tonnage ?? 200,
        tl: playerShip?.spec.tl ?? 12,
        hull_max: playerShip?.spec.hull_points_max ?? 80,
        hull_current: playerShip?.hull_current ?? 80,
        armor: playerShip?.spec.armor_rating ?? 4,
        thrust_max: playerShip?.spec.thrust ?? 1,
      },
      {
        name: "Vargr Corsair Raider",
        side: "hostile",
        tonnage: 400,
        tl: 12,
        hull_max: 160,
        hull_current: 160,
        armor: 4,
        thrust_max: 2,
      },
    ];
    await callSpace({ action: "start", range_band: startRange, ships });
  }

  async function handleStartPersonalEncounter() {
    const party = snapshot.characters.slice(0, 3).map((r) => ({
      character_id: r.character.id,
      name: r.character.name,
      side: "party",
      armor: 8,
      str_cur: r.character.upp_current.str_val,
      dex_cur: r.character.upp_current.dex_val,
      end_cur: r.character.upp_current.end_val,
      weapon_name: "Gauss Rifle",
      damage_dice: 4,
      ap: 5,
      auto_rating: 3,
      magazine_max: 40,
    }));
    const hostiles = [
      {
        name: "Corsair Boarding Marine",
        side: "hostile",
        armor: 6,
        str_cur: 8,
        dex_cur: 8,
        end_cur: 8,
        weapon_name: "Autopistol",
        damage_dice: 3,
        ap: 0,
        auto_rating: 0,
        magazine_max: 15,
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
        operator_skill_dm: 1,
        operator_stat_dm: 1,
        situational_dm: 0,
      });
      setSensorResult(res.sensor_check);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const partyShip = spaceState?.ships.find((s) => s.side === "party") ?? spaceState?.ships[0];
  const hostileShip = spaceState?.ships.find((s) => s.side !== "party") ?? spaceState?.ships[1];
  const partyFighter = personalState?.combatants.find((c) => c.side === "party" && c.conscious);
  const hostileFighter = personalState?.combatants.find((c) => c.side !== "party" && c.alive);

  return (
    <div className="space-y-3 text-xs">
      <div className="grid grid-cols-3 gap-1">
        <button
          onClick={() => setSubTab("space")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "space"
              ? "bg-red-950 border-red-600 text-red-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <Target className="w-3.5 h-3.5" /> Space
        </button>
        <button
          onClick={() => setSubTab("personal")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "personal"
              ? "bg-amber-950 border-amber-600 text-amber-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <Swords className="w-3.5 h-3.5" /> Personal
        </button>
        <button
          onClick={() => setSubTab("sensors")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "sensors"
              ? "bg-cyan-950 border-cyan-600 text-cyan-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <Radar className="w-3.5 h-3.5" /> Sensors
        </button>
      </div>

      {combatLogMsg && (
        <div className="p-2 rounded bg-slate-900 border border-slate-700 font-mono text-[10px] text-cyan-200 break-all">
          {combatLogMsg}
        </div>
      )}

      {subTab === "space" && (
        <div className="space-y-2.5">
          {!spaceState ? (
            <div className="bg-slate-900 border border-slate-800 rounded p-3 space-y-2">
              <div className="font-semibold text-red-300">High Guard 2024 Spacecraft Combat Bridge</div>
              <p className="text-[11px] text-slate-400">
                No active space engagement. Start a tactical skirmish here or let the Referee initiate one.
              </p>
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
                  onClick={handleStartSpaceEncounter}
                  className="flex-1 bg-red-700 hover:bg-red-600 text-white font-semibold rounded py-1"
                >
                  Battle Stations (Start Space Combat)
                </button>
              </div>
            </div>
          ) : (
            <div className="space-y-2.5">
              <div className="flex items-center justify-between bg-red-950/40 border border-red-800 rounded p-2">
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
                    onClick={() => callSpace({ action: "next_round" })}
                    className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2 py-1 text-[11px]"
                  >
                    Next Round
                  </button>
                  <button
                    onClick={() => callSpace({ action: "end" })}
                    className="bg-slate-800 hover:bg-slate-700 text-slate-200 rounded px-2 py-1 text-[11px]"
                  >
                    End
                  </button>
                </div>
              </div>

              {/* Ships Tactical Cards */}
              {spaceState.ships.map((s) => (
                <div
                  key={s.ship_id}
                  className={`rounded p-2 border ${
                    s.side === "party"
                      ? "bg-slate-900 border-cyan-800/70"
                      : "bg-slate-900 border-red-800/70"
                  }`}
                >
                  <div className="flex justify-between font-semibold">
                    <span className={s.side === "party" ? "text-cyan-300" : "text-red-300"}>
                      {s.name} ({s.side})
                    </span>
                    <span className="font-mono text-slate-200">
                      Hull {s.hull_current}/{s.hull_max} · Armor {s.armor}
                    </span>
                  </div>
                  <div className="text-[10px] text-slate-400 flex justify-between mt-0.5">
                    <span>
                      Thrust Pool: {s.thrust_remaining}/{s.thrust_max} · Evade DM-{s.evasive_dm}
                    </span>
                    <span>Locks: {s.sensor_lock_targets.length}</span>
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
                <div className="bg-amber-950/40 border border-amber-700 rounded p-2 space-y-1">
                  <div className="font-semibold text-amber-300 flex items-center gap-1">
                    <Flame className="w-3.5 h-3.5" /> Incoming Missile Salvos
                  </div>
                  {spaceState.active_missiles.map((m) => (
                    <div key={m.id} className="flex items-center justify-between text-[11px]">
                      <span>
                        {m.count}x Missiles — impact in {m.rounds_to_impact} rd(s)
                      </span>
                      <button
                        onClick={() =>
                          callSpace({
                            action: "ew_jam",
                            salvo_id: m.id,
                            sensors_skill: 2,
                            edu_dm: 1,
                          })
                        }
                        className="bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded px-2 py-0.5"
                      >
                        EW Jam
                      </button>
                    </div>
                  ))}
                </div>
              )}

              {/* Tactical Bridge Action Bar */}
              {partyShip && hostileShip && (
                <div className="bg-slate-900 border border-slate-800 rounded p-2 space-y-2">
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
                      className="self-end bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded py-1"
                    >
                      Maneuver
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
                      onClick={() =>
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
                      className="flex-1 bg-red-700 hover:bg-red-600 text-white font-semibold rounded py-1 flex items-center justify-center gap-1"
                    >
                      <Crosshair className="w-3.5 h-3.5" /> Fire Turret
                    </button>
                    <button
                      onClick={() =>
                        callSpace({
                          action: "sensor_lock",
                          ship_id: partyShip.ship_id,
                          target_ship_id: hostileShip.ship_id,
                          sensors_skill: 2,
                          edu_dm: 1,
                        })
                      }
                      className="bg-cyan-800 hover:bg-cyan-700 text-white rounded px-2 py-1"
                    >
                      Sensor Lock
                    </button>
                    <button
                      onClick={() =>
                        callSpace({
                          action: "launch_missiles",
                          ship_id: partyShip.ship_id,
                          target_ship_id: hostileShip.ship_id,
                          missile_count: 3,
                          smart_tl: 12,
                        })
                      }
                      className="bg-amber-700 hover:bg-amber-600 text-white rounded px-2 py-1"
                    >
                      Launch 3x Missiles
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
              <div className="bg-slate-950 border border-slate-800 rounded p-2 max-h-32 overflow-y-auto text-[10px] space-y-0.5 font-mono text-slate-300">
                {spaceState.log.slice(-10).map((l, i) => (
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
            <div className="bg-slate-900 border border-slate-800 rounded p-3 space-y-2">
              <div className="font-semibold text-amber-300">Core 2022 Personal Combat (6s Rounds)</div>
              <p className="text-[11px] text-slate-400">
                No active firefight. Start an encounter with your party or let the Referee trigger combat.
              </p>
              <button
                onClick={handleStartPersonalEncounter}
                className="w-full bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded py-1.5"
              >
                Roll Initiative & Start Personal Combat
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="flex items-center justify-between bg-amber-950/40 border border-amber-700 rounded p-2">
                <span className="font-bold text-amber-200">
                  Personal Combat — Round {personalState.round}
                </span>
                <div className="flex gap-1">
                  <button
                    onClick={() => callPersonal({ action: "next_round" })}
                    className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2 py-1 text-[11px]"
                  >
                    Next Round
                  </button>
                  <button
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
                  className={`p-2 rounded border ${
                    c.side === "party"
                      ? "bg-slate-900 border-cyan-800/60"
                      : "bg-slate-900 border-red-800/60"
                  }`}
                >
                  <div className="flex justify-between font-semibold">
                    <span>
                      {c.name} (Init {c.initiative}) {!c.alive && "☠ DEAD"}{" "}
                      {c.alive && !c.conscious && "⚡ UNCONSCIOUS"}
                    </span>
                    <span className="font-mono text-cyan-300">
                      END {c.end_cur} · DEX {c.dex_cur} · STR {c.str_cur}
                    </span>
                  </div>
                  <div className="text-[10px] text-slate-400 flex justify-between mt-0.5">
                    <span>
                      {c.weapon.name} (Ammo {c.weapon.magazine_current}/{c.weapon.magazine_max}) · Armor{" "}
                      {c.armor}
                    </span>
                    <span>Range: {c.range_band}</span>
                  </div>
                </div>
              ))}

              {partyFighter && hostileFighter && (
                <div className="bg-slate-900 border border-slate-800 rounded p-2 space-y-2">
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
                      <option value="dodge">Defender Dodge (-1 Init)</option>
                      <option value="parry">Defender Parry (-1 Init)</option>
                      <option value="dive_for_cover">Dive for Cover (-2 Init)</option>
                    </select>
                  </div>
                  <button
                    onClick={() =>
                      callPersonal({
                        action: "attack",
                        attacker_id: partyFighter.character_id,
                        defender_id: hostileFighter.character_id,
                        burst_mode: burstMode,
                        reaction,
                        defender_reaction_dm: 1,
                      })
                    }
                    className="w-full bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded py-1"
                  >
                    {partyFighter.name} Attacks {hostileFighter.name}
                  </button>
                </div>
              )}

              <div className="bg-slate-950 border border-slate-800 rounded p-2 max-h-32 overflow-y-auto text-[10px] font-mono text-slate-300">
                {personalState.log.slice(-8).map((l, i) => (
                  <div key={i}>{l}</div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {subTab === "sensors" && (
        <div className="bg-slate-900 border border-slate-800 rounded p-3 space-y-2.5">
          <div className="font-semibold text-cyan-300 flex items-center gap-1">
            <Radar className="w-4 h-4" /> High Guard 2024 Sensor Detection & Stealth Calculator
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
            className="w-full bg-cyan-700 hover:bg-cyan-600 text-white font-semibold rounded py-1.5"
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

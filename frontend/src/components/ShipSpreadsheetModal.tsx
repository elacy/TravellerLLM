import { useEffect, useState } from "react";
import { Plus, Save, ShieldAlert, ShieldCheck, Trash2, Wrench, X } from "lucide-react";
import { api, formatCr, RefitQuote, ShipRecord, WeaponMount } from "../lib/api";

interface Props {
  campaignId: string;
  ship: ShipRecord;
  starport: string;
  worldTl: number;
  companyAvailable: number;
  onClose: () => void;
  onApplied: (msg: string) => Promise<void>;
  onError: (msg: string) => void;
}

const HULL_CONFIGS = [
  { id: "standard", label: "Standard (Cylinder/Wedge)" },
  { id: "streamlined", label: "Streamlined (+10% hull cost, atmospheric)" },
  { id: "partially_streamlined", label: "Partially Streamlined" },
  { id: "distributed", label: "Distributed / Modular" },
  { id: "sphere", label: "Sphere" },
  { id: "close_structure", label: "Close Structure" },
  { id: "dispersed_structure", label: "Dispersed Structure" },
  { id: "planetoid", label: "Planetoid (Asteroid Hull)" },
  { id: "buffered_planetoid", label: "Buffered Planetoid" },
];

const ARMOR_TYPES = [
  { id: "none", label: "None (TL0)", pct: 0, minTl: 0 },
  { id: "titanium_steel", label: "Titanium Steel (TL7, 2.5%/pt)", pct: 2.5, minTl: 7 },
  { id: "crystaliron", label: "Crystaliron (TL10, 1.25%/pt)", pct: 1.25, minTl: 10 },
  { id: "bonded_superdense", label: "Bonded Superdense (TL14, 0.8%/pt)", pct: 0.8, minTl: 14 },
  { id: "molecular_bonded", label: "Molecular Bonded (TL16, 0.5%/pt)", pct: 0.5, minTl: 16 },
];

const STEALTH_OPTS = [
  { dm: 0, label: "None (DM 0)" },
  { dm: -2, label: "Basic Stealth Coating (DM -2 · MCr0.1/t)" },
  { dm: -4, label: "Enhanced Stealth Coating (DM -4 · MCr0.2/t)" },
  { dm: -6, label: "Advanced Stealth Hull (DM -6 · MCr0.5/t)" },
];

const SENSOR_GRADES = [
  { id: "basic", label: "Basic (DM -4, 0t, 0 Pwr)", tons: 0, pwr: 0 },
  { id: "civilian", label: "Civilian (DM -2, 1t, 1 Pwr)", tons: 1, pwr: 1 },
  { id: "military", label: "Military (DM 0, 2t, 2 Pwr)", tons: 2, pwr: 2 },
  { id: "improved", label: "Improved (DM +1, 3t, 4 Pwr)", tons: 3, pwr: 4 },
  { id: "advanced", label: "Advanced (DM +2, 5t, 6 Pwr)", tons: 5, pwr: 6 },
];

const WEAPON_PRESETS: Array<{ label: string; mount: WeaponMount }> = [
  {
    label: "Single Turret · Pulse Laser (1t, 4 Pwr, 1 HP)",
    mount: { name: "Single Pulse Laser Turret", mount_kind: "single_turret", weapons: ["Pulse Laser"], tonnage: 1, power: 4, hardpoints: 1 },
  },
  {
    label: "Double Turret · 2× Pulse Laser (1t, 8 Pwr, 1 HP)",
    mount: { name: "Double Pulse Laser Turret", mount_kind: "double_turret", weapons: ["Pulse Laser", "Pulse Laser"], tonnage: 1, power: 8, hardpoints: 1 },
  },
  {
    label: "Triple Turret · 3× Beam Laser (1t, 12 Pwr, 1 HP)",
    mount: { name: "Triple Beam Laser Turret", mount_kind: "triple_turret", weapons: ["Beam Laser", "Beam Laser", "Beam Laser"], tonnage: 1, power: 12, hardpoints: 1 },
  },
  {
    label: "Triple Turret · 3× Missile Rack (1t, 1 Pwr, 1 HP)",
    mount: { name: "Triple Missile Turret", mount_kind: "triple_turret", weapons: ["Missile Rack", "Missile Rack", "Missile Rack"], tonnage: 1, power: 1, hardpoints: 1 },
  },
  {
    label: "Triple Turret · 3× Sandcaster (1t, 1 Pwr, 1 HP)",
    mount: { name: "Triple Sandcaster Turret", mount_kind: "triple_turret", weapons: ["Sandcaster", "Sandcaster", "Sandcaster"], tonnage: 1, power: 1, hardpoints: 1 },
  },
  {
    label: "Barbette · Particle Beam (5t, 15 Pwr, 1 HP)",
    mount: { name: "Particle Beam Barbette", mount_kind: "barbette", weapons: ["Particle Beam"], tonnage: 5, power: 15, hardpoints: 1 },
  },
];

export function ShipSpreadsheetModal({
  campaignId,
  ship,
  starport,
  worldTl,
  companyAvailable,
  onClose,
  onApplied,
  onError,
}: Props) {
  const [draft, setDraft] = useState<ShipRecord>(() => structuredClone(ship));
  const [quote, setQuote] = useState<RefitQuote | null>(null);
  const [validation, setValidation] = useState<ShipRecord["validation"]>(ship.validation);
  const [quoting, setQuoting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [presetIdx, setPresetIdx] = useState(1);

  useEffect(() => {
    let cancelled = false;
    setQuoting(true);
    const t = setTimeout(() => {
      api
        .refitQuote(campaignId, ship.id, draft)
        .then((res) => {
          if (!cancelled) {
            setQuote(res.quote);
            setValidation(res.validation);
          }
        })
        .catch(() => {})
        .finally(() => {
          if (!cancelled) setQuoting(false);
        });
    }, 180);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [campaignId, ship.id, draft]);

  function patchSpec(partial: Partial<ShipRecord["spec"]>) {
    setDraft((prev) => {
      const nextSpec = { ...prev.spec, ...partial };
      // Recompute dependent tonnages cleanly like a real Traveller High Guard spreadsheet
      const hull = nextSpec.tonnage;
      const armorDef =
        ARMOR_TYPES.find((a) => a.id === (nextSpec.armor_type ?? "crystaliron")) ?? ARMOR_TYPES[2];
      nextSpec.armor_tonnage = Number(
        ((hull * armorDef.pct * Math.max(0, nextSpec.armor_rating)) / 100).toFixed(2)
      );
      nextSpec.m_drive_tonnage =
        nextSpec.thrust > 0 ? Number(Math.max(1, hull * 0.01 * nextSpec.thrust).toFixed(1)) : 0;
      nextSpec.j_drive_tonnage =
        nextSpec.jump_rating > 0
          ? Number(Math.max(5, hull * 0.025 * nextSpec.jump_rating + 5).toFixed(1))
          : 0;
      nextSpec.power_plant_tonnage = Number(
        Math.max(2, nextSpec.power_plant_output / 15).toFixed(1)
      );
      const sDef = SENSOR_GRADES.find((s) => s.id === nextSpec.sensor_grade) ?? SENSOR_GRADES[1];
      nextSpec.sensor_tonnage = sDef.tons;
      return { ...prev, spec: nextSpec };
    });
  }

  function addWeaponMount() {
    const preset = WEAPON_PRESETS[presetIdx];
    if (!preset) return;
    const current = draft.spec.weapon_mounts ?? [];
    patchSpec({ weapon_mounts: [...current, structuredClone(preset.mount)] });
  }

  function removeWeaponMount(idx: number) {
    const current = [...(draft.spec.weapon_mounts ?? [])];
    current.splice(idx, 1);
    patchSpec({ weapon_mounts: current });
  }

  async function handleApply() {
    setSaving(true);
    try {
      const res = await api.updateShip(campaignId, ship.id, draft);
      await onApplied(
        `Shipyard refit applied to ${ship.name} for ${formatCr(res.quote?.total_cost_cr ?? 0)}!`
      );
      onClose();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  const sp = draft.spec;
  const weapons = sp.weapon_mounts ?? [];
  const totalCost = quote?.total_cost_cr ?? 0;
  const canSubmit =
    !saving &&
    !quoting &&
    Boolean(quote?.allowed) &&
    Boolean(quote?.has_changes) &&
    validation.valid &&
    companyAvailable >= totalCost;

  const submitReason = quoting
    ? "Calculating shipyard quote…"
    : !quote?.has_changes
    ? "Modify at least one ship component or repair hull points first"
    : !quote.allowed
    ? quote.block_reason ?? "Refit not allowed at this starport"
    : !validation.valid
    ? "Resolve High Guard tonnage/power/hardpoint errors first"
    : companyAvailable < totalCost
    ? `Insufficient Company funds (need ${formatCr(totalCost)}, available ${formatCr(companyAvailable)})`
    : `Pay ${formatCr(totalCost)} and apply refit`;

  return (
    <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-3">
      <div className="bg-slate-950 border border-slate-700 rounded-xl w-full max-w-6xl max-h-[92vh] flex flex-col shadow-2xl overflow-hidden text-xs">
        {/* Header */}
        <div className="px-4 py-3 bg-slate-900 border-b border-slate-800 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <Wrench className="w-4 h-4 text-amber-400" />
            <div>
              <div className="font-bold text-sm text-amber-300">
                High Guard 2024 Ship Construction &amp; Refit Spreadsheet — {ship.name} ({sp.tonnage}t{" "}
                {ship.class_name})
              </div>
              <div className="text-[11px] text-slate-400">
                Docked at Starport Class {starport} (World TL{worldTl}) · Company Funds:{" "}
                <span className="font-mono text-emerald-300">{formatCr(companyAvailable)}</span>
              </div>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Spreadsheet Table Body */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          <div className="overflow-x-auto border border-slate-800 rounded-lg">
            <table className="w-full border-collapse text-left text-xs">
              <thead className="bg-slate-900 text-[10px] uppercase tracking-wider text-slate-400 sticky top-0">
                <tr>
                  <th className="py-2 px-3 border-b border-slate-800 w-44">Category</th>
                  <th className="py-2 px-3 border-b border-slate-800">Option / Configuration</th>
                  <th className="py-2 px-3 border-b border-slate-800 w-24 text-right">Tons (dT)</th>
                  <th className="py-2 px-3 border-b border-slate-800 w-20 text-right">Power</th>
                  <th className="py-2 px-3 border-b border-slate-800 w-48">Notes</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/80 bg-slate-950">
                {/* HULL & STRUCTURE */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    1. Hull, Configuration &amp; Hull Coatings
                  </td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Hull Tonnage &amp; TL</td>
                  <td className="py-1.5 px-3 flex items-center gap-3">
                    <span className="font-mono text-slate-200">{sp.tonnage} dT (fixed hull)</span>
                    <label className="flex items-center gap-1 text-slate-400">
                      Ship TL:
                      <input
                        type="number"
                        min={7}
                        max={worldTl}
                        value={sp.tl}
                        onChange={(e) => patchSpec({ tl: Number(e.target.value) || sp.tl })}
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono text-slate-100"
                      />
                    </label>
                    <label className="flex items-center gap-1 text-slate-400">
                      Repair Hull:
                      <input
                        type="number"
                        min={ship.hull_current}
                        max={sp.hull_points_max}
                        value={draft.hull_current}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            hull_current: Math.max(
                              ship.hull_current,
                              Math.min(sp.hull_points_max, Number(e.target.value) || ship.hull_current)
                            ),
                          })
                        }
                        className="w-16 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono text-slate-100"
                      />
                      <span>/ {sp.hull_points_max}</span>
                    </label>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-400">0.0</td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-300">
                    {validation.power_basic}
                  </td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Basic ship systems power</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Hull Configuration</td>
                  <td className="py-1.5 px-3">
                    <select
                      className="w-full max-w-sm bg-slate-900 border border-slate-700 rounded px-2 py-1"
                      value={sp.hull_config}
                      onChange={(e) => patchSpec({ hull_config: e.target.value })}
                    >
                      {HULL_CONFIGS.map((h) => (
                        <option key={h.id} value={h.id}>
                          {h.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-400">0.0</td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Streamlined enables safe gas-giant skimming</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Stealth Coating</td>
                  <td className="py-1.5 px-3">
                    <select
                      className="w-full max-w-sm bg-slate-900 border border-slate-700 rounded px-2 py-1"
                      value={sp.stealth_dm}
                      onChange={(e) => patchSpec({ stealth_dm: Number(e.target.value) })}
                    >
                      {STEALTH_OPTS.map((s) => (
                        <option key={s.dm} value={s.dm}>
                          {s.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-400">0.0</td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">DM to enemy sensor detection</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Hull Features</td>
                  <td className="py-1.5 px-3 flex flex-wrap gap-3 text-[11px]">
                    {(
                      [
                        ["hull_reflec", "Reflec (+3 vs Lasers)"],
                        ["radiation_shielding", "Radiation Shielding"],
                        ["heat_shielding", "Heat Shielding"],
                        ["self_sealing", "Self-Sealing"],
                        ["reinforced_hull", "Reinforced (+10% Hull)"],
                        ["light_hull", "Light Hull (-10% Hull)"],
                      ] as const
                    ).map(([key, label]) => (
                      <label key={key} className="flex items-center gap-1 text-slate-200 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={Boolean(sp[key])}
                          onChange={(e) => patchSpec({ [key]: e.target.checked })}
                        />
                        {label}
                      </label>
                    ))}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-400">0.0</td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">High Guard hull options</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Hull Armor</td>
                  <td className="py-1.5 px-3 flex flex-wrap items-center gap-2">
                    <select
                      className="bg-slate-900 border border-slate-700 rounded px-2 py-1"
                      value={sp.armor_type ?? "crystaliron"}
                      onChange={(e) => patchSpec({ armor_type: e.target.value })}
                    >
                      {ARMOR_TYPES.map((a) => (
                        <option key={a.id} value={a.id} disabled={a.minTl > worldTl}>
                          {a.label} {a.minTl > worldTl ? `(World TL${worldTl} too low)` : ""}
                        </option>
                      ))}
                    </select>
                    <label className="flex items-center gap-1 text-slate-400">
                      Rating:
                      <input
                        type="number"
                        min={0}
                        max={20}
                        value={sp.armor_rating}
                        onChange={(e) => patchSpec({ armor_rating: Math.max(0, Number(e.target.value) || 0) })}
                        className="w-16 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono text-slate-100"
                      />
                    </label>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {sp.armor_tonnage.toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Auto-calculated % of hull</td>
                </tr>

                {/* DRIVES, POWER & FUEL */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    2. Engineering — Drives, Power Plant &amp; Fuel Systems
                  </td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">M-Drive (Thrust)</td>
                  <td className="py-1.5 px-3">
                    <input
                      type="number"
                      min={0}
                      max={9}
                      value={sp.thrust}
                      onChange={(e) => patchSpec({ thrust: Math.max(0, Math.min(9, Number(e.target.value) || 0)) })}
                      className="w-20 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                    />{" "}
                    <span className="text-slate-400">G thrust</span>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">{sp.m_drive_tonnage.toFixed(1)}</td>
                  <td className="py-1.5 px-3 text-right font-mono text-amber-300">{validation.power_m_drive}</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">1% hull / G</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">J-Drive (Jump)</td>
                  <td className="py-1.5 px-3">
                    <input
                      type="number"
                      min={0}
                      max={6}
                      value={sp.jump_rating}
                      onChange={(e) =>
                        patchSpec({ jump_rating: Math.max(0, Math.min(6, Number(e.target.value) || 0)) })
                      }
                      className="w-20 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                    />{" "}
                    <span className="text-slate-400">parsecs (needs {validation.jump_fuel_required.toFixed(0)}t fuel)</span>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">{sp.j_drive_tonnage.toFixed(1)}</td>
                  <td className="py-1.5 px-3 text-right font-mono text-amber-300">{validation.power_j_drive}</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">2.5% hull × J + 5t</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Power Plant</td>
                  <td className="py-1.5 px-3">
                    <input
                      type="number"
                      min={10}
                      max={5000}
                      step={5}
                      value={sp.power_plant_output}
                      onChange={(e) =>
                        patchSpec({ power_plant_output: Math.max(10, Number(e.target.value) || 10) })
                      }
                      className="w-24 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                    />{" "}
                    <span className="text-slate-400">Power output</span>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {sp.power_plant_tonnage.toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-emerald-400">+{sp.power_plant_output}</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Fusion TL12 (~15 Pwr/t)</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Fuel Tanks, Scoops &amp; Processor</td>
                  <td className="py-1.5 px-3 flex flex-wrap items-center gap-3">
                    <label className="flex items-center gap-1">
                      Tanks (t):
                      <input
                        type="number"
                        min={0}
                        max={sp.tonnage}
                        value={sp.fuel_capacity_tons}
                        onChange={(e) =>
                          patchSpec({ fuel_capacity_tons: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-20 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.fuel_scoops)}
                        onChange={(e) => patchSpec({ fuel_scoops: e.target.checked })}
                      />
                      Fuel Scoops
                    </label>
                    <label className="flex items-center gap-1">
                      Processor (t):
                      <input
                        type="number"
                        min={0}
                        max={100}
                        value={sp.fuel_processor_tons ?? 0}
                        onChange={(e) =>
                          patchSpec({ fuel_processor_tons: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-16 bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                      />
                    </label>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {(sp.fuel_capacity_tons + (sp.fuel_processor_tons ?? 0)).toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Processor refines 20t/day per ton</td>
                </tr>

                {/* BRIDGE, COMPUTER & SENSORS */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    3. Bridge, Computer &amp; Sensors
                  </td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Bridge &amp; Computer</td>
                  <td className="py-1.5 px-3 flex flex-wrap items-center gap-3">
                    <select
                      className="bg-slate-900 border border-slate-700 rounded px-2 py-1"
                      value={sp.bridge_type ?? "standard"}
                      onChange={(e) => patchSpec({ bridge_type: e.target.value })}
                    >
                      <option value="standard">Standard Bridge</option>
                      <option value="holographic">Holographic Controls (+2 Init)</option>
                      <option value="dual_cockpit">Dual Cockpit</option>
                    </select>
                    <select
                      className="bg-slate-900 border border-slate-700 rounded px-2 py-1 font-mono"
                      value={sp.computer_bandwidth}
                      onChange={(e) => patchSpec({ computer_bandwidth: Number(e.target.value) })}
                    >
                      {[5, 10, 15, 20, 25, 30, 35].map((bw) => (
                        <option key={bw} value={bw}>
                          Computer/{bw}
                        </option>
                      ))}
                    </select>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.computer_bis)}
                        onChange={(e) => patchSpec({ computer_bis: e.target.checked })}
                      />
                      bis (Jump)
                    </label>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.computer_fib)}
                        onChange={(e) => patchSpec({ computer_fib: e.target.checked })}
                      />
                      fib (Hardened)
                    </label>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">{sp.bridge_tonnage.toFixed(1)}</td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Core / High Guard bridge suite</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Sensor Suite</td>
                  <td className="py-1.5 px-3">
                    <select
                      className="w-full max-w-sm bg-slate-900 border border-slate-700 rounded px-2 py-1"
                      value={sp.sensor_grade}
                      onChange={(e) => patchSpec({ sensor_grade: e.target.value })}
                    >
                      {SENSOR_GRADES.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">{sp.sensor_tonnage.toFixed(1)}</td>
                  <td className="py-1.5 px-3 text-right font-mono text-amber-300">
                    {SENSOR_GRADES.find((g) => g.id === sp.sensor_grade)?.pwr ?? 1}
                  </td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Detection &amp; lock DM</td>
                </tr>

                {/* ACCOMMODATIONS (ALL STATEROOM & BERTH TYPES) */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    4. Crew &amp; Passenger Accommodations (All Stateroom &amp; Berth Types)
                  </td>
                </tr>
                {(
                  [
                    ["Standard Staterooms", "staterooms", 4.0, "MCr0.5 each · Crew / Middle Passage"],
                    ["High Staterooms", "high_staterooms", 6.0, "MCr0.8 each · High Passage comfort"],
                    ["Luxury Staterooms", "luxury_staterooms", 10.0, "MCr1.5 each · VIP / Noble suites"],
                    ["Marine Barracks Berths", "barracks_berths", 2.0, "MCr0.1 each · Troop bunks"],
                    ["Low Berths (Cryo)", "low_berths", 0.5, "MCr0.05 each · Low Passage"],
                    ["Emergency Low Berths", "emergency_low_berths", 1.0, "MCr0.1 each · 4-person rescue cryo"],
                  ] as const
                ).map(([label, key, perTon, note]) => {
                  const qty = Number(sp[key] ?? 0);
                  return (
                    <tr key={key}>
                      <td className="py-1.5 px-3 text-slate-300 font-medium">{label}</td>
                      <td className="py-1.5 px-3">
                        <input
                          type="number"
                          min={0}
                          max={200}
                          value={qty}
                          onChange={(e) => patchSpec({ [key]: Math.max(0, Number(e.target.value) || 0) })}
                          className="w-20 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                        />{" "}
                        <span className="text-slate-500">× {perTon}t</span>
                      </td>
                      <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                        {(qty * perTon).toFixed(1)}
                      </td>
                      <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                      <td className="py-1.5 px-3 text-[11px] text-slate-500">{note}</td>
                    </tr>
                  );
                })}
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Common Areas</td>
                  <td className="py-1.5 px-3">
                    <input
                      type="number"
                      min={0}
                      max={200}
                      value={sp.common_areas_tonnage}
                      onChange={(e) =>
                        patchSpec({ common_areas_tonnage: Math.max(0, Number(e.target.value) || 0) })
                      }
                      className="w-20 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                    />{" "}
                    <span className="text-slate-500">tons (MCr0.1/t)</span>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {sp.common_areas_tonnage.toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Rec deck, galley, lounge</td>
                </tr>

                {/* WEAPONS & HARDPOINTS */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    5. Armament &amp; Hardpoints ({validation.hardpoints_used}/{validation.hardpoints_max} HP used)
                  </td>
                </tr>
                {weapons.map((w, idx) => (
                  <tr key={idx}>
                    <td className="py-1.5 px-3 text-slate-300 font-medium">Mount #{idx + 1}</td>
                    <td className="py-1.5 px-3 flex items-center justify-between gap-2">
                      <span>
                        <strong className="text-slate-100">{w.name}</strong>{" "}
                        <span className="text-slate-400">({w.weapons.join(", ")})</span>
                      </span>
                      <button
                        onClick={() => removeWeaponMount(idx)}
                        className="text-red-400 hover:text-red-300 p-1"
                        title="Remove weapon mount"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </td>
                    <td className="py-1.5 px-3 text-right font-mono text-cyan-300">{w.tonnage.toFixed(1)}</td>
                    <td className="py-1.5 px-3 text-right font-mono text-amber-300">{w.power}</td>
                    <td className="py-1.5 px-3 text-[11px] text-slate-500">{w.hardpoints} Hardpoint(s)</td>
                  </tr>
                ))}
                <tr>
                  <td className="py-1.5 px-3 text-slate-400">Add Weapon Mount</td>
                  <td colSpan={4} className="py-1.5 px-3">
                    <div className="flex gap-2 items-center">
                      <select
                        className="flex-1 max-w-md bg-slate-900 border border-slate-700 rounded px-2 py-1"
                        value={presetIdx}
                        onChange={(e) => setPresetIdx(Number(e.target.value))}
                      >
                        {WEAPON_PRESETS.map((p, i) => (
                          <option key={i} value={i}>
                            {p.label}
                          </option>
                        ))}
                      </select>
                      <button
                        disabled={validation.hardpoints_used >= validation.hardpoints_max}
                        title={
                          validation.hardpoints_used >= validation.hardpoints_max
                            ? `All ${validation.hardpoints_max} hardpoint(s) on this ${sp.tonnage}t hull are full`
                            : "Install turret/barbette"
                        }
                        onClick={addWeaponMount}
                        className="bg-cyan-700 hover:bg-cyan-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2.5 py-1 flex items-center gap-1 font-medium"
                      >
                        <Plus className="w-3.5 h-3.5" /> Add Mount
                      </button>
                    </div>
                  </td>
                </tr>

                {/* AUXILIARIES & CARGO */}
                <tr className="bg-slate-900/50">
                  <td colSpan={5} className="py-1.5 px-3 font-bold text-cyan-300 text-[11px] uppercase">
                    6. Ship Facilities, Auxiliaries &amp; Cargo Hold
                  </td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Bays &amp; Facilities</td>
                  <td className="py-1.5 px-3 flex flex-wrap items-center gap-3 text-[11px]">
                    <label className="flex items-center gap-1">
                      Armory (t):
                      <input
                        type="number"
                        min={0}
                        max={50}
                        value={sp.armory_tons ?? 0}
                        onChange={(e) => patchSpec({ armory_tons: Math.max(0, Number(e.target.value) || 0) })}
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1">
                      Med Bay (t):
                      <input
                        type="number"
                        min={0}
                        max={50}
                        value={sp.medical_bay_tons ?? 0}
                        onChange={(e) =>
                          patchSpec({ medical_bay_tons: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1">
                      Workshop (t):
                      <input
                        type="number"
                        min={0}
                        max={50}
                        value={sp.workshop_tons ?? 0}
                        onChange={(e) =>
                          patchSpec({ workshop_tons: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1">
                      Docking Bay (t):
                      <input
                        type="number"
                        min={0}
                        max={100}
                        value={sp.docking_bay_tons ?? 0}
                        onChange={(e) =>
                          patchSpec({ docking_bay_tons: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1">
                      Probes:
                      <input
                        type="number"
                        min={0}
                        max={50}
                        value={sp.probe_drones ?? 0}
                        onChange={(e) =>
                          patchSpec({ probe_drones: Math.max(0, Number(e.target.value) || 0) })
                        }
                        className="w-14 bg-slate-900 border border-slate-700 rounded px-1 py-0.5 font-mono"
                      />
                    </label>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.briefing_room)}
                        onChange={(e) => patchSpec({ briefing_room: e.target.checked })}
                      />
                      Briefing Room (4t)
                    </label>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.library)}
                        onChange={(e) => patchSpec({ library: e.target.checked })}
                      />
                      Library (4t)
                    </label>
                    <label className="flex items-center gap-1 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={Boolean(sp.cargo_scoops)}
                        onChange={(e) => patchSpec({ cargo_scoops: e.target.checked })}
                      />
                      Cargo Scoop (2t)
                    </label>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {(
                      (sp.armory_tons ?? 0) +
                      (sp.medical_bay_tons ?? 0) +
                      (sp.workshop_tons ?? 0) +
                      (sp.docking_bay_tons ?? 0) +
                      (sp.probe_drones ?? 0) * 0.2 +
                      (sp.briefing_room ? 4 : 0) +
                      (sp.library ? 4 : 0) +
                      (sp.cargo_scoops ? 2 : 0)
                    ).toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">High Guard ship systems</td>
                </tr>
                <tr>
                  <td className="py-1.5 px-3 text-slate-300 font-medium">Cargo Hold</td>
                  <td className="py-1.5 px-3 flex items-center gap-2">
                    <input
                      type="number"
                      min={0}
                      max={sp.tonnage}
                      value={sp.cargo_capacity_tons}
                      onChange={(e) =>
                        patchSpec({ cargo_capacity_tons: Math.max(0, Number(e.target.value) || 0) })
                      }
                      className="w-24 bg-slate-900 border border-slate-700 rounded px-2 py-0.5 font-mono"
                    />
                    <button
                      type="button"
                      onClick={() =>
                        patchSpec({
                          cargo_capacity_tons: Math.max(
                            0,
                            Number((sp.cargo_capacity_tons + validation.free_tonnage).toFixed(1))
                          ),
                        })
                      }
                      className="text-[11px] px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-cyan-300"
                    >
                      Auto-fill remaining ({validation.free_tonnage >= 0 ? "+" : ""}
                      {validation.free_tonnage.toFixed(1)}t)
                    </button>
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-cyan-300">
                    {sp.cargo_capacity_tons.toFixed(1)}
                  </td>
                  <td className="py-1.5 px-3 text-right font-mono text-slate-500">0</td>
                  <td className="py-1.5 px-3 text-[11px] text-slate-500">Remaining hull space</td>
                </tr>
              </tbody>
            </table>
          </div>

          {/* Itemized Refit Work Order & Validation */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div className="bg-slate-900 border border-slate-800 rounded-lg p-3 space-y-1.5">
              <div className="font-semibold text-slate-200 flex items-center justify-between">
                <span>High Guard Validation</span>
                {validation.valid ? (
                  <span className="text-emerald-400 flex items-center gap-1">
                    <ShieldCheck className="w-3.5 h-3.5" /> Valid Design
                  </span>
                ) : (
                  <span className="text-red-400 flex items-center gap-1">
                    <ShieldAlert className="w-3.5 h-3.5" /> Errors Present
                  </span>
                )}
              </div>
              <div className="grid grid-cols-3 gap-2 font-mono text-[11px]">
                <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                  <div className="text-slate-500">TONNAGE</div>
                  <div className={validation.free_tonnage < -0.01 ? "text-red-400 font-bold" : "text-cyan-300"}>
                    {validation.used_tonnage.toFixed(1)} / {sp.tonnage}t
                  </div>
                </div>
                <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                  <div className="text-slate-500">PEAK POWER</div>
                  <div
                    className={
                      validation.power_total_peak_combat > sp.power_plant_output
                        ? "text-amber-300"
                        : "text-emerald-300"
                    }
                  >
                    {validation.power_total_peak_combat} / {sp.power_plant_output}
                  </div>
                </div>
                <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                  <div className="text-slate-500">HARDPOINTS</div>
                  <div className="text-slate-200">
                    {validation.hardpoints_used} / {validation.hardpoints_max}
                  </div>
                </div>
              </div>
              {validation.issues.map((iss, i) => (
                <div
                  key={i}
                  className={iss.severity === "error" ? "text-red-400 text-[11px]" : "text-amber-300 text-[11px]"}
                >
                  [{iss.category}] {iss.message}
                </div>
              ))}
            </div>

            <div className="bg-slate-900 border border-amber-800/60 rounded-lg p-3 space-y-1.5">
              <div className="font-semibold text-amber-300 flex items-center justify-between">
                <span>Shipyard Refit Invoice</span>
                <span className="font-mono text-sm">
                  {quoting ? "Pricing…" : formatCr(totalCost)}
                </span>
              </div>
              {quote?.block_reason && <div className="text-red-400">{quote.block_reason}</div>}
              {quote && quote.items.length > 0 ? (
                <div className="max-h-28 overflow-y-auto divide-y divide-slate-800 text-[11px]">
                  {quote.items.map((it, i) => (
                    <div key={i} className="py-1 flex justify-between gap-2">
                      <span className="text-slate-300">{it.item}</span>
                      <span className="font-mono text-amber-200 shrink-0">{formatCr(it.cost_cr)}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-slate-500 text-[11px]">
                  Adjust any component above to see itemized MgT2e shipyard refit pricing.
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Sticky Footer */}
        <div className="px-4 py-3 bg-slate-900 border-t border-slate-800 flex items-center justify-between gap-3">
          <div className="text-[11px] text-slate-400">
            Free Hull Space:{" "}
            <strong className={validation.free_tonnage < -0.01 ? "text-red-400" : "text-emerald-300"}>
              {validation.free_tonnage.toFixed(1)}t
            </strong>{" "}
            · Estimated Shipyard Time: <strong>{quote?.days_required ?? 0} day(s)</strong>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 font-medium"
            >
              Cancel
            </button>
            <button
              disabled={!canSubmit}
              title={submitReason}
              onClick={handleApply}
              className="px-4 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-bold flex items-center gap-1.5 transition-colors"
            >
              <Save className="w-3.5 h-3.5" />
              {saving ? "Applying Refit…" : `Pay ${formatCr(totalCost)} & Apply Refit`}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

import { useState } from "react";
import { Edit3, Package, Rocket, Save, ShieldAlert, ShieldCheck, Wrench } from "lucide-react";
import { api, CampaignSnapshot, formatCr, ShipRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function ShipsPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [editingShipId, setEditingShipId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ShipRecord | null>(null);

  function startEdit(ship: ShipRecord) {
    setEditingShipId(ship.id);
    setDraft(structuredClone(ship));
  }

  async function saveShip() {
    if (!draft) return;
    try {
      await api.updateShip(campaignId, draft.id, draft);
      setEditingShipId(null);
      setDraft(null);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  return (
    <div className="space-y-3 text-xs">
      {snapshot.ships.length === 0 && (
        <div className="text-slate-400 text-center py-6">No ships registered in this campaign.</div>
      )}
      {snapshot.ships.map((ship) => {
        const isEditing = editingShipId === ship.id && draft;
        const cargoUsed = (ship.cargo || []).reduce((acc, c) => acc + (Number(c.tons) || 0), 0);
        return (
          <div key={ship.id} className="bg-slate-900 border border-slate-800 rounded p-3 space-y-2.5">
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="flex items-center gap-1.5">
                  <Rocket className="w-4 h-4 text-cyan-400" />
                  <span className="font-bold text-sm text-slate-100">{ship.name}</span>
                  <span className="text-slate-400">({ship.class_name})</span>
                </div>
                <div className="text-[11px] text-slate-400">
                  {ship.spec.tonnage}t · TL{ship.spec.tl} · J-{ship.spec.jump_rating} / M-{ship.spec.thrust} ·
                  Docked: <span className="text-cyan-300">{ship.docked_system_name}</span>
                </div>
              </div>
              <div className="flex items-center gap-1.5">
                {ship.validation.valid ? (
                  <span className="flex items-center gap-1 text-[10px] px-2 py-0.5 rounded bg-emerald-950 border border-emerald-700 text-emerald-300">
                    <ShieldCheck className="w-3 h-3" /> Valid
                  </span>
                ) : (
                  <span className="flex items-center gap-1 text-[10px] px-2 py-0.5 rounded bg-red-950 border border-red-700 text-red-300">
                    <ShieldAlert className="w-3 h-3" /> Invalid
                  </span>
                )}
                <button
                  onClick={() => (isEditing ? setEditingShipId(null) : startEdit(ship))}
                  className="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-[11px] flex items-center gap-1"
                >
                  <Edit3 className="w-3 h-3" /> {isEditing ? "Cancel" : "Refit"}
                </button>
              </div>
            </div>

            {isEditing && draft ? (
              <div className="bg-slate-950 border border-amber-700/60 rounded p-2.5 space-y-2">
                <div className="font-semibold text-amber-300 flex items-center gap-1">
                  <Wrench className="w-3.5 h-3.5" /> High Guard 2024 Ship Refit & Spec Editor
                </div>
                <div className="grid grid-cols-3 gap-1.5 text-[11px]">
                  {(
                    [
                      ["Tonnage", "tonnage"],
                      ["TL", "tl"],
                      ["Hull Max", "hull_points_max"],
                      ["Armor", "armor_rating"],
                      ["Thrust (M)", "thrust"],
                      ["Jump (J)", "jump_rating"],
                      ["Power Out", "power_plant_output"],
                      ["Stealth DM", "stealth_dm"],
                      ["Fuel Cap (t)", "fuel_capacity_tons"],
                      ["Cargo Cap (t)", "cargo_capacity_tons"],
                      ["Staterooms", "staterooms"],
                      ["Price (Cr)", "purchase_price_cr"],
                    ] as const
                  ).map(([lbl, key]) => (
                    <label key={key} className="block">
                      <span className="text-[10px] text-slate-400">{lbl}</span>
                      <input
                        type="number"
                        className="w-full bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                        value={Number(draft.spec[key] ?? 0)}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            spec: { ...draft.spec, [key]: Number(e.target.value) },
                          })
                        }
                      />
                    </label>
                  ))}
                </div>
                <div className="grid grid-cols-3 gap-1.5 text-[11px]">
                  <label className="block">
                    <span className="text-[10px] text-slate-400">Sensors</span>
                    <select
                      className="w-full bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5"
                      value={draft.spec.sensor_grade}
                      onChange={(e) =>
                        setDraft({ ...draft, spec: { ...draft.spec, sensor_grade: e.target.value } })
                      }
                    >
                      {["basic", "civilian", "military", "improved", "advanced"].map((g) => (
                        <option key={g} value={g}>
                          {g}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="block">
                    <span className="text-[10px] text-slate-400">Hull Current</span>
                    <input
                      type="number"
                      className="w-full bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                      value={draft.hull_current}
                      onChange={(e) => setDraft({ ...draft, hull_current: Number(e.target.value) })}
                    />
                  </label>
                  <label className="block">
                    <span className="text-[10px] text-slate-400">Refined Fuel (t)</span>
                    <input
                      type="number"
                      className="w-full bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                      value={draft.fuel_refined}
                      onChange={(e) => setDraft({ ...draft, fuel_refined: Number(e.target.value) })}
                    />
                  </label>
                </div>
                <button
                  onClick={saveShip}
                  className="w-full bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded py-1 flex items-center justify-center gap-1"
                >
                  <Save className="w-3.5 h-3.5" /> Validate & Save Ship Refit
                </button>
              </div>
            ) : (
              <>
                {/* Key Stats Grid */}
                <div className="grid grid-cols-4 gap-1.5 text-center font-mono text-[11px]">
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">HULL</div>
                    <div className="text-emerald-300 font-bold">
                      {ship.hull_current}/{ship.spec.hull_points_max}
                    </div>
                  </div>
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">FUEL (REF/UNR)</div>
                    <div className="text-cyan-300 font-bold">
                      {ship.fuel_refined}/{ship.fuel_unrefined}t
                    </div>
                  </div>
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">ARMOR / STEALTH</div>
                    <div className="text-slate-200 font-bold">
                      {ship.spec.armor_rating} / {ship.spec.stealth_dm}
                    </div>
                  </div>
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">SENSORS</div>
                    <div className="text-amber-300 capitalize font-bold">{ship.spec.sensor_grade}</div>
                  </div>
                </div>

                {/* High Guard Validation Summary */}
                <div className="bg-slate-950 border border-slate-800 rounded p-2 text-[11px] space-y-1">
                  <div className="flex justify-between text-slate-300">
                    <span>
                      Tonnage: {ship.validation.used_tonnage.toFixed(1)}t / {ship.spec.tonnage}t
                    </span>
                    <span>
                      Power: {ship.validation.power_total_peak_combat} / {ship.spec.power_plant_output}
                    </span>
                    <span>
                      Hardpoints: {ship.validation.hardpoints_used}/{ship.validation.hardpoints_max}
                    </span>
                  </div>
                  {ship.validation.issues.map((iss, i) => (
                    <div
                      key={i}
                      className={iss.severity === "error" ? "text-red-400" : "text-amber-400"}
                    >
                      [{iss.category}] {iss.message}
                    </div>
                  ))}
                </div>

                {/* Mortgage & Maintenance */}
                {ship.mortgage && (
                  <div className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded p-2 text-[11px]">
                    <div>
                      <div className="text-slate-300 font-medium">
                        Mortgage: {formatCr(ship.mortgage.monthly_payment_cr)} / 4wk
                      </div>
                      <div className="text-[10px] text-slate-400">
                        Paid {ship.mortgage.periods_paid}/{ship.mortgage.periods_total} · Overdue:{" "}
                        {ship.mortgage.periods_overdue} · Skipped Maint: {ship.skipped_maintenance_periods}
                      </div>
                    </div>
                    <div className="text-right text-[10px] text-slate-400">
                      Maint: {formatCr(Math.round((ship.spec.purchase_price_cr * 0.001) / 12))}/mo
                    </div>
                  </div>
                )}

                {/* Cargo Hold Manifest */}
                <div className="bg-slate-950 border border-slate-800 rounded p-2 space-y-1">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-slate-300 flex items-center gap-1">
                      <Package className="w-3.5 h-3.5 text-amber-400" /> Cargo Hold Manifest
                    </span>
                    <span className="font-mono text-amber-300">
                      {cargoUsed}t / {ship.spec.cargo_capacity_tons}t
                    </span>
                  </div>
                  {(!ship.cargo || ship.cargo.length === 0) ? (
                    <div className="text-[11px] text-slate-500">Cargo bay empty.</div>
                  ) : (
                    <div className="divide-y divide-slate-800">
                      {ship.cargo.map((lot, idx) => (
                        <div key={idx} className="flex justify-between py-1 text-[11px]">
                          <span className="text-slate-200">
                            {lot.name} ({lot.tons}t)
                          </span>
                          <span className="font-mono text-slate-400">
                            {lot.purchase_price_cr ? `${formatCr(lot.purchase_price_cr)}/t` : ""}
                            {lot.bought_at ? ` @ ${lot.bought_at}` : ""}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Critical Hits */}
                {ship.crits.length > 0 && (
                  <div className="bg-red-950/40 border border-red-800 rounded p-2 text-[11px] space-y-0.5">
                    <div className="font-semibold text-red-300">Active Ship Critical Hits</div>
                    {ship.crits.map((cr, i) => (
                      <div key={i} className="text-red-200">
                        • {cr.name ?? "Crit"}: {cr.effect ?? JSON.stringify(cr)}
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        );
      })}
    </div>
  );
}

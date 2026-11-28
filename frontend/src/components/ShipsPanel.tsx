import { useEffect, useState } from "react";
import {
  Anchor,
  Edit3,
  Fuel,
  Package,
  Rocket,
  Save,
  ShieldAlert,
  ShieldCheck,
  Wrench,
} from "lucide-react";
import { api, CampaignSnapshot, formatCr, RefitQuote, ShipRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function ShipsPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [editingShipId, setEditingShipId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ShipRecord | null>(null);
  const [quote, setQuote] = useState<RefitQuote | null>(null);
  const [draftValidation, setDraftValidation] = useState<ShipRecord["validation"] | null>(null);
  const [quoting, setQuoting] = useState(false);
  const [portMsg, setPortMsg] = useState<string | null>(null);

  const starport = (snapshot.current_system_info?.starport || "A").toUpperCase();
  const gasGiants = snapshot.current_system_info?.gas_giants ?? 1;
  const worldTl = snapshot.current_system_info?.tech_level ?? 12;
  const companyAcc =
    snapshot.accounts.find((a) => a.kind === "company") ?? snapshot.accounts[0];
  const companyAvailable = (companyAcc?.balance_cr ?? 0) + (companyAcc?.credit_limit_cr ?? 0);

  // Live shipyard refit quote whenever the user adjusts refit fields
  useEffect(() => {
    if (!editingShipId || !draft) {
      setQuote(null);
      setDraftValidation(null);
      return;
    }
    let cancelled = false;
    setQuoting(true);
    const t = setTimeout(() => {
      api
        .refitQuote(campaignId, editingShipId, draft)
        .then((res) => {
          if (!cancelled) {
            setQuote(res.quote);
            setDraftValidation(res.validation);
          }
        })
        .catch(() => {
          // ignore transient quote error while typing
        })
        .finally(() => {
          if (!cancelled) setQuoting(false);
        });
    }, 180);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [campaignId, editingShipId, draft]);

  function startEdit(ship: ShipRecord) {
    setEditingShipId(ship.id);
    setDraft(structuredClone(ship));
    setQuote(null);
    setDraftValidation(ship.validation);
  }

  async function saveShip() {
    if (!draft) return;
    setPortMsg(null);
    try {
      const res = await api.updateShip(campaignId, draft.id, draft);
      setEditingShipId(null);
      setDraft(null);
      setQuote(null);
      setPortMsg(
        `Shipyard refit applied for ${formatCr(res.quote?.total_cost_cr ?? 0)}!`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleRefuel(
    shipId: string,
    mode: "refined" | "unrefined" | "gas_giant_skim" | "process_onboard"
  ) {
    setPortMsg(null);
    try {
      const res = await api.refuelShip(campaignId, shipId, mode);
      const q = res.quote as { note?: string } | undefined;
      setPortMsg(q?.note ?? "Refuel complete.");
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleDock(shipId: string) {
    setPortMsg(null);
    try {
      const res = await api.dockShip(campaignId, shipId, 0);
      const fee = res.berthing_fee as
        | { starport?: string; roll_1d6?: number; total_cr?: number }
        | undefined;
      setPortMsg(
        `Paid Starport ${fee?.starport ?? "?"} berthing/docking fee (1D=${fee?.roll_1d6 ?? "?"}): ${formatCr(
          fee?.total_cr ?? 0
        )}.`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  return (
    <div className="space-y-3 text-xs">
      {snapshot.ships.length > 0 && (
        <div className="flex items-center justify-between bg-slate-900 border border-slate-800 rounded-lg px-2.5 py-2 text-[11px]">
          <span className="text-slate-300">
            Fleet: <strong className="text-cyan-300">{snapshot.ships.length}</strong> ship(s) ·
            Starport <strong className="text-amber-300">{starport}</strong> (TL{worldTl})
          </span>
          <span className="font-mono text-amber-300">
            Fleet Max Jump: J-{snapshot.fleet_max_jump ?? snapshot.ships[0].spec.jump_rating}
          </span>
        </div>
      )}
      {portMsg && (
        <div className="p-2 rounded-lg bg-emerald-950/60 border border-emerald-700 text-emerald-200 text-[11px]">
          {portMsg}
        </div>
      )}
      {snapshot.ships.length === 0 && (
        <div className="text-slate-400 text-center py-6">No ships registered in this campaign.</div>
      )}
      {snapshot.ships.map((ship) => {
        const isEditing = editingShipId === ship.id && draft;
        const cargoUsed = (ship.cargo || []).reduce((acc, c) => acc + (Number(c.tons) || 0), 0);
        const freeFuel = Math.max(
          0,
          ship.spec.fuel_capacity_tons - (ship.fuel_refined + ship.fuel_unrefined)
        );
        const refinedCost = Math.round(freeFuel * 500);
        const unrefCost = Math.round(freeFuel * 100);
        const hasScoops = Boolean(ship.spec.fuel_scoops);
        const hasProcessor = Number(ship.spec.fuel_processor_tons ?? 0) > 0;

        const canRefuelRefined =
          (starport === "A" || starport === "B") && freeFuel > 0.1 && companyAvailable >= refinedCost;
        const canRefuelUnref =
          ["A", "B", "C", "D"].includes(starport) && freeFuel > 0.1 && companyAvailable >= unrefCost;
        const canSkim = gasGiants > 0 && hasScoops && freeFuel > 0.1;
        const canProcess = hasProcessor && ship.fuel_unrefined > 0.1;
        const canDock = ["A", "B", "C", "D"].includes(starport);
        const canOpenShipyard = ["A", "B", "C"].includes(starport);

        const refitCanSubmit =
          Boolean(quote) &&
          quote!.allowed &&
          quote!.has_changes &&
          Boolean(draftValidation?.valid) &&
          companyAvailable >= quote!.total_cost_cr &&
          !quoting;

        return (
          <div
            key={ship.id}
            className="bg-slate-900 border border-slate-800 rounded-xl p-3 space-y-2.5 shadow-sm"
          >
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="flex items-center gap-1.5">
                  <Rocket className="w-4 h-4 text-cyan-400" />
                  <span className="font-bold text-sm text-slate-100">{ship.name}</span>
                  <span className="text-slate-400">({ship.class_name})</span>
                </div>
                <div className="text-[11px] text-slate-400">
                  {ship.spec.tonnage}t · TL{ship.spec.tl} · J-{ship.spec.jump_rating} / M-
                  {ship.spec.thrust} · Docked:{" "}
                  <span className="text-cyan-300">{ship.docked_system_name}</span>
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
                  disabled={!isEditing && !canOpenShipyard}
                  title={
                    !canOpenShipyard
                      ? `Starport Class ${starport} has no shipyard or repair facilities (requires Class A–C)`
                      : "Open paid Shipyard Refit & Repair Bay"
                  }
                  onClick={() => (isEditing ? setEditingShipId(null) : startEdit(ship))}
                  className="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-35 disabled:cursor-not-allowed border border-slate-700 text-[11px] flex items-center gap-1 transition-colors"
                >
                  <Edit3 className="w-3 h-3" /> {isEditing ? "Cancel" : "Shipyard"}
                </button>
              </div>
            </div>

            {isEditing && draft ? (
              <div className="bg-slate-950 border border-amber-700/60 rounded-lg p-2.5 space-y-2.5">
                <div className="flex items-center justify-between">
                  <div className="font-semibold text-amber-300 flex items-center gap-1">
                    <Wrench className="w-3.5 h-3.5" /> Paid Shipyard Refit & Repair (Starport{" "}
                    {starport}, TL{worldTl})
                  </div>
                  <span className="text-[10px] font-mono text-slate-400">
                    Funds: {formatCr(companyAvailable)}
                  </span>
                </div>
                <div className="text-[10px] text-slate-400">
                  All upgrades and hull repairs must be paid from your Company account at a
                  qualified Starport shipyard (Class A/B for refits, Class C for repairs). Hull
                  tonnage &amp; fuel cannot be altered for free.
                </div>
                <div className="grid grid-cols-3 gap-1.5 text-[11px]">
                  {(
                    [
                      ["TL (max " + worldTl + ")", "tl", 1, worldTl],
                      ["Hull Max", "hull_points_max", 1, 9999],
                      ["Armor", "armor_rating", 0, 20],
                      ["Thrust (M)", "thrust", 0, 9],
                      ["Jump (J)", "jump_rating", 0, 6],
                      ["Power Out", "power_plant_output", 10, 5000],
                      ["Stealth DM", "stealth_dm", -6, 0],
                      ["Fuel Cap (t)", "fuel_capacity_tons", 0, 5000],
                      ["Cargo Cap (t)", "cargo_capacity_tons", 0, 5000],
                      ["Staterooms", "staterooms", 0, 200],
                    ] as const
                  ).map(([lbl, key, minV, maxV]) => (
                    <label key={key} className="block">
                      <span className="text-[10px] text-slate-400">{lbl}</span>
                      <input
                        type="number"
                        min={minV}
                        max={maxV}
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
                    <span className="text-[10px] text-slate-400">
                      Repair Hull (now {ship.hull_current}/{draft.spec.hull_points_max})
                    </span>
                    <input
                      type="number"
                      min={ship.hull_current}
                      max={draft.spec.hull_points_max}
                      className="w-full bg-slate-900 border border-slate-700 rounded px-1.5 py-0.5 font-mono"
                      value={draft.hull_current}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          hull_current: Math.max(
                            ship.hull_current,
                            Math.min(draft.spec.hull_points_max, Number(e.target.value))
                          ),
                        })
                      }
                    />
                  </label>
                </div>

                {/* Live Itemized Shipyard Quote */}
                <div className="bg-slate-900 border border-slate-800 rounded p-2 space-y-1 text-[11px]">
                  <div className="flex justify-between font-semibold">
                    <span className="text-slate-300">Shipyard Work Order</span>
                    <span className="font-mono text-amber-300">
                      {quoting ? "Pricing…" : formatCr(quote?.total_cost_cr ?? 0)}
                    </span>
                  </div>
                  {quote?.block_reason && (
                    <div className="text-red-400 font-medium">{quote.block_reason}</div>
                  )}
                  {draftValidation && !draftValidation.valid && (
                    <div className="text-red-400">
                      High Guard Validation Error:{" "}
                      {draftValidation.issues.map((i) => i.message).join("; ")}
                    </div>
                  )}
                  {quote && quote.items.length === 0 && !quote.block_reason && (
                    <div className="text-slate-500">Adjust specs or hull repair above to price a refit.</div>
                  )}
                  {quote &&
                    quote.items.map((it, i) => (
                      <div key={i} className="flex justify-between text-[10px] text-slate-300">
                        <span>• {it.item}</span>
                        <span className="font-mono text-amber-200">{formatCr(it.cost_cr)}</span>
                      </div>
                    ))}
                  {quote && quote.total_cost_cr > companyAvailable && (
                    <div className="text-red-400 text-[10px] font-semibold">
                      Insufficient Company credits (short by{" "}
                      {formatCr(quote.total_cost_cr - companyAvailable)}).
                    </div>
                  )}
                </div>

                <button
                  disabled={!refitCanSubmit}
                  title={
                    !quote?.has_changes
                      ? "No changes to apply"
                      : !quote.allowed
                      ? quote.block_reason || "Not allowed at this starport"
                      : !draftValidation?.valid
                      ? "Fix High Guard validation errors first"
                      : companyAvailable < quote.total_cost_cr
                      ? "Insufficient Company funds"
                      : "Pay shipyard invoice and apply refit"
                  }
                  onClick={saveShip}
                  className="w-full bg-emerald-600 hover:bg-emerald-500 disabled:opacity-35 disabled:cursor-not-allowed text-white font-semibold rounded py-1.5 flex items-center justify-center gap-1 transition-colors"
                >
                  <Save className="w-3.5 h-3.5" />
                  {quote?.has_changes
                    ? `Pay ${formatCr(quote.total_cost_cr)} & Apply Refit (${quote.days_required}d)`
                    : "No Refit Changes"}
                </button>
              </div>
            ) : (
              <>
                {/* Key Stats Grid */}
                <div className="grid grid-cols-4 gap-1.5 text-center font-mono text-[11px]">
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">HULL</div>
                    <div
                      className={
                        ship.hull_current < ship.spec.hull_points_max
                          ? "text-amber-300 font-bold"
                          : "text-emerald-300 font-bold"
                      }
                    >
                      {ship.hull_current}/{ship.spec.hull_points_max}
                    </div>
                  </div>
                  <div className="bg-slate-950 border border-slate-800 rounded p-1.5">
                    <div className="text-[9px] text-slate-400">FUEL (REF/UNR)</div>
                    <div className="text-cyan-300 font-bold">
                      {ship.fuel_refined.toFixed(0)}/{ship.fuel_unrefined.toFixed(0)}t
                    </div>
                    <div className="text-[9px] text-slate-500">
                      cap {ship.spec.fuel_capacity_tons}t
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
                    <div className="text-amber-300 capitalize font-bold">
                      {ship.spec.sensor_grade}
                    </div>
                  </div>
                </div>

                {/* Refueling & Starport Docking Operations (unavailable options strictly disabled!) */}
                <div className="bg-slate-950 border border-slate-800 rounded-lg p-2 space-y-1.5">
                  <div className="text-[10px] text-slate-400 uppercase tracking-wider flex items-center justify-between">
                    <span className="flex items-center gap-1">
                      <Fuel className="w-3 h-3 text-cyan-400" /> Starport Refuel &amp; Berthing
                    </span>
                    <span className="font-mono text-slate-500">
                      Free tank: {freeFuel.toFixed(1)}t
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-1 text-[10px]">
                    <button
                      disabled={!canRefuelRefined}
                      title={
                        starport !== "A" && starport !== "B"
                          ? `Starport ${starport} does not sell Refined fuel (Class A/B only)`
                          : freeFuel <= 0.1
                          ? "Fuel tanks are already full"
                          : companyAvailable < refinedCost
                          ? `Need ${formatCr(refinedCost)}`
                          : `Buy ${freeFuel.toFixed(1)}t Refined fuel for ${formatCr(refinedCost)}`
                      }
                      onClick={() => handleRefuel(ship.id, "refined")}
                      className="bg-cyan-900/70 hover:bg-cyan-800 disabled:opacity-35 disabled:cursor-not-allowed text-cyan-100 rounded py-1 px-1.5 transition-colors"
                    >
                      Refuel Refined ({freeFuel > 0.1 ? formatCr(refinedCost) : "Full"})
                    </button>
                    <button
                      disabled={!canRefuelUnref}
                      title={
                        !["A", "B", "C", "D"].includes(starport)
                          ? `Starport ${starport} has no commercial fuel`
                          : freeFuel <= 0.1
                          ? "Fuel tanks are already full"
                          : companyAvailable < unrefCost
                          ? `Need ${formatCr(unrefCost)}`
                          : `Buy ${freeFuel.toFixed(1)}t Unrefined fuel for ${formatCr(unrefCost)}`
                      }
                      onClick={() => handleRefuel(ship.id, "unrefined")}
                      className="bg-slate-800 hover:bg-slate-700 disabled:opacity-35 disabled:cursor-not-allowed text-slate-200 rounded py-1 px-1.5 transition-colors"
                    >
                      Refuel Unref ({freeFuel > 0.1 ? formatCr(unrefCost) : "Full"})
                    </button>
                    <button
                      disabled={!canSkim}
                      title={
                        gasGiants <= 0
                          ? "No gas giant in this system"
                          : !hasScoops
                          ? "Ship has no Fuel Scoops installed"
                          : freeFuel <= 0.1
                          ? "Fuel tanks are already full"
                          : "Skim free unrefined fuel from gas giant (6h)"
                      }
                      onClick={() => handleRefuel(ship.id, "gas_giant_skim")}
                      className="bg-amber-900/60 hover:bg-amber-800 disabled:opacity-35 disabled:cursor-not-allowed text-amber-100 rounded py-1 px-1.5 transition-colors"
                    >
                      Skim Gas Giant (Cr0)
                    </button>
                    <button
                      disabled={!canProcess}
                      title={
                        !hasProcessor
                          ? "Ship has no onboard Fuel Processor"
                          : ship.fuel_unrefined <= 0.1
                          ? "No unrefined fuel onboard to process"
                          : `Refine ${ship.fuel_unrefined.toFixed(1)}t unrefined fuel onboard`
                      }
                      onClick={() => handleRefuel(ship.id, "process_onboard")}
                      className="bg-slate-800 hover:bg-slate-700 disabled:opacity-35 disabled:cursor-not-allowed text-slate-200 rounded py-1 px-1.5 transition-colors"
                    >
                      Refine Onboard ({ship.fuel_unrefined.toFixed(0)}t)
                    </button>
                  </div>
                  <button
                    disabled={!canDock}
                    title={
                      !canDock
                        ? `Starport Class ${starport} charges no berthing fee (Cr0)`
                        : `Roll 1D Starport ${starport} docking fee`
                    }
                    onClick={() => handleDock(ship.id)}
                    className="w-full bg-slate-900 hover:bg-slate-800 disabled:opacity-35 disabled:cursor-not-allowed border border-slate-700 text-slate-200 rounded py-1 text-[10px] flex items-center justify-center gap-1 transition-colors"
                  >
                    <Anchor className="w-3 h-3 text-amber-400" />
                    {canDock
                      ? `Pay Starport ${starport} Docking Fee (1D × Port)`
                      : `Starport ${starport} — No Docking Fee (Cr0)`}
                  </button>
                </div>

                {/* High Guard Validation Summary */}
                <div className="bg-slate-950 border border-slate-800 rounded p-2 text-[11px] space-y-1">
                  <div className="flex justify-between text-slate-300">
                    <span>
                      Tonnage: {ship.validation.used_tonnage.toFixed(1)}t / {ship.spec.tonnage}t
                    </span>
                    <span>
                      Power: {ship.validation.power_total_peak_combat} /{" "}
                      {ship.spec.power_plant_output}
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
                        {ship.mortgage.periods_overdue} · Skipped Maint:{" "}
                        {ship.skipped_maintenance_periods}
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
                  {!ship.cargo || ship.cargo.length === 0 ? (
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

import { useState } from "react";
import {
  Anchor,
  Fuel,
  Package,
  Rocket,
  ShieldAlert,
  ShieldCheck,
  Wrench,
} from "lucide-react";
import { api, CampaignSnapshot, formatCr, ShipRecord } from "../lib/api";
import { ShipSpreadsheetModal } from "./ShipSpreadsheetModal";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function ShipsPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [spreadsheetShip, setSpreadsheetShip] = useState<ShipRecord | null>(null);
  const [portMsg, setPortMsg] = useState<string | null>(null);

  const starport = (snapshot.current_system_info?.starport || "A").toUpperCase();
  const gasGiants = snapshot.current_system_info?.gas_giants ?? 1;
  const worldTl = snapshot.current_system_info?.tech_level ?? 12;
  const companyAcc =
    snapshot.accounts.find((a) => a.kind === "company") ?? snapshot.accounts[0];
  const companyAvailable = (companyAcc?.balance_cr ?? 0) + (companyAcc?.credit_limit_cr ?? 0);

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
      const res = await api.dockShip(campaignId, shipId);
      const note = typeof res.note === "string" ? res.note : null;
      const fee = res.berthing_fee as
        | { starport?: string; roll_1d6?: number; total_cr?: number }
        | undefined;
      setPortMsg(
        note ??
          `Docked at Starport ${fee?.starport ?? "?"} (1D=${fee?.roll_1d6 ?? "?"}): ${formatCr(
            fee?.total_cr ?? 0
          )} initial berthing fee (covers first 6 days).`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleUndock(shipId: string) {
    setPortMsg(null);
    try {
      const res = await api.undockShip(campaignId, shipId);
      setPortMsg(typeof res.note === "string" ? res.note : "Ship undocked and broke orbit.");
      if (spreadsheetShip?.id === shipId) {
        setSpreadsheetShip(null);
      }
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  function berthingDailyRate(sp: string): number {
    switch (sp) {
      case "A":
        return 500;
      case "B":
        return 200;
      case "C":
        return 100;
      case "D":
        return 10;
      default:
        return 0;
    }
  }

  function berthingBaseMult(sp: string): number {
    switch (sp) {
      case "A":
        return 1000;
      case "B":
        return 500;
      case "C":
        return 100;
      case "D":
        return 10;
      default:
        return 0;
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
        const cargoUsed = (ship.cargo || []).reduce((acc, c) => acc + (Number(c.tons) || 0), 0);
        const freeFuel = Math.max(
          0,
          ship.spec.fuel_capacity_tons - (ship.fuel_refined + ship.fuel_unrefined)
        );
        const refinedCost = Math.round(freeFuel * 500);
        const unrefCost = Math.round(freeFuel * 100);
        const hasScoops = Boolean(ship.spec.fuel_scoops);
        const hasProcessor = Number(ship.spec.fuel_processor_tons ?? 0) > 0;

        const isDocked = Boolean(ship.is_docked);
        const daysDocked =
          isDocked &&
          ship.docked_at_total_minutes != null &&
          snapshot.current_total_minutes != null
            ? Math.max(
                0,
                Math.floor((snapshot.current_total_minutes - ship.docked_at_total_minutes) / 1440)
              )
            : 0;
        const prepaidDaysLeft = Math.max(0, 6 - daysDocked);
        const extraDayRate = berthingDailyRate(starport);
        const baseMult = berthingBaseMult(starport);

        const canRefuelRefined =
          isDocked &&
          (starport === "A" || starport === "B") &&
          freeFuel > 0.1 &&
          companyAvailable >= refinedCost;
        const canRefuelUnref =
          isDocked &&
          ["A", "B", "C", "D"].includes(starport) &&
          freeFuel > 0.1 &&
          companyAvailable >= unrefCost;
        const canSkim = !isDocked && gasGiants > 0 && hasScoops && freeFuel > 0.1;
        const canProcess = hasProcessor && ship.fuel_unrefined > 0.1;
        const canDock = !isDocked && ["A", "B", "C", "D", "E"].includes(starport);
        const canOpenShipyard = isDocked && ["A", "B", "C"].includes(starport);

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
                <div className="text-[11px] text-slate-400 flex flex-wrap items-center gap-1.5">
                  <span>
                    {ship.spec.tonnage}t · TL{ship.spec.tl} · J-{ship.spec.jump_rating} / M-
                    {ship.spec.thrust}
                  </span>
                  {isDocked ? (
                    <span className="px-1.5 py-0.5 rounded bg-emerald-950/80 border border-emerald-700/70 text-emerald-300 text-[10px] font-medium">
                      Docked · {ship.docked_system_name} ({daysDocked}d)
                    </span>
                  ) : (
                    <span className="px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700 text-cyan-300 text-[10px] font-medium">
                      In Orbit · {ship.docked_system_name}
                    </span>
                  )}
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
                  disabled={!canOpenShipyard}
                  title={
                    !isDocked
                      ? `Dock at ${snapshot.campaign.current_system_name} Starport first to access the Shipyard`
                      : !["A", "B", "C"].includes(starport)
                      ? `Starport Class ${starport} has no shipyard or repair facilities (requires Class A–C)`
                      : "Open High Guard 2024 Shipyard Refit Spreadsheet"
                  }
                  onClick={() => setSpreadsheetShip(ship)}
                  className="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-35 disabled:cursor-not-allowed border border-slate-700 text-[11px] flex items-center gap-1 transition-colors"
                >
                  <Wrench className="w-3 h-3 text-amber-400" /> Shipyard
                </button>
              </div>
            </div>

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
                        !isDocked
                          ? "Dock at Starport first to purchase refined fuel"
                          : starport !== "A" && starport !== "B"
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
                        !isDocked
                          ? "Dock at Starport first to purchase commercial fuel"
                          : !["A", "B", "C", "D"].includes(starport)
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
                        isDocked
                          ? "Undock from Starport before skimming a gas giant"
                          : gasGiants <= 0
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
                  {isDocked ? (
                    <div className="flex items-center justify-between gap-2 bg-emerald-950/40 border border-emerald-800/70 rounded px-2 py-1.5 text-[10px]">
                      <div className="text-emerald-200 flex items-center gap-1.5 min-w-0">
                        <Anchor className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                        <span className="truncate">
                          <strong>Docked at {ship.docked_system_name} Starport {starport}</strong> ·{" "}
                          {daysDocked}d stayed ·{" "}
                          {extraDayRate === 0
                            ? "Cr0/day"
                            : prepaidDaysLeft > 0
                            ? `${prepaidDaysLeft}d prepaid left, then ${formatCr(extraDayRate)}/d`
                            : `${formatCr(extraDayRate)}/day accruing`}
                        </span>
                      </div>
                      <button
                        onClick={() => handleUndock(ship.id)}
                        title="Undock from starport into orbit (stops daily berthing fees)"
                        className="shrink-0 px-2.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 border border-slate-600 text-slate-100 font-medium transition-colors"
                      >
                        Undock
                      </button>
                    </div>
                  ) : (
                    <button
                      disabled={!canDock}
                      title={
                        !canDock
                          ? `Starport Class ${starport} has no starport facilities to dock at`
                          : baseMult > 0
                          ? `Dock at ${snapshot.campaign.current_system_name} Starport ${starport} (1D × ${formatCr(
                              baseMult
                            )} covers first 6 days, then ${formatCr(extraDayRate)}/day)`
                          : `Dock at Starport Class ${starport} (Cr0 berthing fee)`
                      }
                      onClick={() => handleDock(ship.id)}
                      className="w-full bg-amber-900/50 hover:bg-amber-800/70 disabled:opacity-35 disabled:cursor-not-allowed border border-amber-700/70 text-amber-100 rounded py-1.5 text-[11px] font-medium flex items-center justify-center gap-1.5 transition-colors"
                    >
                      <Anchor className="w-3.5 h-3.5 text-amber-400" />
                      {!canDock
                        ? `Starport ${starport} — Cannot Dock`
                        : baseMult > 0
                        ? `Dock at Starport ${starport} (1D×${formatCr(baseMult)} · 6d, then ${formatCr(
                            extraDayRate
                          )}/d)`
                        : `Dock at Starport ${starport} (Cr0)`}
                    </button>
                  )}
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
          </div>
        );
      })}
      {spreadsheetShip && (
        <ShipSpreadsheetModal
          campaignId={campaignId}
          ship={spreadsheetShip}
          starport={starport}
          worldTl={worldTl}
          companyAvailable={companyAvailable}
          onClose={() => setSpreadsheetShip(null)}
          onApplied={async (msg) => {
            setPortMsg(msg);
            await onRefresh();
          }}
          onError={onError}
        />
      )}
    </div>
  );
}

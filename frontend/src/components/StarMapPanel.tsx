import { useEffect, useState, useMemo } from "react";
import {
  Compass,
  Crosshair,
  Navigation,
  Radar,
  Radio,
  RefreshCw,
  Search,
  Sparkles,
} from "lucide-react";
import { api, CampaignSnapshot, SectorRecord, SystemRecord, SystemShipRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
  onSendChat: (prompt: string) => Promise<void>;
  onSwitchToCombat: () => void;
}

export function StarMapPanel({
  campaignId,
  snapshot,
  onRefresh,
  onError,
  onSendChat,
  onSwitchToCombat,
}: Props) {
  const [sectors, setSectors] = useState<SectorRecord[]>([]);
  const [selectedSectorId, setSelectedSectorId] = useState<string>("");
  const [systems, setSystems] = useState<SystemRecord[]>([]);
  const [selectedSystem, setSelectedSystem] = useState<SystemRecord | null>(null);
  const [searchQ, setSearchQ] = useState("");
  const [searchTradeCode, setSearchTradeCode] = useState("");
  const [searchResults, setSearchResults] = useState<SystemRecord[]>([]);
  const [routeDest, setRouteDest] = useState("");
  const [jumpRating, setJumpRating] = useState(
    snapshot.fleet_max_jump || snapshot.ships[0]?.spec.jump_rating || 1
  );
  useEffect(() => {
    if (snapshot.fleet_max_jump && snapshot.fleet_max_jump > 0) {
      setJumpRating(snapshot.fleet_max_jump);
    }
  }, [snapshot.fleet_max_jump]);
  const [avoidRed, setAvoidRed] = useState(true);
  const [requireRefuel, setRequireRefuel] = useState(true);
  const [routeResult, setRouteResult] = useState<Awaited<ReturnType<typeof api.planRoute>> | null>(
    null
  );
  const [jumping, setJumping] = useState(false);
  const [jumpReport, setJumpReport] = useState<string | null>(null);
  const [scanningTraffic, setScanningTraffic] = useState(false);
  const [vesselScanMap, setVesselScanMap] = useState<Record<string, string>>({});

  useEffect(() => {
    api
      .listSectors()
      .then((res) => {
        setSectors(res.sectors);
        const spin =
          res.sectors.find((s) => s.name.toLowerCase() === "spinward marches") ||
          res.sectors.find((s) => s.name.toLowerCase().includes("spinward")) ||
          res.sectors[0];
        if (spin) setSelectedSectorId(spin.id);
      })
      .catch((e) => onError((e as Error).message));
  }, [onError]);

  useEffect(() => {
    if (!selectedSectorId) return;
    api
      .listSectorSystems(selectedSectorId)
      .then((res) => {
        setSystems(res.systems);
        const cur = res.systems.find(
          (s) => s.name.toLowerCase() === snapshot.campaign.current_system_name.toLowerCase()
        );
        if (cur) setSelectedSystem(cur);
      })
      .catch((e) => onError((e as Error).message));
  }, [selectedSectorId, snapshot.campaign.current_system_name, onError]);

  const routeHexSet = useMemo(() => {
    const m = new Set<string>();
    if (routeResult?.route) {
      for (const leg of routeResult.route.legs) {
        m.add(leg.from.hex);
        m.add(leg.to.hex);
      }
    }
    return m;
  }, [routeResult]);

  async function handleSearch() {
    try {
      const res = await api.searchSystems(searchQ, searchTradeCode || undefined, 40);
      setSearchResults(res.systems);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handlePlanRoute(targetName?: string) {
    const dest = (targetName ?? routeDest).trim();
    if (!dest) return;
    setRouteDest(dest);
    try {
      const res = await api.planRoute(
        snapshot.campaign.current_system_name,
        dest,
        jumpRating,
        avoidRed,
        requireRefuel
      );
      setRouteResult(res);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleJump(destName: string) {
    if (!destName.trim()) return;
    setJumping(true);
    setJumpReport(null);
    setVesselScanMap({});
    try {
      const res = await api.executeJump(campaignId, destName, snapshot.ships[0]?.id);
      const count = Number(res.in_system_ship_count ?? 0);
      const summary = String(res.traffic_summary ?? `${count} ship(s) detected in-system.`);
      setJumpReport(`Arrived at ${res.arrived_at ?? destName} [${res.uwp ?? ""}]. ${summary}`);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setJumping(false);
    }
  }

  async function handleRefreshTraffic() {
    setScanningTraffic(true);
    setVesselScanMap({});
    try {
      const res = await api.scanTraffic(campaignId);
      setJumpReport(res.summary);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setScanningTraffic(false);
    }
  }

  async function handleHailVessel(v: SystemShipRecord) {
    try {
      await api.interactVessel(campaignId, v.id, {
        action: "hail",
        message: `Hailing ${v.name} (${v.ship_class}) in ${v.orbit_location}.`,
      });
      await onRefresh();
      await onSendChat(
        `I open hailing frequencies to ${v.name} (${v.ship_class}, captain ${v.captain_name}, ${v.faction}) currently at ${v.orbit_location} in ${snapshot.campaign.current_system_name}. "Vessel ${v.name}, this is ${snapshot.ships[0]?.name ?? "our ship"}, requesting comms — what's your status and business in system?"`
      );
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleScanVessel(v: SystemShipRecord) {
    try {
      const res = await api.interactVessel(campaignId, v.id, { action: "sensor_scan" });
      const det = res.sensor_detection as {
        detection_level?: string;
        roll?: { total?: number; target?: number; effect?: number; success?: boolean };
      };
      const rev = res.revealed as Record<string, unknown>;
      const txt = det?.roll?.success
        ? `Scan ${det.detection_level?.toUpperCase()} (rolled ${det.roll.total}/${det.roll.target}, Eff +${det.roll.effect}): Hull ${rev?.hull ?? "?"}, Armor ${rev?.armor ?? "?"}, M-${rev?.thrust ?? "?"}, Stealth ${rev?.stealth_dm ?? 0}. ${rev?.cargo_hint ?? ""}`
        : `Scan FAILED (rolled ${det?.roll?.total ?? "?"}/${det?.roll?.target ?? "?"}) — ${rev?.note ?? "signature masked."}`;
      setVesselScanMap((prev) => ({ ...prev, [v.id]: txt }));
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleEngageVessel(v: SystemShipRecord) {
    try {
      await api.interactVessel(campaignId, v.id, { action: "engage", range_band: "long" });
      await onRefresh();
      onSwitchToCombat();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const systemShips = snapshot.system_ships ?? [];

  // SVG Hex coordinates for a 32x40 Traveller sector
  const hexSize = 11;
  const colStep = hexSize * 1.5;
  const rowStep = hexSize * Math.sqrt(3);

  function hexCenter(hex: string): { x: number; y: number } | null {
    if (hex.length !== 4) return null;
    const col = parseInt(hex.slice(0, 2), 10);
    const row = parseInt(hex.slice(2, 4), 10);
    if (Number.isNaN(col) || Number.isNaN(row)) return null;
    const x = 18 + (col - 1) * colStep;
    const y = 18 + (row - 1) * rowStep + (col % 2 === 0 ? rowStep / 2 : 0);
    return { x, y };
  }

  return (
    <div className="space-y-3 text-xs">
      <div className="flex items-center justify-between gap-2 bg-slate-900/90 border border-slate-800 rounded p-2">
        <div className="flex items-center gap-2 min-w-0">
          <Compass className="w-4 h-4 text-cyan-400 shrink-0" />
          <div className="truncate">
            <div className="text-slate-200 font-semibold truncate">
              {snapshot.campaign.current_system_name}{" "}
              <span className="font-mono text-cyan-400">
                [{snapshot.campaign.current_system_uwp}]
              </span>
            </div>
            <div className="text-[10px] text-slate-400">
              {snapshot.pack_stats?.sectors ?? sectors.length} sectors ·{" "}
              <span className="text-amber-300 font-semibold">
                {systemShips.length} ship(s) in system
              </span>
            </div>
          </div>
        </div>
        <select
          className="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-[11px] max-w-[170px]"
          value={selectedSectorId}
          onChange={(e) => setSelectedSectorId(e.target.value)}
        >
          {sectors.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name} ({s.system_count})
            </option>
          ))}
        </select>
      </div>

      {jumpReport && (
        <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200 text-[11px]">
          {jumpReport}
        </div>
      )}

      {/* In-System Starship Traffic & Ship Interactions */}
      <div className="bg-slate-900 border border-cyan-900/70 rounded p-2.5 space-y-2">
        <div className="flex items-center justify-between">
          <div className="font-semibold text-cyan-300 flex items-center gap-1.5">
            <Radar className="w-3.5 h-3.5" /> Ships in {snapshot.campaign.current_system_name} (
            {systemShips.length} detected)
          </div>
          <button
            disabled={scanningTraffic}
            onClick={handleRefreshTraffic}
            className="bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded px-2 py-0.5 text-[10px] flex items-center gap-1"
          >
            <RefreshCw className={`w-3 h-3 ${scanningTraffic ? "animate-spin" : ""}`} /> Sweep
          </button>
        </div>

        {systemShips.length === 0 ? (
          <div className="text-[11px] text-slate-500">
            Sensors report empty space (0 vessels in system).
          </div>
        ) : (
          <div className="max-h-56 overflow-y-auto space-y-1.5">
            {systemShips.map((v) => (
              <div
                key={v.id}
                className="bg-slate-950 border border-slate-800 rounded p-2 space-y-1"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="font-semibold text-slate-100 truncate">
                      {v.name}{" "}
                      <span className="text-[10px] font-normal text-slate-400">
                        ({v.ship_class}, {v.tonnage}t)
                      </span>
                    </div>
                    <div className="text-[10px] text-slate-400">
                      {v.orbit_location} · {v.faction} · {v.captain_name}
                    </div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    {!v.transponder_active && (
                      <span className="px-1 py-0.5 rounded bg-purple-950 text-purple-300 text-[9px] font-mono">
                        DARK
                      </span>
                    )}
                    <span
                      className={`px-1.5 py-0.5 rounded text-[9px] uppercase font-semibold ${
                        v.disposition === "hostile"
                          ? "bg-red-950 text-red-300"
                          : v.disposition === "distress"
                          ? "bg-amber-950 text-amber-300"
                          : v.disposition === "friendly"
                          ? "bg-emerald-950 text-emerald-300"
                          : "bg-slate-800 text-slate-300"
                      }`}
                    >
                      {v.disposition}
                    </span>
                  </div>
                </div>

                <div className="flex gap-1 pt-0.5">
                  <button
                    onClick={() => handleHailVessel(v)}
                    className="flex-1 bg-cyan-900/70 hover:bg-cyan-800 text-cyan-100 rounded py-0.5 text-[10px] flex items-center justify-center gap-1"
                  >
                    <Radio className="w-3 h-3" /> Hail / Talk
                  </button>
                  <button
                    onClick={() => handleScanVessel(v)}
                    className="flex-1 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded py-0.5 text-[10px] flex items-center justify-center gap-1"
                  >
                    <Radar className="w-3 h-3" /> Sensor Scan
                  </button>
                  <button
                    onClick={() => handleEngageVessel(v)}
                    className="bg-red-900/70 hover:bg-red-800 text-red-100 rounded px-2 py-0.5 text-[10px] flex items-center justify-center gap-1"
                  >
                    <Crosshair className="w-3 h-3" /> Engage
                  </button>
                </div>

                {vesselScanMap[v.id] && (
                  <div className="text-[10px] bg-slate-900 border border-cyan-900/60 rounded px-1.5 py-1 text-cyan-200 font-mono">
                    {vesselScanMap[v.id]}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Global Search */}
      <div className="bg-slate-900/80 border border-slate-800 rounded p-2 space-y-2">
        <div className="flex gap-1.5">
          <input
            className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
            placeholder="Search any world or sector (e.g. Regina, Mora, Rhylanor)…"
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
          <select
            className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
            value={searchTradeCode}
            onChange={(e) => setSearchTradeCode(e.target.value)}
          >
            <option value="">Any Code</option>
            {["Hi", "Ht", "In", "Ri", "Ag", "Ga", "Wa", "As", "De", "Ni", "Po", "Lo"].map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
          <button
            onClick={handleSearch}
            className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2.5 py-1 flex items-center gap-1"
          >
            <Search className="w-3 h-3" /> Find
          </button>
        </div>

        {searchResults.length > 0 && (
          <div className="max-h-32 overflow-y-auto divide-y divide-slate-800 border border-slate-800 rounded bg-slate-950">
            {searchResults.map((sys) => (
              <div
                key={sys.id}
                className="flex items-center justify-between px-2 py-1 hover:bg-slate-900 cursor-pointer"
                onClick={() => {
                  setSelectedSectorId(sys.sector_id);
                  setSelectedSystem(sys);
                  setRouteDest(sys.name);
                }}
              >
                <div className="truncate">
                  <span className="font-medium text-slate-200">{sys.name}</span>{" "}
                  <span className="text-slate-500">
                    ({sys.sector_name} {sys.hex})
                  </span>
                </div>
                <div className="flex items-center gap-1.5 shrink-0 font-mono text-[10px]">
                  <span className="text-cyan-400">{sys.uwp}</span>
                  <span className="text-amber-300">{sys.trade_codes.join(" ")}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Interactive Sector SVG Hex Map */}
      <div className="bg-slate-950 border border-slate-800 rounded overflow-auto max-h-60 relative">
        <svg width={32 * colStep + 36} height={40 * rowStep + 36} className="block select-none">
          {[8, 16, 24].map((c) => (
            <line
              key={`vc-${c}`}
              x1={18 + (c - 0.5) * colStep}
              y1={0}
              x2={18 + (c - 0.5) * colStep}
              y2={40 * rowStep + 36}
              stroke="#1e293b"
              strokeDasharray="3 3"
            />
          ))}
          {[10, 20, 30].map((r) => (
            <line
              key={`hr-${r}`}
              x1={0}
              y1={18 + (r - 0.5) * rowStep}
              x2={32 * colStep + 36}
              y2={18 + (r - 0.5) * rowStep}
              stroke="#1e293b"
              strokeDasharray="3 3"
            />
          ))}

          {routeResult?.route?.legs.map((leg, i) => {
            const p1 = hexCenter(leg.from.hex);
            const p2 = hexCenter(leg.to.hex);
            if (!p1 || !p2) return null;
            return (
              <line
                key={i}
                x1={p1.x}
                y1={p1.y}
                x2={p2.x}
                y2={p2.y}
                stroke="#22d3ee"
                strokeWidth={2}
                strokeDasharray="4 2"
              />
            );
          })}

          {systems.map((sys) => {
            const pt = hexCenter(sys.hex);
            if (!pt) return null;
            const isCurrent =
              sys.name.toLowerCase() === snapshot.campaign.current_system_name.toLowerCase();
            const isSel = selectedSystem?.id === sys.id;
            const inRoute = routeHexSet.has(sys.hex);
            const fill =
              sys.starport === "A" || sys.starport === "B"
                ? "#38bdf8"
                : sys.starport === "C" || sys.starport === "D"
                ? "#cbd5e1"
                : "#64748b";
            return (
              <g
                key={sys.id}
                transform={`translate(${pt.x},${pt.y})`}
                className="cursor-pointer"
                onClick={() => {
                  setSelectedSystem(sys);
                  setRouteDest(sys.name);
                }}
              >
                {sys.travel_zone === "Red" && (
                  <circle r={8} fill="none" stroke="#ef4444" strokeWidth={1.5} />
                )}
                {sys.travel_zone === "Amber" && (
                  <circle r={8} fill="none" stroke="#f59e0b" strokeWidth={1.2} />
                )}
                {(isCurrent || isSel || inRoute) && (
                  <circle
                    r={isCurrent ? 9.5 : 7}
                    fill="none"
                    stroke={isCurrent ? "#10b981" : "#22d3ee"}
                    strokeWidth={1.6}
                  />
                )}
                <circle r={3.2} fill={fill} />
                {sys.gas_giants > 0 && <circle cx={4.5} cy={-4.5} r={1.3} fill="#fbbf24" />}
                <text
                  y={10}
                  textAnchor="middle"
                  fill={isCurrent ? "#34d399" : isSel ? "#67e8f9" : "#94a3b8"}
                  fontSize={6.5}
                >
                  {sys.name.slice(0, 9)}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {/* Selected System Inspector */}
      {selectedSystem && (
        <div className="bg-slate-900 border border-slate-700 rounded p-2.5 space-y-1.5">
          <div className="flex items-center justify-between">
            <div>
              <span className="font-bold text-sm text-cyan-300">{selectedSystem.name}</span>{" "}
              <span className="font-mono text-slate-400">
                {selectedSystem.sector_name} {selectedSystem.hex}
              </span>
            </div>
            <span className="font-mono bg-slate-950 border border-slate-700 px-1.5 py-0.5 rounded text-cyan-300">
              {selectedSystem.uwp}
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5 text-[10px]">
            <span className="px-1.5 py-0.5 rounded bg-slate-800 text-slate-300">
              Port {selectedSystem.starport}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-slate-800 text-slate-300">
              TL {selectedSystem.tech_level}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-slate-800 text-slate-300">
              Law {selectedSystem.law_level}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-slate-800 text-amber-300">
              GG: {selectedSystem.gas_giants}
            </span>
            {selectedSystem.trade_codes.map((tc) => (
              <span key={tc} className="px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-300">
                {tc}
              </span>
            ))}
          </div>
          <div className="flex gap-2 pt-1">
            <button
              onClick={() => handlePlanRoute(selectedSystem.name)}
              className="flex-1 bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded py-1 flex items-center justify-center gap-1"
            >
              <Navigation className="w-3 h-3 text-cyan-400" /> Plot Route
            </button>
            <button
              disabled={jumping}
              onClick={() => handleJump(selectedSystem.name)}
              className="flex-1 bg-cyan-700 hover:bg-cyan-600 disabled:opacity-50 text-white rounded py-1 flex items-center justify-center gap-1 font-medium"
            >
              <Sparkles className="w-3 h-3" /> {jumping ? "Jumping…" : "Execute Jump"}
            </button>
          </div>
        </div>
      )}

      {/* A* Jump Route Planner */}
      <div className="bg-slate-900/80 border border-slate-800 rounded p-2.5 space-y-2">
        <div className="font-semibold text-slate-300">Astrogation Jump Route Planner (A*)</div>
        <div className="flex gap-1.5 items-center">
          <input
            className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1 text-xs"
            placeholder="Destination system name…"
            value={routeDest}
            onChange={(e) => setRouteDest(e.target.value)}
          />
          <label className="text-[11px] text-slate-400">
            J-
            <input
              type="number"
              min={1}
              max={6}
              className="w-10 ml-0.5 bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center"
              value={jumpRating}
              onChange={(e) => setJumpRating(Number(e.target.value) || 1)}
            />
          </label>
          <button
            onClick={() => handlePlanRoute()}
            className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2.5 py-1"
          >
            Plot
          </button>
        </div>
        <div className="flex gap-3 text-[11px] text-slate-400">
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={avoidRed} onChange={(e) => setAvoidRed(e.target.checked)} />
            Avoid Red Zones
          </label>
          <label className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={requireRefuel}
              onChange={(e) => setRequireRefuel(e.target.checked)}
            />
            Require Refuel Stops
          </label>
        </div>

        {routeResult && (
          <div className="bg-slate-950 border border-slate-800 rounded p-2 space-y-1">
            {!routeResult.found || !routeResult.route ? (
              <div className="text-amber-400">
                No valid Jump-{routeResult.jump_rating} route found under current constraints.
              </div>
            ) : (
              <>
                <div className="text-cyan-300 font-semibold">
                  {routeResult.route.total_jumps} jump(s) · {routeResult.route.total_parsecs}{" "}
                  parsec(s)
                </div>
                {routeResult.route.legs.map((leg, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between text-[11px] border-t border-slate-800 pt-1"
                  >
                    <span>
                      {i + 1}. {leg.from.name} →{" "}
                      <span className="text-slate-100 font-medium">{leg.to.name}</span>{" "}
                      <span className="text-slate-500">({leg.parsecs}pc)</span>
                    </span>
                    <button
                      disabled={jumping}
                      onClick={() => handleJump(leg.to.name)}
                      className="text-cyan-400 hover:text-cyan-300 underline"
                    >
                      Jump Leg
                    </button>
                  </div>
                ))}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

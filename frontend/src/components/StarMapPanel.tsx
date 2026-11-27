import { useEffect, useState, useMemo } from "react";
import { Compass, Navigation, Search, Sparkles } from "lucide-react";
import { api, CampaignSnapshot, SectorRecord, SystemRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function StarMapPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [sectors, setSectors] = useState<SectorRecord[]>([]);
  const [selectedSectorId, setSelectedSectorId] = useState<string>("");
  const [systems, setSystems] = useState<SystemRecord[]>([]);
  const [selectedSystem, setSelectedSystem] = useState<SystemRecord | null>(null);
  const [searchQ, setSearchQ] = useState("");
  const [searchTradeCode, setSearchTradeCode] = useState("");
  const [searchResults, setSearchResults] = useState<SystemRecord[]>([]);
  const [routeDest, setRouteDest] = useState("");
  const [jumpRating, setJumpRating] = useState(snapshot.ships[0]?.spec.jump_rating || 1);
  const [avoidRed, setAvoidRed] = useState(true);
  const [requireRefuel, setRequireRefuel] = useState(true);
  const [routeResult, setRouteResult] = useState<Awaited<ReturnType<typeof api.planRoute>> | null>(null);
  const [jumping, setJumping] = useState(false);
  const [jumpReport, setJumpReport] = useState<string | null>(null);

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
    try {
      const res = await api.executeJump(campaignId, destName, snapshot.ships[0]?.id);
      setJumpReport(
        `Jumped to ${res.arrived_at ?? destName} (${res.uwp ?? ""}) in ${res.jump_hours ?? "?"}h. Date: ${res.new_date ?? ""}`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setJumping(false);
    }
  }

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
              <span className="font-mono text-cyan-400">[{snapshot.campaign.current_system_uwp}]</span>
            </div>
            <div className="text-[10px] text-slate-400">
              {snapshot.pack_stats?.sectors ?? sectors.length} sectors loaded
            </div>
          </div>
        </div>
        <select
          className="bg-slate-950 border border-slate-700 rounded px-2 py-1 text-[11px] max-w-[180px]"
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
          <div className="max-h-36 overflow-y-auto divide-y divide-slate-800 border border-slate-800 rounded bg-slate-950">
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
      <div className="bg-slate-950 border border-slate-800 rounded overflow-auto max-h-72 relative">
        <svg
          width={32 * colStep + 36}
          height={40 * rowStep + 36}
          className="block select-none"
        >
          {/* Subsector divider lines (4x4 subsectors, 8x10 hexes each) */}
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

          {/* Route polyline */}
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

          {/* Star systems */}
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
            {selectedSystem.travel_zone !== "Green" && (
              <span
                className={`px-1.5 py-0.5 rounded font-semibold ${
                  selectedSystem.travel_zone === "Red"
                    ? "bg-red-900/70 text-red-200"
                    : "bg-amber-900/70 text-amber-200"
                }`}
              >
                {selectedSystem.travel_zone} Zone
              </span>
            )}
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

        {jumpReport && (
          <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200 text-[11px]">
            {jumpReport}
          </div>
        )}

        {routeResult && (
          <div className="bg-slate-950 border border-slate-800 rounded p-2 space-y-1">
            {!routeResult.found || !routeResult.route ? (
              <div className="text-amber-400">
                No valid Jump-{routeResult.jump_rating} route found under current constraints.
              </div>
            ) : (
              <>
                <div className="text-cyan-300 font-semibold">
                  {routeResult.route.total_jumps} jump(s) · {routeResult.route.total_parsecs} parsec(s)
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

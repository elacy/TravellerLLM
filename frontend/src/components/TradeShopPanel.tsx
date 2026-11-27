import { useEffect, useState } from "react";
import { AlertTriangle, Package, Search, ShoppingBag, Store, TrendingUp } from "lucide-react";
import { api, CampaignSnapshot, formatCr, ItemRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

const CATEGORIES = [
  "",
  "weapon",
  "armor",
  "augment",
  "software",
  "electronics",
  "medical",
  "vehicle",
  "robot_drone",
  "equipment",
];

export function TradeShopPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [subTab, setSubTab] = useState<"trade" | "shop">("trade");

  // Trade state
  const [destSystem, setDestSystem] = useState("Efate");
  const [parsecs, setParsecs] = useState(1);
  const [brokerSkill, setBrokerSkill] = useState(1);
  const [stewardSkill, setStewardSkill] = useState(0);
  const [includeIllegal, setIncludeIllegal] = useState(false);
  const [market, setMarket] = useState<Record<string, unknown> | null>(null);
  const [tradeMsg, setTradeMsg] = useState<string | null>(null);
  const [buyTons, setBuyTons] = useState<Record<number, number>>({});

  // Shop state
  const [itemQ, setItemQ] = useState("");
  const [category, setCategory] = useState("");
  const [maxTl, setMaxTl] = useState<string>("");
  const [items, setItems] = useState<ItemRecord[]>([]);
  const [blackMarket, setBlackMarket] = useState(false);
  const [priceMult, setPriceMult] = useState(1);
  const [skillDm, setSkillDm] = useState(1);
  const [selectedAccountId, setSelectedAccountId] = useState<string>(snapshot.accounts[0]?.id ?? "");
  const [selectedCharId, setSelectedCharId] = useState<string>(
    snapshot.characters[0]?.character.id ?? ""
  );
  const [availMap, setAvailMap] = useState<Record<string, Record<string, unknown>>>({});
  const [shopMsg, setShopMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedAccountId && snapshot.accounts[0]) setSelectedAccountId(snapshot.accounts[0].id);
    if (!selectedCharId && snapshot.characters[0]) setSelectedCharId(snapshot.characters[0].character.id);
  }, [snapshot.accounts, snapshot.characters, selectedAccountId, selectedCharId]);

  useEffect(() => {
    if (subTab === "shop" && items.length === 0) {
      loadItems();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [subTab]);

  async function loadItems() {
    try {
      const res = await api.searchItems(
        itemQ,
        category || undefined,
        maxTl ? Number(maxTl) : undefined,
        80
      );
      setItems(res.items);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function generateMarket() {
    setTradeMsg(null);
    try {
      const res = await api.tradeMarket(campaignId, {
        dest_system: destSystem,
        parsecs,
        broker_skill: brokerSkill,
        steward_skill: stewardSkill,
        include_illegal: includeIllegal,
      });
      setMarket(res);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleBuySpeculative(offer: {
    d66: number;
    name: string;
    unit_price_cr: number;
    tons_available: number;
  }) {
    const ship = snapshot.ships[0];
    if (!ship) {
      onError("No ship available to load cargo.");
      return;
    }
    const tons = Math.min(offer.tons_available, Math.max(1, buyTons[offer.d66] ?? 5));
    try {
      const res = await api.tradeBuy(campaignId, {
        ship_id: ship.id,
        d66: offer.d66,
        good_name: offer.name,
        tons,
        unit_price_cr: offer.unit_price_cr,
      });
      setTradeMsg(
        `Purchased ${tons}t ${offer.name} for ${formatCr(Number(res.total_cost_cr ?? tons * offer.unit_price_cr))}.`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleSellCargo(lot: { d66?: number; name: string; tons: number }) {
    const ship = snapshot.ships[0];
    if (!ship) return;
    try {
      const res = await api.tradeSell(campaignId, {
        ship_id: ship.id,
        d66: lot.d66 ?? 11,
        good_name: lot.name,
        tons: lot.tons,
        broker_skill: brokerSkill,
      });
      setTradeMsg(
        `Sold ${lot.tons}t ${lot.name} at ${res.sale_percent ?? "?"}% for ${formatCr(
          Number(res.total_revenue_cr ?? 0)
        )}!`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function checkAvailability(item: ItemRecord) {
    setShopMsg(null);
    try {
      const res = await api.shopAvailability(campaignId, {
        item_id: item.id,
        black_market: blackMarket,
        willing_price_multiplier: priceMult,
        skill_dm: skillDm,
        stat_dm: 1,
      });
      setAvailMap((prev) => ({ ...prev, [item.id]: res }));
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function purchaseItem(item: ItemRecord) {
    if (!selectedAccountId) {
      onError("Select an account first.");
      return;
    }
    const avail = availMap[item.id] as { availability?: { final_unit_price_cr?: number } } | undefined;
    const unitPrice = avail?.availability?.final_unit_price_cr ?? item.cost_cr;
    try {
      const res = await api.shopPurchase(campaignId, {
        item_id: item.id,
        account_id: selectedAccountId,
        character_id: selectedCharId || undefined,
        quantity: 1,
        unit_price_cr: unitPrice,
      });
      setShopMsg(
        `Purchased ${res.purchased ?? item.name} for ${formatCr(Number(res.total_cost_cr ?? unitPrice))}.`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const ship = snapshot.ships[0];
  const freightLots = (market?.freight as Array<{ lot_type: string; tons: number; payout_cr: number }>) || [];
  const pax = market?.passengers as
    | {
        high: number;
        middle: number;
        basic: number;
        low: number;
        rates: { high_passage: number; middle_passage: number; basic_passage: number; low_passage: number };
      }
    | undefined;
  const specGoods =
    (market?.speculative_goods as Array<{
      d66: number;
      name: string;
      tons_available: number;
      price_percent: number;
      unit_price_cr: number;
      illegal: boolean;
    }>) || [];

  return (
    <div className="space-y-3 text-xs">
      <div className="flex gap-1.5">
        <button
          onClick={() => setSubTab("trade")}
          className={`flex-1 py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "trade"
              ? "bg-cyan-950 border-cyan-600 text-cyan-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <TrendingUp className="w-3.5 h-3.5" /> Ship Trade & Cargo
        </button>
        <button
          onClick={() => setSubTab("shop")}
          className={`flex-1 py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "shop"
              ? "bg-amber-950 border-amber-600 text-amber-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <ShoppingBag className="w-3.5 h-3.5" /> Item Market ({snapshot.pack_stats?.items ?? items.length})
        </button>
      </div>

      {subTab === "trade" ? (
        <div className="space-y-2.5">
          {/* Market Generator Controls */}
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-2">
            <div className="font-semibold text-cyan-300 flex items-center gap-1">
              <Store className="w-3.5 h-3.5" /> {snapshot.campaign.current_system_name} Starport Trade Exchange
            </div>
            <div className="grid grid-cols-2 gap-1.5">
              <label className="block">
                <span className="text-[10px] text-slate-400">Destination World</span>
                <input
                  className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1"
                  value={destSystem}
                  onChange={(e) => setDestSystem(e.target.value)}
                />
              </label>
              <div className="grid grid-cols-3 gap-1">
                <label className="block">
                  <span className="text-[10px] text-slate-400">Parsecs</span>
                  <input
                    type="number"
                    min={1}
                    max={6}
                    className="w-full bg-slate-950 border border-slate-700 rounded px-1 py-1 text-center"
                    value={parsecs}
                    onChange={(e) => setParsecs(Number(e.target.value) || 1)}
                  />
                </label>
                <label className="block">
                  <span className="text-[10px] text-slate-400">Broker</span>
                  <input
                    type="number"
                    className="w-full bg-slate-950 border border-slate-700 rounded px-1 py-1 text-center"
                    value={brokerSkill}
                    onChange={(e) => setBrokerSkill(Number(e.target.value))}
                  />
                </label>
                <label className="block">
                  <span className="text-[10px] text-slate-400">Steward</span>
                  <input
                    type="number"
                    className="w-full bg-slate-950 border border-slate-700 rounded px-1 py-1 text-center"
                    value={stewardSkill}
                    onChange={(e) => setStewardSkill(Number(e.target.value))}
                  />
                </label>
              </div>
            </div>
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-1 text-[11px] text-amber-300">
                <input
                  type="checkbox"
                  checked={includeIllegal}
                  onChange={(e) => setIncludeIllegal(e.target.checked)}
                />
                Include Black Market Speculative Goods
              </label>
              <button
                onClick={generateMarket}
                className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-3 py-1 font-medium"
              >
                Roll World Market
              </button>
            </div>
          </div>

          {tradeMsg && (
            <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200">
              {tradeMsg}
            </div>
          )}

          {/* Ship Cargo Hold (Sell on Current World) */}
          {ship && (ship.cargo || []).length > 0 && (
            <div className="bg-slate-900 border border-amber-800/60 rounded p-2.5 space-y-1.5">
              <div className="font-semibold text-amber-300 flex items-center gap-1">
                <Package className="w-3.5 h-3.5" /> {ship.name} Cargo Hold (Sell on{" "}
                {snapshot.campaign.current_system_name})
              </div>
              {ship.cargo.map((lot, i) => (
                <div
                  key={i}
                  className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded px-2 py-1"
                >
                  <div>
                    <span className="font-medium text-slate-200">{lot.name}</span>{" "}
                    <span className="text-slate-400">
                      ({lot.tons}t · paid {lot.purchase_price_cr ? formatCr(lot.purchase_price_cr) : "?"}/t)
                    </span>
                  </div>
                  <button
                    onClick={() => handleSellCargo(lot)}
                    className="bg-emerald-700 hover:bg-emerald-600 text-white rounded px-2 py-0.5 text-[11px]"
                  >
                    Sell Lot
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Market Results */}
          {market && (
            <div className="space-y-2.5">
              {pax && (
                <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1">
                  <div className="font-semibold text-slate-200 flex justify-between">
                    <span>Passengers & Mail → {destSystem}</span>
                    {Boolean(market.mail_available) && (
                      <span className="text-emerald-300 font-mono">Mail: Cr25,000 (5t)</span>
                    )}
                  </div>
                  <div className="grid grid-cols-4 gap-1 text-center font-mono text-[10px]">
                    <div className="bg-slate-950 rounded p-1 border border-slate-800">
                      <div className="text-slate-400">HIGH ({pax.high})</div>
                      <div className="text-cyan-300">{formatCr(pax.rates.high_passage)}</div>
                    </div>
                    <div className="bg-slate-950 rounded p-1 border border-slate-800">
                      <div className="text-slate-400">MID ({pax.middle})</div>
                      <div className="text-cyan-300">{formatCr(pax.rates.middle_passage)}</div>
                    </div>
                    <div className="bg-slate-950 rounded p-1 border border-slate-800">
                      <div className="text-slate-400">BASIC ({pax.basic})</div>
                      <div className="text-cyan-300">{formatCr(pax.rates.basic_passage)}</div>
                    </div>
                    <div className="bg-slate-950 rounded p-1 border border-slate-800">
                      <div className="text-slate-400">LOW ({pax.low})</div>
                      <div className="text-cyan-300">{formatCr(pax.rates.low_passage)}</div>
                    </div>
                  </div>
                </div>
              )}

              {freightLots.length > 0 && (
                <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1">
                  <div className="font-semibold text-slate-200">
                    Freight Contracts ({freightLots.length} lots)
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {freightLots.map((f, i) => (
                      <span
                        key={i}
                        className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-[10px]"
                      >
                        {f.lot_type} {f.tons}t →{" "}
                        <span className="text-emerald-300 font-mono">{formatCr(f.payout_cr)}</span>
                      </span>
                    ))}
                  </div>
                </div>
              )}

              <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5">
                <div className="font-semibold text-slate-200">
                  Speculative Trade Goods ({specGoods.length})
                </div>
                <div className="max-h-60 overflow-y-auto divide-y divide-slate-800">
                  {specGoods.map((g) => (
                    <div key={g.d66} className="py-1.5 flex items-center justify-between gap-2">
                      <div className="min-w-0">
                        <div className="font-medium text-slate-100 truncate">
                          {g.illegal && <span className="text-red-400 mr-1">[ILLEGAL]</span>}
                          {g.name}
                        </div>
                        <div className="text-[10px] text-slate-400 font-mono">
                          {g.tons_available}t avail · {formatCr(g.unit_price_cr)}/t ({g.price_percent}%)
                        </div>
                      </div>
                      <div className="flex items-center gap-1 shrink-0">
                        <input
                          type="number"
                          min={1}
                          max={g.tons_available}
                          className="w-12 bg-slate-950 border border-slate-700 rounded px-1 py-0.5 text-center font-mono"
                          value={buyTons[g.d66] ?? Math.min(5, g.tons_available)}
                          onChange={(e) =>
                            setBuyTons((p) => ({ ...p, [g.d66]: Number(e.target.value) || 1 }))
                          }
                        />
                        <button
                          onClick={() => handleBuySpeculative(g)}
                          className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-2 py-0.5"
                        >
                          Buy
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      ) : (
        /* CSC 2023 Equipment & Black Market Shop */
        <div className="space-y-2.5">
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-2">
            <div className="flex gap-1.5">
              <input
                className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1"
                placeholder="Search weapons, armor, augments, vehicles, robots…"
                value={itemQ}
                onChange={(e) => setItemQ(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && loadItems()}
              />
              <select
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 capitalize"
                value={category}
                onChange={(e) => setCategory(e.target.value)}
              >
                <option value="">All Types</option>
                {CATEGORIES.filter(Boolean).map((c) => (
                  <option key={c} value={c}>
                    {c.replace("_", " ")}
                  </option>
                ))}
              </select>
              <input
                type="number"
                placeholder="Max TL"
                className="w-16 bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-center"
                value={maxTl}
                onChange={(e) => setMaxTl(e.target.value)}
              />
              <button
                onClick={loadItems}
                className="bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded px-2.5 py-1 flex items-center gap-1"
              >
                <Search className="w-3 h-3" /> Search
              </button>
            </div>

            <div className="grid grid-cols-2 gap-1.5 text-[11px]">
              <select
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                value={selectedAccountId}
                onChange={(e) => setSelectedAccountId(e.target.value)}
              >
                {snapshot.accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    Pay from: {a.name} ({formatCr(a.balance_cr)})
                  </option>
                ))}
              </select>
              <select
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                value={selectedCharId}
                onChange={(e) => setSelectedCharId(e.target.value)}
              >
                <option value="">(Ship / Unassigned)</option>
                {snapshot.characters.map((r) => (
                  <option key={r.character.id} value={r.character.id}>
                    Give to: {r.character.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-slate-300">
              <label className="flex items-center gap-1 text-red-300">
                <input
                  type="checkbox"
                  checked={blackMarket}
                  onChange={(e) => setBlackMarket(e.target.checked)}
                />
                Black Market (Streetwise)
              </label>
              <label className="flex items-center gap-1">
                Skill DM:
                <input
                  type="number"
                  className="w-10 bg-slate-950 border border-slate-700 rounded px-1 text-center"
                  value={skillDm}
                  onChange={(e) => setSkillDm(Number(e.target.value))}
                />
              </label>
              {!blackMarket && (
                <label className="flex items-center gap-1">
                  Offer:
                  <select
                    className="bg-slate-950 border border-slate-700 rounded px-1"
                    value={priceMult}
                    onChange={(e) => setPriceMult(Number(e.target.value))}
                  >
                    <option value={1}>1x Price</option>
                    <option value={2}>2x (+1 DM)</option>
                    <option value={3}>3x (+2 DM)</option>
                  </select>
                </label>
              )}
            </div>
          </div>

          {shopMsg && (
            <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200">
              {shopMsg}
            </div>
          )}

          <div className="max-h-96 overflow-y-auto space-y-1.5">
            {items.map((item) => {
              const av = availMap[item.id] as
                | {
                    availability?: {
                      available: boolean;
                      final_unit_price_cr: number;
                      law_response: string;
                      note: string;
                    };
                  }
                | undefined;
              const statsSummary = Object.entries(item.stats || {})
                .filter(([k]) => !k.startsWith("source_"))
                .slice(0, 4)
                .map(([k, v]) => `${k}: ${v}`)
                .join(" · ");

              return (
                <div
                  key={item.id}
                  className="bg-slate-900 border border-slate-800 rounded p-2 space-y-1"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="font-semibold text-slate-100 truncate">
                        {item.name}{" "}
                        <span className="text-[10px] font-mono text-cyan-400">TL{item.tl}</span>
                      </div>
                      <div className="text-[10px] text-slate-400">
                        <span className="uppercase">{item.category}</span> · {formatCr(item.cost_cr)} ·{" "}
                        {item.mass_kg}kg · Law {item.law_restriction}
                      </div>
                      {statsSummary && (
                        <div className="text-[10px] text-slate-300 truncate">{statsSummary}</div>
                      )}
                    </div>
                    <div className="flex gap-1 shrink-0">
                      <button
                        onClick={() => checkAvailability(item)}
                        className="bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded px-2 py-1 text-[10px]"
                      >
                        Check Avail
                      </button>
                      <button
                        onClick={() => purchaseItem(item)}
                        className="bg-emerald-700 hover:bg-emerald-600 text-white rounded px-2 py-1 text-[10px]"
                      >
                        Buy
                      </button>
                    </div>
                  </div>
                  {av?.availability && (
                    <div
                      className={`text-[10px] px-2 py-1 rounded border flex items-center gap-1.5 ${
                        av.availability.available
                          ? "bg-emerald-950/50 border-emerald-700 text-emerald-200"
                          : av.availability.law_response !== "none"
                          ? "bg-red-950/70 border-red-700 text-red-200"
                          : "bg-slate-950 border-slate-700 text-slate-400"
                      }`}
                    >
                      {av.availability.law_response !== "none" && (
                        <AlertTriangle className="w-3.5 h-3.5 text-red-400 shrink-0" />
                      )}
                      <span>{av.availability.note}</span>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

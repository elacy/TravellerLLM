import { useState } from "react";
import { ArrowLeftRight, CalendarClock, CreditCard, Plus, UserCheck, Users } from "lucide-react";
import { api, CampaignSnapshot, CrewCandidate, formatCr } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

export function FinancesCrewPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [subTab, setSubTab] = useState<"finances" | "hiring" | "events">("finances");

  // Transfer state
  const [fromAcc, setFromAcc] = useState(snapshot.accounts[0]?.id ?? "");
  const [toAcc, setToAcc] = useState(snapshot.accounts[1]?.id ?? snapshot.accounts[0]?.id ?? "");
  const [transferCr, setTransferCr] = useState(5000);
  const [transferDesc, setTransferDesc] = useState("Owner draw / Operating transfer");

  // Manual deposit/expense
  const [txAcc, setTxAcc] = useState(snapshot.accounts[0]?.id ?? "");
  const [txAmount, setTxAmount] = useState(-500);
  const [txCategory, setTxCategory] = useState("port_fees");
  const [txDesc, setTxDesc] = useState("Starport berthing & fuel");

  // Hiring Hall state
  const [candidates, setCandidates] = useState<CrewCandidate[]>([]);
  const [hireMsg, setHireMsg] = useState<string | null>(null);

  // Scheduled Event form
  const [evTitle, setEvTitle] = useState("");
  const [evDays, setEvDays] = useState(7);

  async function handleTransfer() {
    const f = fromAcc || snapshot.accounts[0]?.id;
    const t = toAcc || snapshot.accounts[1]?.id;
    if (!f || !t || f === t) {
      onError("Choose two different accounts to transfer between.");
      return;
    }
    try {
      await api.transferFunds(campaignId, f, t, transferCr, transferDesc);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleManualTx() {
    const acc = txAcc || snapshot.accounts[0]?.id;
    if (!acc) return;
    try {
      await api.createTransaction(campaignId, {
        account_id: acc,
        amount_cr: txAmount,
        category: txCategory,
        description: txDesc,
        counterparty: "Manual Entry",
      });
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function loadHiringHall() {
    setHireMsg(null);
    try {
      const res = await api.hiringHall(campaignId, []);
      setCandidates(res.candidates);
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function hireCandidate(cand: CrewCandidate) {
    try {
      await api.hireCrew(campaignId, {
        character: cand.character,
        role: cand.role,
        salary_cr: cand.asking_salary_cr,
        ship_id: snapshot.ships[0]?.id,
      });
      setCandidates((prev) => prev.filter((c) => c.character.id !== cand.character.id));
      setHireMsg(`Hired ${cand.character.name} as ${cand.role} (${formatCr(cand.asking_salary_cr)}/4wk)!`);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleCreateEvent() {
    if (!evTitle.trim()) return;
    try {
      await api.createEvent(campaignId, {
        title: evTitle.trim(),
        kind: "deadline",
        days_from_now: evDays,
      });
      setEvTitle("");
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handlePayMaintenance() {
    try {
      await api.payMaintenance(campaignId);
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const companyAccount =
    snapshot.accounts.find((a) => a.kind === "company") ?? snapshot.accounts[0];
  const companyBalance = companyAccount?.balance_cr ?? 0;
  const maintTotal = snapshot.fleet_running_costs?.total_cr ?? 0;
  const canPayMaintenance = maintTotal > 0 && companyBalance >= maintTotal;

  const effFromId = fromAcc || snapshot.accounts[0]?.id || "";
  const effToId = toAcc || snapshot.accounts[1]?.id || snapshot.accounts[0]?.id || "";
  const srcAccount = snapshot.accounts.find((a) => a.id === effFromId);
  const transferInvalid =
    !effFromId ||
    !effToId ||
    effFromId === effToId ||
    transferCr <= 0 ||
    (srcAccount ? srcAccount.balance_cr < transferCr : true);
  const transferDisabledReason = !effFromId || !effToId || effFromId === effToId
    ? "Choose two different accounts"
    : transferCr <= 0
    ? "Enter an amount greater than Cr0"
    : srcAccount && srcAccount.balance_cr < transferCr
    ? `Insufficient balance in ${srcAccount.name} (have ${formatCr(srcAccount.balance_cr)})`
    : `Transfer ${formatCr(transferCr)}`;

  const effTxAccId = txAcc || snapshot.accounts[0]?.id || "";
  const txInvalid = !effTxAccId || txAmount === 0 || !txDesc.trim();
  const txDisabledReason = !effTxAccId
    ? "No account available"
    : txAmount === 0
    ? "Amount must be non-zero (+ deposit or - expense)"
    : !txDesc.trim()
    ? "Enter a description"
    : "Post ledger transaction";

  const eventInvalid = !evTitle.trim() || evDays < 1;

  return (
    <div className="space-y-3 text-xs">
      <div className="grid grid-cols-3 gap-1">
        <button
          onClick={() => setSubTab("finances")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "finances"
              ? "bg-emerald-950 border-emerald-600 text-emerald-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <CreditCard className="w-3.5 h-3.5" /> Accounts
        </button>
        <button
          onClick={() => setSubTab("hiring")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "hiring"
              ? "bg-cyan-950 border-cyan-600 text-cyan-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <Users className="w-3.5 h-3.5" /> Hiring Hall
        </button>
        <button
          onClick={() => setSubTab("events")}
          className={`py-1.5 rounded font-medium border flex items-center justify-center gap-1 ${
            subTab === "events"
              ? "bg-amber-950 border-amber-600 text-amber-200"
              : "bg-slate-900 border-slate-800 text-slate-400"
          }`}
        >
          <CalendarClock className="w-3.5 h-3.5" /> Events ({snapshot.pending_events?.length ?? 0})
        </button>
      </div>

      {subTab === "finances" && (
        <div className="space-y-2.5">
          {/* Balances */}
          <div className="grid grid-cols-2 gap-2">
            {snapshot.accounts.map((acc) => (
              <div key={acc.id} className="bg-slate-900 border border-slate-800 rounded p-2.5">
                <div className="text-[10px] uppercase text-slate-400">{acc.kind} Account</div>
                <div className="font-semibold text-slate-200 truncate">{acc.name}</div>
                <div
                  className={`text-base font-mono font-bold mt-0.5 ${
                    acc.balance_cr >= 0 ? "text-emerald-400" : "text-red-400"
                  }`}
                >
                  {formatCr(acc.balance_cr)}
                </div>
              </div>
            ))}
          </div>

          {/* Fleet 4-Week Running Costs: Mortgages, Maintenance, Life Support & Crew Salaries */}
          {snapshot.fleet_running_costs && (
            <div className="bg-slate-900 border border-amber-800/60 rounded p-2.5 space-y-1.5">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-amber-300">
                  4-Week Maintenance Period & Payroll
                </span>
                <span className="font-mono font-bold text-amber-200">
                  {formatCr(snapshot.fleet_running_costs.total_cr)} / 28d
                </span>
              </div>
              <div className="grid grid-cols-4 gap-1 text-center font-mono text-[10px]">
                <div className="bg-slate-950 rounded p-1 border border-slate-800">
                  <div className="text-slate-400">MORTGAGE</div>
                  <div className="text-slate-200">{formatCr(snapshot.fleet_running_costs.mortgage_cr)}</div>
                </div>
                <div className="bg-slate-950 rounded p-1 border border-slate-800">
                  <div className="text-slate-400">MAINT</div>
                  <div className="text-slate-200">{formatCr(snapshot.fleet_running_costs.maintenance_cr)}</div>
                </div>
                <div className="bg-slate-950 rounded p-1 border border-slate-800">
                  <div className="text-slate-400">LIFE SUPP</div>
                  <div className="text-slate-200">{formatCr(snapshot.fleet_running_costs.life_support_cr)}</div>
                </div>
                <div className="bg-slate-950 rounded p-1 border border-slate-800">
                  <div className="text-slate-400">SALARIES</div>
                  <div className="text-cyan-300">{formatCr(snapshot.fleet_running_costs.crew_salaries_cr)}</div>
                </div>
              </div>
              <button
                disabled={!canPayMaintenance}
                title={
                  maintTotal <= 0
                    ? "No fleet running costs currently due"
                    : !canPayMaintenance
                    ? `Insufficient Company funds (need ${formatCr(maintTotal)}, have ${formatCr(companyBalance)})`
                    : `Pay ${formatCr(maintTotal)} from Company account`
                }
                onClick={handlePayMaintenance}
                className="w-full bg-amber-700 hover:bg-amber-600 disabled:opacity-35 disabled:cursor-not-allowed text-white font-semibold rounded py-1 text-[11px]"
              >
                Pay Mortgage, Maintenance & Crew Salaries Now
              </button>
            </div>
          )}

          {/* Personal <-> Company Transfer */}
          {snapshot.accounts.length >= 2 && (
            <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5">
              <div className="font-semibold text-slate-300 flex items-center gap-1">
                <ArrowLeftRight className="w-3.5 h-3.5 text-cyan-400" /> Transfer Personal ↔ Company
              </div>
              <div className="grid grid-cols-2 gap-1.5">
                <select
                  className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                  value={fromAcc || snapshot.accounts[0].id}
                  onChange={(e) => setFromAcc(e.target.value)}
                >
                  {snapshot.accounts.map((a) => (
                    <option key={a.id} value={a.id}>
                      From: {a.name}
                    </option>
                  ))}
                </select>
                <select
                  className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                  value={toAcc || snapshot.accounts[1].id}
                  onChange={(e) => setToAcc(e.target.value)}
                >
                  {snapshot.accounts.map((a) => (
                    <option key={a.id} value={a.id}>
                      To: {a.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex gap-1.5">
                <input
                  type="number"
                  className="w-24 bg-slate-950 border border-slate-700 rounded px-2 py-1 font-mono"
                  value={transferCr}
                  onChange={(e) => setTransferCr(Number(e.target.value))}
                />
                <input
                  className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1"
                  value={transferDesc}
                  onChange={(e) => setTransferDesc(e.target.value)}
                />
                <button
                  disabled={transferInvalid}
                  title={transferDisabledReason}
                  onClick={handleTransfer}
                  className="bg-cyan-700 hover:bg-cyan-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2.5 py-1"
                >
                  Move
                </button>
              </div>
            </div>
          )}

          {/* Manual Deposit / Expense */}
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5">
            <div className="font-semibold text-slate-300">Record Deposit (+) or Expense (-)</div>
            <div className="grid grid-cols-2 gap-1.5">
              <select
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                value={txAcc || snapshot.accounts[0]?.id || ""}
                onChange={(e) => setTxAcc(e.target.value)}
              >
                {snapshot.accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name}
                  </option>
                ))}
              </select>
              <input
                type="number"
                className="bg-slate-950 border border-slate-700 rounded px-2 py-1 font-mono"
                value={txAmount}
                onChange={(e) => setTxAmount(Number(e.target.value))}
              />
            </div>
            <div className="flex gap-1.5">
              <input
                className="w-24 bg-slate-950 border border-slate-700 rounded px-1.5 py-1"
                placeholder="Category"
                value={txCategory}
                onChange={(e) => setTxCategory(e.target.value)}
              />
              <input
                className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1"
                placeholder="Description"
                value={txDesc}
                onChange={(e) => setTxDesc(e.target.value)}
              />
              <button
                disabled={txInvalid}
                title={txDisabledReason}
                onClick={handleManualTx}
                className="bg-emerald-700 hover:bg-emerald-600 disabled:opacity-35 disabled:cursor-not-allowed text-white rounded px-2.5 py-1"
              >
                Post
              </button>
            </div>
          </div>

          {/* Ledger */}
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1">
            <div className="font-semibold text-slate-300">Transaction Ledger</div>
            <div className="max-h-56 overflow-y-auto divide-y divide-slate-800">
              {snapshot.transactions.map((t) => (
                <div key={t.id} className="py-1 flex items-center justify-between gap-2 text-[11px]">
                  <div className="min-w-0">
                    <div className="text-slate-200 truncate">{t.description}</div>
                    <div className="text-[10px] text-slate-500">
                      {t.imperial_date} · {t.category} · {t.counterparty}
                    </div>
                  </div>
                  <div
                    className={`font-mono font-semibold shrink-0 ${
                      t.amount_cr >= 0 ? "text-emerald-400" : "text-red-400"
                    }`}
                  >
                    {t.amount_cr >= 0 ? "+" : ""}
                    {formatCr(t.amount_cr)}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {subTab === "hiring" && (
        <div className="space-y-2.5">
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 flex items-center justify-between">
            <div>
              <div className="font-semibold text-cyan-300">
                {snapshot.campaign.current_system_name} Starport Hiring Hall
              </div>
              <div className="text-[10px] text-slate-400">
                Generates authentic MgT2e Lifepath crew candidates scaled by starport class.
              </div>
            </div>
            <button
              onClick={loadHiringHall}
              className="bg-cyan-700 hover:bg-cyan-600 text-white rounded px-3 py-1.5 font-medium shrink-0"
            >
              Browse Applicants
            </button>
          </div>

          {hireMsg && (
            <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200">
              {hireMsg}
            </div>
          )}

          {candidates.map((cand) => (
            <div
              key={cand.character.id}
              className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5"
            >
              <div className="flex items-start justify-between">
                <div>
                  <div className="font-bold text-slate-100">
                    {cand.character.name}{" "}
                    <span className="text-amber-300 font-normal">({cand.role})</span>
                  </div>
                  <div className="text-[10px] text-slate-400">
                    Ex-{cand.career} · {cand.terms} term(s) · Age {cand.character.age} · Asking{" "}
                    <span className="text-emerald-300 font-mono">
                      {formatCr(cand.asking_salary_cr)}/4wk
                    </span>
                  </div>
                </div>
                <button
                  onClick={() => hireCandidate(cand)}
                  className="bg-emerald-700 hover:bg-emerald-600 text-white rounded px-2.5 py-1 flex items-center gap-1"
                >
                  <UserCheck className="w-3.5 h-3.5" /> Hire
                </button>
              </div>
              <div className="flex flex-wrap gap-1">
                {Object.entries(cand.character.skills).map(([sk, lvl]) => (
                  <span
                    key={sk}
                    className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-[10px] text-slate-300"
                  >
                    {sk}-{lvl}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {subTab === "events" && (
        <div className="space-y-2.5">
          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-2">
            <div className="font-semibold text-amber-300">Schedule Imperial Calendar Event</div>
            <div className="flex gap-1.5">
              <input
                className="flex-1 bg-slate-950 border border-slate-700 rounded px-2 py-1"
                placeholder="Contract deadline, patron meeting, overhaul due…"
                value={evTitle}
                onChange={(e) => setEvTitle(e.target.value)}
              />
              <label className="flex items-center gap-1 text-[11px] text-slate-400">
                +
                <input
                  type="number"
                  min={1}
                  className="w-12 bg-slate-950 border border-slate-700 rounded px-1 py-1 text-center"
                  value={evDays}
                  onChange={(e) => setEvDays(Number(e.target.value) || 1)}
                />
                d
              </label>
              <button
                disabled={eventInvalid}
                title={eventInvalid ? "Enter an event title and at least 1 day" : "Schedule event"}
                onClick={handleCreateEvent}
                className="bg-amber-600 hover:bg-amber-500 disabled:opacity-35 disabled:cursor-not-allowed text-slate-950 font-semibold rounded px-2.5 py-1 flex items-center gap-1"
              >
                <Plus className="w-3.5 h-3.5" /> Add
              </button>
            </div>
          </div>

          <div className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5">
            <div className="font-semibold text-slate-300">Upcoming Scheduled Events</div>
            {(!snapshot.pending_events || snapshot.pending_events.length === 0) ? (
              <div className="text-slate-500 text-[11px]">No pending events scheduled.</div>
            ) : (
              snapshot.pending_events.map((ev) => (
                <div
                  key={ev.id}
                  className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded px-2 py-1.5"
                >
                  <div>
                    <div className="text-slate-200 font-medium">{ev.title}</div>
                    <div className="text-[10px] text-slate-500 uppercase">{ev.kind}</div>
                  </div>
                  <span className="font-mono text-amber-300 text-[11px]">{ev.due_label}</span>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

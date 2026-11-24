export interface ImperialDate {
  year: number;
  day: number;
  hour: number;
  minute: number;
}

export interface Campaign {
  id: string;
  name: string;
  date: ImperialDate;
  current_system_name: string;
  current_system_uwp: string;
  referee_model: string;
  worker_model: string;
  hindsight_bank_id: string;
  story_ledger: {
    who_knows_what: string;
    settled: string;
    people: string;
    threads: string;
  };
}

export interface Upp {
  str_val: number;
  dex_val: number;
  end_val: number;
  int_val: number;
  edu_val: number;
  soc_val: number;
  psi_val?: number | null;
}

export interface ExpertSoftware {
  skill: string;
  rating: number;
  active: boolean;
}

export type Augment =
  | { kind: "neural_jack"; tl: number; computer_rating: number; storage_bandwidth: number; loaded_expert: ExpertSoftware[] }
  | { kind: "wafer_jack"; tl: number; computer_rating: number; storage_bandwidth: number; loaded_expert: ExpertSoftware[] }
  | { kind: "neural_link"; tl: number }
  | { kind: "skill_augmentation"; skill: string }
  | { kind: "nerve_plug_response_rig"; plugged_in: boolean }
  | { kind: "peripheral_vision_enhancement" }
  | { kind: "subdermal_armour"; protection: number }
  | { kind: "other"; name: string; tl: number; effect: string };

export interface Character {
  id: string;
  campaign_id: string;
  kind: "player" | "crew" | "npc";
  name: string;
  species: string;
  age: number;
  upp_max: Upp;
  upp_current: Upp;
  skills: Record<string, number>;
  augments: Augment[];
  skill_xp: Record<string, number>;
  characteristic_xp: Record<string, number>;
  general_xp: number;
  salary_cr: number;
  loyalty: number;
  assigned_ship_id?: string | null;
  assigned_station?: string | null;
  notes: string;
}

export interface CharacterRow {
  character: Character;
  equipment: unknown[];
}

export interface ValidationIssue {
  severity: "error" | "warning";
  category: string;
  message: string;
}

export interface ShipRecord {
  id: string;
  name: string;
  class_name: string;
  spec: {
    tonnage: number;
    tl: number;
    hull_config: string;
    hull_points_max: number;
    armor_rating: number;
    thrust: number;
    jump_rating: number;
    power_plant_output: number;
    sensor_grade: string;
    stealth_dm: number;
    fuel_capacity_tons: number;
    staterooms: number;
    low_berths: number;
    cargo_capacity_tons: number;
    purchase_price_cr: number;
  };
  validation: {
    valid: boolean;
    used_tonnage: number;
    free_tonnage: number;
    power_basic: number;
    power_m_drive: number;
    power_j_drive: number;
    power_total_peak_combat: number;
    hardpoints_used: number;
    hardpoints_max: number;
    jump_fuel_required: number;
    min_engineers_required: number;
    issues: ValidationIssue[];
  };
  hull_current: number;
  fuel_refined: number;
  fuel_unrefined: number;
  crits: Array<{ name?: string; effect?: string; date?: string }>;
  mortgage?: {
    active: boolean;
    monthly_payment_cr: number;
    periods_total: number;
    periods_paid: number;
    periods_overdue: number;
    arrears_cr: number;
  } | null;
  skipped_maintenance_periods: number;
  docked_system_name: string;
}

export interface AccountRecord {
  id: string;
  kind: "personal" | "company";
  name: string;
  balance_cr: number;
  credit_limit_cr: number;
}

export interface TransactionRecord {
  id: string;
  account_id: string;
  imperial_date: string;
  amount_cr: number;
  balance_after_cr: number;
  category: string;
  description: string;
  counterparty: string;
}

export interface MessageRecord {
  id: number;
  role: "user" | "assistant" | "system";
  content: string;
  superseded: boolean;
  model?: string | null;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  created_at: string;
}

export interface CampaignSnapshot {
  campaign: Campaign;
  characters: CharacterRow[];
  ships: ShipRecord[];
  accounts: AccountRecord[];
  transactions: TransactionRecord[];
  llm_costs: {
    total_cost_usd: number;
    input_tokens: number;
    output_tokens: number;
    cache_read_tokens: number;
  };
}

export type TurnEvent =
  | { type: "status"; message: string }
  | { type: "tool_call"; id: string; name: string; input: Record<string, unknown> }
  | { type: "tool_result"; id: string; name: string; result: string; is_error: boolean }
  | { type: "state_changed" }
  | { type: "delta"; text: string }
  | { type: "done"; message_id: number; input_tokens: number; output_tokens: number; cache_read_tokens: number }
  | { type: "error"; message: string };

async function req<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data as T;
}

export const api = {
  listCampaigns: () => req<{ campaigns: Campaign[] }>("/api/campaigns"),
  createCampaign: (name: string, systemName = "Regina", systemUwp = "A788899-C") =>
    req<{ campaign: Campaign }>("/api/campaigns", {
      method: "POST",
      body: JSON.stringify({
        name,
        current_system_name: systemName,
        current_system_uwp: systemUwp,
        seed_starter_ship: true,
      }),
    }),
  getSnapshot: (id: string) => req<CampaignSnapshot>(`/api/campaigns/${id}/snapshot`),
  listMessages: (id: string) => req<{ messages: MessageRecord[] }>(`/api/campaigns/${id}/messages`),
  createCharacter: (id: string, body: Record<string, unknown>) =>
    req<{ character: CharacterRow }>(`/api/campaigns/${id}/characters`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  updateCharacter: (id: string, charId: string, character: Character, equipment: unknown[] = []) =>
    req<{ character: Character }>(`/api/campaigns/${id}/characters/${charId}`, {
      method: "PATCH",
      body: JSON.stringify({ character, equipment }),
    }),
  spendXp: (id: string, charId: string, skill: string) =>
    req(`/api/campaigns/${id}/characters/${charId}/spend-xp`, {
      method: "POST",
      body: JSON.stringify({ skill, allow_general_xp: true }),
    }),
  advanceTime: (id: string, body: Record<string, unknown>) =>
    req(`/api/campaigns/${id}/time/advance`, { method: "POST", body: JSON.stringify(body) }),
  transferFunds: (id: string, fromId: string, toId: string, amountCr: number, description: string) =>
    req(`/api/campaigns/${id}/accounts/transfer`, {
      method: "POST",
      body: JSON.stringify({
        from_account_id: fromId,
        to_account_id: toId,
        amount_cr: amountCr,
        description,
      }),
    }),
  rollTask: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/dice/task`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  sensorCheck: (id: string, body: Record<string, unknown>) =>
    req<{ sensor_check: Record<string, unknown> }>(`/api/campaigns/${id}/sensors/check`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  async streamChat(id: string, message: string, onEvent: (ev: TurnEvent) => void): Promise<void> {
    const res = await fetch(`/api/campaigns/${id}/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message }),
    });
    if (!res.ok || !res.body) throw new Error(`Chat stream HTTP ${res.status}`);
    const reader = res.body.getReader();
    const dec = new TextDecoder();
    let buf = "";
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      let idx: number;
      while ((idx = buf.indexOf("\n\n")) !== -1) {
        const raw = buf.slice(0, idx);
        buf = buf.slice(idx + 2);
        for (const line of raw.split("\n")) {
          if (line.startsWith("data: ")) {
            try {
              onEvent(JSON.parse(line.slice(6)) as TurnEvent);
            } catch {
              // ignore malformed keepalive
            }
          }
        }
      }
    }
  },
};

export function formatImperialDate(d: ImperialDate): string {
  const day = String(d.day).padStart(3, "0");
  const yr = String(d.year).padStart(4, "0");
  const hr = String(d.hour).padStart(2, "0");
  const min = String(d.minute).padStart(2, "0");
  return `${day}-${yr} ${hr}:${min}`;
}

export function formatCr(cr: number): string {
  const sign = cr < 0 ? "-" : "";
  return `${sign}Cr${Math.abs(cr).toLocaleString()}`;
}

export function skillUpgradeCost(targetLevel: number): number {
  return targetLevel <= 1 ? 1 : 1 << (targetLevel - 1);
}

export interface AuthUser {
  username: string;
  email?: string | null;
  display_name?: string | null;
  groups: string[];
  auth_mode: "authentik_proxy" | "oidc" | "none";
  logout_url?: string | null;
}

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
  equipment: Array<Record<string, unknown>>;
}

export interface ValidationIssue {
  severity: "error" | "warning";
  category: string;
  message: string;
}

export interface CargoLot {
  d66?: number;
  name: string;
  tons: number;
  purchase_price_cr?: number;
  bought_at?: string;
}

export interface ShipRecord {
  id: string;
  campaign_id?: string;
  name: string;
  class_name: string;
  owner_account_id?: string | null;
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
    weapons?: Array<{ name: string; mount: string; count: number; power: number; tonnage: number }>;
    [key: string]: unknown;
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
  cargo: CargoLot[];
  passengers: Array<Record<string, unknown>>;
  mortgage?: {
    active: boolean;
    monthly_payment_cr: number;
    periods_total: number;
    periods_paid: number;
    periods_overdue: number;
    arrears_cr: number;
  } | null;
  skipped_maintenance_periods: number;
  last_annual_overhaul_year?: number;
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

export interface ScheduledEventRecord {
  id: string;
  campaign_id: string;
  due_total_minutes: number;
  due_label: string;
  kind: string;
  title: string;
  payload: Record<string, unknown>;
  completed: boolean;
}

export interface CombatRecord {
  id: string;
  campaign_id: string;
  kind: "personal" | "space";
  status: string;
  round: number;
  phase: string;
  state: Record<string, unknown>;
}

export interface SectorRecord {
  id: string;
  name: string;
  abbreviation: string;
  sector_x: number;
  sector_y: number;
  system_count: number;
}

export interface SystemRecord {
  id: string;
  sector_id: string;
  sector_name: string;
  hex: string;
  name: string;
  uwp: string;
  starport: string;
  tech_level: number;
  law_level: number;
  trade_codes: string[];
  bases: string[];
  allegiance: string;
  travel_zone: string;
  gas_giants: number;
  global_x: number;
  global_y: number;
  metadata: Record<string, unknown>;
}

export interface ItemRecord {
  id: string;
  pack_id: string;
  category: string;
  name: string;
  tl: number;
  cost_cr: number;
  mass_kg: number;
  law_restriction: number;
  stats: Record<string, unknown>;
  tags: string[];
}

export interface LifepathLogEntry {
  term: number;
  age: number;
  career: string;
  qualified: boolean;
  survived: boolean;
  advanced: boolean;
  rank: number;
  skills_gained: string[];
  event: string;
  aging_note?: string | null;
}

export interface LifepathSession {
  id: string;
  name: string;
  species: string;
  age: number;
  terms_completed: number;
  upp: Upp;
  skills: Record<string, number>;
  current_career?: string | null;
  current_rank: number;
  benefit_rolls_earned: number;
  cash_cr: number;
  ship_shares: number;
  career_ship_benefits: number;
  pension_cr: number;
  allies: string[];
  rivals: string[];
  mustered_out: boolean;
  log: LifepathLogEntry[];
}

export interface CrewCandidate {
  role: string;
  career: string;
  terms: number;
  asking_salary_cr: number;
  character: Character;
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

export interface ModuleKnowledgeFile {
  filename: string;
  title: string;
  content: string;
}

export interface ModuleEventSpec {
  title: string;
  kind: string;
  days_from_now: number;
  hours_from_now?: number;
}

export interface ModuleRecord {
  id: string;
  slug: string;
  name: string;
  version: string;
  description: string;
  author: string;
  starting_system_name?: string | null;
  starting_system_uwp?: string | null;
  referee_brief: string;
  knowledge_files: ModuleKnowledgeFile[];
  npcs: Array<Record<string, unknown>>;
  ships: Array<Record<string, unknown>>;
  events: ModuleEventSpec[];
  tags: string[];
  source: string;
}

export interface CampaignModuleRecord {
  id: string;
  campaign_id: string;
  module_id: string;
  module_slug: string;
  module_name: string;
  referee_brief: string;
  knowledge_file_count: number;
  hindsight_chunks_ingested: number;
  npcs_spawned: number;
  ships_spawned: number;
  events_spawned: number;
  loaded_at: string;
}

export interface SystemShipRecord {
  id: string;
  campaign_id: string;
  system_name: string;
  name: string;
  ship_class: string;
  role: string;
  tonnage: number;
  tl: number;
  hull_max: number;
  hull_current: number;
  armor: number;
  thrust: number;
  jump_rating: number;
  sensor_grade: string;
  stealth_dm: number;
  transponder_active: boolean;
  disposition: string;
  orbit_location: string;
  captain_name: string;
  faction: string;
  cargo_hint: string;
  comms_log: Array<Record<string, unknown>>;
  status: string;
}

export interface FleetBrokerSummary {
  max_broker_skill: number;
  broker_source: string;
  max_steward_skill: number;
  steward_source: string;
  contributors: Array<[string, number]>;
}

export interface FleetRunningCosts {
  mortgage_cr: number;
  maintenance_cr: number;
  life_support_cr: number;
  crew_salaries_cr: number;
  total_cr: number;
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
  active_personal_combat?: CombatRecord | null;
  active_space_combat?: CombatRecord | null;
  pending_events?: ScheduledEventRecord[];
  loaded_modules?: CampaignModuleRecord[];
  system_ships?: SystemShipRecord[];
  system_ship_count?: number;
  fleet_max_jump?: number;
  fleet_broker?: FleetBrokerSummary;
  fleet_running_costs?: FleetRunningCosts;
  pack_stats?: { sectors: number; items: number };
}

export type TurnEvent =
  | { type: "status"; message: string }
  | { type: "tool_call"; id: string; name: string; input: Record<string, unknown> }
  | { type: "tool_result"; id: string; name: string; result: string; is_error: boolean }
  | { type: "state_changed" }
  | { type: "clear_draft" }
  | { type: "delta"; text: string }
  | { type: "done"; message_id: number; input_tokens: number; output_tokens: number; cache_read_tokens: number }
  | { type: "error"; message: string };

export class AuthRequiredError extends Error {
  loginUrl: string;
  constructor(message: string, loginUrl: string) {
    super(message);
    this.loginUrl = loginUrl;
  }
}

async function req<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  const data = await res.json();
  if (res.status === 401) {
    throw new AuthRequiredError(data.error || "Authentication required", data.login_url || "/api/auth/login");
  }
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data as T;
}

export const api = {
  authMe: () => req<{ user: AuthUser }>("/api/auth/me"),
  listCampaigns: () => req<{ campaigns: Campaign[] }>("/api/campaigns"),
  createCampaign: (name: string, systemName = "Regina", systemUwp = "A788899-C") =>
    req<{ campaign: Campaign; in_system_ship_count?: number; traffic_summary?: string }>("/api/campaigns", {
      method: "POST",
      body: JSON.stringify({
        name,
        current_system_name: systemName,
        current_system_uwp: systemUwp,
        seed_starter_ship: true,
      }),
    }),
  deleteCampaign: (id: string) =>
    req<{ deleted: boolean }>(`/api/campaigns/${id}`, { method: "DELETE" }),
  getSnapshot: (id: string) => req<CampaignSnapshot>(`/api/campaigns/${id}/snapshot`),
  listMessages: (id: string) => req<{ messages: MessageRecord[] }>(`/api/campaigns/${id}/messages`),

  // Characters & Lifepath Chargen
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
  deleteCharacter: (id: string, charId: string) =>
    req<{ deleted: boolean }>(`/api/campaigns/${id}/characters/${charId}`, {
      method: "DELETE",
    }),
  spendXp: (id: string, charId: string, skill: string) =>
    req(`/api/campaigns/${id}/characters/${charId}/spend-xp`, {
      method: "POST",
      body: JSON.stringify({ skill, allow_general_xp: true }),
    }),
  chargenStart: (name: string, species = "Human", background_skills: string[] = []) =>
    req<{ session: LifepathSession }>("/api/chargen/start", {
      method: "POST",
      body: JSON.stringify({ name, species, background_skills }),
    }),
  chargenTerm: (session: LifepathSession, career: string, prefer_specialist: boolean) =>
    req<{ session: LifepathSession; entry: LifepathLogEntry }>("/api/chargen/term", {
      method: "POST",
      body: JSON.stringify({ session, career, prefer_specialist }),
    }),
  chargenMuster: (session: LifepathSession, cash_rolls: number) =>
    req<{ session: LifepathSession }>("/api/chargen/muster", {
      method: "POST",
      body: JSON.stringify({ session, cash_rolls }),
    }),
  chargenCommit: (
    id: string,
    session: LifepathSession,
    kind: "player" | "crew" | "npc",
    salary_cr: number,
    station?: string,
    deposit_cash_account_id?: string,
    replace_existing_player = false
  ) =>
    req<{ character: CharacterRow }>(`/api/campaigns/${id}/chargen/commit`, {
      method: "POST",
      body: JSON.stringify({
        session,
        kind,
        salary_cr,
        station,
        deposit_cash_account_id,
        replace_existing_player,
      }),
    }),

  // Campaign Modules (Hindsight Knowledge Files + NPCs)
  listModules: () => req<{ modules: ModuleRecord[] }>("/api/modules"),
  createModule: (body: Record<string, unknown>) =>
    req<{ module: ModuleRecord }>("/api/modules", {
      method: "POST",
      body: JSON.stringify(body),
    }),
  deleteModule: (moduleId: string) =>
    req<{ deleted: boolean }>(`/api/modules/${moduleId}`, { method: "DELETE" }),
  loadModule: (campaignId: string, moduleId: string, moveToStartingSystem = false) =>
    req<{
      loaded: boolean;
      module_name: string;
      hindsight_bank_id: string;
      hindsight_chunks_ingested: number;
      npcs_spawned: number;
      events_spawned: number;
    }>(`/api/campaigns/${campaignId}/modules/${moduleId}/load`, {
      method: "POST",
      body: JSON.stringify({ move_to_starting_system: moveToStartingSystem }),
    }),

  // In-System Starship Traffic
  listTraffic: (id: string) =>
    req<{ system: string; in_system_ship_count: number; ships: SystemShipRecord[] }>(
      `/api/campaigns/${id}/traffic`
    ),
  scanTraffic: (id: string) =>
    req<{ system: string; in_system_ship_count: number; summary: string; ships: SystemShipRecord[] }>(
      `/api/campaigns/${id}/traffic/scan`,
      { method: "POST" }
    ),
  interactVessel: (id: string, vesselId: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/traffic/${vesselId}/interact`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  // Ships & Port Ops (Refuel, Docking Fees, Maintenance)
  updateShip: (id: string, shipId: string, ship: ShipRecord) =>
    req<{ ship: ShipRecord }>(`/api/campaigns/${id}/ships/${shipId}`, {
      method: "PATCH",
      body: JSON.stringify(ship),
    }),
  refuelShip: (
    id: string,
    shipId: string,
    mode: "refined" | "unrefined" | "gas_giant_skim" | "process_onboard",
    tons?: number
  ) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/ships/${shipId}/refuel`, {
      method: "POST",
      body: JSON.stringify({ mode, tons }),
    }),
  dockShip: (id: string, shipId: string, extra_days = 0) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/ships/${shipId}/dock`, {
      method: "POST",
      body: JSON.stringify({ extra_days }),
    }),
  payMaintenance: (id: string) =>
    req<{ total_paid_cr: number }>(`/api/campaigns/${id}/finances/pay-maintenance`, {
      method: "POST",
    }),

  // Universe & Jump
  listSectors: () => req<{ sectors: SectorRecord[] }>("/api/universe/sectors"),
  listSectorSystems: (sectorId: string) =>
    req<{ systems: SystemRecord[] }>(`/api/universe/sectors/${sectorId}/systems`),
  searchSystems: (q: string, tradeCode?: string, limit = 40) => {
    const p = new URLSearchParams();
    if (q) p.set("q", q);
    if (tradeCode) p.set("trade_code", tradeCode);
    p.set("limit", String(limit));
    return req<{ systems: SystemRecord[] }>(`/api/universe/systems/search?${p.toString()}`);
  },
  planRoute: (from_system: string, to_system: string, jump_rating: number, avoid_red_zones = true, require_refuel = true) =>
    req<{ found: boolean; jump_rating: number; route?: { total_jumps: number; total_parsecs: number; legs: Array<{ from: { name: string; hex: string }; to: { name: string; hex: string; uwp: string }; parsecs: number; refuel_options: string[] }> } }>("/api/universe/route", {
      method: "POST",
      body: JSON.stringify({ from_system, to_system, jump_rating, avoid_red_zones, require_refuel }),
    }),
  executeJump: (id: string, to_system: string, ship_id?: string) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/jump`, {
      method: "POST",
      body: JSON.stringify({ to_system, ship_id }),
    }),

  // Items & CSC 2023 Shop
  searchItems: (q: string, category?: string, maxTl?: number, limit = 80) => {
    const p = new URLSearchParams();
    if (q) p.set("q", q);
    if (category) p.set("category", category);
    if (maxTl !== undefined) p.set("max_tl", String(maxTl));
    p.set("limit", String(limit));
    return req<{ items: ItemRecord[] }>(`/api/items?${p.toString()}`);
  },
  shopAvailability: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/shop/availability`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  shopPurchase: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/shop/purchase`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  // Trade
  tradeMarket: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/trade/market`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  tradeBuy: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/trade/buy`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  tradeSell: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/trade/sell`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  // Crew Hiring Hall
  hiringHall: (id: string, roles: string[] = []) =>
    req<{ system: string; candidates: CrewCandidate[] }>(`/api/campaigns/${id}/crew/hiring-hall`, {
      method: "POST",
      body: JSON.stringify({ roles }),
    }),
  hireCrew: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/crew/hire`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  // Combat (Personal & Space)
  personalCombat: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/combat/personal`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  spaceCombat: (id: string, body: Record<string, unknown>) =>
    req<Record<string, unknown>>(`/api/campaigns/${id}/combat/space`, {
      method: "POST",
      body: JSON.stringify(body),
    }),

  // Time, Events, Finances, Dice, Sensors
  advanceTime: (id: string, body: Record<string, unknown>) =>
    req(`/api/campaigns/${id}/time/advance`, { method: "POST", body: JSON.stringify(body) }),
  createEvent: (id: string, body: Record<string, unknown>) =>
    req(`/api/campaigns/${id}/events`, { method: "POST", body: JSON.stringify(body) }),
  createTransaction: (id: string, body: Record<string, unknown>) =>
    req(`/api/campaigns/${id}/transactions`, { method: "POST", body: JSON.stringify(body) }),
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
  reloadPacks: () => req<Record<string, unknown>>("/api/packs/reload", { method: "POST" }),

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

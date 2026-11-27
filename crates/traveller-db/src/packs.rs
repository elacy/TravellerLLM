use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

use crate::Db;

#[derive(Deserialize)]
struct SectorPack {
    sectors: Vec<SectorImport>,
}

#[derive(Deserialize)]
struct SectorImport {
    name: String,
    abbreviation: String,
    sector_x: i32,
    sector_y: i32,
    systems: Vec<SystemImport>,
}

#[derive(Deserialize)]
struct SystemImport {
    hex: String,
    name: String,
    uwp: String,
    starport: String,
    tech_level: i16,
    law_level: i16,
    #[serde(default)]
    trade_codes: Vec<String>,
    #[serde(default)]
    bases: Vec<String>,
    #[serde(default = "default_allegiance")]
    allegiance: String,
    #[serde(default = "default_zone")]
    travel_zone: String,
    #[serde(default)]
    gas_giants: i16,
    global_x: i32,
    global_y: i32,
    #[serde(default)]
    pbg: String,
    #[serde(default)]
    stars: String,
}

fn default_allegiance() -> String {
    "Im".into()
}
fn default_zone() -> String {
    "Green".into()
}

#[derive(Deserialize)]
struct ItemPack {
    items: Vec<ItemImport>,
}

#[derive(Deserialize)]
struct ItemImport {
    pack_id: String,
    category: String,
    name: String,
    tl: i16,
    cost_cr: i64,
    #[serde(default)]
    mass_kg: f64,
    #[serde(default = "default_law")]
    law_restriction: i16,
    #[serde(default)]
    stats: Value,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_law() -> i16 {
    9
}

fn read_json_or_gz(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    if path.extension().and_then(|e| e.to_str()) == Some("gz") {
        let mut dec = GzDecoder::new(f);
        let mut s = String::new();
        dec.read_to_string(&mut s)?;
        Ok(s)
    } else {
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(s)
    }
}

/// Scan `packs_dir` (and its subdirectories like `local/` and `starter-srd/`) and idempotently
/// load any sector packs (`*sectors*.json[.gz]`) and item packs (`*items*.json[.gz]`) into Postgres.
pub async fn load_packs_from_dir(db: &Db, packs_dir: &Path) -> Result<(usize, usize)> {
    if !packs_dir.exists() {
        return Ok((0, 0));
    }

    let mut files = Vec::new();
    collect_files(packs_dir, &mut files);
    files.sort();

    let mut imported_systems = 0usize;
    let mut imported_items = 0usize;

    for path in files {
        let fname = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !(fname.ends_with(".json") || fname.ends_with(".json.gz")) {
            continue;
        }
        if fname.contains("sector") {
            let existing_sectors = db.count_sectors().await.unwrap_or(0);
            if existing_sectors >= 50 {
                info!(
                    "Skipping sector pack {} ({existing_sectors} sectors already loaded in Postgres)",
                    path.display()
                );
                continue;
            }
            match read_json_or_gz(&path).and_then(|s| Ok(serde_json::from_str::<SectorPack>(&s)?)) {
                Ok(pack) => {
                    let count = import_sectors(db, pack).await?;
                    info!("Loaded {count} systems from {}", path.display());
                    imported_systems += count;
                }
                Err(e) => warn!("Failed to parse sector pack {}: {e}", path.display()),
            }
        } else if fname.contains("item") {
            match read_json_or_gz(&path).and_then(|s| Ok(serde_json::from_str::<ItemPack>(&s)?)) {
                Ok(pack) => {
                    let count = import_items(db, pack).await?;
                    info!("Loaded {count} items from {}", path.display());
                    imported_items += count;
                }
                Err(e) => warn!("Failed to parse item pack {}: {e}", path.display()),
            }
        }
    }

    Ok((imported_systems, imported_items))
}

fn collect_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_files(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

async fn import_sectors(db: &Db, pack: SectorPack) -> Result<usize> {
    let mut total_systems = 0usize;
    let pool = db.pool();

    for sec in pack.sectors {
        let sec_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("sector:{}", sec.name).as_bytes());
        sqlx::query(
            "INSERT INTO sectors (id, name, abbreviation, sector_x, sector_y)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (name) DO UPDATE SET abbreviation = EXCLUDED.abbreviation,
                 sector_x = EXCLUDED.sector_x, sector_y = EXCLUDED.sector_y",
        )
        .bind(sec_id)
        .bind(&sec.name)
        .bind(&sec.abbreviation)
        .bind(sec.sector_x)
        .bind(sec.sector_y)
        .execute(pool)
        .await?;

        // Fetch actual sector id in case it pre-existed with a different UUID
        let actual_sec_id: Uuid =
            sqlx::query_scalar("SELECT id FROM sectors WHERE name = $1")
                .bind(&sec.name)
                .fetch_one(pool)
                .await?;

        // Bulk-insert all systems in this sector using a single UNNEST statement
        if sec.systems.is_empty() {
            continue;
        }
        let n = sec.systems.len();
        let mut ids = Vec::with_capacity(n);
        let mut hexes = Vec::with_capacity(n);
        let mut names = Vec::with_capacity(n);
        let mut uwps = Vec::with_capacity(n);
        let mut starports = Vec::with_capacity(n);
        let mut tls = Vec::with_capacity(n);
        let mut laws = Vec::with_capacity(n);
        let mut trade_codes_json = Vec::with_capacity(n);
        let mut bases_json = Vec::with_capacity(n);
        let mut allegiances = Vec::with_capacity(n);
        let mut zones = Vec::with_capacity(n);
        let mut ggs = Vec::with_capacity(n);
        let mut gxs = Vec::with_capacity(n);
        let mut gys = Vec::with_capacity(n);
        let mut metas = Vec::with_capacity(n);

        for sys in sec.systems {
            ids.push(Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("system:{}:{}", sec.name, sys.hex).as_bytes(),
            ));
            hexes.push(sys.hex);
            names.push(sys.name);
            uwps.push(sys.uwp);
            starports.push(sys.starport.chars().next().unwrap_or('X').to_string());
            tls.push(sys.tech_level);
            laws.push(sys.law_level);
            trade_codes_json.push(serde_json::to_value(sys.trade_codes)?);
            bases_json.push(serde_json::to_value(sys.bases)?);
            allegiances.push(sys.allegiance);
            zones.push(sys.travel_zone);
            ggs.push(sys.gas_giants);
            gxs.push(sys.global_x);
            gys.push(sys.global_y);
            metas.push(serde_json::json!({"pbg": sys.pbg, "stars": sys.stars}));
        }

        sqlx::query(
            "INSERT INTO systems
                (id, sector_id, hex, name, uwp, starport, tech_level, law_level,
                 trade_codes, bases, allegiance, travel_zone, gas_giants, global_x, global_y, metadata)
             SELECT
                u.id, $1, u.hex, u.name, u.uwp, u.starport, u.tl, u.law,
                ARRAY(SELECT jsonb_array_elements_text(u.tc)),
                ARRAY(SELECT jsonb_array_elements_text(u.bs)),
                u.alleg, u.tz, u.gg, u.gx, u.gy, u.meta
             FROM UNNEST(
                $2::uuid[], $3::text[], $4::text[], $5::text[], $6::text[], $7::int2[], $8::int2[],
                $9::jsonb[], $10::jsonb[], $11::text[], $12::text[], $13::int2[], $14::int4[], $15::int4[], $16::jsonb[]
             ) AS u(id, hex, name, uwp, starport, tl, law, tc, bs, alleg, tz, gg, gx, gy, meta)
             ON CONFLICT (sector_id, hex) DO UPDATE SET
                name = EXCLUDED.name, uwp = EXCLUDED.uwp, starport = EXCLUDED.starport,
                tech_level = EXCLUDED.tech_level, law_level = EXCLUDED.law_level,
                trade_codes = EXCLUDED.trade_codes, bases = EXCLUDED.bases,
                allegiance = EXCLUDED.allegiance, travel_zone = EXCLUDED.travel_zone,
                gas_giants = EXCLUDED.gas_giants, global_x = EXCLUDED.global_x,
                global_y = EXCLUDED.global_y, metadata = EXCLUDED.metadata",
        )
        .bind(actual_sec_id)
        .bind(&ids)
        .bind(&hexes)
        .bind(&names)
        .bind(&uwps)
        .bind(&starports)
        .bind(&tls)
        .bind(&laws)
        .bind(&trade_codes_json)
        .bind(&bases_json)
        .bind(&allegiances)
        .bind(&zones)
        .bind(&ggs)
        .bind(&gxs)
        .bind(&gys)
        .bind(&metas)
        .execute(pool)
        .await?;
        total_systems += n;
    }
    Ok(total_systems)
}

async fn import_items(db: &Db, pack: ItemPack) -> Result<usize> {
    let mut tx = db.pool().begin().await?;
    let mut count = 0usize;
    for item in pack.items {
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("item:{}:{}", item.pack_id, item.name).as_bytes(),
        );
        sqlx::query(
            "INSERT INTO items
                (id, pack_id, category, name, tl, cost_cr, mass_kg, law_restriction, stats, tags)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT (pack_id, name) DO UPDATE SET
                category = EXCLUDED.category, tl = EXCLUDED.tl, cost_cr = EXCLUDED.cost_cr,
                mass_kg = EXCLUDED.mass_kg, law_restriction = EXCLUDED.law_restriction,
                stats = EXCLUDED.stats, tags = EXCLUDED.tags",
        )
        .bind(id)
        .bind(&item.pack_id)
        .bind(&item.category)
        .bind(&item.name)
        .bind(item.tl)
        .bind(item.cost_cr)
        .bind(item.mass_kg)
        .bind(item.law_restriction)
        .bind(&item.stats)
        .bind(&item.tags)
        .execute(&mut *tx)
        .await?;
        count += 1;
    }
    tx.commit().await?;
    Ok(count)
}

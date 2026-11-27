#!/usr/bin/env python3
"""Offline book-ingest CLI: extract structured MgT2e item catalogs from legally owned PDFs
into a gitignored local JSON pack (packs/local/mgt2e_items.json) for runtime import into Postgres.

NEVER commit packs/local/ to the public repository.

Usage:
  uv run --with pymupdf python3 tools/book-ingest/ingest_books.py \
      --books-dir "/Users/eva/Documents/RPGBooks/Traveller/General" \
      --out packs/local/mgt2e_items.json
"""
import argparse
import json
import pathlib
import re
import sys

import pymupdf

PRICE_RE = re.compile(r"^(M?Cr)\s*([\d,]+(?:\.\d+)?)$", re.I)
TL_RE = re.compile(r"^(?:TL\s*)?(\d{1,2})\+?$", re.I)
DMG_RE = re.compile(r"^\d+D[D]?(?:[+-]\d+)?$", re.I)


def parse_price_cr(s: str) -> int | None:
    first_line = (s or "").strip().splitlines()[0] if (s or "").strip() else ""
    first_line = first_line.split("/")[0].strip().replace(" ", "")
    m = PRICE_RE.match(first_line)
    if not m:
        return None
    unit, num = m.group(1).lower(), float(m.group(2).replace(",", ""))
    val = int(round(num * 1_000_000)) if unit == "mcr" else int(round(num))
    return val if 0 < val <= 500_000_000 else None


def parse_float(s: str) -> float:
    s = re.sub(r"[^0-9.]", "", s.strip())
    if not s:
        return 0.0
    try:
        return float(s)
    except ValueError:
        return 0.0


def infer_category(header: str, stats: dict, source: str) -> str:
    h = header.lower()
    if "damage" in stats or "weapon" in h or "pistol" in h or "rifle" in h or "melee" in h or "grenade" in h:
        return "weapon"
    if "protection" in stats or "armour" in h or "armor" in h or "vacc" in h:
        return "armor"
    if "augment" in h or "wafer" in h or "neural" in h or "cyber" in h or "bionic" in h:
        return "augment"
    if "bandwidth" in stats or "software" in h or "expert" in h:
        return "software"
    if "vehicle" in source.lower() or "speed" in stats or "shipping" in stats:
        return "vehicle"
    if "robot" in source.lower() or "drone" in h:
        return "robot_drone"
    if "sensor" in h or "scanner" in h or "comm" in h or "computer" in h:
        return "electronics"
    if "drug" in h or "medical" in h or "medikit" in h:
        return "medical"
    return "equipment"


def infer_law_restriction(cat: str, name: str, stats: dict) -> int:
    n = name.lower()
    if any(k in n for k in ("nuclear", "antimatter", "meson", "bio-weapon")):
        return 1
    if cat == "weapon":
        if any(k in n for k in ("plasma", "fusion", "fgmp", "pgmp", "missile", "rocket", "grenade", "artillery")):
            return 1
        if any(k in n for k in ("gauss", "laser", "combat rifle", "acr", "auto", "machine", "submachine")):
            return 3
        if any(k in n for k in ("pistol", "revolver", "shotgun", "rifle", "carbine", "snub")):
            return 5
        if any(k in n for k in ("stun", "blade", "sword", "cutlass", "knife", "baton")):
            return 8
        return 6
    if cat == "armor":
        if "battle dress" in n:
            return 1
        if "combat" in n:
            return 3
        if any(k in n for k in ("flak", "cloth", "poly", "mesh", "ablative")):
            return 7
    return 9


def extract_items_from_pdf(pdf_path: pathlib.Path, pack_id: str) -> list[dict]:
    doc = pymupdf.open(pdf_path)
    items = []
    seen = set()

    for page_idx, page in enumerate(doc):
        # Use PyMuPDF's native table finder + fallback line scanner for Mongoose stat tables
        tabs = page.find_tables()
        page_text_head = page.get_text("text")[:300]
        for tab in tabs.tables:
            rows = tab.extract()
            if not rows or len(rows) < 2:
                continue
            headers = [(c or "").strip().replace("\n", " ") for c in rows[0]]
            hdr_lower = [h.lower() for h in headers]

            # Identify Mongoose equipment/weapon/armor/augment tables by Cost + TL columns
            cost_col = next((i for i, h in enumerate(hdr_lower) if "cost" in h or "price" in h), None)
            tl_col = next((i for i, h in enumerate(hdr_lower) if h in ("tl", "tech level") or h.startswith("tl ")), None)
            if cost_col is None or tl_col is None:
                continue

            name_col = 0 if cost_col != 0 and tl_col != 0 else next((i for i in range(len(headers)) if i not in (cost_col, tl_col)), 0)
            mass_col = next((i for i, h in enumerate(hdr_lower) if "kg" in h or "mass" in h or "weight" in h), None)

            for r in rows[1:]:
                if not r or len(r) <= max(cost_col, tl_col, name_col):
                    continue
                name = (r[name_col] or "").strip().replace("\n", " ")
                name = re.sub(r"\s+", " ", name)
                if not name or len(name) < 2 or len(name) > 90:
                    continue
                if name.lower() in ("item", "weapon", "armour", "armor", "augment", "software", "type", "name"):
                    continue

                cost_cr = parse_price_cr(r[cost_col] or "")
                tl_m = TL_RE.match((r[tl_col] or "").strip())
                if cost_cr is None or not tl_m:
                    continue
                tl = int(tl_m.group(1))
                mass_kg = parse_float(r[mass_col] or "") if mass_col is not None and mass_col < len(r) else 0.0

                stats = {}
                for i, h in enumerate(headers):
                    if i in (name_col, cost_col, tl_col, mass_col) or i >= len(r):
                        continue
                    val = (r[i] or "").strip().replace("\n", " ")
                    if val and val != "—":
                        key = re.sub(r"[^a-z0-9]+", "_", h.lower()).strip("_") or f"col_{i}"
                        stats[key] = val

                # Extract armor stealth / sensor modifiers if mentioned
                traits_str = str(stats.get("traits", "") or stats.get("effect", "") or stats.get("notes", ""))
                stealth_m = re.search(r"(?:stealth|sensor)[^-\d]*(-[1-6])", traits_str, re.I)
                if stealth_m:
                    stats["stealth_dm"] = int(stealth_m.group(1))

                cat = infer_category(" ".join(headers) + " " + page_text_head, stats, pdf_path.name)
                law = infer_law_restriction(cat, name, stats)
                dedup_key = (name.lower(), tl, cost_cr)
                if dedup_key in seen:
                    continue
                seen.add(dedup_key)

                items.append({
                    "pack_id": pack_id,
                    "category": cat,
                    "name": name,
                    "tl": tl,
                    "cost_cr": cost_cr,
                    "mass_kg": mass_kg,
                    "law_restriction": law,
                    "stats": {**stats, "source_book": pdf_path.stem, "source_page": page_idx + 1},
                    "tags": [cat, f"tl{tl}", pdf_path.stem.lower().split()[0]],
                })
    return items


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--books-dir", required=True)
    ap.add_argument("--out", default="packs/local/mgt2e_items.json")
    args = ap.parse_args()

    books_dir = pathlib.Path(args.books_dir)
    target_pdfs = [
        ("csc-2023", "CSC Update 2023 23-04-24.pdf"),
        ("core-2022", "Traveller Core Rulebook Update 2022 Jun23.pdf"),
        ("field-cat", "Field_Catalogue.pdf"),
        ("iiss-gear", "IISS Field Gear.pdf"),
        ("vehicles-2026", "Vehicle Handbook Update 2026.pdf"),
        ("robots", "Robot Handbook Ebook.pdf"),
    ]

    all_items = []
    global_seen = set()
    for pack_id, fname in target_pdfs:
        p = books_dir / fname
        if not p.exists():
            print(f"  skipping missing {fname}", file=sys.stderr)
            continue
        extracted = extract_items_from_pdf(p, pack_id)
        added = 0
        for item in extracted:
            k = (item["name"].lower(), item["tl"])
            if k not in global_seen:
                global_seen.add(k)
                all_items.append(item)
                added += 1
        print(f"  {fname}: extracted {len(extracted)} rows ({added} unique added)", file=sys.stderr)

    out_path = pathlib.Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps({"pack_id": "mgt2e-local-books", "items": all_items}, indent=2))
    print(f"Total unique items exported: {len(all_items)} -> {out_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Download Charted Space sectors from TravellerMap API into a TravellerLLM data pack.

Usage:
  uv run python3 tools/travellermap-sync/sync_sectors.py --out packs/local/sectors.json.gz
  uv run python3 tools/travellermap-sync/sync_sectors.py --official-only --out packs/local/sectors.json.gz
"""
import argparse
import concurrent.futures
import gzip
import json
import pathlib
import sys
import urllib.parse
import urllib.request

UNIVERSE_URL = "https://travellermap.com/api/universe?era=M1105&requireData=1"
SECTOR_TAB_URL = "https://travellermap.com/data/{sector}/tab"


def fetch_json(url: str):
    req = urllib.request.Request(url, headers={"User-Agent": "TravellerLLM-Sync/0.1"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def fetch_text(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "TravellerLLM-Sync/0.1"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read().decode("utf-8", errors="replace")


def parse_ehex(ch: str) -> int:
    ch = ch.upper()
    if "0" <= ch <= "9":
        return ord(ch) - ord("0")
    if "A" <= ch <= "H":
        return 10 + ord(ch) - ord("A")
    if "J" <= ch <= "N":
        return 18 + ord(ch) - ord("J")
    if "P" <= ch <= "Z":
        return 23 + ord(ch) - ord("P")
    return 0


def parse_sector_tab(sector_meta: dict, tab_text: str) -> dict:
    sx = int(sector_meta["X"])
    sy = int(sector_meta["Y"])
    name = sector_meta["Names"][0]["Text"]
    abbr = sector_meta.get("Abbreviation") or name[:4].upper()

    lines = [ln for ln in tab_text.splitlines() if ln.strip() and not ln.startswith("#")]
    if not lines:
        return {"name": name, "abbreviation": abbr, "sector_x": sx, "sector_y": sy, "systems": []}

    header = lines[0].split("\t")
    col = {h.strip(): i for i, h in enumerate(header)}

    systems = []
    for ln in lines[1:]:
        parts = ln.split("\t")
        if len(parts) < 4:
            continue

        def get(key: str, default: str = "") -> str:
            idx = col.get(key)
            return parts[idx].strip() if idx is not None and idx < len(parts) else default

        hx = get("Hex")
        uwp = get("UWP")
        sys_name = get("Name") or f"Unnamed {hx}"
        if len(hx) != 4 or len(uwp) < 9:
            continue
        try:
            hx_col = int(hx[:2])
            hx_row = int(hx[2:])
        except ValueError:
            continue

        # Global odd-q coordinates across sectors (32x40 hexes per sector)
        global_x = sx * 32 + hx_col
        global_y = sy * 40 + hx_row
        pbg = get("PBG", "000")
        gas_giants = parse_ehex(pbg[2]) if len(pbg) >= 3 else 0
        zone_raw = get("Zone", "").upper()
        travel_zone = "Red" if zone_raw == "R" else ("Amber" if zone_raw == "A" else "Green")
        remarks = [t for t in get("Remarks").split() if len(t) == 2 and t[0].isalpha()]
        bases = [b for b in list(get("Bases").replace(" ", "")) if b.isalpha()]

        systems.append({
            "hex": hx,
            "name": sys_name,
            "uwp": uwp,
            "starport": uwp[0],
            "tech_level": parse_ehex(uwp[8]),
            "law_level": parse_ehex(uwp[6]),
            "trade_codes": remarks,
            "bases": bases,
            "allegiance": get("Allegiance", "Im"),
            "travel_zone": travel_zone,
            "gas_giants": gas_giants,
            "global_x": global_x,
            "global_y": global_y,
            "pbg": pbg,
            "stars": get("Stars"),
            "ix": get("{Ix}"),
            "ex": get("(Ex)"),
            "cx": get("[Cx]"),
        })

    return {
        "name": name,
        "abbreviation": abbr,
        "sector_x": sx,
        "sector_y": sy,
        "systems": systems,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="packs/local/charted_space_sectors.json.gz")
    ap.add_argument("--official-only", action="store_true", default=True)
    ap.add_argument("--all-sectors", action="store_true", help="Include non-official/OTU sectors too")
    ap.add_argument("--workers", type=int, default=8)
    args = ap.parse_args()

    uni = fetch_json(UNIVERSE_URL)
    sectors_meta = uni.get("Sectors", [])
    if not args.all_sectors:
        sectors_meta = [s for s in sectors_meta if s.get("Tags", "").find("OTU") != -1 or s.get("Milieu") == "M1105"]
    print(f"Fetching {len(sectors_meta)} sectors from TravellerMap API…", file=sys.stderr)

    def load_one(meta):
        name = meta["Names"][0]["Text"]
        url = SECTOR_TAB_URL.format(sector=urllib.parse.quote(name))
        try:
            tab = fetch_text(url)
            return parse_sector_tab(meta, tab)
        except Exception as e:
            print(f"  skip {name}: {e}", file=sys.stderr)
            return None

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for res in pool.map(load_one, sectors_meta):
            if res and res["systems"]:
                results.append(res)

    total_sys = sum(len(s["systems"]) for s in results)
    out_path = pathlib.Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {"pack_id": "travellermap-charted-space", "sectors": results}
    if out_path.suffix == ".gz":
        with gzip.open(out_path, "wt", encoding="utf-8") as f:
            json.dump(payload, f, separators=(",", ":"))
    else:
        out_path.write_text(json.dumps(payload, separators=(",", ":")))
    print(f"Saved {len(results)} sectors / {total_sys} systems -> {out_path} ({out_path.stat().st_size:,} bytes)")


if __name__ == "__main__":
    main()

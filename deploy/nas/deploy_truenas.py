#!/usr/bin/env python3
"""Deploy or update the 'travellerllm' custom compose app on TrueNAS SCALE.

Pins ghcr.io/elacy/travellerllm:main to its current @sha256 digest so TrueNAS
always pulls fresh builds, mirroring travellerweb's NAS deployment workflow.

Required env vars:
  TRUENAS_API_KEY
  TRAVELLERLLM_PG_PASSWORD
  LITELLM_API_KEY

Optional env vars:
  TRUENAS_URL       (default: wss://nas.lacy.ie:443/websocket)
  APP_NAME          (default: travellerllm)
  HOST_PORT         (default: 3080)
  AUTH_PROXY_SECRET (recommended; must match Traefik X-Auth-Proxy-Secret)
  REFEREE_MODEL     (default: deepseek/deepseek-chat)
  WORKER_MODEL      (default: deepseek/deepseek-v4.1-flash-1017)
"""
import asyncio
import json
import os
import ssl
import sys
import urllib.request

APP = os.environ.get("APP_NAME", "travellerllm")
IMAGE = "ghcr.io/elacy/travellerllm"
TAG = os.environ.get("APP_TAG", "main")
HOST_PORT = int(os.environ.get("HOST_PORT", "3080"))
WS_URL = os.environ.get("TRUENAS_URL", "wss://nas.lacy.ie:443/websocket")
KEY = os.environ.get("TRUENAS_API_KEY", "").strip()
PG_PASS = os.environ.get("TRAVELLERLLM_PG_PASSWORD", "").strip()
LITELLM_KEY = os.environ.get("LITELLM_API_KEY", "").strip()
HINDSIGHT_KEY = os.environ.get("HINDSIGHT_API_KEY", "").strip()
PROXY_SECRET = os.environ.get("AUTH_PROXY_SECRET", "").strip()
REFEREE_MODEL = os.environ.get("REFEREE_MODEL", "openrouter/deepseek/deepseek-v4-pro")
WORKER_MODEL = os.environ.get("WORKER_MODEL", "openrouter/deepseek/deepseek-v4.1-flash")

if not KEY or not PG_PASS or not LITELLM_KEY:
    sys.exit("ERROR: set TRUENAS_API_KEY, TRAVELLERLLM_PG_PASSWORD, and LITELLM_API_KEY")

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE


def resolve_ghcr_digest() -> str:
    repo = IMAGE.replace("ghcr.io/", "", 1)
    with urllib.request.urlopen(
        f"https://ghcr.io/token?scope=repository:{repo}:pull&service=ghcr.io",
        timeout=30,
    ) as r:
        tok = json.load(r)["token"]
    req = urllib.request.Request(
        f"https://ghcr.io/v2/{repo}/manifests/{TAG}",
        headers={
            "Authorization": f"Bearer {tok}",
            "Accept": (
                "application/vnd.oci.image.index.v1+json, "
                "application/vnd.docker.distribution.manifest.list.v2+json, "
                "application/vnd.oci.image.manifest.v1+json, "
                "application/vnd.docker.distribution.manifest.v2+json"
            ),
        },
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        digest = resp.headers.get("Docker-Content-Digest") or ""
    if not digest:
        raise RuntimeError(f"Could not resolve digest for {IMAGE}:{TAG}")
    return f"{IMAGE}@{digest}"


def build_compose(pinned_image: str) -> dict:
    return {
        "services": {
            "postgres": {
                "image": "postgres:17-alpine",
                "restart": "unless-stopped",
                "environment": {
                    "POSTGRES_DB": "travellerllm",
                    "POSTGRES_USER": "traveller",
                    "POSTGRES_PASSWORD": PG_PASS,
                },
                "healthcheck": {
                    "test": ["CMD-SHELL", "pg_isready -U traveller -d travellerllm"],
                    "interval": "10s",
                    "timeout": "5s",
                    "retries": 10,
                    "start_period": "30s",
                },
                "volumes": [
                    {
                        "type": "bind",
                        "source": "/mnt/bulk/hermes/travellerllm/postgres",
                        "target": "/var/lib/postgresql/data",
                    }
                ],
            },
            "travellerllm": {
                "image": pinned_image,
                "restart": "unless-stopped",
                "depends_on": {"postgres": {"condition": "service_healthy"}},
                "environment": {
                    "TZ": "America/Los_Angeles",
                    "PORT": "3080",
                    "FRONTEND_DIR": "/app/frontend/dist",
                    "DATA_PACKS_DIR": "/data/packs",
                    "DATABASE_URL": f"postgresql://traveller:{PG_PASS}@postgres:5432/travellerllm",
                    "AUTH_MODE": "authentik_proxy",
                    "AUTH_PROXY_SECRET": PROXY_SECRET,
                    "AUTH_LOGIN_URL": "https://auth.i.lacy.ie",
                    "AUTH_LOGOUT_URL": "/outpost.goauthentik.io/sign_out",
                    "LITELLM_BASE_URL": "http://10.233.12.5:4000",
                    "LITELLM_API_KEY": LITELLM_KEY,
                    "REFEREE_MODEL": REFEREE_MODEL,
                    "WORKER_MODEL": WORKER_MODEL,
                    "HINDSIGHT_BASE_URL": "http://10.233.12.5:8888",
                    "HINDSIGHT_API_KEY": HINDSIGHT_KEY,
                    "HINDSIGHT_ENABLED": "true",
                },
                "ports": [{"published": HOST_PORT, "target": 3080}],
                "volumes": [
                    {
                        "type": "bind",
                        "source": "/mnt/bulk/hermes/travellerllm/packs",
                        "target": "/data/packs",
                        "read_only": True,
                    }
                ],
                "labels": [
                    "homepage.group=Games",
                    "homepage.name=TravellerLLM",
                    "homepage.href=https://travellerllm.i.lacy.ie",
                    "homepage.description=Mongoose Traveller 2e LLM Engine",
                ],
            },
        }
    }


async def rpc(ws, msg_id: str, method: str, params: list):
    await ws.send(json.dumps({"msg": "method", "method": method, "params": params, "id": msg_id}))
    while True:
        resp = json.loads(await ws.recv())
        if resp.get("id") == msg_id:
            if resp.get("error"):
                raise RuntimeError(f"{method} error: {resp['error']}")
            return resp.get("result")


async def wait_job(ws, job_id: int):
    while True:
        jobs = await rpc(ws, f"job-{job_id}", "core.get_jobs", [[["id", "=", job_id]]])
        j = jobs[0]
        state = j.get("state")
        if state == "SUCCESS":
            return j
        if state in ("FAILED", "ABORTED"):
            raise RuntimeError(f"TrueNAS job {job_id} {state}: {j.get('error')}")
        await asyncio.sleep(2)


async def main():
    import websockets

    pinned = resolve_ghcr_digest()
    print(f"Resolved image: {pinned}")
    compose = build_compose(pinned)

    ws = await websockets.connect(WS_URL, ssl=ctx, max_size=64 * 1024 * 1024)
    try:
        await ws.send(json.dumps({"msg": "connect", "version": "1", "support": ["1"]}))
        await ws.recv()
        ok = await rpc(ws, "auth", "auth.login_with_api_key", [KEY])
        if ok is not True:
            raise RuntimeError("TrueNAS API key login failed")

        existing = await rpc(ws, "list", "app.query", [[["name", "=", APP]]])
        if existing:
            print(f"Updating existing TrueNAS custom app '{APP}'…")
            job_id = await rpc(
                ws,
                "upd",
                "app.update",
                [APP, {"custom_compose_config": compose}],
            )
        else:
            print(f"Creating TrueNAS custom app '{APP}'…")
            job_id = await rpc(
                ws,
                "cre",
                "app.create",
                [{"app_name": APP, "custom_app": True, "custom_compose_config": compose}],
            )
        await wait_job(ws, job_id)
        print(f"Done: '{APP}' deployed on port {HOST_PORT} -> https://travellerllm.i.lacy.ie")
    finally:
        await ws.close()


if __name__ == "__main__":
    asyncio.run(main())

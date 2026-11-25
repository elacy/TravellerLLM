# Deploying TravellerLLM to `https://travellerllm.i.lacy.ie` (TrueNAS + Traefik + Authentik)

`*.i.lacy.ie` already resolves via wildcard DNS to `10.233.12.5` (Traefik v3.5). Because `https://traveller.i.lacy.ie` is used by `travellerweb`, TravellerLLM uses **`https://travellerllm.i.lacy.ie`**.

## 1. Prepare NAS directories
On `nas.lacy.ie`:
```sh
mkdir -p /mnt/bulk/hermes/travellerllm/postgres /mnt/bulk/hermes/travellerllm/packs
```

## 2. Create Authentik Proxy Provider & Application
In **https://auth.i.lacy.ie** (Admin → Applications):
1. Create a **Proxy Provider** (`Forward auth (single application)`):
   - External host: `https://travellerllm.i.lacy.ie`
2. Create an **Application** (`TravellerLLM`, slug `travellerllm`) linked to that provider.
3. Attach the application to the **authentik Embedded Outpost** (Applications → Outposts → Edit).

## 3. Add Traefik Dynamic Route
Generate a shared proxy secret (`openssl rand -hex 32`), substitute `<AUTH_PROXY_SECRET>` in `traefik-travellerllm.yml`, and copy it into Traefik's watched dynamic folder on `nas.lacy.ie`:
```sh
cp traefik-travellerllm.yml /mnt/bulk/hermes/traefik/dynamic/travellerllm.yml
```

## 4. Deploy TrueNAS Custom Compose App
Either paste `compose.yaml` into TrueNAS UI (**Apps → Discover Apps → Install via YAML**), or run the reconciler script from any machine on the LAN:
```sh
export TRUENAS_API_KEY="..."
export TRAVELLERLLM_PG_PASSWORD="$(openssl rand -hex 24)"
export LITELLM_API_KEY="sk-..."
export AUTH_PROXY_SECRET="<same secret as Traefik>"
uv run --with websockets python3 deploy/nas/deploy_truenas.py
```

Verify:
- `curl -sI https://travellerllm.i.lacy.ie` → `302` redirect to `https://auth.i.lacy.ie/...`
- After signing in with Authentik, open `https://travellerllm.i.lacy.ie`!

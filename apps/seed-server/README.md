# seed-server

A tiny single-seed server for Modrinthium. One running instance hosts exactly
one seed (one modpack line): it serves a manifest and the `.mrpack` files it
points at, and accepts authenticated uploads to publish new versions straight
from the Modrinthium launcher.

## Run

```sh
SEED_KEY="a-long-random-secret" \
SEED_PUBLIC_URL="https://seed.example.com" \
SEED_DATA_DIR="./data" \
SEED_BIND="0.0.0.0:8000" \
SEED_NAME="My SMP" \
cargo run -p seed-server --release
```

| Variable          | Required | Default        | Purpose                                             |
| ----------------- | -------- | -------------- | --------------------------------------------------- |
| `SEED_KEY`        | yes      | —              | Secret bearer token required to publish.            |
| `SEED_PUBLIC_URL` | yes      | —              | Public base URL, used to build absolute file links. |
| `SEED_DATA_DIR`   | no       | `./data`       | Where the manifest and files are stored.            |
| `SEED_BIND`       | no       | `0.0.0.0:8000` | Address to listen on.                               |
| `SEED_NAME`       | no       | `Seed`         | Fallback pack name before anything is published.    |

Put a reverse proxy with TLS in front of it and point `SEED_PUBLIC_URL` at the
public HTTPS URL.

## Endpoints

- `GET /manifest.json` — the seed manifest. **This is the link users paste into
  the launcher.** Returns `404` until the first publish.
- `GET /files/{name}` — serves an uploaded `.mrpack` or icon.
- `POST /publish` — publish a new version. Requires
  `Authorization: Bearer $SEED_KEY`. `multipart/form-data` fields:

  | Field       | Required | Notes                                             |
  | ----------- | -------- | ------------------------------------------------- |
  | `mrpack`    | yes      | The `.mrpack` file.                               |
  | `version`   | yes      | Version string, e.g. `1.4.0`.                     |
  | `name`      | no       | Pack name (falls back to the previous / `SEED_NAME`). |
  | `changelog` | no       | Free text.                                        |
  | `servers`   | no       | JSON array of `{ "address": "...", "name": "..." }`. |
  | `icon`      | no       | Image file; kept until replaced.                  |

The launcher's **Publish to seed** flow fills all of this in for you — you only
enter the server URL and key once.

## Manual publish example

```sh
curl -X POST https://seed.example.com/publish \
  -H "Authorization: Bearer $SEED_KEY" \
  -F "mrpack=@pack.mrpack" \
  -F "version=1.4.0" \
  -F "changelog=Added Sodium" \
  -F 'servers=[{"address":"play.example.com","name":"My SMP"}]'
```

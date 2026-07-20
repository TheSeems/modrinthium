# Modrinthium

Modrinthium is a de-branded, telemetry-free fork of the
[Modrinth App](https://modrinth.com/app) — an open-source Minecraft launcher for
mods and modpacks.

> **Modrinthium is an independent fork. It is not affiliated with, endorsed by,
> or sponsored by Rinth, Inc. or the Modrinth project.** "Modrinth" and the
> Modrinth logo are trademarks of Rinth, Inc.

> [!WARNING]
> Work in progress

## What's different from the Modrinth App

- **De-branded** — Modrinth branding, logo, and wordmark removed; the app is
  named "Modrinthium".
- **No telemetry** — analytics (PostHog), the in-app ad webview, and the in-app
  survey system have been removed.
- **No Modrinth account required** — the Modrinth account sign-in / social layer
  has been removed from the app; you still sign in to Minecraft with your
  Microsoft account to play.
- **Seed modpacks** — install and auto-update a modpack straight from a hosted
  manifest URL, without it being published on Modrinth, plus a tiny companion
  server ([`apps/seed-server`](./apps/seed-server)) for hosting one.

See [`NOTICE.md`](./NOTICE.md) for the full list of changes and attribution.

## License

- The app and its libraries (`apps/app`, `apps/app-frontend`, `packages/*`) are
  licensed under **GPL-3.0-only**.
- The backend (`apps/labrinth`) is licensed under **AGPL-3.0**.

This project preserves the upstream copyright notices and remains under the same
licenses. Modrinth branding assets are not covered by these licenses and have
been removed; see [`COPYING.md`](./COPYING.md).

## Development

This is a pnpm + Turborepo monorepo. See [`CLAUDE.md`](./CLAUDE.md) for the
architecture overview and per-app guides. Common commands:

- App (desktop): `pnpm app:dev`
- Storybook (UI package): `pnpm storybook`

By default the app talks to Modrinth's hosted API (`api.modrinth.com`). To run
fully independently, self-host the backend (`apps/labrinth`, AGPL-3.0) and point
the app at it via `MODRINTH_API_URL` in `packages/app-lib/.env`.

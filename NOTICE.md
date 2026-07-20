# NOTICE

Modrinthium is a modified version (a "fork") of the **Modrinth App** and the
supporting libraries in the Modrinth monorepo.

## Original work

- Copyright © 2020–2025 Rinth, Inc. and the Modrinth contributors.
- The desktop app and its libraries (`apps/app`, `apps/app-frontend`,
  `packages/*`) are licensed under the **GNU General Public License, version 3
  only (GPL-3.0-only)**. See [`LICENSE`](./apps/app/LICENSE).
- The backend service (`apps/labrinth`) is licensed under the **GNU Affero
  General Public License, version 3 (AGPL-3.0)**. See
  [`apps/labrinth/LICENSE.txt`](./apps/labrinth/LICENSE.txt).

Modrinthium continues to be distributed under those same licenses.

## Changes made in this fork

In accordance with GPL-3.0 §5(a), this is a modified version. The significant
changes from the upstream Modrinth App include:

- Rebranded the desktop application from "Modrinth App" to "Modrinthium" and
  removed the Modrinth logo, wordmark, and other Modrinth branding assets.
- Removed the Modrinth account sign-in / social features from the app UI.
- Removed analytics and telemetry (PostHog), the in-app advertisement webview,
  and the in-app user-survey system.
- Added a self-hosted "seed" modpack feature (install and update a modpack from
  a hosted manifest URL) and a companion minimal seed server (`apps/seed-server`).
- Changed the network User-Agent strings to identify as Modrinthium.

## Trademarks

"Modrinth", the Modrinth wrench-in-labyrinth logo, and related marks are
trademarks of Rinth, Inc. They are **not** licensed under the GPL and are used
here only nominatively to describe this project's origin. The Modrinth branding
assets have been removed from the shipped application.

**Modrinthium is an independent fork. It is not affiliated with, endorsed by, or
sponsored by Rinth, Inc. or the Modrinth project.**

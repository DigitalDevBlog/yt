# Phase 4 Context: Upload output to Capacities

**Status:** Discussed — ready for planning (opens a new milestone, e.g. v0.3 "Integrations")
**Created:** 2026-07-15
**Research:** Likely (Capacities new-API endpoints, space ID, inbox targeting)

## Goal

Push the Claude summary of a watched YouTube video into Capacities as an atomic
node, to build a personal knowledge base of videos. Triggered by a flag.

## Goals (user-stated)

- The **Claude summary** ends up in Capacities.
- Added as an **atomic node**, placed on the **inbox** page.
- The node includes the **video title** and, preferably, a **link to the video**.
- Triggered by a flag: `-u` / `--upload`.
- Purpose: a growing knowledge base of YouTube videos the user has watched.

## Approach

- **REST API, not MCP.** Capacities exposes an MCP server (OAuth 2.1) *and* a REST
  API (Bearer token). For a Rust CLI the REST API is the right fit; MCP is built
  for LLM agents and is overkill here.
- **Endpoint fit — `save-weblink`** is near-perfect: creates an atomic weblink
  node from a URL, with `titleOverwrite` (video title), `mdText` (the summary,
  added to notes), and `tags`. The URL *is* the link-to-video, satisfying most of
  the requirements in one call.
- **Flag semantics (recommended):** `-u`/`--upload` **implies `-s`** (you can't
  upload without a summary). Still write the local `<title>.md` as well, unless
  decided otherwise.
- **Config:** reuse `~/.config/yt/.env`. Both added by the user:
  - **`CAPACITIES_IO_API_KEY`** (validated present, 48 chars)
  - **`CAPACITIES_IO_SPACE_ID="Research & Study"`**
  ⚠️ NOTE: that value is a space **name/title**, but the API's `spaceId` is a
  **UUID**. The tool must **resolve name → UUID** via the spaces endpoint (accept
  either a name or a raw UUID). This also becomes the foundation for multi-space
  support later.

## Decision: API generation — LOCKED (Option A, new API)

**Which Capacities API generation?**
- The user's token is a **new-API (v2.0)** token. It returned
  `401 cap_not_authenticated: "requires a legacy Public API token"` against
  `GET https://api.capacities.io/spaces`.
- The endpoints found in `https://api.capacities.io/openapi.json` are the
  **Beta API (v0.1.0)**: `GET /spaces`, `GET /space-info`, `POST /lookup`,
  `POST /save-weblink`, `POST /save-to-daily-note`. **Beta shuts down 2026-09-01.**
- **DECIDED — Option A: target the new API** (`developers.capacities.io`, full
  CRUD, uses the token already created) — future-proof for a keep-forever KB tool.
  Beta `save-weblink` rejected (deprecated 2026-09-01; needs a legacy token).

## Reference — Beta `save-weblink` body (for shape; confirm new-API equivalent)

```
POST https://api.capacities.io/save-weblink   (Authorization: Bearer <token>)
  spaceId          string   *required
  url              string   *required   (the YouTube URL)
  titleOverwrite   string   optional    (video title)
  descriptionOverwrite string optional
  tags             string[] optional    (must match existing tag names exactly)
  mdText           string   optional    (markdown → notes section; the summary)
```
Rate limit: 120 req / 60s per user per endpoint.

## Open questions (for planning / research)

1. **New-API endpoints:** exact path + body for creating a weblink/object on the
   v2.0 API (`developers.capacities.io`); how to get the space ID there.
2. **Inbox targeting:** no explicit "inbox" field on the Beta endpoints — a
   weblink lands in the space's weblink collection. Can the new API target the
   **Inbox** (or a collection) directly? If not: tag-based routing, or accept the
   default location, or use `save-to-daily-note` instead? (User specifically wants
   the inbox.)
3. **`-u` implies `-s`?** Confirm the flag runs summarize then upload.
4. **Keep local `<title>.md`** on upload, or upload-only? (Recommend: keep.)
5. **Tags:** create/require a `YouTube` (or similar) tag? Tags must pre-exist and
   match exactly on the Beta API.
6. **Errors:** clear messaging when token/space/inbox is missing or the API 4xxs;
   don't fail the whole run if only the upload step fails (mirror the graceful
   degradation used elsewhere).

## Deferred to a future phase (user request)

- **Multiple spaces + multiple API keys** — e.g. route different video categories
  to different Capacities spaces/accounts. Out of scope for Phase 4, which targets
  the single `CAPACITIES_IO_SPACE_ID`. The name → UUID resolution above is a
  deliberate stepping stone toward this.

## Constraints / notes

- Add the upload as a new step in the existing `Steps` progress UI (e.g.
  "Uploading to Capacities"), on stderr.
- Keep stdout clean; upload status/errors to stderr.
- This is a new milestone — Phase 4 must be added to ROADMAP.md
  (`/paul:add-phase` or `/paul:milestone`) before/at planning.

---
*Sources:* Capacities MCP docs, `docs.capacities.io/developer/api`,
`api.capacities.io/openapi.json` (Beta), `developers.capacities.io` (new API),
"What's new: API 2.0" (release-67).

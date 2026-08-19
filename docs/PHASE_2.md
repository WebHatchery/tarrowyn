# Phase 2 — The Persistent Settlement

> Implementation status: complete for the documented vertical slice. The
> release proof is recorded in [`PHASE_2_RUNBOOK.md`](PHASE_2_RUNBOOK.md).
>
> Current validation focus: deterministic tests and automated HTTP acceptance.
> The human 60–90 minute settlement session is deferred until the client is
> more playable.

## Purpose

Make the shared settlement worth returning to. Phase 2 adds the smallest
persistent social and economic loop from the GDD: three players can occupy
different roles, exchange useful goods or information, and find the same world
after a server restart.

The server becomes authoritative for the accelerated clock, characters,
inventory, plots, crops, chat history needed by the current session, and direct
trades. The client renders projections and submits intent-like commands.

## Build scope

### Persistent domain

Add server-owned records for:

- accounts, characters, and last-seen/session state;
- the world clock and calendar position;
- the tiny settlement, tavern, and shared farm plots;
- crop type, plot state, growth stage, planting/tending/harvesting history;
- inventory and gold with server-side validation;
- direct trade offers, acceptance, expiry, and atomic exchange;
- bounded chat messages and a tavern notice/rumour feed; and
- an append-only or cursor-addressable settlement event record sufficient for
  reconnects and debugging.

Keep storage behind repositories and migrations. A browser client must not use
toolkit local persistence as a substitute for this data. If a command is
repeated because of reconnect or retry, the server uses a request ID or other
idempotency key so it cannot duplicate gold, crops, or items.

### Farming loop

Implement three data-defined crops from the Phase 0 definitions. A player can
plant, tend, and harvest a plot only when the server says the character is near
the plot and has the required seed/tool state. Growth follows the one shared
server clock and continues while players are offline, while active tending
improves quality, reliability, or harvest readiness.

The first loop should remain small and legible:

```text
arrive at fields → plant → tend during a later visit → harvest
      ↓                                      ↓
 inventory changes                 another player can trade/use the crop
```

### Social and economic loop

- Give the tavern a real social location with chat, notices, and a rumour feed.
- Add direct player trading with an explicit review/accept flow and visible
  pending/complete/expired states.
- Let a primarily farming player, primarily adventuring placeholder, and
  social/trading player all contribute something another player can use.
- Add server-side fallbacks for an empty settlement: a limited NPC/travelling
  service may keep essential actions possible, but should not erase the value
  of player supply.

## Client/toolkit work

Continue using `macroquad_toolkit::net::HttpClient` and `Pending<T>` for all
HTTP requests. Add an application-owned request coordinator that tracks:

- pending request type and request ID;
- retry cooldown and maximum retry count;
- connection state (`Connecting`, `Online`, `Degraded`, `Offline`);
- the last accepted server cursor/tick; and
- whether an action is awaiting authoritative confirmation.

The UI must distinguish “command sent”, “server accepted”, “server rejected”,
and “request timed out”. Optimistic visual feedback is allowed for movement
intent, but inventory, trade completion, gold, crop growth, and skill progress
must update only from an accepted server projection.

## Minimum protocol additions

| Endpoint | Purpose |
|---|---|
| `GET /v1/state` | Return the authenticated player projection and current world cursor. |
| `POST /v1/farming/actions` | Plant, tend, or harvest with an idempotent request ID. |
| `GET /v1/inventory` | Return authoritative inventory and currency. |
| `POST /v1/trades` | Create, review, accept, cancel, or expire a direct trade. |
| `GET /v1/trades` | Return the player’s active and recently completed offers. |
| `GET /v1/tavern/feed` | Return bounded notices, rumours, and recent social context. |
| `GET /v1/events?since=<cursor>` | Continue the Phase 1 event stream with domain changes. |

Protocol types should be versioned and tested against malformed, stale, replayed,
and partially missing requests. The server should return a stable error code
plus readable context; the client can localize/present the message later.

## Acceptance test

The phase passes its product test when three real players can complete one
60–90 minute session in which:

1. one player plants/tends/harvests;
2. one player gathers or prepares a useful substitute/service;
3. one player trades, scouts, or keeps the tavern/social loop active;
4. at least one item or piece of information crosses between players;
5. all three see the same day/night progression and settlement changes;
6. a server restart preserves accounts, characters, clock, plots, crops,
   inventory, and completed trades; and
7. a repeated or retried command does not duplicate a reward or exchange.

## Explicitly deferred

The full combat model, monster ecology, complex NPC households, governance,
aging land claims, frontier expeditions, generational characters, PvP, and
kingdom-scale settlement growth remain Phase 3 or later. Phase 2 should prove
interdependence and persistence without building a spreadsheet-sized economy.

## Exit artifact

The current automated gate is `cargo test --workspace`, the Phase 2 HTTP
acceptance script, the protocol compatibility tests, migration coverage, and
the concurrent development-client soak included in the Phase 3 acceptance
script. The human three-client farming/trade session is intentionally deferred
until the client is more playable.

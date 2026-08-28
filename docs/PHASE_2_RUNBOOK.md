# Phase 2 persistence and acceptance runbook

## Server state

The native server persists the authoritative settlement after every accepted
mutation and every clock tick. Set an explicit path when running a disposable
development world:

```powershell
$env:TARROWYN_STATE_PATH = "D:\temp\tarrowyn-phase2.json"
cargo run -p tarrowyn-server
```

The document contains `storage_version`, the accelerated clock, accounts and
characters, farm plots, inventories, bounded tavern history, trades, and the
cursor-addressable event history. Sessions and bearer tokens are deliberately
not persisted. Repeating the same guest `client_key` creates a fresh session
for the same durable character; using `reset: true` creates a new development
identity.

The current repository storage version is `12`. JSON migrations read older
documents through serde defaults and write the upgraded document. The MySQL
backend applies its checked-in schema migration at startup and keeps the same
storage version inside its authoritative snapshot. Any future migration must
fill new fields with safe defaults and keep the HTTP protocol unchanged.
Do not use the client’s local toolkit save slots for online inventory, crops,
gold, or trades.

## Phase 2 endpoints

All endpoints except `/health` and `/v1/session/guest` require the bearer token
returned by the guest-session response.

| Endpoint | Use |
|---|---|
| `GET /v1/state` | Atomic player, world, farm-plot, clock, and tavern projection. |
| `POST /v1/farming/actions` | `plant`, `tend`, or `harvest` with `request_id` and plot position. |
| `GET /v1/inventory` | Current authoritative inventory, gold, skill, and reputation. |
| `POST /v1/trades` | `create`, `review`, `accept`, or `cancel` a direct offer. |
| `GET /v1/trades` | Active and recently completed offers addressed to the player. |
| `GET /v1/tavern/feed` | Bounded notices, rumours, and recent chat. |
| `GET /v1/events?since=<cursor>` | Cursor-addressable shared changes, including crops and trades. |

Every farming and trade command is idempotent per durable account and request
ID. The server validates location, ownership, inventory, recipient, status,
and expiry while holding the repository lock, then persists the resulting
projection. A retry therefore returns the original accepted response instead
of applying a second reward or exchange.

## Acceptance pass

Run the deterministic tests and the live three-player Phase 2 check from the
game directory:

```powershell
cargo test --workspace
.\scripts\verify_phase2.ps1
```

The script starts a disposable server, creates three guest identities, checks
the shared state and tavern feed, walks one player to the fields, plants and
harvests across server ticks, retries the plant and trade commands, exchanges
seeds for gold, posts tavern chat, stops and restarts the server, and confirms
that the character, clock, and completed trade remain available.

For a visual browser/native check, start the server and client using the main
README instructions. The online sidebar exposes visible `Plant`, `Tend`, and
`Harvest` controls beside the movement pad; the player ledger is refreshed from
`/v1/state`, and command notices distinguish the sent/accepted/rejected path.

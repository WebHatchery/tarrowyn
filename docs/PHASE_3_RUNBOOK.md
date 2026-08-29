# Phase 3 frontier and acceptance runbook

## Durable state

The server now stores the Phase 3 frontier beside the Phase 2 settlement in
the versioned JSON document: the Brambleback zone, per-account contracts and
request results, household opportunity state, chronicle entries, one renewable
homestead lease, and the pioneer expedition/outpost. Set a disposable path
when testing restarts:

```powershell
$env:TARROWYN_STATE_PATH = "D:\temp\tarrowyn-phase3.json"
cargo run -p tarrowyn-server
```

The storage version is `2`. Loading a Phase 2 version-1 document fills the
new frontier state from safe defaults; guest sessions and bearer tokens remain
ephemeral, while the same `client_key` resumes the durable character.

## Phase 3 endpoints

All endpoints except `/health` and `/v1/session/guest` require the bearer token
returned by guest session creation.

| Endpoint | Use |
|---|---|
| `GET /v1/contracts` | Read the repeatable Brambleback tavern contract. |
| `POST /v1/contracts/brambleback-watch` | `accept`, `progress`, or `report` the contract with an idempotent `request_id`. |
| `POST /v1/combat/actions` | Submit an authoritative `strike` or `retreat` with `iron_sword` or `improvised_club`. |
| `POST /v1/recovery` | Choose `self_recover`, `ask_rescuer`, or `pay_healer` after knockout. |
| `GET /v1/settlement/chronicle?since=<cursor>` | Read cursorable settlement history. |
| `GET /v1/settlement/opportunities` | Read visible household demand and service clues. |
| `POST /v1/claims` | `request`, `renew`, `abandon`, or `inspect` the recognised homestead lease. |
| `POST /v1/expeditions` | `announce`, `join`, `supply`, `launch`, or `resolve` the pioneer expedition. |

The Brambleback initially closes the north road, raises harvest value demand,
changes the rumour feed, and drives the repeatable contract and household
signals. An iron-sword strike clears it; the improvised club deliberately
demonstrates the knockout/recovery path. Stored goods remain safe while one
carried seed may be lost, and all commands return stable cursor metadata.

## Acceptance pass

Run the deterministic repository/protocol/client fixtures and the live HTTP
vertical slice from the game directory:

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
.\scripts\verify_phase3.ps1
```

The script creates three named roles, reads a contract, walks a scout to the
wilderness, reports progress, demonstrates an inferior-weapon knockout and
rescuer recovery, checks that another player can read the chronicle and the
household signal, establishes a claim, resolves a three-role pioneer group,
reconnects at the accepted event cursor, and runs three rounds of concurrent
state, event, movement, and chat polling for twenty clients.

The repository tests additionally cover the same domain rules without a live
process, including durable claim/expedition state and request replay.

## Client controls

The online sidebar exposes touch targets for `Contract`, `Strike`, `Recover`,
`Claim`, `Pioneer`, and `Chronicle`. While the Brambleback threat is active,
the Contract slot becomes the visible `Retreat` action; `Contract`, `Claim`,
and `Pioneer` otherwise advance their next server-owned action from the
current projection, so the browser does not require a keyboard or hidden
command string. The frontier projection is frame-polled through the toolkit
HTTP transport and never mutates inventory, injury, contract, claim, or
expedition state before server confirmation.

## Production review / deferred work

Phase 3 intentionally keeps one monster, one household, one claim, and one
outpost. Kingdom governance, permanent death, full NPC family simulation,
PvP law, broad topology, and final authentication/deployment remain deferred
as recorded in [`PHASE_3.md`](PHASE_3.md).

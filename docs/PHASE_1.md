# Phase 1 — The Shared Road

## Purpose

Turn the Phase 0 client into the first real multiplayer build. A player should
be able to start a development session, enter the same tiny settlement as
other players, move through the server-owned map, see other characters, and
exchange short messages. This phase proves that the game’s basic client/server
shape is correct before farming and economy systems depend on it.

This is the first phase where `GameSession` stops being the authority for shared
world state. It becomes a client projection and a place for temporary loading or
disconnected presentation only.

## Toolkit integration

Enable the optional transport in the client crate:

```toml
macroquad-toolkit = { path = "../macroquad-toolkit", features = ["net"] }
```

The client owns a small API/session module around the toolkit:

```rust
use macroquad_toolkit::net::{HttpClient, Pending};
use tarrowyn_protocol::StateSnapshot;

let mut api = HttpClient::new(server_url);
api.set_bearer_token(Some(account_token));
let mut state: Pending<StateSnapshot> = api.get("/v1/state");
// Each update tick:
if let Some(result) = state.poll_timed(dt, 6.0) {
    // Adopt the snapshot or show a connection error.
}
```

The original Phase 1 contract exposed `/v1/world` as the initial projection.
The current client uses authenticated `/v1/state`, whose `data.world` carries
that projection alongside the authenticated player and tavern feed. The
server keeps `/v1/world` for compatibility with the early acceptance script.

The client must retain pending requests, poll them once per frame, and put
retries behind an application-owned cooldown. A timed-out request is a state
transition (`Degraded` or `Offline`), not a reason to issue another request on
the next frame. The server and protocol types remain game-owned; do not copy
the transport into the project or add a blocking HTTP client.

## Build scope

### Server foundation

- Add a separate Rust server crate or workspace member with a clear process
  entry point and development configuration.
- Add `/health` and `/v1/session/guest` endpoints for local development. Guest
  sessions receive a stable development account/character identity that can be
  upgraded to real account linking later.
- Choose and document the server HTTP framework and the first persistence
  backend. Keep this decision isolated behind server repositories; the client
  must not know whether storage is SQLite, Postgres, or another backend.
- Own a server tick, accelerated world clock, world dimensions, walkability,
  character position, and connection/session expiry.

### Shared protocol

Create a versioned protocol crate or module containing the types shared by the
client and server. The first contract should include:

| Endpoint | Purpose |
|---|---|
| `GET /health` | Process and protocol compatibility check. |
| `POST /v1/session/guest` | Create or resume a development account and character. |
| `GET /v1/world` | Return the initial world projection, clock, and player presence. |
| `POST /v1/movement` | Submit an intent with a client request ID; server accepts or rejects it. |
| `GET /v1/events?since=<cursor>` | Poll presence, clock, and chat changes after a cursor. |
| `POST /v1/chat` | Submit a bounded message for the current social channel. |

Every response carries a protocol version, server tick, and request/cursor
metadata. Movement is never accepted because the client’s local map says it is
valid; the server checks bounds, collision, session identity, and rate limits.

### Client changes

- Replace local movement mutation with a movement intent and a server result.
- Render a connecting/loading/degraded/offline state with visible recovery
  actions.
- Show remote players using server snapshots/events, including a stale-player
  indicator when a presence update has aged out.
- Add a small chat panel with a visible send target and message length limit.
- Keep Phase 0’s map and touch controls so the online build remains playable in
  a browser.
- Preserve the local first-evening fixture as an offline development mode, but
  label it clearly and never mix its state with an online session.

The client presents the authoritative clock as a time and a shared
morning/afternoon/evening/night period. The period is derived from the server
clock for online play and from the same boundary helper in the offline fixture;
future time-specific services must retain an alternate access path.

## Acceptance test

Run one local server and place at least three clients in the settlement. The
test passes when:

1. all clients receive distinct character identities;
2. each client sees the others move through the same collision map;
3. an invalid movement intent is rejected by the server and corrected in the
   client;
4. a message sent by one client appears for the other two in order;
5. stopping the server produces a readable degraded/offline state, and a
   reconnect action restores the world without freezing the frame loop; and
6. the server clock advances once for the world, not once per connected client.

## Explicitly deferred

Farming, inventory, direct trade, combat, NPC migration, land leases, permanent
accounts, production authentication, and a public deployment are Phase 2 or
later. Phase 1 should make the wire and authority model boring before adding
more things to replicate.

## Exit artifact

Deliver a local developer runbook covering server start, client configuration,
guest-session reset, protocol versioning, test fixtures, and how to capture a
three-client verification session. Update the client’s `game_page.json` status
only after the three-client test passes repeatedly.

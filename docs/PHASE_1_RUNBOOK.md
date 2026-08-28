# Phase 1 local developer runbook

## Start the server

The development server is a separate workspace member with a clear native
process entry point:

```powershell
cd D:\WebHatchery\RustGames\tarrowyn
cargo run -p tarrowyn-server
```

It listens on `127.0.0.1:8787` by default. The process owns one background
tick loop. The default tick is 250 ms and advances the accelerated world clock
by one world second, so an 80-minute in-game day lasts 20 real minutes.

Useful server configuration variables are:

| Variable | Default | Purpose |
|---|---:|---|
| `TARROWYN_SERVER_ADDR` | `127.0.0.1:8787` | Bind address. |
| `TARROWYN_TICK_MS` | `250` | Server tick interval. |
| `TARROWYN_WORLD_SECONDS_PER_TICK` | `1` | Accelerated clock step. |
| `TARROWYN_DAY_LENGTH_SECONDS` | `4800` | World day length (80 real minutes). |
| `TARROWYN_SESSION_TTL_SECONDS` | `30` | Guest session expiry window. |
| `TARROWYN_CHAT_MAX_LENGTH` | `160` | Server chat bound. |

The deterministic local backend is a versioned JSON repository. Shared preview
can select the transactional MySQL backend with `DB_DRIVER=mysql`; both are
isolated behind `WorldRepository` so the protocol and HTTP handlers stay
unchanged. Keep preview credentials in the ignored `.env.preview` contract.

## Start the client

In a second terminal:

```powershell
$env:TARROWYN_SERVER_URL = "http://127.0.0.1:8787"
$env:TARROWYN_CLIENT_KEY = "desk-one"
Remove-Item Env:TARROWYN_OFFLINE -ErrorAction SilentlyContinue
cargo run -p years_of_tarrowyn
```

The client requests a guest session, adopts the server world snapshot, then
polls `/v1/events?since=<cursor>` once per frame. Movement and chat are
queued as intent requests; the client never changes online position based on
its local collision map.

## Guest reset and protocol versioning

Guest sessions are stable when the same `client_key` is sent again. A reset
creates a fresh development account and character for that key. The reset also
clears the old guest's private progression and replay caches, releases its
travel and land ownership, and closes its unsettled development orders; public
chronicle history remains a world fixture:

```powershell
$body = '{"client_key":"desk-one","reset":true}'
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8787/v1/session/guest `
  -ContentType 'application/json' -Body $body
```

The wire contract lives in `protocol/src/lib.rs`; `PROTOCOL_VERSION` is `1`.
Update that value and the client/server contract together when a breaking
response or request shape is introduced. All responses include `meta` with
protocol version and server tick. Movement and chat responses also echo the
client request ID, while world and event responses carry the cursor.

## Fixtures and acceptance test

Run deterministic server, protocol, client projection, and Phase 0 fixture
tests with:

```powershell
cargo test --workspace
```

Run the live HTTP three-client check with:

```powershell
.\scripts\verify_three_clients.ps1
```

For three visible client windows, set a different `TARROWYN_CLIENT_KEY` in each
terminal. The server also assigns a unique key when no key is supplied, and
the response returns that key for reconnect. Place all three clients at the starting Hearth, tap the
movement pad in one window, and observe the same presence event in the other
two. Tap an invalid map direction or target to see the server rejection and
corrected position. Use a quick phrase in each chat panel and confirm cursor
order.

To capture the preserved offline fixture without requiring a server:

```powershell
$env:TARROWYN_OFFLINE = "1"
.\scripts\capture_ui.ps1 -Scenes title,gameplay -Frames 8
```

The online state renders `CONNECTING`, `ONLINE`, `DEGRADED`, or `OFFLINE` in
the header. A stopped server produces a readable message and a visible
`Reconnect` button; it does not freeze the Macroquad frame loop.

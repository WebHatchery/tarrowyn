# Phase 5 regional runbook

## Clean fixture

Stop the native server before resetting a fixture. The default state and backup
are named files, so reset only those files:

```powershell
Remove-Item -LiteralPath "dist/tarrowyn-phase5-state.json" -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "dist/tarrowyn-phase5-state.json.backup" -ErrorAction SilentlyContinue
$env:TARROWYN_STATE_PATH = "dist/tarrowyn-phase5-state.json"
$env:TARROWYN_BACKUP_PATH = "dist/tarrowyn-phase5-state.json.backup"
```

Start the server with `cargo run -p tarrowyn-server`. Create a development
guest with `POST /v1/session/guest`, then use its bearer token for the regional
requests below. The guest path is a fixture and is not a production account.

## Regional inspection and recovery

Inspect `/v1/region`, `/v1/settlements`, `/v1/routes`,
`/v1/households/region`, and `/v1/law`. Start a journey with a unique
`TravelRequest.request_id`. The server response supplies the durable
`travel_id`. Re-submit the same request ID to verify idempotency. Interrupt
the journey, reconnect or issue `Recover`, and confirm that the original
travel ID continues to the destination exactly once. In the touch client,
`Travel` becomes `Interrupt` while a journey is active; after an interruption,
the dedicated `Recover` control is enabled and `Travel` remains unavailable
until recovery is chosen.
The online sidebar's `Repair` control exercises the route action for the first
non-operational road connected to the current location: initially the north
pack road at the Hearth, or the watch trail after arriving at Whisperwood.
Player travel treats every recorded road or ferry as bidirectional, so after
arriving at Saltmere the `Travel` control can return over the Saltmere ferry.
Confirm the route response and refreshed road status before repeating the action
with the same request boundary.

Tap `Inspect` to open the authoritative regional detail panel. It lists each
visible route by name with status, risk, and condition, followed by the first
market stock and price notes. Use the visible `Escort road` control to reduce
route risk, or `Improve road` to increase route capacity and shorten its travel
time. Both actions select the first open route connected to the current
location and remain server-authoritative; the panel also remains the place to
read the detail behind the compact sidebar telemetry.

## Market and event inspection

Use `/v1/market/orders` to record stock notes, price notes, and open orders.
Create a small seed or crop order from The Hearth to Saltmere, travel to the
destination, and fulfil it. Record the order status, inventory projection,
gold, and price index before and after settlement. Before fulfilment, also
cancel an owned open order and confirm that its escrow returns to origin stock
and its status becomes `Cancelled`; the online client exposes this through the
visible `Cancel` control. Seed a regional event with
`POST /v1/events/region`, wait for escalation, choose an intervention, and then
resolve it. Poll with the returned cursor; a cursor ahead of the server must
return a readable `cursor_ahead` error. A cursor older than the retained event
window must return `cursor_stale` rather than an incomplete event list.
The client handles that boundary without disconnecting: it clears the regional
map, settlement, household, market, law, and event projections, returns to
cursor zero, and reloads the latest regional state. The Phase 6 client recovery
path also resets this regional stream when the shared settlement event cursor
reports a restore.

## Identity, operations, and support

Use the visible Account control or `POST /v1/auth/link` with the configured
`webhatchery-identity-oidc` provider fixture. Call `/v1/account`, refresh with
`POST /v1/auth/refresh`, and revoke with `POST /v1/auth/revoke`. Verify that a
revoked access token cannot read the account. The visible Report control
creates a queued moderation report; `/v1/support/repair` is the audited local
operator repair surface for stuck travel, inventory normalisation, and failed
orders. Never include access or refresh tokens in an audit note.

`GET /v1/ops/health` is safe for readiness checks. Authenticated operators may
read `/v1/ops/metrics`. The backup record in the health response must show the
last successful backup tick and path. Run `scripts/phase5_region_soak.ps1`
after changing the route or interest-radius fixture.

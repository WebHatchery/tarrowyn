# Phase 4 runbook — The Enduring Society

## Start the local authority

From the project directory:

```powershell
$env:TARROWYN_STATE_PATH = "dist/tarrowyn-phase4-state.json"
cargo run -p tarrowyn-server
```

In a second terminal:

```powershell
$env:TARROWYN_SERVER_URL = "http://127.0.0.1:8787"
cargo run -p years_of_tarrowyn
```

The server owns all online state. The client uses frame-polled toolkit HTTP
requests and never treats a button press as accepted until the response arrives.

## Reset and migration

Stop the server before resetting a fixture. Remove only the named Phase 4
fixture file when a clean world is required:

```powershell
Remove-Item -LiteralPath "dist/tarrowyn-phase4-state.json" -ErrorAction SilentlyContinue
```

An existing Phase 1–3 JSON state may be reused directly. The missing `phase4`
field migrates to the default governance, infrastructure, claim, profession,
knowledge, household, and local-combat records. A migrated identity keeps its
character, inventory, gold, crops, Phase 3 claim, chronicle, and the default
Bellweather animal record.

Useful deterministic settings for quick checks are:

```powershell
$env:TARROWYN_GOVERNANCE_INACTIVITY_TICKS = "2"
$env:TARROWYN_LEASE_DURATION_TICKS = "2"
$env:TARROWYN_CLAIM_RECLAIM_GRACE_TICKS = "1"
$env:TARROWYN_HOUSEHOLD_DECISION_INTERVAL_TICKS = "1"
```

## Endpoint and fixture checks

After obtaining a guest token from `POST /v1/session/guest`, inspect:

- `GET /v1/settlement/governance` and `GET /v1/infrastructure` for offices,
  vacancies, treasury, decisions, condition, upkeep, and failure notes;
- `GET /v1/claims` for available land and `POST /v1/claims/lifecycle` for
  request, approve, renew, transfer, inherit, abandon, reclaim, and inspect;
- `GET /v1/professions` and `POST /v1/professions/orders` for the material,
  credential, accept, and complete loop;
- `GET /v1/knowledge` and `POST /v1/knowledge` for discover, record, teach,
  and apply; and
- `GET /v1/households`, `GET /v1/combat/local`, and
  `POST /v1/combat/local` for local-life clues and combat recovery.

The repository fixtures cover these checks without a live server. Run the
focused Phase 4 fixture with:

```powershell
cargo test -p tarrowyn-server phase4
```

For the live HTTP, restart, and two-account acceptance pass, run:

```powershell
.\scripts\verify_phase4.ps1
```

## Touch-control pass

In the online client, use only visible controls:

1. Tap `Town hall` repeatedly to claim the Steward office, propose, approve,
   and complete north-road repair. Read the public cost in the response notice.
2. Tap `Registry` to request and approve a plot, then renew it. If testing
   reclamation, abandon it and tap `Registry` again after the grace interval.
3. Tap `Order` to learn Carpentry, create the displayed service order, and
   accept/complete it from a second client.
4. Tap `Care` beside the shared fields to tend Bellweather and read the
   animal condition and Animal Husbandry practice in the player ledger.
5. Tap `Knowledge` to discover and apply the Moonberry trellis method. Use the
   second client to verify that a taught item appears after transfer.
6. Tap `Households` and read the service clue before changing road or service
   conditions.
7. Walk near Whisperwood, tap `Local fight`, and use the visible buttons to
   prepare, try the first-exchange `Technique`, then strike or guard. Use
   `Bandage`, `Reposition`, or `Spell` as the encounter permits, and tap the
   existing `Recover` control after a knockout.

`Reconnect` is the recovery path for timeouts and rejected commands; no step in
this checklist requires a physical keyboard.

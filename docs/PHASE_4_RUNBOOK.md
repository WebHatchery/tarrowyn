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
$env:TARROWYN_LEASE_DURATION_SECONDS = "2"
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
  `POST /v1/combat/local` for local-life clues and combat recovery. Combat
  responses include `action_available_at_tick`; the default one-tick server
  action window rejects same-tick bursts and is configurable with
  `TARROWYN_COMBAT_ACTION_COOLDOWN_TICKS`.

The repository fixtures cover these checks without a live server. Run the
focused Phase 4 fixture with:

```powershell
cargo test -p tarrowyn-server phase4
```

For the live HTTP, restart, and three-role acceptance pass, run:

```powershell
.\scripts\verify_phase4.ps1
```

## Touch-control pass

In the online client, use only visible controls:

1. Tap `Town hall` repeatedly to claim the Steward office, propose, approve,
   and complete north-road repair. Read the public cost in the response notice.
2. Tap `Registry` to request and approve a plot, then renew it. Read the
   registry summary for the lease status and remaining real-time countdown.
   Use the visible `Abandon` control to release it, or stand beside another
   recognised player and use `Transfer` to pass the active lease. For
   reclamation, tap `Registry` again after the grace interval.
3. Tap `Order` to learn Carpentry, create the displayed service order, and
   accept/complete it from a second client.
4. Tap `Care` beside the shared fields to tend Bellweather and read the
   animal condition and Animal Husbandry practice in the player ledger.
5. Tap `Knowledge` to discover, then tap it again to record the Moonberry
   trellis method. Stand beside the second client and tap the now-labelled
   `Teach` control to transfer it; the next tap becomes `Apply`. Use the second
   client to verify that the taught item appears.
6. Tap `Households` and read the service clue before changing road or service
   conditions.
7. Walk near Whisperwood, tap `Local fight`, and use the visible buttons to
   prepare, try the first-exchange `Technique`, then strike or guard. Use
   `Retreat` to leave an active encounter without a knockout. Use `Bandage`,
   `Reposition`, or `Spell` as the encounter permits. After a
   knockout, read the visible carried-loss risk, healer gold cost, and stored
   property safety, then choose `Self`, `Rescuer`, or `Healer` from the visible
   recovery row. After each accepted action, read the visible `Action ready`
   or `Action opens in … beat` status before sending the next action.

`Reconnect` is the recovery path for timeouts and rejected commands; no step in
this checklist requires a physical keyboard.

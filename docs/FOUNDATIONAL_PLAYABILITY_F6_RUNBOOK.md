# F6 connected storehouse runbook

Status: connected automated acceptance passed on 2026-09-02. The proof covers
deterministic construction authority, visible stage projections, and durable
completion. Whether the pace and presentation feel satisfying still requires
the human first-hour study in F7.

## Accepted outcome

Mara's first communal project requires exactly eight timber and six stone.
Players may contribute carried goods or replace timber at two gold per unit and
stone at three gold per unit. The server owns the project ledger and advances
it through marked site, dry-stone foundation, raised timber frame, and one
operational public storehouse. Every contribution records its actor and input;
completion retains the distinct contributors and creates exactly one durable
infrastructure record.

The connected proof uses one hauler, one patron, and an independent observer.
The hauler gathers the actual goods at the shared woodland and seam, while the
patron funds part of the stone requirement beside Mara. Stable request IDs
make retries return their first result before and after authority restart.

## Automated connected scenario

From the repository root, run:

```powershell
.\scripts\verify_foundation_storehouse.ps1
```

The harness creates a unique JSON world under the system temporary directory,
uses a server on `127.0.0.1:8875`, restores the caller's environment, and
removes the isolated world afterward. It:

1. Validates exactly 132 audit rows and the F6 status totals, then runs focused
   client touch-path and server authority tests.
2. Creates separate hauler, patron, and observer sessions in one connected
   world.
3. Walks the hauler to the seam and woodland and gathers exactly two stone and
   eight timber through the real resource endpoint.
4. Reads the exact need from the nearby noticeboard and contributes at the
   physical storehouse site or beside Mara.
5. Mixes seven accepted contributions: two stone and eight timber as goods,
   plus twelve gold credited as the remaining four stone.
6. Requires an independent state read at each of the four stages, with exact
   project revision and ledger length.
7. Replays material and completion requests and verifies identical responses,
   unchanged inventory/gold, and one infrastructure record.
8. Restarts the authority, reconnects all identities, replays completion, and
   requires unchanged contributors, revision, completion-event count, balances,
   and infrastructure count in the current storage version with healthy integrity.

## Visible touch path

Use two clients against the same authority; a third spectator is useful for
confirming that progress is public rather than local UI state.

1. Walk beside **NEEDS** and tap **Read local need**. Confirm it names eight
   timber, six stone, and the exact gold substitutes.
2. Walk beside the woodland and seam. Tap **Gather timber** and **Mine stone**;
   confirm the same carried inventory later appears in the contribution deck.
3. Walk beside **SITE** or **MARA**. Confirm separate touch actions offer
   carried goods and affordable exact gold substitutions. Keep **All tools**
   available but secondary.
4. Contribute until the first threshold. Confirm the map changes from the
   marked site to **Dry-stone foundation** and the remaining need updates.
5. Continue with a second player until **Raised timber frame** appears. Confirm
   both clients observe the same stage after their next state refresh.
6. Supply the exact remainder. Confirm **Operational storehouse** appears at
   the site and **Use storehouse** replaces contribution actions.
7. Retry the last action, reconnect both clients, and restart the authority.
   Confirm no second charge, contribution, completion message, structure, or
   reward appears.

Record viewport sizes, input type, every offered nearby label, project revision
at each stage, the two contributor identities, balances before and after, and
any confusing moment. Automation proves correctness and reachability, not
subjective clarity, pacing, or enjoyment.

## Authority and retry boundaries

- `/v1/foundation/resources` owns proximity, depletion, recovery, and the
  inventory used by construction.
- `/v1/foundation/storehouse` owns inspection, exact goods/gold validation,
  atomic charges, contribution attribution, stage changes, and completion.
- `/v1/state` projects the canonical project and current stage consumed by the
  connected client; the client never predicts a successful contribution.
- `/v1/infrastructure` exposes the one completed public-building record.
- Stable request IDs return their original responses across transient retry and
  restart. A different request after completion is rejected rather than
  reopening the ledger.
- Mara, the board, and the site are fixed low-population access points. No
  specialist, profession, or second player gates completion; multiple players
  simply share attribution.

## Verification record

On 2026-09-02 the connected harness passed its focused tests and live
three-identity scenario. The integrated F6 tree then passed the full workspace
tests, clippy with warnings denied, formatting, diff hygiene, and the Rust
source-size limit. `publish.ps1` built and packaged Windows and WebGL releases
and deployed the preview successfully.

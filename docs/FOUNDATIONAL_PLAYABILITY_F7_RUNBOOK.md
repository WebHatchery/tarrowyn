# F7 connected first-hour journey runbook

Status: connected automated acceptance passed on 2026-09-02. The proof covers
the deterministic journey, short/long session outcomes, persistence, and a
durable reason to return. It does not claim that a human found the experience
enjoyable, naturally preferred one activity, or considered it memorable.

## Accepted outcome

Every fresh identity arrives with one credited milestone and an authoritative,
personal twelve-step journey. It moves through First Beacon orientation, the
current communal need, planting, woodland and seam exploration, logging,
mining, a lasting field tool, direct barter, storehouse contribution, harvest,
and replant. The policy is guided-not-gated: no milestone unlocks an activity,
no profession or class is selected, and **All tools** remains available as a
secondary inspection and recovery surface.

The useful short visit targets fifteen minutes and requires only consulting the
need and leaving a crop growing. The cohesive longer visit targets sixty
minutes and records all twelve canonical activities. Completing it activates
**Return for the next harvest**; a later mature-crop harvest completes that
future goal once without erasing the first-hour record.

## Automated connected scenario

From the repository root, run:

```powershell
.\scripts\verify_foundation_journey.ps1
```

The harness creates a unique JSON world in the system temporary directory,
uses `127.0.0.1:8876`, restores the caller's environment, and removes its
isolated state afterward. It:

1. Requires exactly 132 audited requirements with F7 totals of 60 usable, 31
   partial, 27 missing, one conflicting, and 13 deliberately deferred rows.
2. Runs focused client guidance and server journey-authority tests.
3. Creates a short-visit identity, consults the local need, plants a shared
   crop, and requires the contract's exact fifteen-minute milestone set.
4. Creates a separate first-hour identity and neighbour, then follows map
   movement and canonical HTTP actions through all twelve ordered milestones.
5. Uses real logging and mining yields to prepare charcoal and a handle and
   forge the six-action iron field tool.
6. Completes a one-stone-for-one-gold direct barter, replays acceptance with
   the same request ID, and contributes remaining stone at the physical site.
7. Models a fifteen-minute absence, restarts authority, reconnects the same
   identity, and requires the unchanged ten-credit ledger before harvest.
8. Harvests and replants, replays the planting response, and requires revision
   13 with all twelve milestones and the active return goal.
9. Models the return interval, restarts again, harvests the mature replanted
   crop, replays that request, and requires revision 14, one completed future
   goal, current storage version, and healthy repository integrity.

## Visible touch/click playthrough

Use a fresh browser identity. Keep a second connected client nearby for the
barter step. Do not use developer commands or edit stored state.

1. At the Beacon, confirm the nearby line reads **NEXT 2/12** and points to
   Mara or **NEEDS**. Approach either and use the visible context action.
2. Walk to an empty shared plot and tap **Plant crop**. Confirm progress points
   out of camp rather than opening a permanent task menu.
3. Walk to the marked woodland, tap **Gather timber**, then follow guidance to
   the shallow seam and tap **Mine stone** until two ore are carried.
4. At the rough forge, follow the visible preparation actions for charcoal and
   handle, then forge the iron field tool. Confirm its `6/6` condition appears.
5. Complete one direct trade with the second client. Either the nearby ore
   shortcut or the general trade ledger is valid; verify the same authoritative
   progress appears after returning to the world view.
6. Approach Mara or **SITE**, contribute an eligible material or exact gold
   substitute, and confirm the next step points back to the growing field.
7. When the planted crop is mature, tap **Harvest crop**, then **Plant crop**.
   Confirm progress reads `12/12` and changes to **RETURN GOAL** rather than
   claiming the whole game is finished.
8. Reload during any unfinished step. The last accepted milestone must return;
   an unavailable journey read must leave ordinary nearby actions usable.
9. Return after the replanted crop matures and harvest it. Confirm the return
   goal completes once and all broader tools remain accessible.

## Human observation record

For each participant, record the build/commit, browser and input method,
elapsed time at the short and long checkpoints, any step where direction was
unclear, whether **All tools** distracted from the nearby action, whether the
participant voluntarily chose a different activity, and their own words about
usefulness or memorability. Until that record exists, the audit intentionally
keeps subjective enjoyment and preference rows partial or missing.

## Release gate

After the connected harness passes, require:

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
.\publish.ps1
```

The Windows and WebGL preview builds must both succeed. Public HTTPS,
target-MySQL concurrency/restore, configured identity/TLS/secrets/alerts, and
human multi-session evidence remain tracked by the separate deployment gate;
this isolated JSON acceptance does not relabel them as complete.

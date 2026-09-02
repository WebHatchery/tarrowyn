# F4 connected production runbook

Status: connected automated acceptance passed on 2026-09-02. The fixed
comparison proves a concrete tool-economy benefit; it does not claim that the
current recipe pacing is final or subjectively enjoyable.

## Accepted outcome

A newcomer can gather every required input rather than receive hidden recipe
materials. Two mine actions supply two iron ore, and one woodland action
supplies two timber. At the rough forge, the touch-first nearby deck shows
carried counts, exact missing costs, tool kind and condition, then offers the
next useful action: **Burn charcoal**, **Shape tool handle**, or **Forge iron
field tool**. **Inspect forge** remains available when no recipe step is ready,
and **All tools** remains visible as the recovery path.

The authority atomically turns one timber into one charcoal, one timber into
one handle, and two ore plus those prepared parts into an iron field tool. The
shared crude fallback performs three useful field actions; the iron tool
performs six. Smithing has no class or credential gate.

## Automated connected scenario

From the repository root, run:

```powershell
.\scripts\verify_foundation_forge.ps1
```

The harness creates a unique JSON world under the system temporary directory,
uses a server on `127.0.0.1:8873`, restores the caller's environment, and
removes the isolated world afterward. It:

1. Runs focused client nearby-forge tests and server forge-authority tests.
2. Walks one authenticated identity to the shallow seam and woodland, gathers
   exactly two ore and two timber, and returns to the physical forge.
3. Inspects three typed recipes and their projected 3-action crude and
   6-action iron capacities.
4. Burns charcoal, shapes a handle, and forges the tool, replaying every
   preparation/crafting request to prove costs and outputs occur once.
5. Uses a fresh crude identity for exactly three accepted field tends and
   verifies a fourth useful action is rejected after replanting.
6. Uses the iron tool for exactly six accepted tends across two crop cycles
   and verifies a seventh useful action is rejected.
7. Restarts the authority, recovers the same smith identity and exhausted iron
   tool, replays the tool request, and proves no material or condition returns.
8. Requires the restarted repository to remain ready with integrity checks
   passing.

## Visible player path

1. Walk beside the shallow seam and tap **Mine stone** twice. Confirm two iron
   ore are visible in inventory.
2. Walk beside the woodland and tap **Gather timber** once. Confirm two timber
   are visible.
3. Return beside the rough forge. Confirm the nearby line names every material,
   the crude tool's condition, and the exact iron-tool recipe.
4. Tap **Burn charcoal**, then **Shape tool handle**. After each result, confirm
   the counts update and the next useful forge choice replaces the prior one.
5. Tap **Forge iron field tool**. Confirm the feedback names an iron field tool
   at `6/6`, and reconnect to confirm it persists.
6. Tend crops until the tool reaches zero. Compare this with a fresh character's
   crude `3/3` tool; the fixed acceptance result is six actions versus three.

Record viewport, input type, wall-clock duration, each offered nearby label,
and any confusing travel or recipe transition. Human pacing observations may
change recipe balance later, but must not weaken authority, replay safety, or
the crude fallback.

## Authority and retry boundaries

- `/v1/state` owns position, inventory, tool kind, and tool condition.
- `/v1/foundation/resources` owns bounded timber and ore extraction.
- `/v1/foundation/forge` revalidates exact proximity, knockout state, recipe
  costs, and the improved-tool state before an atomic write.
- Stable request IDs preserve the original full response across transient
  retries and server restarts. They cannot consume another input, mint another
  output, or restore spent tool condition.
- `/v1/farming/actions` consumes condition from the authoritative typed tool
  ceiling. The crude tool remains the default and is never profession-gated.

## Verification record

On 2026-09-02 the harness passed 10 client nearby-context tests, 3 server forge
tests, and the complete live HTTP scenario. The integrated F4 tree then passed
810 tests across protocol, server, client, and integration targets, clippy
with warnings denied, formatting, diff hygiene, and the Rust source-size limit.
`publish.ps1` built and packaged Windows and WebGL releases and deployed the
preview successfully.

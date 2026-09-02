# F3 useful short-session runbook

Status: connected automated acceptance passed on 2026-09-02. This is a
repeatable pacing and usability proof, not a claim about subjective player
enjoyment.

## Accepted outcome

A returning farmer can approach any shared plot and use the nearby deck
without a keyboard. The deck identifies an empty, growing, or mature crop and
offers exactly one useful action: **Plant crop**, **Tend / water**, or
**Harvest crop**. Its outlook names crop stage, tool condition, weather, pest
pressure, and the optional value of tending.

The authoritative server owns crop progress and applies bounded elapsed time
when a persisted world reopens. Crops mature without repetitive tending.
Tending remains worthwhile because it advances a young crop, improves quality,
and buffers environmental pressure, while visibly consuming field-tool
condition.

## Automated connected scenario

From the repository root, run:

```powershell
.\scripts\verify_foundation_farming.ps1
```

The harness uses a unique JSON world under the system temporary directory and
a server on `127.0.0.1:8872`. It restores the caller's environment and removes
the isolated world afterward. The fixed scenario:

1. Runs the client nearby-field tests, farming-authority tests, and offline
   crop-growth tests.
2. Starts a fresh connected world, confirms three shared plots and readable
   tool, weather, and pest state, then walks the farmer to `(10,8)`.
3. Plants one seed and inspects the authoritative crop at stage 0.
4. Stops the server and moves only the snapshot wall-clock marker back 15
   minutes. With a one-second server tick, one world second per tick, and a
   five-minute crop stage, the modeled absence is exactly three growth stages.
5. Restarts and proves the untouched crop reached stage 3 with at least 900
   elapsed growth ticks. The world tick is not rewritten by the harness.
6. Harvests once, retries the same request ID, and proves the retry returns the
   original result without a second crop award.
7. Replants, chooses the optional **Tend / water** action, and proves stage,
   quality, and tool condition change once only.
8. Restarts again and proves identity, the replanted crop, tending history,
   tool condition, replay records, and repository readiness survived.

The 15-minute marker is an accelerated stand-in for the farmer's absence. The
same elapsed-time implementation is bounded to seven real days and is covered
for future timestamps, legacy snapshots, deterministic reopening, and
file-backed persistence.

## Visible player path

For a manual connected pass, start the server and client using the standard
development instructions, then:

1. Walk beside a shared field. Confirm the **NEARBY** deck replaces a generic
   inspect action with the crop outlook and a large touch action.
2. At an empty plot, tap **Plant crop**. Confirm one seed is consumed and the
   plot becomes a named stage-0 crop.
3. Leave and return after the crop has progressed. Confirm the deck shows the
   new stage and offers **Tend / water**, while explicitly calling it optional.
4. Let one crop mature without tending. Confirm the deck says it is ready and
   offers **Harvest crop**.
5. Harvest, then tap **Plant crop** on the same plot. Confirm the crop award,
   seed cost, and replanted state remain visible after reconnecting.
6. On another young crop, tap **Tend / water** and confirm the improved stage
   or quality plus the reduced field-tool condition.

Record wall-clock start/end, viewport and input type, accepted actions, and any
confusing or repetitive moment. A human observation is still required before
claiming that the visit *feels* worthwhile; this automated proof establishes
that the fixed work and outcome fit the intended short-session boundary.

## Authority and retry boundaries

- `/v1/state` is the source of crop, inventory, field condition, weather, pest,
  position, and clock truth.
- `/v1/farming/actions` checks the authoritative player position and exact
  shared plot before accepting `plant`, `tend`, or `harvest`.
- Client buttons choose a nearby eligible projected plot, but the server
  revalidates it and returns the authoritative result.
- A stable `request_id` returns the stored original result across retries and
  restarts; it cannot spend another seed, duplicate a harvest, or consume more
  tool condition.
- Offline progress advances crop growth only. It does not simulate unrelated
  world ticks or turn maintenance into a login tax.

## Verification record

On 2026-09-02 the harness passed 8 client nearby-field tests, 11 server farming
tests, 5 offline-growth tests, and the complete live HTTP scenario. The full
workspace gate then passed 797 tests across protocol, server, client, and
integration targets, clippy with warnings denied, formatting, diff hygiene,
and the Rust source-size limit. `publish.ps1` built and packaged Windows and
WebGL releases and deployed the preview successfully.


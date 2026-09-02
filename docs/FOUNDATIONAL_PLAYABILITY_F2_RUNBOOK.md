# F2 — Living off the land

Status: the project-27 resource boundary and automated connected acceptance
completed on 2026-09-02.

F2 makes the nearby woodland, shallow mine, crude-tool rack, and shared cache
productive through the same touch-first context introduced at the First
Beacon. The server owns resource quantities, recovery time, player inventory,
cache capacity, command replay, and persistence. The client selects a nearby
action and changes its projection only after the authority accepts it.

This runbook follows the exact project-27 F2 scope: crude tools, logging,
mining, depletion/recovery, and shared storage. The larger design matrix still
tracks personal tent placement separately as missing; this proof does not
recast that unimplemented feature as usable.

## Player path

1. Start the authority and client, then enter the First Beacon with an
   authoritative player position.
2. Walk east and north to the **Shallow stone seam**. The nearby control must
   read **Mine stone**. Tap it and confirm the inventory gains stone and iron
   ore.
3. Walk east to the **Whisperwood edge**. The nearby control must read
   **Gather timber**. Tap it and confirm timber enters the authoritative
   inventory.
4. Continue logging until the shared timber deposit is empty. The next action
   must explain the depletion rather than grant another yield. After the
   recovery interval, logging must become productive again.
5. Return to the **Shared cache**. When carrying resources, its nearby control
   must offer **Store timber**, **Store stone**, or **Store iron ore**. After
   storing the carried material, it must offer a matching **Collect** action
   when appropriate; an empty exchange falls back to **Inspect cache**.
6. Restart the authority and reconnect with the same client identity. Carried
   resources, cache contents, node state, and recovery progress must remain.

The tool rack projects a shared hand axe and stone pick available to every
player. They are authority prerequisites for the logging and mining commands,
not client-owned inventory items or profession gates.

## Automated connected acceptance

Run from the project root:

```powershell
.\scripts\verify_foundation_resources.ps1
```

The harness creates a unique temporary JSON repository and uses a live HTTP
server. It first runs the focused client and server foundation suites, then:

- verifies the projected shared hand axe and stone pick;
- walks an authenticated character to the mine and woodland;
- proves stone/ore and timber yields;
- depletes the timber node, rejects another yield, waits for deterministic
  recovery, and gathers again;
- inspects the shared cache, deposits timber, replays the same request ID,
  and withdraws timber;
- stops and restarts the server against the same repository;
- verifies durable identity, carried inventory, cache contents, and resource
  recovery state;
- replays both cache and logging request IDs after restart and proves neither
  duplicates inventory; and
- requires `/v1/ops/health` to remain ready with integrity checks passing.

On 2026-09-02 the connected harness passed with 11 focused client tests and 16
focused server tests. The integrated F2 tree also passed the full workspace
suite (21 protocol, 507 server, 258 client, and two integration tests), clippy
with warnings denied, formatting, the 800-line source limit, and `publish.ps1`
with Windows/WebGL preview deployment.

## Authority and retry boundaries

`POST /v1/foundation/resources` validates the stable node/action pair,
authoritative proximity, knockout state, and shared crude-tool access. A
successful logging action removes one deposit unit and grants two timber. A
successful mining action removes bounded stone and ore units and grants two
stone plus one iron ore while ore remains. Server ticks recover deposits up to
their configured capacities.

`POST /v1/foundation/cache` supports typed inspect, deposit, and withdraw
commands. It validates proximity, ownership, amount, and the cache's 64-item
capacity before moving material atomically. Both endpoints retain bounded
per-identity results keyed by request ID. The client keeps the same request
body and ID across transient retries, and persisted replay records prevent a
lost response or restart from duplicating goods.

The wire contract remains protocol version 7. Client and server must be
deployed together.

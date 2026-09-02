# F1 — Arriving at the First Beacon

Status: implementation and automated acceptance complete on 2026-09-02; the
human observation scenario below remains the repeatable product check.

F1 turns the F0 content fixture into the first connected player experience.
The WebGL client now presents the authored tent camp instead of the mature
regional overlay, renders every visible First Beacon landmark, and makes the
nearest world context the primary bottom-deck action. Advanced Phase 2–6
systems remain available behind **All tools** but do not dominate arrival.

## Player path

1. Open three clients with distinct browser profiles or development client
   keys and wait until each shows **ONLINE** at the First Beacon.
2. Confirm all three player figures are present in the same camp.
3. In each client, press and hold on the map toward the tents. A physical
   keyboard is not required.
4. Move beside **MARA**, then tap **Talk to Mara**. Her authoritative response
   must introduce the first storehouse and point to the noticeboard.
5. Move beside **NEEDS**, then tap **Read local need**. The board must name
   timber for the frame and stone for the foundation.
6. Close all three clients, restart the authority, and reopen the same client
   profiles. Each character must return at its last authoritative camp
   position and see the same fixture and other connected players.
7. Repeat **Read local need** after reconnecting to prove that proximity and
   interaction authority were restored with the shared state.

The expected arrival view is stored in
[`verification/ui_gameplay.png`](verification/ui_gameplay.png). The short
**MARA** and **NEEDS** labels remain visible as spatial anchors, while only the
nearest landmark receives the detailed context and action.

## Automated acceptance

Run from the project root:

```powershell
.\scripts\verify_foundation_arrival.ps1
```

The gate runs the focused protocol, server, projection, and contextual-UI
tests. It then starts an isolated JSON-backed authority and creates three
distinct clients. Every client must:

- arrive at `(8,6)` with all three presences and the same
  `first-beacon-baseline-v1` fixture;
- walk north into the camp;
- receive accepted, proximity-checked responses from Mara and the
  noticeboard; and
- reconnect after an authority restart at `(8,5)`, recover the three-player
  shared state, and use the noticeboard again.

On 2026-09-02 the gate passed. The final F1 tree also passed `publish.ps1`
with no parameters, producing and deploying both Windows and WebGL preview
artifacts. The connected WebGL preview was opened in the in-app browser and
visually checked before `verification/ui_gameplay.png` was refreshed through
the deterministic capture harness.

## Authority and deferral boundaries

`POST /v1/foundation/interactions` accepts a request only when its stable
interaction ID belongs to the server-loaded fixture and the authoritative
character position is within one tile of the referenced landmark. F1 enables
arrival, tent inspection, the communal fire, Mara, and the noticeboard.

The cache, crude tools, woodland, mine, forge, and storehouse site are visible
and contextual, but their productive actions deliberately remain unavailable
until F2, F4, and F6. The endpoint returns an explicit later-milestone response
instead of pretending those loops exist.

The wire contract is protocol version 7. Client and server must be deployed
together.

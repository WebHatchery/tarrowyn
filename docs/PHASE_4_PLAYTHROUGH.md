# Phase 4 settlement playthrough report

## Scope

This report records the deterministic two-character settlement pass used for
the Phase 4 repository fixtures. `Phase4 Steward` is the established player;
`Phase4 Provider` is the newer player who learns a capability and answers a
service order. The same sequence is available as the touch-control checklist
in [`PHASE_4_RUNBOOK.md`](PHASE_4_RUNBOOK.md).

## Observed sequence

1. The Steward claims the vacant Settlement Steward office. The town-hall
   ledger shows the authority boundary and the vacancy fallback.
2. The Steward proposes, approves, and completes north-road repair. The public
   treasury falls by 8, the road condition reaches 100, a decision contains
   actor/cost/service/tick fields, and the chronicle receives the completion.
3. The first character requests and approves a lease, renews it, transfers it
   to the Provider, abandons it, and reclaims it. The first character's gold
   remains 12 and its unrelated inventory/progression are preserved.
4. The first character escrows wood, iron, and a tool for a field-tool repair.
   The Provider learns Carpentry, accepts the order, completes it, receives
   gold and skill progress, and leaves a completed quality record for the
   requesting role.
5. The field ledger reports the current weather and pest pressure. An
   unattended growth pulse loses bounded quality under pressure, while recent
   tending protects the pulse; the same ledger shows the field-tool condition.
6. Bellweather, the shared-field goat, appears in the world projection at
   condition 2/3. After a shared-day rollover it falls to 1/3; the first
   character taps Care beside the fields, restores the animal to 3/3, and
   records Animal Husbandry practice.
7. The first character discovers the Moonberry trellis method and teaches it
   to the Provider. The Provider applies it from its own server-owned knowledge
   list.
8. The Bellweather household remains visible with miller and herbal-healer
   members, complementary work, needs, service quality, and causal clues. Its
   bounded decision updates on the shared tick.
9. A character walks to Whisperwood Edge, prepares the local encounter, and
   defeats it with two iron-sword strikes. The encounter records bounded health,
   turn count, readable prompts, and stored-property safety. The improvised
   weapon path can instead produce a bounded knockout and the visible recovery
   path.

## Reconnect and migration result

Every mutating fixture is request-idempotent and returns the same result when
the request is retried. The repository persists Phase 4 alongside the Phase 3
world document at storage version 19. Loading a document without `phase4`
creates safe defaults while retaining the existing account and world records.
The complete automated proof is `cargo test --workspace`; the browser-facing
publisher proof remains `publish.ps1`.

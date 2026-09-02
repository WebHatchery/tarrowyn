# F5 connected cooperation runbook

Status: connected automated acceptance passed on 2026-09-02. The fixed result
proves a one-action economic saving through voluntary practice and direct
barter. It does not claim that players subjectively prefer these roles; that
remains human evidence for the cohesive first-hour study.

## Accepted outcome

The First Beacon goal asks two connected players to make one iron field tool
from two timber and two iron ore. An unrestricted newcomer self-supplies it in
six accepted gather/forge actions: mine twice, log once, burn charcoal, shape a
handle, and forge. Four voluntary Mining practices raise Mining to mastery two;
that player can extract the two required ore in one accepted mining action.
A logger/smith gathers two timber in one action, accepts the miner's atomic
two-ore offer, and performs the same three forge actions. The authoritative
result is therefore five actions, two contributors, one accepted trade
reference, and one action saved.

No class, profession, credential, or online specialist gates the recipe. The
solo six-action path remains available through shared crude tools. Practice
changes efficiency, not permission.

## Automated connected scenario

From the repository root, run:

```powershell
.\scripts\verify_foundation_cooperation.ps1
```

The harness creates a unique JSON world under the system temporary directory,
uses a server on `127.0.0.1:8874`, restores the caller's environment, and
removes the isolated world afterward. It:

1. Runs focused client touch-cooperation and server authority tests.
2. Creates separate miner and logger/smith sessions in one connected world.
3. Uses four explicit Mining practice requests and verifies mastery two.
4. Walks both players to their physical resources; the miner extracts two ore
   in one action and the smith gathers two timber in one action.
5. Creates and accepts the exact two-ore direct offer, repeats acceptance with
   the same request ID, and verifies one atomic inventory change plus a
   two-action cooperation ledger.
6. Walks the smith to the rough forge and completes charcoal, handle, and tool,
   then requires exactly `5` work actions, `1` saved action, two participants,
   contribution totals, and the accepted trade ID.
7. Creates an uncommitted third player who completes the same tool through two
   mines, one log, and three forge actions without replacing the cooperative
   result.
8. Restarts the authority, reconnects both identities, replays trade and forge
   requests, verifies no duplicated ore or changed result, checks storage
   version 25, and requires healthy persistent integrity.

## Visible touch path

Use two clients against the same authority.

1. Miner: tap **All tools**, **Practice**, choose **Mining**, and repeat until
   Mining shows mastery two. No role selection or profession is required.
2. Miner: walk beside the shallow seam and tap **Mine stone**. The nearby goal
   explains that mastery two supplies both ore in one accepted action.
3. Smith: walk beside the woodland and tap **Gather timber** once.
4. Miner: with the smith connected, tap **Offer 2 ore** in the nearby deck.
   Confirm the feedback says the target is five actions together versus six
   solo.
5. Smith: tap **Accept 2 ore**. The next-work feedback names charcoal, handle,
   and the final forge.
6. Smith: walk beside the rough forge and tap **Burn charcoal**, **Shape tool
   handle**, then **Forge iron field tool**.
7. Confirm the nearby result reports `5/6 accepted actions` and `1 saved
   through barter`. Reconnect and confirm the result persists.
8. In a fresh solo client, skip practice and trade; mine twice, log once, and
   complete the three forge steps. Confirm the tool remains obtainable.

Record both viewport sizes, input type, every offered nearby label, the trade
ID, the final measured result, reconnect behavior, and any confusing moment.
Human observers should separately record whether the saving motivates trade;
automation proves the saving and access path, not enjoyment or preference.

## Authority and retry boundaries

- `/v1/skills` owns voluntary practice and the mastery threshold.
- `/v1/foundation/resources` owns resource proximity, depletion, practice-based
  yield, and accepted work credits.
- `/v1/trades` remains the only atomic player-barter authority. Cooperation
  consumes eligible source credits once and records its accepted trade ID.
- `/v1/foundation/forge` owns costs, output, final work attribution, and the
  bounded latest result.
- `/v1/state` projects the goal, eligible credits, active attempt, measured
  result, inventory, and connected presence used by the touch client.
- Stable request IDs return their original response across transient retries
  and restart; they cannot reopen an attempt, move goods twice, add work twice,
  or mint a second tool.
- **All tools** remains available throughout. Missing partners never remove
  shared crude tools or the solo recipe.

## Verification record

On 2026-09-02 the connected harness passed the focused touch and authority
tests and its live three-identity scenario. The integrated F5 tree then passed
the full workspace tests, clippy with warnings denied, formatting, diff
hygiene, and the Rust source-size limit. `publish.ps1` built and packaged
Windows and WebGL releases and deployed the preview successfully.

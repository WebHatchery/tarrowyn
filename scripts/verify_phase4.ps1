$ErrorActionPreference = "Stop"

$gameDir = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path ([System.IO.Path]::GetTempPath()) "tarrowyn-phase4-$PID.json"
$server = $null

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw "Phase 4 acceptance failed: $message" }
}

function Post-Json([string]$path, [hashtable]$body, [hashtable]$headers) {
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787$path" `
        -Headers $headers -ContentType "application/json" -Body ($body | ConvertTo-Json -Compress)
}

function Start-Phase4Server {
    Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $gameDir -WindowStyle Hidden -PassThru
}

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function Stop-Phase4Server([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            Start-Sleep -Milliseconds 100
        } catch { return }
    }
    throw "Phase 4 acceptance failed: previous server did not stop"
}

function Wait-Healthy {
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            if ($health.data.status -eq "ok") { return }
        } catch { Start-Sleep -Milliseconds 100 }
    }
    throw "Phase 4 acceptance failed: server did not become healthy"
}

try {
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_LEASE_DURATION_TICKS = "20"
    $env:TARROWYN_CLAIM_RECLAIM_GRACE_TICKS = "2"
    $env:TARROWYN_SESSION_TTL_SECONDS = "120"

    $server = Start-Phase4Server
    Wait-Healthy
    $one = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = "phase4-steward"; reset = $true } | ConvertTo-Json -Compress)
    $two = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = "phase4-provider"; reset = $true } | ConvertTo-Json -Compress)
    $oneHeaders = @{ Authorization = "Bearer $($one.data.account_token)" }
    $twoHeaders = @{ Authorization = "Bearer $($two.data.account_token)" }

    $office = Post-Json "/v1/settlement/governance" @{ request_id = "office"; action = "claim_office"; office_id = "steward" } $oneHeaders
    Assert-True $office.data.accepted "the Steward office was not claimable"
    $proposal = Post-Json "/v1/settlement/governance" @{ request_id = "proposal"; action = "propose"; public_action = "repair_road" } $oneHeaders
    $proposalId = $proposal.data.governance.proposals[0].proposal_id
    Assert-True ($null -ne $proposalId) "the public proposal was not recorded"
    $approval = Post-Json "/v1/settlement/governance" @{ request_id = "approval"; action = "approve"; proposal_id = $proposalId } $oneHeaders
    $completion = Post-Json "/v1/settlement/governance" @{ request_id = "completion"; action = "complete"; proposal_id = $proposalId } $oneHeaders
    Assert-True ($approval.data.accepted -and $completion.data.accepted) "the public action did not complete"
    $infrastructure = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/infrastructure" -Headers $oneHeaders
    Assert-True (($infrastructure.data.records | Where-Object infrastructure_id -eq "north-road").condition -eq 100) "road repair was not visible"

    $requested = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-request"; action = "request" } $oneHeaders
    $claimId = $requested.data.claim.claim_id
    $approved = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-approve"; action = "approve"; claim_id = $claimId } $oneHeaders
    $renewed = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-renew"; action = "renew"; claim_id = $claimId } $oneHeaders
    $transferred = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-transfer"; action = "transfer"; claim_id = $claimId; target_account_id = $two.data.account_id } $oneHeaders
    $abandoned = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-abandon"; action = "abandon"; claim_id = $claimId } $twoHeaders
    $reclaimed = Post-Json "/v1/claims/lifecycle" @{ request_id = "lease-reclaim"; action = "reclaim"; claim_id = $claimId } $twoHeaders
    Assert-True ($approved.data.accepted -and $renewed.data.accepted -and $transferred.data.accepted -and $abandoned.data.accepted -and $reclaimed.data.accepted) "the full lease lifecycle did not complete"
    $oneInventory = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/inventory" -Headers $oneHeaders
    Assert-True ($oneInventory.data.gold -eq 12) "claim reclamation changed unrelated character state"

    $learn = Post-Json "/v1/professions/orders" @{ request_id = "learn"; action = "learn_capability"; profession = "carpenter" } $twoHeaders
    $order = Post-Json "/v1/professions/orders" @{ request_id = "create-order"; action = "create_order"; profession = "carpenter"; service = "Repair a field tool" } $oneHeaders
    $orderId = $order.data.order.order_id
    $accepted = Post-Json "/v1/professions/orders" @{ request_id = "accept-order"; action = "accept_order"; order_id = $orderId } $twoHeaders
    $completed = Post-Json "/v1/professions/orders" @{ request_id = "complete-order"; action = "complete_order"; order_id = $orderId } $twoHeaders
    Assert-True ($learn.data.accepted -and $accepted.data.accepted -and $completed.data.accepted) "the profession order loop did not complete"

    $discovered = Post-Json "/v1/knowledge" @{ request_id = "discover"; action = "discover" } $oneHeaders
    $taught = Post-Json "/v1/knowledge" @{ request_id = "teach"; action = "teach"; knowledge_id = "moonberry-tending"; target_account_id = $two.data.account_id } $oneHeaders
    $applied = Post-Json "/v1/knowledge" @{ request_id = "apply"; action = "apply"; knowledge_id = "moonberry-tending" } $twoHeaders
    Assert-True ($discovered.data.accepted -and $taught.data.accepted -and $applied.data.accepted) "knowledge discovery and transfer did not complete"

    $households = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/households" -Headers $oneHeaders
    Assert-True ($households.data.households[0].members.Count -eq 2) "the complementary household was not visible"
    $beforeAnimal = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $oneHeaders
    Assert-True ($beforeAnimal.data.world.animals.Count -eq 1 -and $beforeAnimal.data.player.animal_condition -eq 2) "Bellweather was not visible at its starting condition"
    $animalSteps = @(@{ dx = -1; dy = 0 }, @{ dx = -1; dy = 0 }, @{ dx = -1; dy = 0 }, @{ dx = -1; dy = 0 }, @{ dx = 0; dy = -1 })
    for ($index = 0; $index -lt $animalSteps.Count; $index++) {
        $move = Post-Json "/v1/movement" @{ request_id = "animal-step-$index"; dx = $animalSteps[$index].dx; dy = $animalSteps[$index].dy } $oneHeaders
        Assert-True $move.data.accepted "the animal-care test could not reach the shared fields"
    }
    $care = Post-Json "/v1/farming/actions" @{ request_id = "animal-care"; action = "tend_animal"; position = @{ x = 3; y = 5 } } $oneHeaders
    Assert-True ($care.data.accepted -and $care.data.animal.condition -eq 3 -and $care.data.player.animal_condition -eq 3) "Bellweather care did not restore the authoritative condition"
    $afterAnimal = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $oneHeaders
    Assert-True ($afterAnimal.data.world.animals[0].condition -eq 3) "the cared animal was not retained in the world projection"
    $steps = @(
        @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 },
        @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 },
        @{ dx = 0; dy = -1 }
    )
    for ($index = 0; $index -lt $steps.Count; $index++) {
        $move = Post-Json "/v1/movement" @{ request_id = "phase4-step-$index"; dx = $steps[$index].dx; dy = $steps[$index].dy } $oneHeaders
        Assert-True $move.data.accepted "the local combat test could not reach Whisperwood"
    }
    $prepared = Post-Json "/v1/combat/local" @{ request_id = "prepare"; action = "prepare"; weapon = "iron_sword" } $oneHeaders
    $strikeOne = Post-Json "/v1/combat/local" @{ request_id = "strike-one"; action = "strike"; weapon = "iron_sword" } $oneHeaders
    $strikeTwo = Post-Json "/v1/combat/local" @{ request_id = "strike-two"; action = "strike"; weapon = "iron_sword" } $oneHeaders
    Assert-True ($prepared.data.accepted -and $strikeOne.data.accepted -and $strikeTwo.data.combat.status -eq "victorious") "the local combat loop did not resolve"

    $characterId = $one.data.character_id
    Stop-Phase4Server $server
    $server = Start-Phase4Server
    Wait-Healthy
    $resumed = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = "phase4-steward"; reset = $false } | ConvertTo-Json -Compress)
    Assert-True ($resumed.data.character_id -eq $characterId) "the Phase 4 identity did not survive restart"
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedGovernance = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/settlement/governance" -Headers $resumedHeaders
    Assert-True ($resumedGovernance.data.governance.decisions.Count -ge 1) "the governance decision did not survive restart"
    $resumedState = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $resumedHeaders
    Assert-True ($resumedState.data.world.animals[0].condition -eq 3) "the cared animal condition did not survive restart"

    Write-Host "Phase 4 acceptance passed: governance, infrastructure, lease lifecycle, profession order, knowledge transfer, animal care, complementary household, local combat, and restart." -ForegroundColor Green
} finally {
    if ($null -ne $server -and -not $server.HasExited) { Stop-Phase4Server $server }
    Remove-Item Env:TARROWYN_STATE_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:TARROWYN_MOVEMENT_COOLDOWN_TICKS -ErrorAction SilentlyContinue
    Remove-Item Env:TARROWYN_TICK_MS -ErrorAction SilentlyContinue
    Remove-Item Env:TARROWYN_LEASE_DURATION_TICKS -ErrorAction SilentlyContinue
    Remove-Item Env:TARROWYN_CLAIM_RECLAIM_GRACE_TICKS -ErrorAction SilentlyContinue
    Remove-Item Env:TARROWYN_SESSION_TTL_SECONDS -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
}

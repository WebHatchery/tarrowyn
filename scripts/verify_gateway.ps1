# Verify the public gateway and the proxied Tarrowyn authority health response.
# This is read-only: registration and server startup are separate operations.

param(
    [string]$GatewayUrl = 'https://webhatchery.au/local_gateway/api',
    [string]$Service = 'tarrowyn',
    [int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'

if ($TimeoutSeconds -lt 1) {
    Write-Error 'TimeoutSeconds must be at least 1.'
    exit 1
}

$baseUrl = $GatewayUrl.TrimEnd('/')
if ($baseUrl -notmatch '^https://[^/\s]+(?:/.*)?$') {
    Write-Error 'GatewayUrl must be an HTTPS origin or HTTPS path.'
    exit 1
}

$encodedService = [Uri]::EscapeDataString($Service)
$checks = @(
    @{ Name = 'gateway health'; Url = "$baseUrl/health" },
    @{ Name = 'Tarrowyn proxy health'; Url = "$baseUrl/p/$encodedService/health" }
)

function Get-Json($check) {
    try {
        $response = Invoke-WebRequest -Uri $check.Url -Method Get -TimeoutSec $TimeoutSeconds
        if ([int]$response.StatusCode -ne 200) {
            throw "HTTP $([int]$response.StatusCode)"
        }
        return ($response.Content | ConvertFrom-Json)
    } catch {
        throw "$($check.Name) failed at $($check.Url): $($_.Exception.Message)"
    }
}

$gateway = $null
$server = $null
$failures = [System.Collections.Generic.List[string]]::new()

try {
    $gateway = Get-Json $checks[0]
    if ($gateway.ok -ne $true -or $gateway.service -ne 'local_gateway') {
        $failures.Add('gateway health returned an unexpected response')
    }
} catch {
    $failures.Add($_.Exception.Message)
}

try {
    $server = Get-Json $checks[1]
    if ($server.data.status -ne 'ok' -or $server.data.service -ne 'tarrowyn-server') {
        $failures.Add('Tarrowyn proxy health returned an unexpected response')
    } elseif ($server.meta.protocol_version -ne '7' -or $server.data.protocol_version -ne '7') {
        $failures.Add('Tarrowyn proxy health returned the wrong protocol version')
    }
} catch {
    $failures.Add($_.Exception.Message)
}

if ($failures.Count -gt 0) {
    throw "Gateway verification failed: $($failures -join ' | ')"
}

Write-Host "Gateway health: $($gateway.service)" -ForegroundColor Green
Write-Host "Tarrowyn health: $($server.data.service), protocol $($server.meta.protocol_version)" -ForegroundColor Green

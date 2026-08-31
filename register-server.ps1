# Advertise this desktop's tarrowyn-server to the public gateway on a heartbeat.
#
# The published client reaches the world through the local_gateway reverse proxy
# on webhatchery.au. The gateway only knows where to forward after this script
# registers the home server, and the heartbeat keeps that registration fresh.
#
# Prerequisites:
#   - tarrowyn-server running locally on -Port.
#   - The gateway service policy contains the `tarrowyn` service.
#   - The gateway admin token is passed with -Token or GATEWAY_ADMIN_TOKEN.
#   - The router forwards -Port to this desktop, or -Target names an HTTPS
#     tunnel that forwards to the local server.
#
# Examples:
#   $env:GATEWAY_ADMIN_TOKEN = '...'; .\register-server.ps1
#   .\register-server.ps1 -Once
#   .\register-server.ps1 -Target 'https://abc123.trycloudflare.com'

param(
    [string]$GatewayUrl = 'https://webhatchery.au/local_gateway/api',
    [string]$Service = 'tarrowyn',
    [int]$Port = 8787,
    # An explicit reachable base URL (for example, an HTTPS tunnel). When empty,
    # the gateway derives http://<public request IP>:<Port>.
    [string]$Target = '',
    [string]$Token = $env:GATEWAY_ADMIN_TOKEN,
    [int]$IntervalSeconds = 60,
    [switch]$Once
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($Token)) {
    Write-Error "No gateway token. Pass -Token or set `$env:GATEWAY_ADMIN_TOKEN."
    exit 1
}

if ($Port -lt 1 -or $Port -gt 65535) {
    Write-Error "Port must be between 1 and 65535."
    exit 1
}

if ($IntervalSeconds -lt 1) {
    Write-Error "IntervalSeconds must be at least 1."
    exit 1
}

$endpoint = ($GatewayUrl.TrimEnd('/')) + '/register'
$body = if ([string]::IsNullOrWhiteSpace($Target)) {
    @{ service = $Service; port = $Port }
} else {
    @{ service = $Service; target = $Target.TrimEnd('/') }
}
$json = $body | ConvertTo-Json -Compress
$headers = @{ 'X-Gateway-Token' = $Token }

$where = if ($Target) { $Target } else { "http://<your public IP>:$Port" }
Write-Host ''
Write-Host "Gateway   : $endpoint" -ForegroundColor Green
Write-Host "Service   : $Service -> $where" -ForegroundColor Green
Write-Host $(if ($Once) { 'Mode      : one shot' } else { "Mode      : heartbeat every ${IntervalSeconds}s (Ctrl+C to stop)" }) -ForegroundColor DarkGray
Write-Host ''

function Send-Registration {
    try {
        $response = Invoke-RestMethod -Uri $endpoint -Method Post -Headers $headers `
            -ContentType 'application/json' -Body $json -TimeoutSec 15
        $stamp = (Get-Date).ToString('HH:mm:ss')
        Write-Host "[$stamp] registered: $($response.target) (ttl $($response.ttl)s)" -ForegroundColor Green
    } catch {
        $stamp = (Get-Date).ToString('HH:mm:ss')
        Write-Host "[$stamp] register failed: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Send-Registration
if ($Once) { return }

while ($true) {
    Start-Sleep -Seconds $IntervalSeconds
    Send-Registration
}

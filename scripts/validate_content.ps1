$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$dataRoot = Join-Path $projectRoot "assets\data"
$required = @("region.json", "settlements.json", "events.json", "items.json", "content_schema.json")
foreach ($name in $required) {
    $path = Join-Path $dataRoot $name
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing content manifest: $name" }
    $null = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
}
$region = Get-Content -Raw -LiteralPath (Join-Path $dataRoot "region.json") | ConvertFrom-Json
$locations = @($region.locations | ForEach-Object { $_.id })
$routes = @($region.routes | ForEach-Object { $_.id })
if ($locations.Count -ne ($locations | Sort-Object -Unique).Count) { throw "Region location IDs must be unique." }
if ($routes.Count -ne ($routes | Sort-Object -Unique).Count) { throw "Region route IDs must be unique." }
if ($locations.Count -lt 3) { throw "The regional manifest needs three locations." }
foreach ($route in $region.routes) {
    if ($locations -notcontains $route.origin -or $locations -notcontains $route.destination) { throw "Route $($route.id) references an unknown location." }
}
Write-Host "Content manifests valid: $($required.Count) files, $($locations.Count) locations, $($routes.Count) routes."

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$dataRoot = Join-Path $projectRoot "assets\data"
$required = @("content_schema.json", "game_config.json", "actions.json", "crops.json", "contracts.json", "events.json", "items.json", "threats.json", "region.json", "households.json", "infrastructure.json", "npc_households.json", "recipes.json", "settlements.json", "skills.json")
$requiredManifests = $required | Where-Object { $_ -ne "content_schema.json" }
$manifests = @{}

function Assert-TextFields([string]$label, [object[]]$records, [string[]]$fields) {
    foreach ($record in @($records)) {
        foreach ($field in $fields) {
            $property = $record.PSObject.Properties[$field]
            if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
                throw "$label records need a non-empty $field field."
            }
        }
    }
}

function Assert-ArrayFields([string]$label, [object[]]$records, [string[]]$fields) {
    foreach ($record in @($records)) {
        foreach ($field in $fields) {
            $property = $record.PSObject.Properties[$field]
            if ($null -eq $property -or @($property.Value).Count -eq 0 -or @($property.Value | Where-Object { [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0) {
                throw "$label records need non-empty $field arrays."
            }
        }
    }
}

function Assert-Records([string]$label, [object[]]$records, [string[]]$textFields, [string[]]$arrayFields) {
    $records = @($records)
    if ($records.Count -eq 0) { throw "$label manifest must contain at least one record." }
    $ids = @()
    foreach ($record in $records) {
        $property = $record.PSObject.Properties["id"]
        if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
            throw "$label records need unique, non-empty id fields."
        }
        $ids += [string]$property.Value
    }
    $duplicates = @($ids | Group-Object | Where-Object Count -gt 1)
    if ($duplicates.Count -gt 0) {
        throw "$label IDs must be unique: $($duplicates.Name -join ', ')."
    }
    Assert-TextFields $label $records $textFields
    Assert-ArrayFields $label $records $arrayFields
}

function Get-Records([object]$document, [string]$property, [string]$label) {
    if ([string]::IsNullOrEmpty($property)) {
        if ($document -isnot [array]) { throw "$label manifest must be an array." }
        return @($document)
    }
    $field = $document.PSObject.Properties[$property]
    if ($null -eq $field -or $field.Value -isnot [array]) {
        throw "$label manifest must contain an array named $property."
    }
    return @($field.Value)
}

function Assert-ManifestSchema([object]$schema) {
    if ($schema.schema_version -lt 1 -or [string]::IsNullOrWhiteSpace([string]$schema.compatibility)) {
        throw "Content schema needs a positive version and compatibility rule."
    }
    $declared = @($schema.required_manifests)
    if ($declared.Count -eq 0 -or @($declared | Where-Object { [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0) {
        throw "Content schema must declare non-empty required manifest names."
    }
    $duplicateNames = @($declared | Group-Object | Where-Object Count -gt 1)
    if ($duplicateNames.Count -gt 0) { throw "Content schema manifest names must be unique." }
    $missing = @($requiredManifests | Where-Object { $declared -notcontains $_ })
    $extra = @($declared | Where-Object { $requiredManifests -notcontains $_ })
    if ($missing.Count -gt 0 -or $extra.Count -gt 0) {
        throw "Content schema manifest set differs from the release contract. Missing: $($missing -join ', '); extra: $($extra -join ', ')."
    }
}

foreach ($name in $required) {
    $path = Join-Path $dataRoot $name
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing content manifest: $name" }
    $manifests[$name] = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
}
$schema = Get-Content -Raw -LiteralPath (Join-Path $dataRoot "content_schema.json") | ConvertFrom-Json
Assert-ManifestSchema $schema

Assert-Records "actions" (Get-Records $manifests["actions.json"] $null "actions") @("name", "description", "kind") @()
Assert-Records "crops" (Get-Records $manifests["crops.json"] $null "crops") @("name", "description") @()
Assert-Records "contracts" (Get-Records $manifests["contracts.json"] "contracts" "contracts") @("title", "description", "target") @()
Assert-Records "events" (Get-Records $manifests["events.json"] "events" "events") @("title", "kind", "cause") @("stages", "affected_systems", "effects", "intervention_options")
Assert-Records "items" (Get-Records $manifests["items.json"] "items" "items") @("kind", "sink") @()
Assert-Records "threats" (Get-Records $manifests["threats.json"] "threats" "threats") @("name", "monster", "resource_demand", "rumour") @()
Assert-Records "households" (Get-Records $manifests["households.json"] "households" "households") @("name", "occupation", "home_settlement", "service", "clue", "reason", "regional_service") @("members", "history")
Assert-Records "infrastructure" (Get-Records $manifests["infrastructure.json"] "infrastructure" "infrastructure") @("name", "kind", "note") @()
Assert-Records "npc households" (Get-Records $manifests["npc_households.json"] "npc_households" "npc households") @("household_name", "home", "work", "demand", "clue") @("members", "needs")
Assert-Records "recipes" (Get-Records $manifests["recipes.json"] "recipes" "recipes") @("name", "profession", "service", "benefit") @()
Assert-Records "settlements" (Get-Records $manifests["settlements.json"] "settlements" "settlements") @("location", "name", "governance", "condition") @("infrastructure", "milestones", "vacancies", "demand", "abundant", "scarce")
Assert-Records "skills" (Get-Records $manifests["skills.json"] "skills" "skills") @("name", "family", "description", "entry_hint") @()
$skillsVersion = $manifests["skills.json"].version
if ($skillsVersion -lt 1) { throw "Skills manifest needs a positive version." }

$region = Get-Content -Raw -LiteralPath (Join-Path $dataRoot "region.json") | ConvertFrom-Json
$calendar = $region.calendar
if ($null -eq $calendar -or $calendar.day_seconds -lt 1 -or $calendar.season_days -lt 1 -or $calendar.year_days -lt 1 -or @($calendar.seasons).Count -ne 4) {
    throw "Region calendar must define positive day, season, and year values plus four seasons."
}
$locationsRecords = @($region.locations)
$routesRecords = @($region.routes)
Assert-Records "region locations" $locationsRecords @("name", "kind", "role", "access_note") @("resources", "services")
Assert-Records "region routes" $routesRecords @("name", "transport", "origin", "destination", "status", "note") @()
$locations = @($region.locations | ForEach-Object { $_.id })
$routes = @($region.routes | ForEach-Object { $_.id })
$farmPlots = @($region.farm_plots | ForEach-Object { "$($_.x),$($_.y)" })
if ($locations.Count -ne ($locations | Sort-Object -Unique).Count) { throw "Region location IDs must be unique." }
if ($routes.Count -ne ($routes | Sort-Object -Unique).Count) { throw "Region route IDs must be unique." }
if ($farmPlots.Count -lt 1 -or $farmPlots.Count -ne ($farmPlots | Sort-Object -Unique).Count) { throw "Region farm plot positions must be present and unique." }
if ($locations.Count -lt 3) { throw "The regional manifest needs three locations." }
foreach ($route in $region.routes) {
    if ($locations -notcontains $route.origin -or $locations -notcontains $route.destination) { throw "Route $($route.id) references an unknown location." }
}
Write-Host "Content manifests valid: $($required.Count) files, $($locations.Count) locations, $($routes.Count) routes, $($farmPlots.Count) farm plots."

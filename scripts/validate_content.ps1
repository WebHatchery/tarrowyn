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

function Assert-RequiredRecordIds([string]$label, [object[]]$records, [string[]]$requiredIds) {
    $availableIds = @($records | ForEach-Object { [string]$_.id })
    foreach ($requiredId in $requiredIds) {
        if ($availableIds -notcontains $requiredId) {
            throw "$label are missing launch record $requiredId."
        }
    }
}

function Assert-SkillRecords([object[]]$records) {
    $records = @($records)
    $skillIds = @($records | ForEach-Object { [string]$_.id })
    $families = @("combat", "magic", "gathering", "farming", "production", "social")
    foreach ($skill in $records) {
        $skillId = [string]$skill.id
        if ($families -notcontains [string]$skill.family) {
            throw "Skill $skillId must use a supported family."
        }
        $depth = [int]$skill.depth
        if ($depth -lt 1 -or $depth -gt 5) {
            throw "Skill $skillId must use a depth from one through five."
        }
        $prerequisites = @($skill.prerequisites)
        $prerequisiteIds = @($prerequisites | ForEach-Object { [string]$_ })
        if (@($prerequisiteIds | Where-Object { [string]::IsNullOrWhiteSpace($_) }).Count -gt 0) {
            throw "Skill $skillId cannot contain an empty prerequisite."
        }
        $duplicatePrerequisites = @($prerequisiteIds | Group-Object | Where-Object Count -gt 1)
        if ($duplicatePrerequisites.Count -gt 0) {
            throw "Skill $skillId cannot repeat a prerequisite: $($duplicatePrerequisites.Name -join ', ')."
        }
        foreach ($prerequisite in $prerequisiteIds) {
            if ($skillIds -notcontains $prerequisite) {
                throw "Skill $skillId names an unknown prerequisite: $prerequisite."
            }
        }

        $practiceKey = [string]$skill.practice_key
        if (-not [string]::IsNullOrWhiteSpace($practiceKey) -and $skillIds -notcontains $practiceKey) {
            throw "Skill $skillId names an unknown practice_key: $practiceKey."
        }
        if ($depth -eq 1) {
            if (-not [bool]$skill.directly_teachable -or $practiceKey -ne $skillId -or $prerequisiteIds.Count -ne 0) {
                throw "Root skill $skillId needs a direct practice path and no prerequisites."
            }
        } else {
            $qualifyingEvent = [string]$skill.qualifying_event
            $qualifyingCount = [int]$skill.qualifying_count
            if ($prerequisiteIds.Count -eq 0 -or [string]::IsNullOrWhiteSpace($qualifyingEvent) -or $qualifyingCount -lt 1) {
                throw "Advanced skill $skillId needs prerequisites, a qualifying event, and a positive qualifying_count."
            }
            if ($qualifyingEvent -eq "weapon_defeats" -and [int]$skill.minimum_per_prerequisite -lt 1) {
                throw "Advanced skill $skillId needs a positive minimum_per_prerequisite."
            }
        }
    }
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

$gameConfig = $manifests["game_config.json"]
$requiredGameText = @("game_name", "display_name", "save_slot", "version")
foreach ($field in $requiredGameText) {
    $property = $gameConfig.PSObject.Properties[$field]
    if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
        throw "Game config needs a non-empty $field field."
    }
}
$dayLength = [double]$gameConfig.day_length_seconds
if ($gameConfig.world_width -lt 1 -or $gameConfig.world_height -lt 1 -or
    $dayLength -le 0 -or [double]::IsNaN($dayLength) -or [double]::IsInfinity($dayLength) -or
    $gameConfig.starting_gold -lt 1 -or $gameConfig.starting_seeds -lt 1 -or $gameConfig.starting_skill -lt 1) {
    throw "Game config needs positive world, clock, and starting-resource values."
}

$actionRecords = Get-Records $manifests["actions.json"] $null "actions"
Assert-Records "actions" $actionRecords @("name", "description", "kind") @()
if (@($actionRecords | Where-Object {
        @("plant", "tend", "harvest", "listen") -notcontains [string]$_.kind
    }).Count -gt 0) {
    throw "Actions must use a supported protocol kind."
}
$requiredActions = @{
    "plant" = "plant"
    "tend" = "tend"
    "harvest" = "harvest"
    "listen" = "listen"
}
Assert-RequiredRecordIds "Actions" $actionRecords @($requiredActions.Keys)
foreach ($actionId in $requiredActions.Keys) {
    $action = $actionRecords | Where-Object { [string]$_.id -eq $actionId } | Select-Object -First 1
    if ($null -eq $action) {
        throw "Actions are missing launch action $actionId."
    }
    if ([string]$action.kind -ne $requiredActions[$actionId]) {
        throw "Launch action $actionId must use kind $($requiredActions[$actionId])."
    }
}
$cropRecords = Get-Records $manifests["crops.json"] $null "crops"
Assert-Records "crops" $cropRecords @("name", "description") @()
Assert-RequiredRecordIds "Crops" $cropRecords @("wheat", "turnip", "moonberry")
$contractRecords = Get-Records $manifests["contracts.json"] "contracts" "contracts"
Assert-Records "contracts" $contractRecords @("title", "description", "target") @()
Assert-RequiredRecordIds "Contracts" $contractRecords @("brambleback-watch")
$eventRecords = Get-Records $manifests["events.json"] "events" "events"
Assert-Records "events" $eventRecords @("title", "kind", "cause") @("stages", "affected_systems", "affected_locations", "effects", "intervention_options")
Assert-RequiredRecordIds "Events" $eventRecords @("river-thaw")
$supportedEventInterventions = @("repair ferry markers", "escort the grain caravan", "open the frontier storehouse")
$eventInterventionLocations = @{
    "repair ferry markers" = @("hearth", "saltmere")
    "escort the grain caravan" = @("hearth", "whisperwood-outpost")
    "open the frontier storehouse" = @("whisperwood-outpost")
}
foreach ($event in $eventRecords) {
    $affectedLocations = @($event.affected_locations | ForEach-Object { [string]$_ })
    foreach ($intervention in @($event.intervention_options)) {
        $interventionName = [string]$intervention
        if ($supportedEventInterventions -notcontains $interventionName) {
            throw "Events must use a supported intervention choice: $intervention"
        }
        if ($eventInterventionLocations.ContainsKey($interventionName) -and
            @($eventInterventionLocations[$interventionName] | Where-Object { $affectedLocations -contains $_ }).Count -eq 0) {
            throw "Event $($event.id) must include a location affected by $interventionName."
        }
    }
}
$itemRecords = Get-Records $manifests["items.json"] "items" "items"
Assert-Records "items" $itemRecords @("kind", "sink") @()
Assert-RequiredRecordIds "Items" $itemRecords @("wheat", "turnips", "moonberries", "seeds", "timber", "stone", "bandages")
if (@($itemRecords | Where-Object { [int]$_.base_price -lt 1 }).Count -gt 0) {
    throw "Items must define positive base_price values."
}
$threatRecords = Get-Records $manifests["threats.json"] "threats" "threats"
Assert-Records "threats" $threatRecords @("name", "monster", "resource_demand", "rumour") @()
Assert-RequiredRecordIds "Threats" $threatRecords @("whisperwood-edge")
$householdRecords = Get-Records $manifests["households.json"] "households" "households"
Assert-Records "households" $householdRecords @("opportunity_id", "regional_id", "name", "occupation", "home_settlement", "service", "clue", "reason", "regional_service") @("members", "history")
Assert-RequiredRecordIds "Households" $householdRecords @("household-maren")
$opportunityIds = @($householdRecords | ForEach-Object { [string]$_.opportunity_id })
if ($opportunityIds.Count -ne ($opportunityIds | Sort-Object -Unique).Count) { throw "Household opportunity IDs must be unique." }
$regionalHouseholdIds = @($householdRecords | ForEach-Object { [string]$_.regional_id })
if ($regionalHouseholdIds.Count -ne ($regionalHouseholdIds | Sort-Object -Unique).Count) { throw "Regional household IDs must be unique." }
$infrastructureRecords = Get-Records $manifests["infrastructure.json"] "infrastructure" "infrastructure"
Assert-Records "infrastructure" $infrastructureRecords @("name", "kind", "note") @()
Assert-RequiredRecordIds "Infrastructure" $infrastructureRecords @("north-road", "hearth-services")
$npcHouseholdRecords = Get-Records $manifests["npc_households.json"] "npc_households" "npc households"
Assert-Records "npc households" $npcHouseholdRecords @("household_id", "household_name", "home", "work", "demand", "clue") @("members", "needs")
Assert-RequiredRecordIds "NPC households" $npcHouseholdRecords @("bellweather")
$npcHouseholdIds = @($npcHouseholdRecords | ForEach-Object { [string]$_.household_id })
if ($npcHouseholdIds.Count -ne ($npcHouseholdIds | Sort-Object -Unique).Count) { throw "NPC household projection IDs must be unique." }
$recipeRecords = Get-Records $manifests["recipes.json"] "recipes" "recipes"
Assert-Records "recipes" $recipeRecords @("name", "profession", "service", "benefit") @()
Assert-RequiredRecordIds "Recipes" $recipeRecords @("field-tool-repair")
$settlementRecords = Get-Records $manifests["settlements.json"] "settlements" "settlements"
Assert-Records "settlements" $settlementRecords @("location", "name", "governance", "condition") @("infrastructure", "milestones", "vacancies", "demand", "abundant", "scarce", "initial_stock")
Assert-RequiredRecordIds "Settlements" $settlementRecords @("hearth-settlement", "whisperwood-settlement", "saltmere-settlement")
$duplicateSettlementLocations = @($settlementRecords | ForEach-Object { [string]$_.location } | Group-Object | Where-Object Count -gt 1)
if ($duplicateSettlementLocations.Count -gt 0) {
    throw "Settlements cannot share a regional location: $($duplicateSettlementLocations.Name -join ', ')."
}
$skillRecords = Get-Records $manifests["skills.json"] "skills" "skills"
Assert-Records "skills" $skillRecords @("name", "family", "description", "entry_hint") @()
Assert-RequiredRecordIds "Skills" $skillRecords @(
    "unarmed-fighting", "sword-fighting", "axe-fighting", "spear-fighting", "bow-fighting", "shield-use",
    "wind-magic", "water-magic", "electricity-magic", "fire-magic", "earth-magic", "restoration-magic",
    "foraging", "forestry", "mining", "fishing", "hunting", "survival", "navigation",
    "crop-tending", "animal-husbandry", "carpentry", "smithing", "tailoring", "cooking", "alchemy", "masonry", "general-crafting",
    "trade", "teaching", "leadership", "weapon-fighting", "storm-magic"
)
Assert-SkillRecords $skillRecords
$skillsVersion = $manifests["skills.json"].version
if ($skillsVersion -lt 1) { throw "Skills manifest needs a positive version." }

$region = Get-Content -Raw -LiteralPath (Join-Path $dataRoot "region.json") | ConvertFrom-Json
$calendar = $region.calendar
if ([string]$region.region_id -ne "hearthlands") { throw "Region manifest must use the hearthlands launch ID." }
if ($null -eq $calendar -or $calendar.day_seconds -lt 1 -or $calendar.season_days -lt 1 -or $calendar.year_days -lt 1 -or @($calendar.seasons).Count -ne 4 -or $calendar.year_days -ne ($calendar.season_days * 4)) {
    throw "Region calendar must define positive day, season, and year values plus four seasons."
}
if ($dayLength -ne [double]$calendar.day_seconds) {
    throw "Game config day length must match the region calendar."
}
$locationsRecords = @($region.locations)
$routesRecords = @($region.routes)
Assert-Records "region locations" $locationsRecords @("name", "kind", "role", "access_note") @("resources", "services")
Assert-Records "region routes" $routesRecords @("name", "transport", "origin", "destination", "status", "note") @()
if (@($routesRecords | Where-Object { [string]$_.origin -eq [string]$_.destination }).Count -gt 0) {
    throw "Region routes must connect two distinct locations."
}
$locations = @($region.locations | ForEach-Object { $_.id })
$routes = @($region.routes | ForEach-Object { $_.id })
$settlementIds = @($settlementRecords | ForEach-Object { [string]$_.id })
$requiredLocations = @("hearth", "whisperwood-outpost", "saltmere")
$requiredRoutes = @("north-pack-road", "saltmere-ferry", "watch-trail")
$requiredSettlements = @("hearth-settlement", "whisperwood-settlement", "saltmere-settlement")
foreach ($requiredLocation in $requiredLocations) {
    if ($locations -notcontains $requiredLocation) { throw "Region is missing launch location $requiredLocation." }
}
foreach ($requiredRoute in $requiredRoutes) {
    if ($routes -notcontains $requiredRoute) { throw "Region is missing launch route $requiredRoute." }
}
foreach ($requiredSettlement in $requiredSettlements) {
    if ($settlementIds -notcontains $requiredSettlement) { throw "Settlements are missing launch record $requiredSettlement." }
}
$expectedSettlementLocations = @{
    "hearth-settlement" = "hearth"
    "whisperwood-settlement" = "whisperwood-outpost"
    "saltmere-settlement" = "saltmere"
}
foreach ($settlementId in $expectedSettlementLocations.Keys) {
    $settlement = $settlementRecords | Where-Object { $_.id -eq $settlementId } | Select-Object -First 1
    if ([string]$settlement.location -ne $expectedSettlementLocations[$settlementId]) {
        throw "Settlement $settlementId must belong to $($expectedSettlementLocations[$settlementId])."
    }
}
$expectedRouteEndpoints = @{
    "north-pack-road" = @("hearth", "whisperwood-outpost")
    "saltmere-ferry" = @("hearth", "saltmere")
    "watch-trail" = @("whisperwood-outpost", "saltmere")
}
foreach ($routeId in $expectedRouteEndpoints.Keys) {
    $route = $routesRecords | Where-Object { $_.id -eq $routeId } | Select-Object -First 1
    $endpoints = $expectedRouteEndpoints[$routeId]
    if ([string]$route.origin -ne $endpoints[0] -or [string]$route.destination -ne $endpoints[1]) {
        throw "Route $routeId must connect $($endpoints[0]) to $($endpoints[1])."
    }
}
$itemIds = @($manifests["items.json"].items | ForEach-Object { [string]$_.id })
foreach ($settlement in $settlementRecords) {
    $stockRecords = @($settlement.initial_stock)
    $stockCommodities = @($stockRecords | ForEach-Object { [string]$_.commodity })
    if ($stockRecords.Count -eq 0 -or $stockCommodities.Count -ne ($stockCommodities | Sort-Object -Unique).Count) {
        throw "Settlement $($settlement.id) needs unique initial market stock records."
    }
    if (@($stockRecords | Where-Object {
            [string]::IsNullOrWhiteSpace([string]$_.commodity) -or
            [int]$_.quantity -lt 1 -or
            $itemIds -notcontains [string]$_.commodity
        }).Count -gt 0) {
        throw "Settlement $($settlement.id) initial stock must use positive quantities and known item IDs."
    }
}
if (@($region.locations | Where-Object {
        $null -eq $_.position -or $null -eq $_.position.x -or $null -eq $_.position.y -or
        [int]$_.position.x -lt 0 -or [int]$_.position.y -lt 0 -or
        [int]$_.position.x -ge [int]$gameConfig.world_width -or
        [int]$_.position.y -ge [int]$gameConfig.world_height
    }).Count -gt 0) {
    throw "Region locations must be inside the configured world."
}
if (@($threatRecords | Where-Object {
        $null -eq $_.position -or $null -eq $_.position.x -or $null -eq $_.position.y -or
        [int]$_.position.x -lt 0 -or [int]$_.position.y -lt 0 -or
        [int]$_.position.x -ge [int]$gameConfig.world_width -or
        [int]$_.position.y -ge [int]$gameConfig.world_height
    }).Count -gt 0) {
    throw "Threat positions must be inside the configured world."
}
if (@($region.farm_plots | Where-Object {
        $null -eq $_.x -or $null -eq $_.y -or
        [int]$_.x -lt 0 -or [int]$_.y -lt 0 -or
        [int]$_.x -ge [int]$gameConfig.world_width -or
        [int]$_.y -ge [int]$gameConfig.world_height
    }).Count -gt 0) {
    throw "Region farm plot positions must be inside the configured world."
}
$farmPlots = @($region.farm_plots | ForEach-Object { "$($_.x),$($_.y)" })
if ($locations.Count -ne ($locations | Sort-Object -Unique).Count) { throw "Region location IDs must be unique." }
if ($routes.Count -ne ($routes | Sort-Object -Unique).Count) { throw "Region route IDs must be unique." }
foreach ($event in $eventRecords) {
    $affectedLocations = @($event.affected_locations | ForEach-Object { [string]$_ })
    if ($affectedLocations.Count -eq 0 -or $affectedLocations.Count -ne ($affectedLocations | Sort-Object -Unique).Count -or
        @($affectedLocations | Where-Object { $locations -notcontains $_ }).Count -gt 0) {
        throw "Event $($event.id) needs unique affected locations from the regional manifest."
    }
}
if ($farmPlots.Count -lt 1 -or $farmPlots.Count -ne ($farmPlots | Sort-Object -Unique).Count) { throw "Region farm plot positions must be present and unique." }
if ($null -eq $region.farm_animal_position -or
    $null -eq $region.farm_animal_position.x -or $null -eq $region.farm_animal_position.y) {
    throw "Region farm animal position must be present."
}
$animalX = [int]$region.farm_animal_position.x
$animalY = [int]$region.farm_animal_position.y
if ($animalX -lt 0 -or $animalY -lt 0 -or
    $animalX -ge [int]$gameConfig.world_width -or $animalY -ge [int]$gameConfig.world_height) {
    throw "Region farm animal position must be inside the world."
}
if ($farmPlots -contains "$animalX,$animalY") {
    throw "Region farm animal position must not overlap a farm plot."
}
$adjacentPlots = @($region.farm_plots | Where-Object {
    ([math]::Abs(([int]$_.x) - $animalX) + [math]::Abs(([int]$_.y) - $animalY)) -eq 1
})
if ($adjacentPlots.Count -eq 0) { throw "Region farm animal position must be one tile from a farm plot." }
if (@($infrastructureRecords | Where-Object {
        $null -eq $_.position -or $null -eq $_.position.x -or $null -eq $_.position.y -or
        [int]$_.position.x -lt 0 -or [int]$_.position.y -lt 0 -or
        [int]$_.position.x -ge [int]$gameConfig.world_width -or
        [int]$_.position.y -ge [int]$gameConfig.world_height
    }).Count -gt 0) {
    throw "Infrastructure positions must be inside the configured world."
}
if ($locations.Count -lt 3) { throw "The regional manifest needs three locations." }
foreach ($route in $region.routes) {
    if ($locations -notcontains $route.origin -or $locations -notcontains $route.destination) { throw "Route $($route.id) references an unknown location." }
}
Write-Host "Content manifests valid: $($required.Count) files, $($locations.Count) locations, $($routes.Count) routes, $($farmPlots.Count) farm plots, animal at $animalX,$animalY."

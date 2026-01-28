param(
    [string]$Branch = "", # Allow specifying branch as a parameter
    [string]$Repository = "obsproject/obs-studio",
    [switch]$DryRun = $false # Enable dry-run mode without making changes
)

# Enable error handling
$ErrorActionPreference = "Stop"

if ($DryRun) {
    Write-Host "================================" -ForegroundColor Yellow
    Write-Host "DRY RUN MODE ENABLED" -ForegroundColor Yellow
    Write-Host "No files will be modified or pushed" -ForegroundColor Yellow
    Write-Host "================================" -ForegroundColor Yellow
}

# Function to get the latest release tag from GitHub
function Get-LatestReleaseTag {
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest"
    return $releases.tag_name
}

# Function to parse semantic version
function Parse-SemVer {
    param([string]$Version)

    # Remove 'v' prefix if present
    $Version = $Version -replace '^v', ''

    if ($Version -match '^(\d+)\.(\d+)\.(\d+)') {
        return @{
            Major       = [int]$matches[1]
            Minor       = [int]$matches[2]
            Patch       = [int]$matches[3]
            FullVersion = $Version
        }
    }

    throw "Invalid version format: $Version"
}

# Function to compare versions and determine bump type
function Get-VersionBumpType {
    param(
        [hashtable]$OldVersion,
        [hashtable]$NewVersion
    )

    if ($NewVersion.Major -gt $OldVersion.Major) {
        return "major"
    }
    elseif ($NewVersion.Minor -gt $OldVersion.Minor) {
        return "minor"
    }
    elseif ($NewVersion.Patch -gt $OldVersion.Patch) {
        return "patch"
    }
    else {
        return "none"
    }
}

# Function to bump semver version
function Bump-SemVer {
    param(
        [hashtable]$CurrentVersion,
        [string]$BumpType
    )

    switch ($BumpType) {
        "major" {
            return @{
                Major = $CurrentVersion.Major + 1
                Minor = 0
                Patch = 0
            }
        }
        "minor" {
            return @{
                Major = $CurrentVersion.Major
                Minor = $CurrentVersion.Minor + 1
                Patch = 0
            }
        }
        "patch" {
            return @{
                Major = $CurrentVersion.Major
                Minor = $CurrentVersion.Minor
                Patch = $CurrentVersion.Patch + 1
            }
        }
        default {
            return $CurrentVersion
        }
    }
}

# Function to update version in Cargo.toml
function Update-CargoVersion {
    param(
        [string]$FilePath,
        [string]$NewVersion,
        [string]$Metadata
    )

    $content = Get-Content -LiteralPath $FilePath -Raw
    $versionString = if ($Metadata) { "$NewVersion+$Metadata" } else { $NewVersion }

    # Update version = "x.y.z+metadata"
    $content = $content -replace 'version\s*=\s*"[^"]+"', "version = `"$versionString`""

    Set-Content -LiteralPath $FilePath -Value $content -Encoding UTF8
}

# Get the workspace root
$workspaceRoot = Join-Path -Path $PSScriptRoot -ChildPath ".."
Push-Location $workspaceRoot

try {
    # Get current libobs version from Cargo.toml
    $libobsCargoPath = Join-Path -Path $workspaceRoot -ChildPath "libobs/Cargo.toml"
    $libobsCargoContent = Get-Content -LiteralPath $libobsCargoPath -Raw

    if ($libobsCargoContent -match 'version\s*=\s*"([^"]+)"') {
        $currentVersionString = $matches[1]
        # Split version and metadata
        if ($currentVersionString -match '^([^+]+)(?:\+(.+))?$') {
            $currentVersion = Parse-SemVer -Version $matches[1]
            $currentMetadata = $matches[2]
        }
    }
    else {
        throw "Could not find version in libobs/Cargo.toml"
    }

    Write-Host "Current libobs version: $currentVersionString"

    # Parse current OBS version from metadata
    $currentObsVersion = if ($currentMetadata) {
        Parse-SemVer -Version $currentMetadata
    }
    else {
        @{ Major = 0; Minor = 0; Patch = 0; FullVersion = "0.0.0" }
    }
    Write-Host "Current OBS Studio version (from metadata): $($currentObsVersion.Major).$($currentObsVersion.Minor).$($currentObsVersion.Patch)"

    # Determine OBS Studio branch/tag to use
    if ([string]::IsNullOrEmpty($Branch)) {
        $Branch = Get-LatestReleaseTag
        Write-Host "No branch specified. Using latest release tag: $Branch"
    }
    else {
        Write-Host "Using specified branch/tag: $Branch"
    }

    # Parse new OBS version
    $obsVersion = Parse-SemVer -Version $Branch
    Write-Host "New OBS Studio version: $($obsVersion.Major).$($obsVersion.Minor).$($obsVersion.Patch)"

    # Determine version bump type by comparing OBS versions, not libobs version
    $bumpType = Get-VersionBumpType -OldVersion $currentObsVersion -NewVersion $obsVersion
    Write-Host "Version bump type: $bumpType"

    # Calculate new libobs version
    if ($bumpType -eq "none") {
        Write-Host "No version change detected in OBS Studio. Keeping current version."
        $newVersion = $currentVersion
    }
    else {
        $newVersion = Bump-SemVer -CurrentVersion $currentVersion -BumpType $bumpType
    }

    $newVersionString = "$($newVersion.Major).$($newVersion.Minor).$($newVersion.Patch)"
    $obsVersionString = "$($obsVersion.Major).$($obsVersion.Minor).$($obsVersion.Patch)"

    # Create metadata with OBS version
    $newMetadata = $obsVersionString

    Write-Host "New libobs version: $newVersionString+$newMetadata"

    # Create and checkout to new branch
    $branchName = "libobs-obs-$($obsVersion.FullVersion)-update"
    Write-Host "Creating branch: $branchName"

    git fetch origin
    git checkout -b $branchName origin/main 2>$null || (git checkout -b $branchName)

    # Run update-headers.ps1
    Write-Host "Running update-headers.ps1..."
    $updateHeadersPath = Join-Path -Path $workspaceRoot -ChildPath "libobs/scripts/update_headers.ps1"

    if ($DryRun) {
        & $updateHeadersPath -Branch $Branch -Repository $Repository -DryRun
    } else {
        & $updateHeadersPath -Branch $Branch -Repository $Repository
    }

    if (-not $?) {
        throw "update-headers.ps1 failed"
    }

    # Update version in all Cargo.toml files
    Write-Host "Updating version in Cargo.toml files..."

    # Update libobs
    Update-CargoVersion -FilePath $libobsCargoPath -NewVersion $newVersionString -Metadata $newMetadata

    # Update workspace dependencies if they reference libobs
    $workspaceCargoPath = Join-Path -Path $workspaceRoot -ChildPath "Cargo.toml"
    $workspaceContent = Get-Content -LiteralPath $workspaceCargoPath -Raw
    $workspaceContent = $workspaceContent -replace 'libobs = \{ path = "\./libobs", version = "[^"]+"', "libobs = { path = `"./libobs`", version = `"$($newVersion.Major)`""
    Set-Content -LiteralPath $workspaceCargoPath -Value $workspaceContent -Encoding UTF8

    # Commit changes
    Write-Host "Committing changes..."
    if ($DryRun) {
        Write-Host "[DRY RUN] Would execute: git add -A" -ForegroundColor Cyan
        Write-Host "[DRY RUN] Would commit with message:" -ForegroundColor Cyan
        Write-Host "chore: update libobs to $newVersionString with OBS Studio $($obsVersion.FullVersion)" -ForegroundColor Cyan
    }
    else {
        git add -A
        git commit -m "chore: update libobs to $newVersionString with OBS Studio $($obsVersion.FullVersion)`n`nBump libobs version from $currentVersionString to $newVersionString+$newMetadata`nUpdate OBS Studio headers and bindings from $($obsVersion.FullVersion)"

        if (-not $?) {
            throw "Failed to commit changes"
        }
    }

    # Push to remote
    Write-Host "Pushing to remote..."
    if ($DryRun) {
        Write-Host "[DRY RUN] Would execute: git push -u origin $branchName" -ForegroundColor Cyan
    }
    else {
        git push -u origin $branchName

        if (-not $?) {
            throw "Failed to push to remote"
        }
    }

    # Create PR using gh
    Write-Host "Creating pull request..."

    $prTitle = "chore: update libobs to $newVersionString with OBS Studio $($obsVersion.FullVersion)"
    $prDescription = @"
## Description
Updates libobs bindings to the latest OBS Studio release ($($obsVersion.FullVersion)).

## Changes
- Bumped libobs version from $currentVersionString to $newVersionString
- Updated OBS Studio headers and bindings
- Updated semver metadata to $newMetadata

## Type of Change
- [x] Update/Bump version
- [x] Update dependencies
- [x] Update headers/bindings
"@

    if ($DryRun) {
        Write-Host "[DRY RUN] Would create PR with:" -ForegroundColor Cyan
        Write-Host "  Title: $prTitle" -ForegroundColor Cyan
        Write-Host "  Base: main" -ForegroundColor Cyan
        Write-Host "  Head: $branchName" -ForegroundColor Cyan
        Write-Host "  Description:" -ForegroundColor Cyan
        Write-Host $prDescription -ForegroundColor Cyan
        Write-Host "[DRY RUN] Would enable automerge with squash" -ForegroundColor Cyan
    }
    else {
        gh pr create --title $prTitle --body $prDescription --base main --head $branchName

        if (-not $?) {
            Write-Warning "Failed to create PR using gh"
        }
        else {
            Write-Host "Pull request created successfully"

            # Enable automerge
            Write-Host "Enabling automerge (squash)..."
            gh pr merge $branchName --auto --squash

            if (-not $?) {
                Write-Warning "Failed to enable automerge"
            }
            else {
                Write-Host "Automerge enabled successfully"
            }
        }
    }

    Write-Host "Script completed successfully!"
}
catch {
    Write-Error "An error occurred: $_"
    exit 1
}
finally {
    Pop-Location
}

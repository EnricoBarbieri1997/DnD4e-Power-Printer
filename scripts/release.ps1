param (
    [Parameter(Mandatory=$true)]
    [ValidateSet("patch", "minor", "major")]
    [string]$release_type
)

$CargoTomlPath = "Cargo.toml"
$CargoTomlContent = Get-Content $CargoTomlPath
$currentVersionLine = $CargoTomlContent | Select-String -Pattern 'version\s*=\s*"(.*)"'
$currentVersion = $currentVersionLine.Matches.Groups[1].Value

$versionParts = $currentVersion -split '\.'
$major = [int]$versionParts[0]
$minor = [int]$versionParts[1]
$patch = [int]$versionParts[2]

switch ($release_type) {
    "patch" { $patch++ }
    "minor" { $minor++; $patch = 0 }
    "major" { $major++; $minor = 0; $patch = 0 }
}

$newVersion = "$major.$minor.$patch"

$updatedCargoTomlContent = $CargoTomlContent -replace "version\s*=\s*`"$currentVersion`"", "version = `"$newVersion`""
Set-Content -Path $CargoTomlPath -Value $updatedCargoTomlContent

git add $CargoTomlPath
git commit -m "Bump version to $newVersion"
git tag "v$newVersion"
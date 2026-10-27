# 原生 Rust 地图构建入口
[CmdletBinding()]
param(
    [string]$Map,
    [switch]$Install,
    [switch]$Bump,
    [switch]$Release,
    [switch]$Wb,
    [string]$WbRoot,
    [string]$Note,
    [string[]]$Only,
    [string[]]$Skip,
    [switch]$NoZip
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$cargoArgs = @('run', '--release', '--manifest-path', (Join-Path $root 'Cargo.toml'), '--', 'build', '--root', $root)
if ($Map) { $cargoArgs += @('--map', $Map) }
if ($Install) { $cargoArgs += '--install' }
if ($Bump) { $cargoArgs += '--bump' }
if ($Release) { $cargoArgs += '--release' }
if ($Wb) { $cargoArgs += '--wb' }
if ($WbRoot) { $cargoArgs += @('--wb-root', $WbRoot) }
if ($Note) { $cargoArgs += @('--note', $Note) }
if ($NoZip) { $cargoArgs += '--no-zip' }
foreach ($stage in $Only) { $cargoArgs += @('--only', $stage) }
foreach ($stage in $Skip) { $cargoArgs += @('--skip', $stage) }
& cargo @cargoArgs
exit $LASTEXITCODE

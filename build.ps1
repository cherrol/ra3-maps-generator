# 一键构建 RA3 地图（PowerShell 包装）
#
#   .\build.ps1                       # 默认配方
#   .\build.ps1 -Map <id> -Install
#   .\build.ps1 -Map <id> -Only map,verify
[CmdletBinding()]
param(
    [string]$Map,
    [switch]$Install,
    [switch]$Bump,
    [switch]$Release,
    [switch]$Wb,
    [string]$Note,
    [string[]]$Only,
    [string[]]$Skip,
    [string]$Python = "$env:USERPROFILE\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$args = @("$root\tools\build_all.py")
if ($Map) { $args += @('--map', $Map) }
if ($Install) { $args += '--install' }
if ($Bump) { $args += '--bump' }
if ($Release) { $args += '--release' }
if ($Wb) { $args += '--wb' }
if ($Note) { $args += @('--note', $Note) }
foreach ($o in $Only) { $args += @('--only', $o) }
foreach ($s in $Skip) { $args += @('--skip', $s) }

if (-not (Test-Path -LiteralPath $Python)) {
    $Python = 'python'
}
Write-Host "==> $Python $($args -join ' ')" -ForegroundColor Cyan
& $Python -X utf8 @args
exit $LASTEXITCODE

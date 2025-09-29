# Build script for all architectures with automatic naming
# Usage: .\build-all.ps1

param(
    [switch]$Release = $true,
    [switch]$Debug = $false
)

$ErrorActionPreference = "Stop"

# Get version info
$version = (Get-Content Cargo.toml | Select-String 'version = "(.+)"').Matches[0].Groups[1].Value
$buildNumber = if (Test-Path "build_number.txt") { Get-Content "build_number.txt" } else { "1" }
$gitHash = try { (git rev-parse --short HEAD 2>$null) } catch { "unknown" }
$fullVersion = "$version+$buildNumber.$gitHash"

Write-Host "Building udp-over-tcp v$fullVersion" -ForegroundColor Green

# Determine build type
$buildType = if ($Debug) { "debug" } else { "release" }
$buildFlag = if ($Debug) { "" } else { "--release" }

# Define targets and their output names
$targets = @(
    @{
        target = "x86_64-pc-windows-gnu"
        suffix = "x86_64-windows.exe"
        description = "Windows x64"
    },
    @{
        target = "x86_64-unknown-linux-musl"
        suffix = "x86_64-linux"
        description = "Linux x64 (static)"
    },
    @{
        target = "aarch64-unknown-linux-musl"
        suffix = "aarch64-linux"
        description = "Linux ARM64 (static)"
    }
)

# Create output directory
$outputDir = "dist"
if (Test-Path $outputDir) {
    Remove-Item $outputDir -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDir | Out-Null

Write-Host "Output directory: $outputDir" -ForegroundColor Cyan

foreach ($target in $targets) {
    Write-Host "`nBuilding for $($target.description)..." -ForegroundColor Yellow
    
    # Add target if not already installed
    & "C:\Users\Nicolas\.cargo\bin\rustup.exe" target add $($target.target) 2>$null

    # Build for target
    $buildCmd = "C:\Users\Nicolas\.cargo\bin\cargo.exe build --target $($target.target) $buildFlag"
    Write-Host "Running: $buildCmd" -ForegroundColor Gray
    
    try {
        Invoke-Expression $buildCmd
        
        # Determine source path
        $sourcePath = "target\$($target.target)\$buildType\udp-over-tcp"
        if ($target.target -like "*windows*") {
            $sourcePath += ".exe"
        }
        
        # Create output filename with version
        $outputName = "udp-over-tcp-v$version-build$buildNumber-$($target.suffix)"
        $outputPath = Join-Path $outputDir $outputName
        
        # Copy and rename binary
        if (Test-Path $sourcePath) {
            Copy-Item $sourcePath $outputPath
            $size = [math]::Round((Get-Item $outputPath).Length / 1MB, 2)
            Write-Host "[OK] Built: $outputName ($size MB)" -ForegroundColor Green
        } else {
            Write-Host "[FAIL] Failed: Source not found at $sourcePath" -ForegroundColor Red
        }
        
    } catch {
        Write-Host "[FAIL] Build failed for $($target.description): $($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "`nBuild Summary:" -ForegroundColor Cyan
Write-Host "Version: $fullVersion" -ForegroundColor White
Write-Host "Build Type: $buildType" -ForegroundColor White
Write-Host "Output Directory: $outputDir" -ForegroundColor White

if (Test-Path $outputDir) {
    Write-Host "`nGenerated Binaries:" -ForegroundColor Cyan
    Get-ChildItem $outputDir | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
        Write-Host "  $($_.Name) ($size MB)" -ForegroundColor White
    }
}

Write-Host "`nBuild completed!" -ForegroundColor Green

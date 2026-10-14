# Authenticode-sign aravath_greybox.exe with a PFX when the secrets are present.
# Azure Artifact Signing is a separate workflow step. With neither configured,
# this script exits 0 so the Windows build still publishes an unsigned exe.
param(
    [Parameter(Mandatory = $true)]
    [string]$Exe
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Error "Executable not found: $Exe"
}

$hasPfx = -not [string]::IsNullOrWhiteSpace($env:WINDOWS_PFX_BASE64) -and -not [string]::IsNullOrWhiteSpace($env:WINDOWS_PFX_PASSWORD)
if (-not $hasPfx) {
    Write-Host "WINDOWS_PFX_BASE64 is not set. Skipping PFX Authenticode."
    exit 0
}

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
$pfx = Join-Path $tempRoot "aravath-codesign.pfx"
try {
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_PFX_BASE64))
    & signtool.exe sign /fd SHA256 /td SHA256 /tr http://timestamp.digicert.com /f $pfx /p $env:WINDOWS_PFX_PASSWORD $Exe
    if ($LASTEXITCODE -ne 0) {
        throw "signtool failed with exit code $LASTEXITCODE"
    }
    Write-Host "Signed $Exe with the PFX certificate."
}
finally {
    if (Test-Path -LiteralPath $pfx) {
        Remove-Item -LiteralPath $pfx -Force
    }
}

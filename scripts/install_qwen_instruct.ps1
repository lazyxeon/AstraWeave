param(
    [string]$Model = "qwen3.5:4b",
    [switch]$SkipProfiles
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found. Install Ollama, then rerun this script."
    }
}

Require-Command "ollama"

$repoRoot = Split-Path -Parent $PSScriptRoot
$modelfileDir = Join-Path $repoRoot "examples\llm_modelfiles"

Write-Host "Installing AstraWeave local Qwen LLM: $Model"
ollama pull $Model

if (-not $SkipProfiles) {
    $profiles = @(
        @{ Name = "qwen3-game"; File = "Modelfile.qwen3-game" },
        @{ Name = "qwen3-fast"; File = "Modelfile.qwen3-fast" },
        @{ Name = "qwen3-strategic"; File = "Modelfile.qwen3-strategic" }
    )

    foreach ($profile in $profiles) {
        $path = Join-Path $modelfileDir $profile.File
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Missing modelfile: $path"
        }
        Write-Host "Creating Ollama profile $($profile.Name) from $($profile.File)"
        ollama create $profile.Name -f $path
    }
}

Write-Host "Qwen install complete. Active default model: $Model"
Write-Host "Run checks with: cargo run -p ollama_probe_example"

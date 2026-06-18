# AstraWeave Benchmark Threshold Validation Script
# Week 3 Action 11: CI Benchmark Pipeline
# Week 4 Action 15: Enhanced with GitHub issue creation
# Validates benchmark results against performance thresholds

param(
    [Parameter(Mandatory=$false)]
    [string]$BenchmarkJsonPath = "benchmark_results/benchmarks.json",
    
    [Parameter(Mandatory=$false)]
    [string]$ThresholdsJsonPath = ".github/benchmark_thresholds.json",
    
    [Parameter(Mandatory=$false)]
    [switch]$Strict = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$UpdateBaseline = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$ShowDetails = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$CreateIssue = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$DryRun = $false
)

$ErrorActionPreference = "Stop"

# ANSI color codes for terminal output
$ColorReset = "`e[0m"
$ColorRed = "`e[31m"
$ColorGreen = "`e[32m"
$ColorYellow = "`e[33m"
$ColorCyan = "`e[36m"
$ColorBold = "`e[1m"

function Write-ColorOutput {
    param([string]$Message, [string]$Color = $ColorReset)
    Write-Host "${Color}${Message}${ColorReset}"
}

function Write-Header {
    param([string]$Message)
    Write-ColorOutput "`n=== $Message ===" $ColorCyan
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "✅ $Message" $ColorGreen
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "⚠️  $Message" $ColorYellow
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "❌ $Message" $ColorRed
}

function Format-Time {
    param([double]$Nanoseconds)
    
    if ($Nanoseconds -lt 1000) {
        return "{0:F2} ns" -f $Nanoseconds
    } elseif ($Nanoseconds -lt 1000000) {
        return "{0:F2} µs" -f ($Nanoseconds / 1000)
    } elseif ($Nanoseconds -lt 1000000000) {
        return "{0:F2} ms" -f ($Nanoseconds / 1000000)
    } else {
        return "{0:F2} s" -f ($Nanoseconds / 1000000000)
    }
}

Write-Header "AstraWeave Benchmark Threshold Validation"

# Check if benchmark results exist
if (-not (Test-Path $BenchmarkJsonPath)) {
    Write-Error "Benchmark results not found: $BenchmarkJsonPath"
    exit 1
}

# Load benchmark results
Write-ColorOutput "📊 Loading benchmark results from: $BenchmarkJsonPath"
try {
    $benchmarkResults = Get-Content $BenchmarkJsonPath -Raw | ConvertFrom-Json
} catch {
    Write-Error "Failed to parse benchmark JSON: $_"
    exit 1
}

Write-Success "Loaded $($benchmarkResults.Count) benchmark results"

# Check if thresholds file exists
$useThresholds = Test-Path $ThresholdsJsonPath
if ($useThresholds) {
    Write-ColorOutput "📋 Loading thresholds from: $ThresholdsJsonPath"
    try {
        $thresholds = Get-Content $ThresholdsJsonPath -Raw | ConvertFrom-Json
    } catch {
        Write-Error "Failed to parse thresholds JSON: $_"
        exit 1
    }
    Write-Success "Loaded $($thresholds.benchmarks.Count) threshold definitions"
} else {
    Write-Warning "No thresholds file found, running in discovery mode"
    Write-ColorOutput "  (Thresholds will be created from current results if -UpdateBaseline is specified)"
    $thresholds = @{
        version = "1.0"
        generated = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
        benchmarks = @{}
    }
}

# Validation results
$totalBenchmarks = 0
$passedBenchmarks = 0
$failedBenchmarks = 0
$newBenchmarks = 0
$regressions = @()

Write-Header "Validating Benchmark Results"

foreach ($result in $benchmarkResults) {
    $totalBenchmarks++
    $benchmarkName = $result.name
    $currentValue = [double]$result.value
    $unit = $result.unit
    
    if ($ShowDetails) {
        Write-ColorOutput "`n📈 Checking: $benchmarkName" $ColorBold
        Write-ColorOutput "   Current: $(Format-Time $currentValue)"
    }
    
    # Check if threshold exists for this benchmark
    if ($thresholds.benchmarks.PSObject.Properties.Name -contains $benchmarkName) {
        $threshold = $thresholds.benchmarks.$benchmarkName
        $baselineValue = [double]$threshold.baseline
        $maxValue = [double]$threshold.max_allowed
        $warnValue = if ($threshold.PSObject.Properties.Name -contains "warn_threshold") { 
            [double]$threshold.warn_threshold 
        } else { 
            $maxValue * 0.9 
        }
        
        # Calculate percentage change
        $percentChange = (($currentValue - $baselineValue) / $baselineValue) * 100
        
        if ($ShowDetails) {
            Write-ColorOutput "   Baseline: $(Format-Time $baselineValue)"
            Write-ColorOutput "   Max Allowed: $(Format-Time $maxValue)"
            Write-ColorOutput ("   Change: {0:F1}%" -f $percentChange)
        }
        
        # Determine pass/fail/warn
        if ($currentValue -le $maxValue) {
            if ($currentValue -le $warnValue) {
                $passedBenchmarks++
                $timeFormatted = Format-Time $currentValue
                $changeFormatted = "{0:F1}" -f $percentChange
                Write-Success "${benchmarkName}: ${timeFormatted} (✓ PASS, ${changeFormatted}% change)"
            } else {
                $passedBenchmarks++
                $timeFormatted = Format-Time $currentValue
                $changeFormatted = "{0:F1}" -f $percentChange
                Write-Warning "${benchmarkName}: ${timeFormatted} (⚠ WARN, ${changeFormatted}% above baseline, under limit)"
            }
        } else {
            $failedBenchmarks++
            $exceedance = (($currentValue - $maxValue) / $maxValue) * 100
            $timeFormatted = Format-Time $currentValue
            $exceedanceFormatted = "{0:F1}" -f $exceedance
            Write-Error "${benchmarkName}: ${timeFormatted} (❌ FAIL, ${exceedanceFormatted}% OVER LIMIT)"
            
            $regressions += [PSCustomObject]@{
                Name = $benchmarkName
                Current = $currentValue
                Baseline = $baselineValue
                MaxAllowed = $maxValue
                Exceedance = $exceedance
                PercentChange = $percentChange
            }
        }
        
        # Update baseline if requested
        if ($UpdateBaseline) {
            $thresholds.benchmarks.$benchmarkName.baseline = $currentValue
            # Keep max_allowed as 150% of new baseline (default policy)
            $thresholds.benchmarks.$benchmarkName.max_allowed = $currentValue * 1.5
            $thresholds.benchmarks.$benchmarkName.warn_threshold = $currentValue * 1.25
            $thresholds.benchmarks.$benchmarkName.last_updated = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
        }
        
    } else {
        # New benchmark (no threshold defined)
        $newBenchmarks++
        $timeFormatted = Format-Time $currentValue
        Write-ColorOutput "🆕 ${benchmarkName}: ${timeFormatted} (NEW)" $ColorYellow
        
        # Add to thresholds with conservative limits if updating baseline
        if ($UpdateBaseline) {
            $thresholds.benchmarks | Add-Member -NotePropertyName $benchmarkName -NotePropertyValue @{
                baseline = $currentValue
                max_allowed = $currentValue * 1.5  # Allow 50% regression by default
                warn_threshold = $currentValue * 1.25  # Warn at 25% regression
                unit = $unit
                created = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
                last_updated = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
                description = "Auto-generated threshold"
            }
        }
    }
}

# Summary
Write-Header "Validation Summary"
Write-ColorOutput "Total benchmarks: $totalBenchmarks"
Write-Success "Passed: $passedBenchmarks"
if ($failedBenchmarks -gt 0) {
    Write-Error "Failed: $failedBenchmarks"
} else {
    Write-Success "Failed: $failedBenchmarks"
}
if ($newBenchmarks -gt 0) {
    Write-ColorOutput "New: $newBenchmarks" $ColorYellow
}

# Detailed regression report
if ($regressions.Count -gt 0) {
    Write-Header "Performance Regressions Detected"
    Write-ColorOutput ""
    Write-ColorOutput "| Benchmark | Current | Baseline | Max Allowed | Over Limit |" $ColorRed
    Write-ColorOutput "|-----------|---------|----------|-------------|------------|" $ColorRed
    
    foreach ($regression in $regressions) {
        $current = Format-Time $regression.Current
        $baseline = Format-Time $regression.Baseline
        $maxAllowed = Format-Time $regression.MaxAllowed
        $exceedance = "{0:F1}%" -f $regression.Exceedance
        
        Write-ColorOutput "| $($regression.Name) | $current | $baseline | $maxAllowed | $exceedance |" $ColorRed
    }
    Write-ColorOutput ""
}

# Update baseline file if requested
if ($UpdateBaseline) {
    Write-Header "Updating Baseline Thresholds"
    
    # Update metadata
    $thresholds.version = "1.0"
    $thresholds.generated = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    
    # Save to file
    $thresholdsDir = Split-Path -Parent $ThresholdsJsonPath
    if (-not (Test-Path $thresholdsDir)) {
        New-Item -ItemType Directory -Path $thresholdsDir -Force | Out-Null
    }
    
    $thresholds | ConvertTo-Json -Depth 10 | Set-Content $ThresholdsJsonPath -Encoding UTF8
    Write-Success "Baseline thresholds updated: $ThresholdsJsonPath"
    Write-ColorOutput "  - Total benchmarks: $totalBenchmarks"
    Write-ColorOutput "  - New benchmarks added: $newBenchmarks"
}

# GitHub issue creation for regressions
if ($CreateIssue -and $failedBenchmarks -gt 0) {
    Write-Header "Creating GitHub Issue for Regressions"
    
    $issueTitle = "⚠️ Benchmark Regression Detected"
    $issueBody = @"
## Automated Regression Alert

**Date**: $(Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
**Regressions Found**: $failedBenchmarks benchmark(s)

### Failed Benchmarks

| Benchmark | Current | Threshold | Exceeded By |
|-----------|---------|-----------|-------------|
"@
    
    foreach ($result in $results) {
        if ($result.Status -eq "FAILED") {
            $exceedPct = (($result.CurrentValue - $result.MaxAllowed) / $result.MaxAllowed * 100).ToString("F1")
            $issueBody += "`n| ``$($result.Name)`` | $($result.CurrentValue) $($result.Unit) | $($result.MaxAllowed) $($result.Unit) | +$exceedPct% |"
        }
    }
    
    $issueBody += @"


### Investigation Steps

1. **Review Dashboard**: [View 30-day trends](https://lazyxeon.github.io/AstraWeave/)
2. **Run Local Benchmarks**: ``cargo bench``
3. **Validate Thresholds**: ``./scripts/check_benchmark_thresholds.ps1 -ShowDetails``
4. **Profile Hot Paths**: ``cargo flamegraph --bench <benchmark_name>``

### Checklist

- [ ] Identify root cause (code change vs. threshold drift)
- [ ] Bisect commits to find performance regression
- [ ] Fix performance issue OR update threshold if intentional
- [ ] Verify fix with local benchmarks
- [ ] Update ``benchmark_thresholds.json`` if needed

**Auto-generated by check_benchmark_thresholds.ps1**
"@
    
    if ($DryRun) {
        Write-ColorOutput "`n--- GitHub Issue (DRY RUN) ---" $ColorYellow
        Write-ColorOutput "Title: $issueTitle"
        Write-ColorOutput "Body:`n$issueBody"
        Write-ColorOutput "--- End Dry Run ---`n"
    } else {
        # Attempt to create issue using GitHub CLI (gh)
        try {
            $ghInstalled = Get-Command gh -ErrorAction SilentlyContinue
            if ($ghInstalled) {
                $tempFile = [System.IO.Path]::GetTempFileName()
                Set-Content -Path $tempFile -Value $issueBody
                
                gh issue create --title $issueTitle --body-file $tempFile --label "performance,regression,automated"
                
                Remove-Item $tempFile
                Write-Success "GitHub issue created successfully"
            } else {
                Write-Warning "GitHub CLI (gh) not installed. Skipping issue creation."
                Write-Warning "Install: https://cli.github.com/"
            }
        } catch {
            Write-Warning "Failed to create GitHub issue: $_"
        }
    }
}

# Exit code logic
if ($failedBenchmarks -gt 0) {
    if ($Strict) {
        Write-Error "`n🚨 STRICT MODE: Exiting with failure due to performance regressions"
        exit 1
    } else {
        Write-Warning "`n⚠️  Performance regressions detected, but not failing (use -Strict to enforce)"
        exit 0
    }
} elseif ($newBenchmarks -gt 0 -and -not $UpdateBaseline) {
    Write-Warning "`n⚠️  New benchmarks found without thresholds (use -UpdateBaseline to add them)"
    exit 0
} else {
    Write-Success "`n✅ All benchmarks passed validation!"
    exit 0
}


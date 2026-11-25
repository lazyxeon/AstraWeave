# Export Benchmark Results to JSONL History
# Extracts Criterion benchmark data and appends to history.jsonl for dashboard visualization
#
# Usage:
#   .\scripts\export_benchmark_jsonl.ps1
#   .\scripts\export_benchmark_jsonl.ps1 -BenchmarkDir "custom/path"
#   .\scripts\export_benchmark_jsonl.ps1 -MaxAgeDays 60

param(
    [string]$BenchmarkDir = "target/criterion",
    [string]$OutputFile = "target/benchmark-data/history.jsonl",
    [int]$MaxAgeDays = 30,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "HH:mm:ss"
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARN" { "Yellow" }
        "SUCCESS" { "Green" }
        default { "White" }
    }
    Write-Host "[$timestamp] $Message" -ForegroundColor $color
}

function Get-GitMetadata {
    try {
        $sha = git rev-parse HEAD 2>&1
        $branch = git rev-parse --abbrev-ref HEAD 2>&1
        $dirty = (git status --porcelain 2>&1).Count -gt 0
        
        return @{
            sha = if ($sha -match "^[0-9a-f]{40}$") { $sha.Substring(0, 8) } else { "unknown" }
            branch = if ($branch -and $branch -notmatch "fatal") { $branch } else { "unknown" }
            dirty = $dirty
        }
    }
    catch {
        return @{ sha = "unknown"; branch = "unknown"; dirty = $false }
    }
}

function Get-HardwareProfile {
    <#
    .SYNOPSIS
    Detects the current hardware and returns a stable ID + human-readable label.
    This allows the dashboard to separate baseline (HP Pavilion) data from user runs.
    #>
    try {
        $cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
        $ramGB = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
        $model = (Get-CimInstance Win32_ComputerSystem).Model.Trim()
        $manufacturer = (Get-CimInstance Win32_ComputerSystem).Manufacturer.Trim()
        $os = [System.Environment]::OSVersion.VersionString

        # Build a stable hash from CPU + RAM + model so the same machine always gets the same ID
        $raw = "$cpu|${ramGB}GB|$model"
        $hashBytes = [System.Security.Cryptography.SHA256]::Create().ComputeHash(
            [System.Text.Encoding]::UTF8.GetBytes($raw)
        )
        $hardwareId = [BitConverter]::ToString($hashBytes[0..3]).Replace('-','').ToLower()

        # Human-friendly label
        $label = "$manufacturer $model ($cpu, ${ramGB}GB)"

        return @{
            hardware_id    = $hardwareId
            hardware_label = $label
            cpu            = $cpu
            ram_gb         = $ramGB
            model          = $model
            manufacturer   = $manufacturer
            os             = $os
        }
    }
    catch {
        return @{
            hardware_id    = "unknown"
            hardware_label = "Unknown Hardware"
            cpu            = "unknown"
            ram_gb         = 0
            model          = "unknown"
            manufacturer   = "unknown"
            os             = "unknown"
        }
    }
}

function Parse-CriterionEstimates {
    param([string]$EstimatesJsonPath)
    
    if (-not (Test-Path $EstimatesJsonPath)) {
        Write-Log "Estimates file not found: $EstimatesJsonPath" "WARN"
        return $null
    }
    
    try {
        $estimates = Get-Content $EstimatesJsonPath -Raw | ConvertFrom-Json
        
        # Criterion stores times in nanoseconds
        $meanNs = $estimates.mean.point_estimate
        $stddevNs = $estimates.std_dev.point_estimate
        
        return @{
            mean_ns = [math]::Round($meanNs, 2)
            stddev_ns = [math]::Round($stddevNs, 2)
            unit = "ns"
        }
    }
    catch {
        Write-Log "Failed to parse $EstimatesJsonPath : $_" "WARN"
        return $null
    }
}

function Get-FriendlyName {
    param([string]$BenchmarkName)
    
    # Comprehensive friendly name mapping for common benchmarks
    # Organized by system for maintainability
    $friendlyNames = @{
        # === MATH/SIMD ===
        'vec3_dot/scalar' = 'Vector Dot Product (Scalar)'
        'vec3_dot/simd' = 'Vector Dot Product (SIMD)'
        'vec3_cross/scalar' = 'Vector Cross Product (Scalar)'
        'vec3_cross/simd' = 'Vector Cross Product (SIMD)'
        'vec3_normalize/scalar' = 'Vector Normalize (Scalar)'
        'vec3_normalize/simd' = 'Vector Normalize (SIMD)'
        'mat4_mul/scalar' = 'Matrix Multiplication (Scalar)'
        'mat4_mul/simd' = 'Matrix Multiplication (SIMD)'
        'simd_movement' = 'SIMD Movement Batch Processing'
        'quat_multiply' = 'Quaternion Multiplication'
        'transform_points' = 'Transform Points (Batch)'
        
        # === RENDERING ===
        'culling_performance/with_backface_culling' = 'Rendering with Back-Face Culling'
        'culling_performance/without_backface_culling' = 'Rendering without Back-Face Culling'
        'rendering_frame_time' = 'Frame Time Baseline'
        'shader_compilation' = 'Shader Compilation Time'
        'texture_operations' = 'Texture Operations'
        'mesh_optimization' = 'Mesh Optimization Pipeline'
        'vertex_compression' = 'Vertex Compression (Octahedral Normals)'
        'lod_generation' = 'LOD Generation (Quadric Error)'
        'instancing' = 'GPU Instancing (Batch Draw)'
        'cluster_culling_gpu' = 'GPU Cluster Culling'
        'cluster_culling_cpu' = 'CPU Cluster Culling'
        
        # === ECS CORE ===
        'world_creation' = 'ECS World Creation'
        'entity_spawn' = 'Entity Spawn (Single)'
        'entity_spawn_batch' = 'Entity Spawn (Batch)'
        'system_execution' = 'System Execution Pipeline'
        'archetype_move' = 'Archetype Component Migration'
        'query_iteration' = 'Component Query Iteration'
        'event_dispatch' = 'Event System Dispatch'
        'ecs_performance' = 'ECS Stress Test (High Load)'
        
        # === AI SYSTEMS ===
        'ai_core_loop' = 'AI Core Loop (Perception->Reasoning->Planning->Action)'
        'goap_planning' = 'GOAP Planning (Goal-Oriented)'
        'behavior_tree' = 'Behavior Tree Execution'
        'utility_ai' = 'Utility AI Decision Making'
        'llm_prompt_generation' = 'LLM Prompt Generation'
        'llm_response_parsing' = 'LLM Response Parsing'
        'llm_tool_extraction' = 'LLM Tool Extraction'
        'context_building' = 'AI Context Building'
        'memory_recall' = 'AI Memory Recall'
        'persona_simulation' = 'NPC Persona Simulation'
        'rag_retrieval' = 'RAG Document Retrieval'
        'rag_embedding' = 'RAG Embedding Generation'
        
        # === PHYSICS ===
        'raycast' = 'Physics Raycast'
        'character_controller' = 'Character Controller Movement'
        'rigid_body' = 'Rigid Body Physics Step'
        'collision_detection' = 'Collision Detection (Broad Phase)'
        'spatial_hash' = 'Spatial Hash Grid Partitioning'
        
        # === NAVIGATION ===
        'navmesh_pathfinding' = 'Navmesh A* Pathfinding'
        'navmesh_generation' = 'Navmesh Generation'
        'portal_graph' = 'Portal Graph Navigation'
        
        # === TERRAIN & WORLD ===
        'terrain_generation' = 'Terrain Generation (Chunk)'
        'voxel_meshing' = 'Voxel Meshing (Marching Cubes)'
        'pcg_dungeon' = 'PCG Dungeon Generation'
        'pcg_encounter' = 'PCG Encounter Generation'
        
        # === GAMEPLAY ===
        'enemy_spawner/determine_archetype' = 'Enemy Archetype Determination'
        'player_abilities' = 'Player Ability System'
        'player_ability_activation' = 'Player Ability Activation'
        'quest_objectives' = 'Quest Objective Tracking'
        'integrated_systems' = 'Integrated System Performance'
        'weaving' = 'Fate-Weaving System'
        
        # === PERSISTENCE & NETWORKING ===
        'save_serialization' = 'Save File Serialization (ECS)'
        'save_deserialization' = 'Save File Deserialization (ECS)'
        'network_replication' = 'Network Entity Replication'
        'network_prediction' = 'Network Client-Side Prediction'
        'persistence_stress' = 'Persistence Stress Test'
        
        # === INPUT & AUDIO ===
        'input_binding' = 'Input Binding Creation'
        'input_polling' = 'Input State Polling'
        'audio_mixing' = 'Audio Mixing (Multi-Bus)'
        'spatial_audio' = 'Spatial Audio Positioning'
        
        # === UI & TOOLS ===
        'ui_rendering' = 'UI Rendering (egui)'
        'gizmo_rendering' = 'Editor Gizmo Rendering'
        'widget_interaction' = 'Widget Interaction Handling'
        'hash_perf' = 'Build Hash Performance'
    }
    
    # Check for exact match
    if ($friendlyNames.ContainsKey($BenchmarkName)) {
        return $friendlyNames[$BenchmarkName]
    }
    
    # Check for partial match (for parameterized benchmarks)
    foreach ($key in $friendlyNames.Keys) {
        if ($BenchmarkName -like "$key*") {
            $param = $BenchmarkName -replace "^$key/", ""
            return "$($friendlyNames[$key]) ($param)"
        }
    }
    
    # Fallback: convert underscores to spaces and title case
    $readable = $BenchmarkName -replace '_', ' ' -replace '/', ' - '
    return (Get-Culture).TextInfo.ToTitleCase($readable.ToLower())
}

function Export-BenchmarkResults {
    param([string]$BenchmarkRoot, [string]$OutputPath)
    
    Write-Log "Scanning Criterion benchmark results in $BenchmarkRoot..."
    
    if (-not (Test-Path $BenchmarkRoot)) {
        Write-Log "Benchmark directory not found: $BenchmarkRoot" "ERROR"
        Write-Log "Run 'cargo bench' first to generate benchmark data" "ERROR"
        return 0
    }
    
    # Ensure output directory exists
    $outputDir = Split-Path $OutputPath -Parent
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
        Write-Log "Created output directory: $outputDir"
    }
    
    # Load existing entries to avoid duplicates
    $existingEntries = @{}
    if (Test-Path $OutputPath) {
        Get-Content $OutputPath | ForEach-Object {
            try {
                $entry = $_ | ConvertFrom-Json
                $key = "$($entry.timestamp)_$($entry.benchmark_name)"
                $existingEntries[$key] = $true
            } catch { }
        }
        Write-Log "Loaded $($existingEntries.Count) existing entries for deduplication"
    }
    
    # Get git metadata and hardware profile
    $git = Get-GitMetadata
    $hw = Get-HardwareProfile
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    
    Write-Log "Hardware: $($hw.hardware_label) [ID: $($hw.hardware_id)]"
    
    # Find all benchmark results (Criterion stores in */base/estimates.json)
    $benchmarkDirs = Get-ChildItem -Path $BenchmarkRoot -Recurse -Filter "estimates.json" -File |
                     Where-Object { $_.DirectoryName -match '[/\\]base$' }
    
    Write-Log "Found $($benchmarkDirs.Count) benchmark result directories"
    
    $exportCount = 0
    $skippedCount = 0
    $entries = @()
    
    foreach ($estimatesFile in $benchmarkDirs) {
        # Parse path to extract benchmark name
        # Examples:
        #   target/criterion/culling_performance/with_backface_culling/base/estimates.json
        #   target/criterion/enemy_spawner/determine_archetype/1/base/estimates.json
        #   target/criterion/vec3_dot/scalar/base/estimates.json
        $fullPath = $estimatesFile.FullName
        
        # Extract the benchmark path between criterion/ and /base/estimates.json
        if ($fullPath -match 'criterion[/\\](.+)[/\\]base[/\\]estimates\.json$') {
            $benchmarkPath = $matches[1]
            $pathParts = $benchmarkPath -split '[/\\]'
            
            if ($pathParts.Count -lt 1) {
                if ($Verbose) { Write-Log "Skipping invalid path: $benchmarkPath" "WARN" }
                continue
            }
            
            # Handle different path structures:
            # 1. <group>/<variant>/base (2 parts) - e.g., vec3_dot/scalar
            # 2. <group>/<subgroup>/<variant>/base (3 parts) - e.g., enemy_spawner/determine_archetype/1
            # 3. <crate>/<group>/<variant>/base (3+ parts) - e.g., astraweave-math/vec3_dot/scalar
            
            $group = $pathParts[0]
            
            if ($pathParts.Count -eq 1) {
                # Single-level benchmark
                $benchName = $group
                $fullName = $group
                $crate = $group
            } elseif ($pathParts.Count -eq 2) {
                # Two-level: group/variant
                $variant = $pathParts[1]
                $benchName = $variant
                $fullName = "${group}/${variant}"
                $crate = $group
            } else {
                # Three+ levels: could be crate/group/variant or group/subgroup/variant
                # Use all parts as the full name
                $benchName = $pathParts[-1]  # Last part is the specific variant/parameter
                $fullName = $pathParts -join '/'
                $crate = $pathParts[0]
            }
            
            # Generate friendly display name
            $displayName = Get-FriendlyName -BenchmarkName $fullName
        } else {
            if ($Verbose) { Write-Log "Skipping non-matching path: $fullPath" "WARN" }
            continue
        }
        
        # Parse estimates
        $stats = Parse-CriterionEstimates -EstimatesJsonPath $estimatesFile.FullName
        
        if ($null -eq $stats) {
            continue
        }
        
        # Check for duplicate entry
        $entryKey = "${timestamp}_${fullName}"
        if ($existingEntries.ContainsKey($entryKey)) {
            if ($Verbose) {
                Write-Log "Skipping duplicate: $displayName" "WARN"
            }
            $skippedCount++
            continue
        }
        
        # Create JSONL entry (includes hardware identification for baseline vs user tracking)
        $entry = @{
            timestamp = $timestamp
            benchmark_name = $fullName
            display_name = $displayName
            value = $stats.mean_ns
            stddev = $stats.stddev_ns
            unit = $stats.unit
            git_sha = $git.sha
            git_branch = $git.branch
            git_dirty = $git.dirty
            crate = $crate
            group = $group
            name = $benchName
            hardware_id = $hw.hardware_id
            hardware_label = $hw.hardware_label
        } | ConvertTo-Json -Compress
        
        $entries += $entry
        $exportCount++
        $existingEntries[$entryKey] = $true
        
        if ($Verbose) {
            Write-Log "Exported: $displayName = $($stats.mean_ns) ns"
        }
    }
    
    if ($exportCount -eq 0 -and $skippedCount -eq 0) {
        Write-Log "No benchmark results found in $BenchmarkRoot" "WARN"
        Write-Log "Run 'cargo bench' to generate data" "WARN"
        return 0
    }
    
    if ($skippedCount -gt 0) {
        Write-Log "Skipped $skippedCount duplicate entries (already in history)" "SUCCESS"
    }
    
    # Append to history file (JSONL = one JSON object per line)
    if ($entries.Count -gt 0) {
        $entries | Out-File -FilePath $OutputPath -Append -Encoding utf8
        Write-Log "Exported $exportCount new benchmark results to $OutputPath" "SUCCESS"
    }
    else {
        Write-Log "No new benchmarks to export (all $skippedCount entries were duplicates)" "SUCCESS"
    }
    
    # Rotate old entries (keep last N days)
    Rotate-OldEntries -HistoryFile $OutputPath -MaxAgeDays $MaxAgeDays
    
    return $exportCount
}

function Write-Metadata {
    param([string]$HistoryFile, [string]$OutputDir)
    if (-not (Test-Path $HistoryFile)) { return }
    $lines = Get-Content $HistoryFile | Where-Object { $_ -ne '' }
    $total = $lines.Count

    $dates = $lines | ForEach-Object { ($_ | ConvertFrom-Json).timestamp } | Sort-Object
    $oldest = $dates | Select-Object -First 1
    $newest = $dates | Select-Object -Last 1

    $meta = @{ generated_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'); total_snapshots = $total; oldest = $oldest; newest = $newest }
    $meta | ConvertTo-Json -Compress | Set-Content -Path (Join-Path $OutputDir 'metadata.json') -Encoding utf8
    Write-Log "Wrote metadata to $OutputDir/metadata.json" "SUCCESS"
}

function Rotate-OldEntries {
    param([string]$HistoryFile, [int]$MaxAgeDays)
    
    if (-not (Test-Path $HistoryFile)) {
        return
    }
    
    Write-Log "Rotating entries older than $MaxAgeDays days..."
    
    $cutoffDate = (Get-Date).AddDays(-$MaxAgeDays).ToUniversalTime()
    $lines = Get-Content $HistoryFile
    $kept = 0
    $removed = 0
    
    $tempFile = "$HistoryFile.tmp"
    
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        
        try {
            $entry = $line | ConvertFrom-Json
            $entryDate = [DateTime]::Parse($entry.timestamp)
            
            if ($entryDate -gt $cutoffDate) {
                $line | Out-File -FilePath $tempFile -Append -Encoding utf8
                $kept++
            }
            else {
                $removed++
            }
        }
        catch {
            # Keep malformed entries (don't lose data)
            $line | Out-File -FilePath $tempFile -Append -Encoding utf8
            $kept++
        }
    }
    
    if ($removed -gt 0) {
        Move-Item -Path $tempFile -Destination $HistoryFile -Force
        Write-Log "Removed $removed old entries, kept $kept" "SUCCESS"
    }
    else {
        if (Test-Path $tempFile) {
            Remove-Item $tempFile
        }
        Write-Log "No entries to rotate (all within $MaxAgeDays days)"
    }
}

# Main execution
Write-Log "=== AstraWeave Benchmark JSONL Exporter ==="
Write-Log "Benchmark Directory: $BenchmarkDir"
Write-Log "Output File: $OutputFile"
Write-Log "Max Age: $MaxAgeDays days"
Write-Log ""

$count = Export-BenchmarkResults -BenchmarkRoot $BenchmarkDir -OutputPath $OutputFile

if ($count -gt 0) {
    Write-Log ""
    Write-Log "=== Export Complete ===" "SUCCESS"
    Write-Log "Total benchmarks exported: $count" "SUCCESS"
    
    $fileSize = (Get-Item $OutputFile).Length
    Write-Log "History file size: $([math]::Round($fileSize / 1KB, 2)) KB" "SUCCESS"

    # Write small metadata JSON to the output directory for dashboard summaries
    $outputDir = Split-Path $OutputFile -Parent
    Write-Metadata -HistoryFile $OutputFile -OutputDir $outputDir
    
    Write-Log ""
    Write-Log "Next steps:"
    Write-Log "  1. View JSONL: Get-Content $OutputFile | Select -Last 5 | ConvertFrom-Json | Format-Table"
    Write-Log "  2. Build dashboard: python -m http.server 8000, from tools/benchmark-dashboard"
    Write-Log "  3. Open dashboard: http://localhost:8000"
}
else {
    Write-Log ""
    Write-Log "=== Export Failed ===" "ERROR"
    Write-Log "No benchmark data found. Run 'cargo bench' first." "ERROR"
    exit 1
}

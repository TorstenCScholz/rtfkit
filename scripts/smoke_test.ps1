<#
.SYNOPSIS
    Smoke Test Script for rtfkit (Windows PowerShell)
    Verifies that the binary runs correctly and basic conversion works.

.DESCRIPTION
    This script performs a series of smoke tests on the rtfkit binary to ensure
    it is functioning correctly. It tests basic functionality like version output,
    help output, RTF to DOCX conversion, JSON report generation, and error handling.

.PARAMETER BinaryPath
    Path to the rtfkit binary to test.

.EXAMPLE
    .\smoke_test.ps1 -BinaryPath .\target\release\rtfkit.exe

.EXAMPLE
    .\smoke_test.ps1 .\artifacts\rtfkit.exe

.NOTES
    Exit codes:
        0 - All tests passed
        1 - Test failure
        2 - Missing arguments or invalid state
#>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$BinaryPath
)

# Test counters
$script:TestsPassed = 0
$script:TestsFailed = 0

# Helper functions
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-TestHeader {
    param([string]$TestName)
    Write-Host ""
    Write-Host "=== TEST: $TestName ===" -ForegroundColor Green
}

function Test-Passed {
    param([string]$Message)
    Write-Info "✅ PASSED: $Message"
    $script:TestsPassed++
}

function Test-Failed {
    param([string]$Message)
    Write-Error "❌ FAILED: $Message"
    $script:TestsFailed++
}

# Verify binary exists
if (-not (Test-Path $BinaryPath)) {
    Write-Error "Binary not found: $BinaryPath"
    exit 2
}

# Create temporary working directory
$WorkDir = Join-Path $env:TEMP "rtfkit-smoke-test-$(Get-Random)"
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null
Write-Info "Working directory: $WorkDir"

# Cleanup function
function Cleanup {
    if (Test-Path $WorkDir) {
        Remove-Item -Path $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Register cleanup
trap {
    Cleanup
    exit 1
}

# ============================================
# Test 1: Binary runs and shows version
# ============================================
Write-TestHeader "Binary version check"

try {
    $VersionOutput = & $BinaryPath --version 2>&1
    if ($LASTEXITCODE -eq 0 -and $VersionOutput) {
        Write-Info "Version output: $VersionOutput"
        Test-Passed "Binary executes and shows version"
    } else {
        Test-Failed "Binary failed to execute with --version"
    }
} catch {
    Test-Failed "Exception running --version: $_"
}

# ============================================
# Test 2: Binary shows help
# ============================================
Write-TestHeader "Help output check"

try {
    $HelpOutput = & $BinaryPath --help 2>&1
    if ($LASTEXITCODE -eq 0 -and $HelpOutput -match "rtfkit" -and $HelpOutput -match "convert") {
        Write-Info "Help output contains expected content"
        Test-Passed "Help output is valid"
    } else {
        Test-Failed "Help output missing expected content"
    }
} catch {
    Test-Failed "Exception running --help: $_"
}

# ============================================
# Test 3: Convert simple RTF to DOCX
# ============================================
Write-TestHeader "Basic RTF to DOCX conversion"

$SimpleRtf = Join-Path $WorkDir "simple.rtf"
$RtfContent = @"
{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
\viewkind4\uc1\pard\f0\fs24
Hello, World!\par
}
"@
Set-Content -Path $SimpleRtf -Value $RtfContent -NoNewline

$OutputDocx = Join-Path $WorkDir "output.docx"

try {
    $ConvertOutput = & $BinaryPath convert $SimpleRtf --output $OutputDocx 2>&1
    if ($LASTEXITCODE -eq 0) {
        if (Test-Path $OutputDocx) {
            # Verify it's a valid ZIP (DOCX is a ZIP file)
            try {
                Add-Type -AssemblyName System.IO.Compression.FileSystem
                $Zip = [System.IO.Compression.ZipFile]::OpenRead($OutputDocx)
                $HasDocumentXml = $Zip.Entries | Where-Object { $_.FullName -eq "word/document.xml" }
                $Zip.Dispose()
                
                if ($HasDocumentXml) {
                    Write-Info "DOCX file is valid and contains word/document.xml"
                    Test-Passed "RTF to DOCX conversion produces valid DOCX"
                } else {
                    Test-Failed "DOCX missing word/document.xml"
                }
            } catch {
                Test-Failed "Output is not a valid ZIP/DOCX file: $_"
            }
        } else {
            Test-Failed "Output DOCX file was not created"
        }
    } else {
        Test-Failed "Conversion command failed with exit code $LASTEXITCODE"
    }
} catch {
    Test-Failed "Exception during conversion: $_"
}

# ============================================
# Test 4: Convert RTF to JSON report
# ============================================
Write-TestHeader "RTF to JSON report conversion"

try {
    $JsonOutput = & $BinaryPath convert $SimpleRtf --format json 2>&1
    if ($LASTEXITCODE -eq 0 -and $JsonOutput) {
        try {
            $JsonObj = $JsonOutput | ConvertFrom-Json
            if ($JsonObj.stats.paragraph_count -ne $null) {
                Write-Info "JSON report contains stats"
                Test-Passed "JSON report generation works"
            } else {
                Test-Failed "JSON report missing stats field"
            }
        } catch {
            Test-Failed "Output is not valid JSON: $_"
        }
    } else {
        Test-Failed "JSON report generation failed"
    }
} catch {
    Test-Failed "Exception during JSON report: $_"
}

# ============================================
# Test 5: Convert RTF to text report
# ============================================
Write-TestHeader "RTF to text report conversion"

try {
    $TextOutput = & $BinaryPath convert $SimpleRtf --format text 2>&1
    if ($LASTEXITCODE -eq 0 -and $TextOutput -match "Conversion Report") {
        Write-Info "Text report contains expected header"
        Test-Passed "Text report generation works"
    } else {
        Test-Failed "Text report missing expected content"
    }
} catch {
    Test-Failed "Exception during text report: $_"
}

# ============================================
# Test 6: Emit IR to JSON file
# ============================================
Write-TestHeader "IR emission to JSON file"

$IrFile = Join-Path $WorkDir "ir.json"

try {
    $IrOutput = & $BinaryPath convert $SimpleRtf --emit-ir $IrFile 2>&1
    if ($LASTEXITCODE -eq 0) {
        if (Test-Path $IrFile) {
            try {
                $IrContent = Get-Content $IrFile -Raw | ConvertFrom-Json
                if ($IrContent.blocks -ne $null) {
                    Write-Info "IR file contains blocks array"
                    Test-Passed "IR emission works"
                } else {
                    Test-Failed "IR file missing blocks field"
                }
            } catch {
                Test-Failed "IR file is not valid JSON: $_"
            }
        } else {
            Test-Failed "IR file was not created"
        }
    } else {
        Test-Failed "IR emission command failed"
    }
} catch {
    Test-Failed "Exception during IR emission: $_"
}

# ============================================
# Test 7: Handle non-existent file gracefully
# ============================================
Write-TestHeader "Error handling for non-existent file"

$NonExistent = Join-Path $WorkDir "nonexistent.rtf"

try {
    $ErrorOutput = & $BinaryPath convert $NonExistent 2>&1
    $ExitCode = $LASTEXITCODE
    
    if ($ExitCode -ne 0) {
        Write-Info "Non-zero exit code: $ExitCode"
        if ($ErrorOutput -match "Failed to read" -or $ErrorOutput -match "Error") {
            Test-Passed "Error handling for missing file works"
        } else {
            Test-Failed "Error message not informative"
        }
    } else {
        Test-Failed "Should have failed for non-existent file"
    }
} catch {
    Test-Failed "Exception during error handling test: $_"
}

# ============================================
# Test 8: Strict mode with clean document
# ============================================
Write-TestHeader "Strict mode with clean document"

try {
    $StrictOutput = & $BinaryPath convert $SimpleRtf --strict 2>&1
    if ($LASTEXITCODE -eq 0 -and $StrictOutput) {
        Test-Passed "Strict mode works with clean document"
    } else {
        Test-Failed "Strict mode failed on clean document"
    }
} catch {
    Test-Failed "Exception during strict mode test: $_"
}

# ============================================
# Test 9: Force flag overwrites existing file
# ============================================
Write-TestHeader "Force flag overwrites existing file"

$ExistingDocx = Join-Path $WorkDir "existing.docx"
Set-Content -Path $ExistingDocx -Value "dummy"

# First try without force - should fail
try {
    $NoForceOutput = & $BinaryPath convert $SimpleRtf --output $ExistingDocx 2>&1
    $ExitCode = $LASTEXITCODE
    
    if ($ExitCode -ne 0 -and $NoForceOutput -match "already exists") {
        Write-Info "Correctly refuses to overwrite without --force"
        
        # Now try with force - should succeed
        $ForceOutput = & $BinaryPath convert $SimpleRtf --output $ExistingDocx --force 2>&1
        if ($LASTEXITCODE -eq 0) {
            if (Test-Path $ExistingDocx) {
                # Verify it's now a valid DOCX
                try {
                    $Zip = [System.IO.Compression.ZipFile]::OpenRead($ExistingDocx)
                    $Zip.Dispose()
                    Test-Passed "Force flag overwrites existing file"
                } catch {
                    Test-Failed "Force flag did not create valid output"
                }
            } else {
                Test-Failed "Output file not created with --force"
            }
        } else {
            Test-Failed "Force flag conversion failed"
        }
    } else {
        Test-Failed "Should have refused to overwrite without --force"
    }
} catch {
    Test-Failed "Exception during force flag test: $_"
}

# ============================================
# Test 10: Complex RTF with tables
# ============================================
Write-TestHeader "Complex RTF with table conversion"

$TableRtf = Join-Path $WorkDir "table.rtf"
$TableRtfContent = @"
{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}}
\trowd\cellx1000\cellx2000
\intbl Cell 1\cell
\intbl Cell 2\cell
\row
\trowd\cellx1000\cellx2000
\intbl Cell 3\cell
\intbl Cell 4\cell
\row
}
"@
Set-Content -Path $TableRtf -Value $TableRtfContent -NoNewline

$TableDocx = Join-Path $WorkDir "table.docx"

try {
    $TableOutput = & $BinaryPath convert $TableRtf --output $TableDocx 2>&1
    if ($LASTEXITCODE -eq 0) {
        if (Test-Path $TableDocx) {
            try {
                $Zip = [System.IO.Compression.ZipFile]::OpenRead($TableDocx)
                $Zip.Dispose()
                Test-Passed "Table RTF conversion works"
            } catch {
                Test-Failed "Table DOCX output invalid"
            }
        } else {
            Test-Failed "Table DOCX file not created"
        }
    } else {
        Test-Failed "Table RTF conversion failed"
    }
} catch {
    Test-Failed "Exception during table conversion: $_"
}

# ============================================
# Summary
# ============================================
Write-Host ""
Write-Host "============================================"
Write-Host "           SMOKE TEST SUMMARY"
Write-Host "============================================"
Write-Host ""
Write-Host "Tests Passed: $TestsPassed" -ForegroundColor Green
Write-Host "Tests Failed: $TestsFailed" -ForegroundColor Red
Write-Host ""

# Cleanup
Cleanup

if ($TestsFailed -eq 0) {
    Write-Host "✅ All smoke tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ Some smoke tests failed!" -ForegroundColor Red
    exit 1
}

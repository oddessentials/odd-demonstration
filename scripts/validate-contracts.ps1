# validate-contracts.ps1
# Contract Validation Suite - Pure PowerShell
# Validates golden fixtures (must pass) and negative fixtures (must fail)

param(
    [string]$ContractsPath = "$PSScriptRoot\..\contracts"
)

$ErrorActionPreference = "Stop"

# JSON output for CI
$results = @{
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    passed = @()
    failed = @()
    errors = @()
}

function Test-JsonAgainstSchema {
    param(
        [string]$SchemaPath,
        [string]$DataPath
    )
    
    # Load schema and data
    $schema = Get-Content $SchemaPath -Raw | ConvertFrom-Json
    $data = Get-Content $DataPath -Raw | ConvertFrom-Json
    
    $errors = @()
    
    # Check required fields
    if ($schema.required) {
        foreach ($field in $schema.required) {
            if (-not ($data.PSObject.Properties.Name -contains $field)) {
                $errors += "Missing required property '$field'"
            }
        }
    }
    
    # Check property types and enums
    if ($schema.properties) {
        foreach ($prop in $schema.properties.PSObject.Properties) {
            $propName = $prop.Name
            $propSchema = $prop.Value
            
            if ($data.PSObject.Properties.Name -contains $propName) {
                $value = $data.$propName
                
                # Check enum values
                if ($propSchema.enum) {
                    if ($propSchema.enum -notcontains $value) {
                        $errors += "Property '$propName' value '$value' not in enum [$($propSchema.enum -join ', ')]"
                    }
                }
                
                # Check type
                if ($propSchema.type) {
                    $actualType = switch ($value.GetType().Name) {
                        "String" { "string" }
                        "Int32" { "integer" }
                        "Int64" { "integer" }
                        "Double" { "number" }
                        "Boolean" { "boolean" }
                        "PSCustomObject" { "object" }
                        "Object[]" { "array" }
                        default { "unknown" }
                    }
                    
                    if ($propSchema.type -ne $actualType -and $actualType -ne "unknown") {
                        # Allow null for optional fields
                        if ($null -ne $value) {
                            $errors += "Property '$propName' expected type '$($propSchema.type)' but got '$actualType'"
                        }
                    }
                }
                
                # Check nested required (for producer object)
                if ($propSchema.type -eq "object" -and $propSchema.required -and $value) {
                    foreach ($nestedField in $propSchema.required) {
                        if (-not ($value.PSObject.Properties.Name -contains $nestedField)) {
                            $errors += "Missing required nested property '$propName.$nestedField'"
                        }
                    }
                }
            }
        }
    }
    
    return $errors
}

Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "  CONTRACT VALIDATION SUITE" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host ""

# Determine schema for a fixture
function Get-SchemaForFixture {
    param([string]$FixtureName)
    
    if ($FixtureName -match "event-envelope") {
        return "$ContractsPath\schemas\event-envelope.json"
    } elseif ($FixtureName -match "job") {
        return "$ContractsPath\schemas\job.json"
    }
    return $null
}

# Validate golden fixtures (must pass)
Write-Host ">> Validating Golden Fixtures (must pass)" -ForegroundColor Yellow
$goldenPath = "$ContractsPath\fixtures\golden"
if (Test-Path $goldenPath) {
    Get-ChildItem $goldenPath -Filter "*.json" | ForEach-Object {
        $fixture = $_
        $schema = Get-SchemaForFixture $fixture.Name
        
        if ($schema -and (Test-Path $schema)) {
            $errors = Test-JsonAgainstSchema -SchemaPath $schema -DataPath $fixture.FullName
            
            if ($errors.Count -eq 0) {
                Write-Host "  [PASS] $($fixture.Name)" -ForegroundColor Green
                $results.passed += @{ fixture = $fixture.Name; type = "golden" }
            } else {
                Write-Host "  [FAIL] $($fixture.Name) - $($errors -join '; ')" -ForegroundColor Red
                $results.failed += @{ fixture = $fixture.Name; type = "golden"; errors = $errors }
            }
        } else {
            Write-Host "  [SKIP] $($fixture.Name) - No matching schema" -ForegroundColor Yellow
        }
    }
}

Write-Host ""

# Validate negative fixtures (must fail)
Write-Host ">> Validating Negative Fixtures (must fail)" -ForegroundColor Yellow
$negativePath = "$ContractsPath\fixtures\negative"
if (Test-Path $negativePath) {
    Get-ChildItem $negativePath -Filter "*.json" | ForEach-Object {
        $fixture = $_
        $schema = Get-SchemaForFixture $fixture.Name
        
        if ($schema -and (Test-Path $schema)) {
            $errors = Test-JsonAgainstSchema -SchemaPath $schema -DataPath $fixture.FullName
            
            if ($errors.Count -gt 0) {
                Write-Host "  [PASS] $($fixture.Name) - Correctly failed: $($errors[0])" -ForegroundColor Green
                $results.passed += @{ fixture = $fixture.Name; type = "negative"; expectedFailure = $true }
            } else {
                Write-Host "  [FAIL] $($fixture.Name) - Should have failed but passed" -ForegroundColor Red
                $results.failed += @{ fixture = $fixture.Name; type = "negative"; errors = @("Expected failure, got success") }
            }
        } else {
            Write-Host "  [SKIP] $($fixture.Name) - No matching schema" -ForegroundColor Yellow
        }
    }
}

# Summary
Write-Host ""
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "  RESULTS: $($results.passed.Count) passed, $($results.failed.Count) failed" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

# Output JSON for CI
$results | ConvertTo-Json -Depth 5 | Out-File "$ContractsPath\validation-results.json" -Encoding utf8

if ($results.failed.Count -gt 0) {
    exit 1
} else {
    exit 0
}

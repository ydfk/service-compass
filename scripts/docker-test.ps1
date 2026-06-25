param(
  [switch]$NoCache,
  [switch]$StopAfterTest,
  [int]$TimeoutSeconds = 120,
  [string]$HealthUrl = 'http://127.0.0.1:3010/api/health'
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Invoke-Checked {
  param(
    [string]$FilePath,
    [string[]]$Arguments
  )

  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$FilePath $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
  }
}

$buildArgs = @('compose', 'build')
if ($NoCache) {
  $buildArgs += '--no-cache'
}

try {
  Write-Host '==> Build Docker image'
  Invoke-Checked -FilePath 'docker' -Arguments $buildArgs

  Write-Host '==> Start Docker Compose service'
  Invoke-Checked -FilePath 'docker' -Arguments @('compose', 'up', '-d')

  Write-Host "==> Wait for health check: $HealthUrl"
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  $health = $null
  while ((Get-Date) -lt $deadline) {
    try {
      $health = Invoke-RestMethod -Uri $HealthUrl -TimeoutSec 3
      if ($health.status -eq 'ok') {
        break
      }
    } catch {
      Start-Sleep -Seconds 2
    }
  }

  if (-not $health -or $health.status -ne 'ok') {
    Write-Host '==> Health check failed. Recent logs:'
    & docker compose logs --tail 80 service-compass
    throw "Docker local test failed: $HealthUrl did not return healthy status"
  }

  Write-Host "==> Docker local test passed: $($health.name) $($health.version)"
  Write-Host '==> Open: http://127.0.0.1:3010'
} finally {
  if ($StopAfterTest) {
    Write-Host '==> Stop Docker Compose service'
    & docker compose down
  }
}

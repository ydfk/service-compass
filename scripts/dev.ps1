$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot

$frontend = Start-Process -FilePath 'pnpm' -ArgumentList '-C', (Join-Path $Root 'frontend'), 'dev' -WindowStyle Hidden -PassThru
try {
  Set-Location $Root
  $env:DATABASE_URL = 'sqlite:data/service-compass.db'
  cargo run -p service-compass-backend
}
finally {
  Stop-Process -Id $frontend.Id -Force -ErrorAction SilentlyContinue
}

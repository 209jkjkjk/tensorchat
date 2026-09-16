$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location -LiteralPath $repoRoot

function Assert-Success([string]$step) {
  if ($LASTEXITCODE -ne 0) {
    throw "$step failed with exit code $LASTEXITCODE"
  }
}

Write-Host '==> fmt'
cargo fmt --all --check
Assert-Success 'cargo fmt --all --check'

Write-Host '==> clippy'
cargo clippy --workspace --all-targets -- -D warnings
Assert-Success 'cargo clippy'

Write-Host '==> test'
cargo test --workspace
Assert-Success 'cargo test --workspace'

Write-Host '==> web'
Push-Location -LiteralPath (Join-Path $repoRoot 'web')
try {
  if (-not (Test-Path -LiteralPath 'node_modules')) {
    npm ci
    Assert-Success 'npm ci'
  }

  npm run typecheck
  Assert-Success 'npm run typecheck'

  npm test
  Assert-Success 'npm test'

  npm run build
  Assert-Success 'npm run build'
}
finally {
  Pop-Location
}

Write-Host ''
Write-Host 'all checks passed'

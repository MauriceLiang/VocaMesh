$ErrorActionPreference = 'Continue'
Push-Location (Join-Path $PSScriptRoot '..')
$script:failed = $false

function Invoke-Check {
    param(
        [string]$Label,
        [string]$Executable,
        [string[]]$Arguments
    )

    if (-not (Get-Command $Executable -ErrorAction SilentlyContinue)) {
        Write-Output "[BLOCKED] $Label (missing $Executable)"
        $script:failed = $true
        return
    }

    Write-Output "`n[RUN] $Label"
    & $Executable @Arguments
    if ($LASTEXITCODE -eq 0) {
        Write-Output "[PASS] $Label"
    } else {
        Write-Output "[FAIL] $Label (exit $LASTEXITCODE)"
        $script:failed = $true
    }
}

Invoke-Check 'cargo fmt --all -- --check' 'cargo' @('fmt', '--all', '--', '--check')
Invoke-Check 'cargo clippy --workspace --all-targets --all-features --offline -- -D warnings' 'cargo' @('clippy', '--workspace', '--all-targets', '--all-features', '--offline', '--', '-D', 'warnings')
Invoke-Check 'cargo test --workspace --offline --quiet' 'cargo' @('test', '--workspace', '--offline', '--quiet')
Invoke-Check 'cargo check --workspace --all-targets --all-features --offline' 'cargo' @('check', '--workspace', '--all-targets', '--all-features', '--offline')
Invoke-Check 'pnpm --dir frontend typecheck' 'pnpm' @('--dir', 'frontend', 'typecheck')
Invoke-Check 'pnpm --dir frontend test' 'pnpm' @('--dir', 'frontend', 'test')
Invoke-Check 'pnpm --dir frontend build' 'pnpm' @('--dir', 'frontend', 'build')
Invoke-Check 'git diff --check' 'git' @('diff', '--check')

Pop-Location
if ($script:failed) {
    Write-Output "`nLocal verification failed or was blocked."
    exit 1
}
Write-Output "`nAll local verification steps passed."
exit 0

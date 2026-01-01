<#
.SYNOPSIS
    List all active git worktrees

.EXAMPLE
    .\scripts\worktree-list.ps1
#>

$ErrorActionPreference = "Stop"

$MainRepo = git rev-parse --show-toplevel
Push-Location $MainRepo

try {
    Write-Host "Active worktrees:" -ForegroundColor Green
    Write-Host ""
    git worktree list
    Write-Host ""
    Write-Host "To remove a worktree:"
    Write-Host "  .\scripts\worktree-cleanup.ps1 <branch-name>" -ForegroundColor Cyan
} finally {
    Pop-Location
}

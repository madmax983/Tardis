<#
.SYNOPSIS
    Clean up a merged worktree

.DESCRIPTION
    Removes the worktree and deletes the local branch after PR merge.

.PARAMETER BranchName
    The name of the branch to clean up

.EXAMPLE
    .\scripts\worktree-cleanup.ps1 feature/add-metrics
#>

param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$BranchName
)

$ErrorActionPreference = "Stop"

$WorktreeRoot = "..\tardis-worktrees"

# Sanitize branch name for directory
$DirName = $BranchName -replace "/", "-"
$WorktreePath = Join-Path $WorktreeRoot $DirName

$MainRepo = git rev-parse --show-toplevel
Push-Location $MainRepo

try {
    # Remove the worktree
    if (Test-Path $WorktreePath) {
        Write-Host "Removing worktree at $WorktreePath..."
        git worktree remove $WorktreePath
    } else {
        Write-Host "Worktree not found at $WorktreePath" -ForegroundColor Yellow
    }

    # Delete the local branch (only if merged)
    $BranchExists = git show-ref --verify --quiet "refs/heads/$BranchName" 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Deleting local branch $BranchName..."
        git branch -d $BranchName
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Branch not fully merged. Use 'git branch -D $BranchName' to force delete." -ForegroundColor Yellow
        }
    } else {
        Write-Host "Branch $BranchName not found locally" -ForegroundColor Yellow
    }

    # Prune worktree references
    git worktree prune

    Write-Host ""
    Write-Host "Cleanup complete!" -ForegroundColor Green

} finally {
    Pop-Location
}

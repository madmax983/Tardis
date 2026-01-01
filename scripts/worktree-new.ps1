<#
.SYNOPSIS
    Create a new git worktree for feature development

.DESCRIPTION
    Creates a new worktree at ../tardis-worktrees/<branch-name>
    allowing multiple Claude agents to work on different features in parallel.
    Automatically symlinks Claude settings from main repo.

.PARAMETER BranchName
    The name of the branch to create (e.g., feature/add-metrics)

.PARAMETER BaseBranch
    The base branch to branch from (default: trunk)

.EXAMPLE
    .\scripts\worktree-new.ps1 feature/add-metrics

.EXAMPLE
    .\scripts\worktree-new.ps1 fix/memory-leak trunk
#>

param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$BranchName,

    [Parameter(Position=1)]
    [string]$BaseBranch = "trunk"
)

$ErrorActionPreference = "Stop"

$WorktreeRoot = "..\tardis-worktrees"

# Sanitize branch name for directory (replace / with -)
$DirName = $BranchName -replace "/", "-"
$WorktreePath = Join-Path $WorktreeRoot $DirName

# Ensure we're in the main repo
$MainRepo = git rev-parse --show-toplevel
Push-Location $MainRepo

try {
    # Check for existing worktree or branch
    if (Test-Path $WorktreePath) {
        Write-Host "Error: Worktree path '$WorktreePath' already exists." -ForegroundColor Red
        Write-Host "Consider running '.\scripts\worktree-cleanup.ps1 $BranchName'" -ForegroundColor Yellow
        exit 1
    }

    $BranchCheck = git rev-parse --verify --quiet $BranchName 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Error: Branch '$BranchName' already exists." -ForegroundColor Red
        exit 1
    }

    # Create worktrees directory if needed
    if (-not (Test-Path $WorktreeRoot)) {
        New-Item -ItemType Directory -Path $WorktreeRoot | Out-Null
    }

    # Fetch latest from origin
    Write-Host "Fetching latest from origin..."
    git fetch origin

    # Create the worktree with a new branch
    Write-Host "Creating worktree at $WorktreePath..."
    git worktree add -b $BranchName $WorktreePath "origin/$BaseBranch"

    # Symlink Claude settings from main repo to worktree
    $ClaudeSettings = Join-Path $MainRepo ".claude\settings.local.json"

    if (Test-Path $ClaudeSettings) {
        Write-Host "Symlinking Claude settings..."

        $WorktreeClaudeDir = Join-Path (Resolve-Path $WorktreePath) ".claude"
        if (-not (Test-Path $WorktreeClaudeDir)) {
            New-Item -ItemType Directory -Path $WorktreeClaudeDir | Out-Null
        }

        $LinkPath = Join-Path $WorktreeClaudeDir "settings.local.json"

        # Create symlink (requires admin or developer mode on Windows)
        New-Item -ItemType SymbolicLink -Path $LinkPath -Target $ClaudeSettings -Force | Out-Null
        Write-Host "  Linked: $LinkPath -> $ClaudeSettings"
    }

    Write-Host ""
    Write-Host "Worktree created successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "To start working:"
    Write-Host "  cd $WorktreePath" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "When done, create a PR:"
    Write-Host "  git push -u origin $BranchName" -ForegroundColor Cyan
    Write-Host "  gh pr create --base $BaseBranch" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To clean up after PR is merged:"
    Write-Host "  .\scripts\worktree-cleanup.ps1 $BranchName" -ForegroundColor Cyan

} finally {
    Pop-Location
}

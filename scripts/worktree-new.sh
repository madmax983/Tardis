#!/usr/bin/env bash
# Create a new git worktree for feature development
#
# Usage: ./scripts/worktree-new.sh <branch-name> [base-branch]
#
# Examples:
#   ./scripts/worktree-new.sh feature/add-metrics
#   ./scripts/worktree-new.sh fix/memory-leak trunk
#
# This creates a new worktree at ../tardis-worktrees/<branch-name>
# allowing multiple Claude agents to work on different features in parallel.

set -euo pipefail

BRANCH_NAME="${1:-}"
BASE_BRANCH="${2:-trunk}"
WORKTREE_ROOT="../tardis-worktrees"

if [[ -z "$BRANCH_NAME" ]]; then
    echo "Usage: $0 <branch-name> [base-branch]"
    echo ""
    echo "Examples:"
    echo "  $0 feature/add-metrics"
    echo "  $0 fix/memory-leak trunk"
    exit 1
fi

# Sanitize branch name for directory (replace / with -)
DIR_NAME="${BRANCH_NAME//\//-}"
WORKTREE_PATH="$WORKTREE_ROOT/$DIR_NAME"

# Ensure we're in the main repo
cd "$(git rev-parse --show-toplevel)"

# Check for existing worktree or branch
if [[ -d "$WORKTREE_PATH" ]]; then
    echo "Error: Worktree path '$WORKTREE_PATH' already exists." >&2
    echo "Consider running './scripts/worktree-cleanup.sh $BRANCH_NAME'" >&2
    exit 1
fi

if git rev-parse --verify --quiet "$BRANCH_NAME" >/dev/null 2>&1; then
    echo "Error: Branch '$BRANCH_NAME' already exists." >&2
    exit 1
fi

# Create worktrees directory if needed
mkdir -p "$WORKTREE_ROOT"

# Fetch latest from origin
echo "Fetching latest from origin..."
git fetch origin

# Create the worktree with a new branch
echo "Creating worktree at $WORKTREE_PATH..."
git worktree add -b "$BRANCH_NAME" "$WORKTREE_PATH" "origin/$BASE_BRANCH"

# Symlink Claude settings from main repo to worktree
# This preserves permissions granted in the main repo
MAIN_REPO="$(pwd)"
CLAUDE_SETTINGS="$MAIN_REPO/.claude/settings.local.json"

if [[ -f "$CLAUDE_SETTINGS" ]]; then
    echo "Symlinking Claude settings..."
    mkdir -p "$WORKTREE_PATH/.claude"

    # Create symlink (use relative path for portability)
    # From worktree/.claude/ back to main/.claude/settings.local.json
    ln -sf "$CLAUDE_SETTINGS" "$WORKTREE_PATH/.claude/settings.local.json"
    echo "  Linked: $WORKTREE_PATH/.claude/settings.local.json -> $CLAUDE_SETTINGS"
fi

echo ""
echo "Worktree created successfully!"
echo ""
echo "To start working:"
echo "  cd $WORKTREE_PATH"
echo ""
echo "When done, create a PR:"
echo "  git push -u origin $BRANCH_NAME"
echo "  gh pr create --base $BASE_BRANCH"
echo ""
echo "To clean up after PR is merged:"
echo "  ./scripts/worktree-cleanup.sh $BRANCH_NAME"

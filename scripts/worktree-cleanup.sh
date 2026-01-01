#!/usr/bin/env bash
# Clean up a merged worktree
#
# Usage: ./scripts/worktree-cleanup.sh <branch-name>
#
# This removes the worktree and deletes the local branch after PR merge.

set -euo pipefail

BRANCH_NAME="${1:-}"
WORKTREE_ROOT="../tardis-worktrees"

if [[ -z "$BRANCH_NAME" ]]; then
    echo "Usage: $0 <branch-name>"
    echo ""
    echo "Example:"
    echo "  $0 feature/add-metrics"
    exit 1
fi

# Sanitize branch name for directory
DIR_NAME="${BRANCH_NAME//\//-}"
WORKTREE_PATH="$WORKTREE_ROOT/$DIR_NAME"

cd "$(git rev-parse --show-toplevel)"

# Remove the worktree
if [[ -d "$WORKTREE_PATH" ]]; then
    echo "Removing worktree at $WORKTREE_PATH..."
    git worktree remove "$WORKTREE_PATH"
else
    echo "Worktree not found at $WORKTREE_PATH"
fi

# Delete the local branch
if git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
    echo "Deleting local branch $BRANCH_NAME..."
    git branch -d "$BRANCH_NAME"
else
    echo "Branch $BRANCH_NAME not found locally"
fi

# Prune worktree references
git worktree prune

echo ""
echo "Cleanup complete!"

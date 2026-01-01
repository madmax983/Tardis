#!/usr/bin/env bash
# List all active git worktrees
#
# Usage: ./scripts/worktree-list.sh

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

echo "Active worktrees:"
echo ""
git worktree list
echo ""
echo "To remove a worktree:"
echo "  git worktree remove <path>"

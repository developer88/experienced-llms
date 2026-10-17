#!/usr/bin/env bash
# Local Release Publisher (Zero Cloud GitHub Workers)
# Publishes locally compiled artifacts in ./dist/ to GitHub Releases using GitHub CLI
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <tag_name> [title] [notes_file]"
    echo "Example: $0 1.0.0rc4 'Release 1.0.0rc4' docs/RELEASE_NOTES_rc4.md"
    exit 1
fi

TAG="$1"
TITLE="${2:-Release $TAG}"
NOTES_FILE="${3:-}"

cd "${REPO_ROOT}"

if [ ! -d "${DIST_DIR}" ] || [ -z "$(ls -A "${DIST_DIR}")" ]; then
    echo "[-] Error: ./dist/ is empty or does not exist."
    echo "    Run './scripts/build_release_docker.sh' first to compile artifacts locally."
    exit 1
fi

echo "================================================================="
echo "   Experienced LLMs: Local Release Publisher"
echo "   Target Tag: ${TAG}"
echo "   Title:      ${TITLE}"
echo "   (Zero Cloud Workers - Direct Local Upload via GitHub CLI)"
echo "================================================================="

# Check gh CLI availability
if ! command -v gh &> /dev/null; then
    echo "[-] Error: 'gh' CLI is required. Install GitHub CLI or ensure it is in PATH."
    exit 1
fi

# Check if release already exists
if gh release view "${TAG}" &> /dev/null; then
    echo "[+] Release '${TAG}' exists. Uploading local artifacts from ./dist/..."
    gh release upload "${TAG}" "${DIST_DIR}"/* --clobber
else
    echo "[+] Creating GitHub Release '${TAG}' and uploading local artifacts..."
    EXTRA_ARGS=()
    if [ -n "${NOTES_FILE}" ] && [ -f "${NOTES_FILE}" ]; then
        EXTRA_ARGS+=("--notes-file" "${NOTES_FILE}")
    else
        EXTRA_ARGS+=("--notes" "Release ${TAG} compiled locally with zero cloud worker minutes.")
    fi
    gh release create "${TAG}" "${DIST_DIR}"/* --title "${TITLE}" "${EXTRA_ARGS[@]}"
fi

echo ""
echo "[✓] Successfully published release ${TAG} using local Docker pipeline!"
echo "    GitHub Actions compute minutes consumed: 0 minutes."

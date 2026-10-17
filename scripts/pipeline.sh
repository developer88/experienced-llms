#!/usr/bin/env bash
# Unified Local Docker Pipeline CLI for Experienced LLMs (0 Cloud Workers)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
    echo "Experienced LLMs - Local Docker Pipeline (Zero Cloud Workers)"
    echo ""
    echo "Usage: $0 [command] [args...]"
    echo ""
    echo "Commands:"
    echo "  test              Run full test suite in an isolated Docker container"
    echo "  build             Cross-compile Linux (musl & gnu) and Windows binaries to ./dist/"
    echo "  publish <tag>     Upload pre-built artifacts from ./dist/ to GitHub Releases"
    echo "  all <tag>         Execute test -> build -> publish in a single local workflow"
    echo "  help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 test"
    echo "  $0 build"
    echo "  $0 publish 1.0.0rc4 'Release 1.0.0rc4' docs/RELEASE_NOTES_rc4.md"
    echo "  $0 all 1.0.0rc4"
    exit 1
}

CMD="${1:-help}"
shift || true

case "${CMD}" in
    test)
        bash "${SCRIPT_DIR}/run_docker_tests.sh" "$@"
        ;;
    build)
        bash "${SCRIPT_DIR}/build_release_docker.sh" "$@"
        ;;
    publish)
        bash "${SCRIPT_DIR}/publish_release_local.sh" "$@"
        ;;
    all)
        if [ $# -lt 1 ]; then
            echo "[-] Error: 'all' requires a release tag argument."
            echo "    Example: $0 all 1.0.0rc4"
            exit 1
        fi
        echo "=== [1/3] Running Container Tests ==="
        bash "${SCRIPT_DIR}/run_docker_tests.sh"
        echo ""
        echo "=== [2/3] Building Release Binaries in Docker ==="
        bash "${SCRIPT_DIR}/build_release_docker.sh"
        echo ""
        echo "=== [3/3] Publishing to GitHub Releases ==="
        bash "${SCRIPT_DIR}/publish_release_local.sh" "$@"
        echo ""
        echo "================================================================="
        echo "[SUCCESS] Entire local pipeline completed with ZERO cloud worker minutes!"
        echo "================================================================="
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        echo "Unknown command: ${CMD}"
        usage
        ;;
esac

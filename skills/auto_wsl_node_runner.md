# SKILL: auto_wsl_node_runner
## Role
Ensures all node and npm scripts are executed inside WSL via bash.

## Rules
- Wrap any npm/npx/node command with 'wsl -e bash -c ...'
- Never run node.exe on Windows host
- Handle /mnt/c path translation

**Contract**:
Input: Windows path or npm command
Output: WSL-safe execution string

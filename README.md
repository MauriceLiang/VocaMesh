# VocaMesh

**A local-first multi-agent orchestration and collaboration platform.**

VocaMesh is a product of **Vocafect Effective Vocabulary Technology Limited**.

> Runtime foundation: Vue frontend, Tauri 2 desktop shell, local SQLite Task/Run history, and a restricted Claude Code execution implementation. The native desktop stores Agent roles, Workspaces, Teams, organization relations, audit records, and appearance preferences in the same SQLite database. Native Approval and Notification sources are intentionally empty, while browser preview keeps its full Mock dataset. Real execution remains fail-closed because VocaMesh cannot reliably verify whether managed Claude Code hooks may run. Unknown Runs have a backend-checked, human-confirmed recovery flow; it does not unlock new tasks or rerun old ones. Desktop readiness remains blocked until the policy boundary can be verified; a prior authorized live attempt also exited with code 1 and did not pass acceptance.

See [development](docs/development.md), [architecture](docs/architecture.md), and [security](docs/security.md) for setup and runtime boundaries.

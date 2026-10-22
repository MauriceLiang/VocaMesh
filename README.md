# VocaMesh

**A local-first multi-agent orchestration and collaboration platform.**

VocaMesh is a product of **Vocafect Effective Vocabulary Technology Limited**.

> Runtime foundation: Vue frontend, Tauri 2 desktop shell, local SQLite Task/Run history, and a restricted Claude Code execution implementation. Real execution is currently fail-closed because VocaMesh cannot reliably verify whether managed Claude Code hooks may run. Desktop readiness therefore remains blocked until that policy boundary can be verified; a prior authorized live attempt also exited with code 1 and did not pass acceptance. The browser continues to use Mock data, and Agent/Team/Approval workflows remain Mock.

See [development](docs/development.md), [architecture](docs/architecture.md), and [security](docs/security.md) for setup and runtime boundaries.

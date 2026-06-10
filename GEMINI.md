# Project Principles for Gemini / AI Assistants

Here are the guidelines to follow when generating documentation or modifying files within this workspace.

## 1. Relative Link Formatting in Workspace Documentation
When referencing files, directories, or modules inside workspace documentation (like `codebase_structure.md` or other markdown files):
* **Prefer Relative Links**: Use relative workspace links (e.g., `[Cargo.toml](Cargo.toml)` or `[src/lib.rs](src/lib.rs)`) instead of absolute `file:///` URIs.
* **Why**: Absolute URLs encode user-specific and machine-specific directories (e.g. `/home/jeteve/...`), which breaks portability and makes links unusable for other users or under different development environments.
* **Exception**: When sending responses directly in the chat interface to the user (outside of project files), follow the client/system default formatting requirements (e.g., absolute `file://` links to help the user navigate from their editor client).

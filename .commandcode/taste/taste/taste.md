# Taste
- Writes in casual Indonesian (informal, abbreviations like "knp", "ttp cuman") and expects replies/answers in Indonesian. Confidence: 0.7
- Describes bugs conversationally and briefly ("kenapa ya ..."), leaving diagnosis to the agent rather than pre-diagnosing the cause. Confidence: 0.6
- Actively narrows the investigation scope, telling the agent when a line of inquiry is moot (e.g. "dev is the same, so no need to look into packaging"). Confidence: 0.5
- Works on Tauri (Rust backend + Svelte frontend) desktop apps for Windows. Confidence: 0.5
- Uses pnpm as the package manager and checks behavior via `pnpm tauri dev` as well as release builds. Confidence: 0.6
- Maintains a GitHub Actions release workflow for their Tauri app, publishing releases via pushed version tags (e.g. `sw-tauri-vX.Y.Z`). Confidence: 0.5
- Dislikes value duplication across config files (e.g. version scattered across `package.json`, `Cargo.toml`, tauri config) and favors consolidating to a single source of truth; will approve refactors that remove the duplication. Confidence: 0.6
- Prefers release-please (Conventional Commits driven) automation for versioning/releases, and points to an existing sibling/example project as the template to mirror. Confidence: 0.6
- Treats CI/release plumbing as a black box: prefers leveraging GitHub-provided defaults (built-in `GITHUB_TOKEN`, read/write workflow permissions) and would rather receive a concrete "settings you must change in GitHub" checklist than set up secrets/tokens manually. Confidence: 0.4

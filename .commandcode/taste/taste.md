# Taste

## Communication
- Writes in casual Indonesian (informal, abbreviations like "knp", "ttp cuman") and expects replies/answers in Indonesian. Confidence: 0.7
- Describes bugs conversationally and briefly ("kenapa ya ..."), leaving diagnosis to the agent rather than pre-diagnosing the cause. Confidence: 0.6
- Actively narrows the investigation scope, telling the agent when a line of inquiry is moot (e.g. "dev is the same, so no need to look into packaging"). Confidence: 0.5

## Tooling
- Works on Tauri (Rust backend + Svelte frontend) desktop apps for Windows. Confidence: 0.5
- Uses pnpm as the package manager and checks behavior via `pnpm tauri dev` as well as release builds. Confidence: 0.6

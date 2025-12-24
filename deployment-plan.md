     ╭───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
     │ Plan: Package TUI as npm Global CLI                                                                                                   │
     │                                                                                                                                       │
     │ Overview                                                                                                                              │
     │                                                                                                                                       │
     │ Package the Rust TUI (observatory-tui) as an npm package that users can install globally and run with odd-dashboard.                  │
     │                                                                                                                                       │
     │ Approach: Use the same pattern as esbuild/biome/turbo - platform-specific npm packages with pre-built binaries, plus a wrapper        │
     │ package that resolves the correct binary at runtime.                                                                                  │
     │                                                                                                                                       │
     │ ---                                                                                                                                   │
     │ Package Structure                                                                                                                     │
     │                                                                                                                                       │
     │ @odd/dashboard                    <- Main wrapper (what users install)                                                                │
     │ @odd/dashboard-win32-x64          <- Windows x64 binary                                                                               │
     │ @odd/dashboard-darwin-x64         <- macOS Intel binary                                                                               │
     │ @odd/dashboard-darwin-arm64       <- macOS Apple Silicon binary                                                                       │
     │ @odd/dashboard-linux-x64          <- Linux x64 binary                                                                                 │
     │                                                                                                                                       │
     │ Directory layout in monorepo:                                                                                                         │
     │ packages/                                                                                                                             │
     │   dashboard/                      <- Main package                                                                                     │
     │     package.json                                                                                                                      │
     │     bin/odd-dashboard.js          <- Node CLI entry point                                                                             │
     │     lib/binary.js                 <- Binary resolution                                                                                │
     │     lib/prerequisites.js          <- First-run checks                                                                                 │
     │     lib/setup.js                  <- Guided setup output                                                                              │
     │   dashboard-win32-x64/            <- Platform packages (binaries added by CI)                                                         │
     │   dashboard-darwin-x64/                                                                                                               │
     │   dashboard-darwin-arm64/                                                                                                             │
     │   dashboard-linux-x64/                                                                                                                │
     │                                                                                                                                       │
     │ ---                                                                                                                                   │
     │ Implementation Steps                                                                                                                  │
     │                                                                                                                                       │
     │ 1. Create main wrapper package                                                                                                        │
     │                                                                                                                                       │
     │ Files to create:                                                                                                                      │
     │ - packages/dashboard/package.json - optionalDependencies on platform packages                                                         │
     │ - packages/dashboard/bin/odd-dashboard.js - CLI entry, spawns binary                                                                  │
     │ - packages/dashboard/lib/binary.js - Resolves correct platform binary                                                                 │
     │ - packages/dashboard/lib/prerequisites.js - Checks Docker, kubectl, kind, pwsh                                                        │
     │ - packages/dashboard/lib/setup.js - Displays guided setup instructions                                                                │
     │                                                                                                                                       │
     │ 2. Create platform package templates                                                                                                  │
     │                                                                                                                                       │
     │ Files to create:                                                                                                                      │
     │ - packages/dashboard-win32-x64/package.json                                                                                           │
     │ - packages/dashboard-darwin-x64/package.json                                                                                          │
     │ - packages/dashboard-darwin-arm64/package.json                                                                                        │
     │ - packages/dashboard-linux-x64/package.json                                                                                           │
     │                                                                                                                                       │
     │ Each has os and cpu fields so npm only installs the matching one.                                                                     │
     │                                                                                                                                       │
     │ 3. Create CI/CD workflow for releases                                                                                                 │
     │                                                                                                                                       │
     │ File to create:                                                                                                                       │
     │ - .github/workflows/release-dashboard.yml                                                                                             │
     │                                                                                                                                       │
     │ Workflow:                                                                                                                             │
     │ 1. Build Rust binary on each platform (windows-latest, macos-13, macos-14, ubuntu-latest)                                             │
     │ 2. Copy binaries into platform package directories                                                                                    │
     │ 3. Publish platform packages to npm first                                                                                             │
     │ 4. Publish main wrapper package last                                                                                                  │
     │                                                                                                                                       │
     │ 4. Create build helper script                                                                                                         │
     │                                                                                                                                       │
     │ File to create:                                                                                                                       │
     │ - scripts/generate-platform-package.js - Generates platform package.json with correct version                                         │
     │                                                                                                                                       │
     │ 5. Update monorepo configuration                                                                                                      │
     │                                                                                                                                       │
     │ Files to modify:                                                                                                                      │
     │ - pnpm-workspace.yaml - Add packages/*                                                                                                │
     │ - package.json (root) - Add dashboard scripts                                                                                         │
     │ - .gitignore - Add packages/*/bin/ (binaries are build artifacts)                                                                     │
     │                                                                                                                                       │
     │ ---                                                                                                                                   │
     │ User Experience                                                                                                                       │
     │                                                                                                                                       │
     │ Install:                                                                                                                              │
     │ npm install -g @odd/dashboard                                                                                                         │
     │                                                                                                                                       │
     │ First run:                                                                                                                            │
     │ odd-dashboard                                                                                                                         │
     │ - Checks prerequisites (Docker, kubectl, kind, PowerShell)                                                                            │
     │ - Shows platform-specific install commands for missing tools                                                                          │
     │ - Creates ~/.odd-dashboard.json to mark setup complete                                                                                │
     │                                                                                                                                       │
     │ Normal run:                                                                                                                           │
     │ odd-dashboard                                                                                                                         │
     │ - Skips prerequisite check                                                                                                            │
     │ - Launches TUI directly                                                                                                               │
     │ - TUI handles cluster detection/launch internally                                                                                     │
     │                                                                                                                                       │
     │ Force check:                                                                                                                          │
     │ odd-dashboard --check                                                                                                                 │
     │                                                                                                                                       │
     │ ---                                                                                                                                   │
     │ Key Files Reference                                                                                                                   │
     │                                                                                                                                       │
     │ | Existing File                  | Relevance                            |                                                             │
     │ |--------------------------------|--------------------------------------|                                                             │
     │ | src/interfaces/tui/Cargo.toml  | Binary name config                   |                                                             │
     │ | src/interfaces/tui/src/main.rs | Has prerequisite logic to align with |                                                             │
     │ | scripts/start-all.ps1          | Cluster setup script TUI invokes     |                                                             │
     │ | pnpm-workspace.yaml            | Workspace config to extend           |                                                             │
     │                                                                                                                                       │
     │ ---                                                                                                                                   │
     │ CI/CD Requirements                                                                                                                    │
     │                                                                                                                                       │
     │ - GitHub Actions runners for each platform                                                                                            │
     │ - NPM_TOKEN secret for publishing to npm                                                                                              │
     │ - Tag-based releases: dashboard-v* triggers publish                                                                                   │
     ╰───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯

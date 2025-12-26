# [2.0.0](https://github.com/oddessentials/odd-demonstration/compare/v1.4.0...v2.0.0) (2025-12-26)


### Bug Fixes

* **compose:** add restart policy for RabbitMQ race condition ([73f2668](https://github.com/oddessentials/odd-demonstration/commit/73f266807834b9d58d854544965b83dcea310ff2))
* **docker:** resolve Dockerfile parse error in mock-builder stage ([127a276](https://github.com/oddessentials/odd-demonstration/commit/127a276c4d5c6814ee6dee9f8a45bfd839cab8b7))
* **docker:** visual test failures ([7a529d8](https://github.com/oddessentials/odd-demonstration/commit/7a529d840715cacdcb3078da1fbca48fd32279b8))
* increase integration budget to 180s, skip flaky visual tests ([fde985e](https://github.com/oddessentials/odd-demonstration/commit/fde985e101f47b5ac9b8f5b9495ee58f6fee306a))
* re-enable behavioral tests, add timeout sync check ([a3a0440](https://github.com/oddessentials/odd-demonstration/commit/a3a04406e5934471c93e51cd4da04255ef24bf50))
* **web-pty-server:** add --help handling to mock TUI binary ([43b33c0](https://github.com/oddessentials/odd-demonstration/commit/43b33c039ba82fc07c47cdc561c586f4541fb685))
* **web-pty:** fix P1 WebSocket auth and document reconnect behavior ([13cc3c7](https://github.com/oddessentials/odd-demonstration/commit/13cc3c71344b2c3158fc5c70e40b4f3ecc2cc17f))


### Documentation

* update for Web Terminal modernization (Phase 20) ([0e3ba9d](https://github.com/oddessentials/odd-demonstration/commit/0e3ba9d56e543e1aa53e42f5305c6b7f50adc816))


### Features

* **ci:** add visual regression tests with automatic cluster setup ([efb4e25](https://github.com/oddessentials/odd-demonstration/commit/efb4e253f07a65991716a0909fc24481f377c1ba))
* **integration:** add server mode, Prometheus/Grafana, and I7 parity invariant ([417d4bf](https://github.com/oddessentials/odd-demonstration/commit/417d4bfae27e156b6111f0e28c3bf28c8e4f6af1))
* **tui:** add Server Mode for container deployment (W11) ([fa0f006](https://github.com/oddessentials/odd-demonstration/commit/fa0f00626c9b918f1f6d44d89471ed339b666d5a))
* **web-pty-server:** add PTY broker for xterm.js terminal mirroring ([8af3f5e](https://github.com/oddessentials/odd-demonstration/commit/8af3f5e8e32eb9efe349e736fba0e37390913d65))
* **web-pty:** implement Phase 7 PTY state preservation (steps 3-7) ([2fbfdd3](https://github.com/oddessentials/odd-demonstration/commit/2fbfdd36752d6bb69b48e8c3d876554627ea46b6))
* **web-pty:** implement state machine and config for PTY preservation ([ac380de](https://github.com/oddessentials/odd-demonstration/commit/ac380de4b90972aa824bfde7bd468bae97580cb1))
* **web-pty:** multi-stage Dockerfile with real/mock TUI modes ([f6460aa](https://github.com/oddessentials/odd-demonstration/commit/f6460aaa07f84c27abc846987eef713de3e55209))
* **web-terminal:** add xterm.js PTY-based terminal mirror ([8c1365d](https://github.com/oddessentials/odd-demonstration/commit/8c1365df4ff5a67b5da4ac8b0fe5ff68b7832a56))
* **web-ui:** add keyboard hints, fix terminal sizing, copy contracts for U key ([a087435](https://github.com/oddessentials/odd-demonstration/commit/a08743522dd6eaeb05ecf7c14b403f30667be9d1))


### BREAKING CHANGES

* Web Dashboard replaced with PTY-streaming Web Terminal.
The single-page glassmorphic UI is removed in favor of xterm.js mirroring
the native TUI via WebSocket. API remains unchanged but UI is fundamentally
different - no more Add Task form or UI Launcher modal in the web interface.

# [1.4.0](https://github.com/oddessentials/odd-demonstration/compare/v1.3.0...v1.4.0) (2025-12-26)


### Bug Fixes

* **integration:** complete harness fixes for all proof paths ([2a92f89](https://github.com/oddessentials/odd-demonstration/commit/2a92f898843b5a2929c4727e4b68e601c5010fd9))


### Features

* **docker:** fix Gateway health check and compose SSL config ([fe153dc](https://github.com/oddessentials/odd-demonstration/commit/fe153dcdc18b897daa22e05948d08aa0b52c711a))

# [1.3.0](https://github.com/oddessentials/odd-demonstration/compare/v1.2.1...v1.3.0) (2025-12-25)


### Bug Fixes

* **ci:** add --lib --exclude-files to TUI coverage ([16622ae](https://github.com/oddessentials/odd-demonstration/commit/16622ae5105a00c94de2c49c23112924ddfc06c0))
* **ci:** make integration-phase informational per I6 ([858c901](https://github.com/oddessentials/odd-demonstration/commit/858c9010b088063e1f10194b68abdfb4f9e7805d))
* **ci:** temporarily skip integration-phase until Docker Hub images ([b25a8ef](https://github.com/oddessentials/odd-demonstration/commit/b25a8efe6238634e83c7bea524b93c5dd7a6f9ed))
* **ci:** track package-lock.json files for npm ci ([6788a31](https://github.com/oddessentials/odd-demonstration/commit/6788a3155a8734cf92bf46c26792939d6bc820cd))
* **coverage:** lower TUI threshold from 33% to 32% ([97ad17c](https://github.com/oddessentials/odd-demonstration/commit/97ad17cfec5031b02d362888cc534629170c88df))
* **gateway:** fix TypeScript type declarations ([b2e6545](https://github.com/oddessentials/odd-demonstration/commit/b2e654596e478ee2446e6ed9376e8e408604550a))
* **gateway:** regenerate package-lock.json for CI compatibility ([8a9ea1f](https://github.com/oddessentials/odd-demonstration/commit/8a9ea1ff6d6af44633ce772109a9c724ddb26a4d))
* **governance:** correct INVARIANTS.md accuracy issues ([395644b](https://github.com/oddessentials/odd-demonstration/commit/395644b34137dcdd916c292fb7e865444bfba5af))
* **integration:** build services before running in docker-compose ([5de44af](https://github.com/oddessentials/odd-demonstration/commit/5de44af3a7878289c2a7509bf7bb868308ba17d3))
* **integration:** handle JSON health response format ([ece9d72](https://github.com/oddessentials/odd-demonstration/commit/ece9d720102c7f116f560557262742367f3366b0))
* **typing:** threshold issue ([29efc3b](https://github.com/oddessentials/odd-demonstration/commit/29efc3b4386205397d38d3177f814b076cb9848e))


### Features

* **gateway:** achieve 87% TypeScript coverage with refactored lib modules ([6094bd0](https://github.com/oddessentials/odd-demonstration/commit/6094bd00163a849604f149effa58220c0c850a0b))
* **governance:** add TypeScript enforcement check to pre-commit ([8b0f174](https://github.com/oddessentials/odd-demonstration/commit/8b0f174df6e5abe11b258df58add359b19816841))
* **integration:** add self-contained integration harness (Phase 18) ([40c1c54](https://github.com/oddessentials/odd-demonstration/commit/40c1c542b91f7db6ffa38a7686136e79e15d3918))
* **tui:** raise lib coverage to 33%+ with tiered strategy ([020d418](https://github.com/oddessentials/odd-demonstration/commit/020d4180fb59c619dc1f90f7b652c891d7507c6a))

## [1.2.1](https://github.com/oddessentials/odd-demonstration/compare/v1.2.0...v1.2.1) (2025-12-25)


### Bug Fixes

* set processor coverage threshold to current reality (33%) ([ebd9abf](https://github.com/oddessentials/odd-demonstration/commit/ebd9abfaf565805663b02c01566efd791fffc459))


### Performance Improvements

* parallelize Go tests with pwsh < 7 fallback ([5329009](https://github.com/oddessentials/odd-demonstration/commit/5329009143de88db0085db15e53ba6b7dc4d0925))

# [1.2.0](https://github.com/oddessentials/odd-demonstration/compare/v1.1.1...v1.2.0) (2025-12-25)


### Bug Fixes

* **infra:** correct RedisInsight port and enable changelog generation ([ab9a126](https://github.com/oddessentials/odd-demonstration/commit/ab9a126f5df93a60321e7a1c65a0f47336b78ace))


### Features

* **tui:** show OS-specific install commands in doctor output ([aa55c4f](https://github.com/oddessentials/odd-demonstration/commit/aa55c4f4591af46f0b529841c9be1eaded7dc236))

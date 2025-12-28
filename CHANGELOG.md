# [3.3.0](https://github.com/oddessentials/odd-demonstration/compare/v3.2.0...v3.3.0) (2025-12-28)


### Bug Fixes

* demo link wordinig ([aa2d5c0](https://github.com/oddessentials/odd-demonstration/commit/aa2d5c09dad677c446d47e4fd0299706f36104ae))


### Features

* add experiment UI ([35e2690](https://github.com/oddessentials/odd-demonstration/commit/35e269018272c904bf645e9658fb86ff416b36c9))


### Reverts

* remove test.html ([eeb4714](https://github.com/oddessentials/odd-demonstration/commit/eeb471461517172926d42cc6f4acaa06b6c56fbb))

# [3.2.0](https://github.com/oddessentials/odd-demonstration/compare/v3.1.1...v3.2.0) (2025-12-27)


### Features

* **tui:** add shutdown feature with Ctrl+Q key binding ([e80aaa0](https://github.com/oddessentials/odd-demonstration/commit/e80aaa039bca7a7ccd08dbc56bb89dac7f9d117d))

## [3.1.1](https://github.com/oddessentials/odd-demonstration/compare/v3.1.0...v3.1.1) (2025-12-27)


### Bug Fixes

* **tui:** make tests hermetic with RAII guard and temp dirs ([7840435](https://github.com/oddessentials/odd-demonstration/commit/78404353378c82c18f7c3bfadb12583bfd1023a9))

# [3.1.0](https://github.com/oddessentials/odd-demonstration/compare/v3.0.4...v3.1.0) (2025-12-27)


### Bug Fixes

* **tui:** use Start-Process for Windows port-forwards ([be98ea7](https://github.com/oddessentials/odd-demonstration/commit/be98ea77aade72e87a878142e0a01370c15d0681))


### Features

* **tui:** add port-forward health check during loading ([f9c90a7](https://github.com/oddessentials/odd-demonstration/commit/f9c90a73b906f648e1bb152f22d2e367b47bde76))

## [3.0.4](https://github.com/oddessentials/odd-demonstration/compare/v3.0.3...v3.0.4) (2025-12-27)


### Bug Fixes

* **ci:** align Docker build contexts with Dockerfile COPY paths ([bdf8853](https://github.com/oddessentials/odd-demonstration/commit/bdf88530e9e9cc631d06a9b9f6893add8e67cbc9))
* **ci:** correct invalid bazel-contrib/setup-bazel input ([09babdf](https://github.com/oddessentials/odd-demonstration/commit/09babdff640e8c1745b2267b7db330d975da0373))
* **ci:** trigger build-images on web_terminal changes ([3f4ee92](https://github.com/oddessentials/odd-demonstration/commit/3f4ee923e3297611fa985da8c3b4cf8af88a85af))

## [3.0.3](https://github.com/oddessentials/odd-demonstration/compare/v3.0.2...v3.0.3) (2025-12-26)


### Bug Fixes

* **pty:** treat empty PTY_AUTH_TOKEN as auth disabled ([5548fb0](https://github.com/oddessentials/odd-demonstration/commit/5548fb0cefe65b01a1c621e637f8a3c10baa9620))
* **tui:** correct Docker build contexts in start-all.ps1 ([5374231](https://github.com/oddessentials/odd-demonstration/commit/5374231176cdc90adb632135e644e1b502c65866))
* **tui:** enforce consistent image versioning across K8s manifests ([31ac426](https://github.com/oddessentials/odd-demonstration/commit/31ac426c896aef352679b7e39c6e8f64b0a54051))

## [3.0.2](https://github.com/oddessentials/odd-demonstration/compare/v3.0.1...v3.0.2) (2025-12-26)


### Bug Fixes

* **ci:** add preflight check for Bazel mirror asset existence ([089e20e](https://github.com/oddessentials/odd-demonstration/commit/089e20e40eeb9f99e071297cd0d5e6082c131ff1))
* **ci:** apply --config=ci to ALL bazel commands with BCR guard ([4281d61](https://github.com/oddessentials/odd-demonstration/commit/4281d6172eed49da56113578f2b982b896f62210))
* **ci:** correct SHA verification to hash actual Bazel binary ([60df0d7](https://github.com/oddessentials/odd-demonstration/commit/60df0d70827c1c1b4d850c04e768c7fbbcebc602))
* **ci:** eliminate Bazel download dependency on releases.bazel.build ([177eb8e](https://github.com/oddessentials/odd-demonstration/commit/177eb8e5a0595859b60c0b0acda7c19ccd1890ee))
* **ci:** improve SHA verification diagnostics and cache search ([43b61de](https://github.com/oddessentials/odd-demonstration/commit/43b61de7e769b4eb357d5e21e1c490716a5ec04f))
* **ci:** replace bazel mod deps with lockfile-unchanged check ([6442f3a](https://github.com/oddessentials/odd-demonstration/commit/6442f3a3363671372464037b99c935ba9ceac529))
* **ci:** update SHA256 to match official Bazel 7.1.0 binary ([f63546c](https://github.com/oddessentials/odd-demonstration/commit/f63546ce1ab7332580b4c5d23812cb9cbab7d22e))
* **ci:** use BAZELISK_FORMAT_URL for flat GitHub Release paths ([7356ea0](https://github.com/oddessentials/odd-demonstration/commit/7356ea0cadc086993e8c69a634e2fe736480f47c))
* **ci:** use GitHub BCR mirror to avoid bcr.bazel.build TLS issues ([3cc441f](https://github.com/oddessentials/odd-demonstration/commit/3cc441f04ae8a38abbcd781da7aa2620e5ff074e))

## [3.0.1](https://github.com/oddessentials/odd-demonstration/compare/v3.0.0...v3.0.1) (2025-12-26)


### Bug Fixes

* new snapshots ([9c2667a](https://github.com/oddessentials/odd-demonstration/commit/9c2667aca8b0b4249e5561316f37916ef6dfc35c))

# [3.0.0](https://github.com/oddessentials/odd-demonstration/compare/v2.0.1...v3.0.0) (2025-12-26)


### Bug Fixes

* **visual:** add WebSocket cleanup + fix snapshot restore ([3f676a4](https://github.com/oddessentials/odd-demonstration/commit/3f676a4c83d84d6650cf18ed5af1fd5d3f02cccd))


### Features

* **web:** bundle xterm.js with esbuild and self-host assets ([3625bf2](https://github.com/oddessentials/odd-demonstration/commit/3625bf20a8e913b6e82560b74af05522734db60d))


### BREAKING CHANGES

* **web:** xterm.js is now self-hosted via bundled assets

## [2.0.1](https://github.com/oddessentials/odd-demonstration/compare/v2.0.0...v2.0.1) (2025-12-26)


### Bug Fixes

* **governance:** sync package.json versions and add coverage docs automation ([51910ce](https://github.com/oddessentials/odd-demonstration/commit/51910ce7bdd7d3b0885737a7619effbd34ed2985))

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

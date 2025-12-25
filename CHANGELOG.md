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

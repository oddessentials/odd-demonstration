#45 1.786 Downloaded crossbeam-utils v0.8.21
#45 1.792 Downloaded byteorder v1.5.0
#45 1.837 Compiling proc-macro2 v1.0.103
#45 1.837 Compiling quote v1.0.42
#45 1.837 Compiling unicode-ident v1.0.22
#45 1.837 Compiling libc v0.2.178
#45 1.925 Compiling cfg-if v1.0.4
#45 ...

#46 [web-pty-server tui-builder 10/10] RUN touch src/main.rs src/lib.rs && cargo build --release
#46 0.291 Downloading crates ...
#46 0.298 error: failed to download `moxcms v0.7.11`
#46 0.298
#46 0.298 Caused by:
#46 0.298 unable to get packages from source
#46 0.298
#46 0.298 Caused by:
#46 0.298 failed to parse manifest at `/usr/local/cargo/registry/src/index.crates.io-6f17d22bba15001f/moxcms-0.7.11/Cargo.toml`
#46 0.298
#46 0.298 Caused by:
#46 0.299 feature `edition2024` is required
#46 0.299
#46 0.299 The package requires the Cargo feature called `edition2024`, but that feature is not stabilized in this version of Cargo (1.83.0 (5ffbef321 2024-10-29)).
#46 0.299 Consider trying a newer version of Cargo (this may require the nightly release).
#46 0.299 See https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#edition-2024 for more information about the status of this feature.
#46 ERROR: process "/bin/sh -c touch src/main.rs src/lib.rs && cargo build --release" did not complete successfully: exit code: 101

#45 [web-pty-server pty-builder 6/10] RUN cargo build --release
#45 1.973 Compiling stable_deref_trait v1.2.1
#45 CANCELED

---

> [web-pty-server tui-builder 10/10] RUN touch src/main.rs src/lib.rs && cargo build --release:
> 0.298
> 0.298 Caused by:
> 0.298 failed to parse manifest at `/usr/local/cargo/registry/src/index.crates.io-6f17d22bba15001f/moxcms-0.7.11/Cargo.toml`
> 0.298
> 0.298 Caused by:
> 0.299 feature `edition2024` is required
> Dockerfile:78

---

76 |

77 | # Build the actual application

78 | >>> RUN touch src/main.rs src/lib.rs && cargo build --release

79 |

80 | # ============================================================

---

target web-pty-server: failed to solve: process "/bin/sh -c touch src/main.rs src/lib.rs && cargo build --release" did not complete successfully: exit code: 101

0.299
0.299 The package requires the Cargo feature called `edition2024`, but that feature is not stabilized in this version of Cargo (1.83.0 (5ffbef321 2024-10-29)).
0.299 Consider trying a newer version of Cargo (this may require the nightly release).
0.299 See https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#edition-2024 for more information about the status of this feature.

---

Error: Process completed with exit code 1.

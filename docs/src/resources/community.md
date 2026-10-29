# Community

AstraWeave is developed in the [lazyxeon/AstraWeave](https://github.com/lazyxeon/AstraWeave) repository. GitHub issues are the working contact channel. This repository does not publish a Discord invite, a social account, or an `astraweave.dev` mailbox. GitHub Discussions are disabled (`has_discussions` is false).

## GitHub

| Resource | Description |
|----------|-------------|
| [Repository](https://github.com/lazyxeon/AstraWeave) | Source and pull requests |
| [Issues](https://github.com/lazyxeon/AstraWeave/issues) | Bugs, questions, and feature requests |
| [Releases](https://github.com/lazyxeon/AstraWeave/releases) | Tags the repository has published |

The old Discussions URLs `github.com/astraweave/astraweave` and `github.com/lazyxeon/Veilweaver-Threads-of-Eternity` are not this repository. Do not use them.

## Getting help

Search the [book](../README.md), the [FAQ](faq.md), and [open issues](https://github.com/lazyxeon/AstraWeave/issues) before opening a new issue.

A report that can be reproduced names the command, the toolchain, and the error text. From a fresh clone:

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustc --version
cargo run -p hello_companion --release
```

`rustc --version` inside the repo should be 1.89.0. Paste that line and the compiler or runtime error into the issue. The companion binary's source is [Hello Companion](../examples/hello-companion.md). A Bevy-style `Query<(&mut T, &U), With<Marker>>` system is not the `astraweave-ecs` API. `astraweave_core::World::spawn` takes a name, an `IVec2`, a `Team`, hp, and ammo.

| Question type | Where |
|--------------|--------------|
| Bugs, questions, and feature requests | [GitHub issues](https://github.com/lazyxeon/AstraWeave/issues) |
| Security issues | [GitHub private vulnerability reporting](https://github.com/lazyxeon/AstraWeave/security/advisories/new) |

The runnable companion demo is [hello_companion](../examples/hello-companion.md). Issue reports that paste a Bevy-style `Query` system are not using the `astraweave-ecs` API.

## Contributing

The repository guide is [CONTRIBUTING.md](https://github.com/lazyxeon/AstraWeave/blob/main/CONTRIBUTING.md). The book page is [Contributing](../dev/contributing.md).

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
git checkout -b feature/my-feature
cargo test -p astraweave-core
```

Forks clone `https://github.com/YOUR_USERNAME/AstraWeave.git`. One change per pull request.

Code of conduct: [CODE_OF_CONDUCT.md](https://github.com/lazyxeon/AstraWeave/blob/main/CODE_OF_CONDUCT.md).

## Examples in this repo

| Package | Page |
|---------|------|
| `hello_companion` | [Hello Companion](../examples/hello-companion.md) |
| `adaptive_boss` | [Adaptive boss](../examples/adaptive-boss.md) |

There is no public event calendar, office hours, or sponsorship desk in this repository. Sponsorship and press notes belong in a GitHub issue.

## API docs

Of the 141 workspace member entries, 30 set `publish` in the package table. The workspace value in the root `Cargo.toml` is `publish = false`. `astraweave-core` is not a workspace member; its manifest sets `publish = { workspace = true }`. `astraweave-ecs` is a member and does not set `publish`. These crates are not the `docs.rs/astraweave` crate. Generate local rustdoc with `cargo doc -p astraweave-core --no-deps`.

## Contact

| Purpose | Contact |
|---------|---------|
| General inquiries | [GitHub issues](https://github.com/lazyxeon/AstraWeave/issues) |
| Security issues | [GitHub private vulnerability reporting](https://github.com/lazyxeon/AstraWeave/security/advisories/new) |
| Sponsorship | [GitHub issues](https://github.com/lazyxeon/AstraWeave/issues) |
| Press | [GitHub issues](https://github.com/lazyxeon/AstraWeave/issues) |

`hello@astraweave.dev`, `sponsorship@astraweave.dev`, and `press@astraweave.dev` are removed. The domain does not resolve.

## Related

- [Contributing](../dev/contributing.md)
- [Code style](../dev/code-style.md)
- [Roadmap](roadmap.md)
- [FAQ](faq.md)

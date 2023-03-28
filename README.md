# github.alfredworkflow

[![Build Status](https://img.shields.io/github/actions/workflow/status/rossmacarthur/github.alfredworkflow/build.yaml?branch=trunk)](https://github.com/rossmacarthur/github.alfredworkflow/actions/workflows/build.yaml)
[![Latest release](https://img.shields.io/github/v/release/rossmacarthur/github.alfredworkflow)](https://github.com/rossmacarthur/github.alfredworkflow/releases/latest)

:octocat: Alfred workflow to search GitHub repositories.

<img width="605" alt="Screenshot" src="https://user-images.githubusercontent.com/17109887/228236202-dee99039-5ffc-451b-8a38-9b541a5cdcf7.png">

## Features

- List repositories for any configured users and/or organizations.
- Open the selected repository in your browser.
- Blazingly fast 🤸.

## 📦 Installation

### Pre-packaged

Grab the latest release from
[the releases page](https://github.com/rossmacarthur/github.alfredworkflow/releases).

Because the release contains an executable binary later versions of macOS will
mark it as untrusted and Alfred won't be able to execute it. You can run the
following to explicitly trust the release before installing to Alfred.
```sh
xattr -c ~/Downloads/github-*-apple-darwin.alfredworkflow
```

### Building from source

This workflow is written in Rust, so to install it from source you will first
need to install Rust and Cargo using [rustup](https://rustup.rs/). Then install
[powerpack](https://github.com/rossmacarthur/powerpack). Then you can run the
following to build an `.alfredworkflow` file.

```sh
git clone https://github.com/rossmacarthur/github.alfredworkflow.git
cd github.alfredworkflow
powerpack package
```

The release will be available at `target/workflow/github.alfredworkflow`.

## Configuration

You can configure the users and organizations from which the list of
repositories is fetched for by setting the following environment variables.

| Name           | Example                 | Description                                                      |
| -------------- | ----------------------- | ---------------------------------------------------------------- |
| `GITHUB_TOKEN` | `ghp_pv7K2GA...`        | GitHub [personal access token] with `repo` and `read:org` scopes |
| `GITHUB_USERS` | `rossmacarthur`         | Comma separated list of GitHub users                             |
| `GITHUB_ORGS`  | `extractions,rust-lang` | Comma separated list of GitHub organizations                     |

[personal access token]: https://github.com/settings/tokens/new?description=github.alfredworkflow&scopes=repo,read:org

## License

This project is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

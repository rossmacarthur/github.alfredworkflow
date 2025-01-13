# github.alfredworkflow

[![Build Status](https://badgers.space/github/checks/rossmacarthur/github.alfredworkflow?label=build)](https://github.com/rossmacarthur/github.alfredworkflow/actions/workflows/build.yaml)
[![Latest release](https://badgers.space/github/release/rossmacarthur/github.alfredworkflow)](https://github.com/rossmacarthur/github.alfredworkflow/releases/latest)

:octocat: Alfred workflow to search GitHub repositories and pull requests.

## Features

- Configurable commands.
- List repositories for any configured users and/or organizations.
  - Open the selected repository in your browser.
- List pull requests for any configured repositories.
  - Open the selected pull request in your browser.
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

Behaviour of the workflow is configured using environment variables. At the top
level there are "commands" which can be arbitrarily named. A command takes one
of the following forms where `<cmd>` is the user defined name for the command.

| Key                  | Value            | Description                    |
| -------------------- | ---------------- | ------------------------------ |
| `GITHUB_REPOS_<cmd>` | `org:<owner>`    | List organization repositories |
| `GITHUB_REPOS_<cmd>` | `user:<owner>`   | List user repositories         |
| `GITHUB_PULLS_<cmd>` | `<owner>/<repo>` | List repository pull requests  |

Authentication for the workflow is configured using environment variables. For
any given command if there is an access token specific for that owner then it
will be used, otherwise the workflow will fallback to the default access token.
The default access token should always be set even if the command doesn't
require special access levels because of the increased rate limits provided.

| Name                  | Example          | Description                                            |
| --------------------- | ---------------- | ------------------------------------------------------ |
| `GITHUB_TOKEN`        | `ghp_pv7K2GA...` | GitHub [personal access token] to use                  |
| `GITHUB_TOKEN_owner1` | `ghp_11aEcRG..`  | GitHub [personal access token] used to access `owner1` |
| `GITHUB_TOKEN_owner2` | `ghp_11aEcRG..`  | GitHub [personal access token] used to access `owner2` |

### Examples

Let's say you wanted to have two commands:
- `repos` that listed all the repositories in the `rust-lang` organization
- `pulls` that listed all the pull requests in the `rust-lang/rust` repository

You would define the following environment variables.

| Key                  | Value            | Description                                             |
| -------------------- | ---------------- | ------------------------------------------------------- |
| `GITHUB_TOKEN`       | `ghp_pv7K2GA...` | GitHub [personal access token]                          |
| `GITHUB_REPOS_repos` | `org:rust-lang`  | List the repositories on organization [`rust-lang`]     |
| `GITHUB_PULLS_pulls` | `rust-lang/rust` | List the pull requests on repository [`rust-lang/rust`] |

[`rust-lang`]: https://github.com/rust-lang
[`rust-lang/rust`]: https://github.com/rust-lang/rust

<img width="605" alt="image" src="https://github.com/user-attachments/assets/4320599e-45cb-476a-82b2-9c734a2c774a" />
<img width="605" alt="image" src="https://github.com/user-attachments/assets/6e80d2ec-858f-4131-a120-6b889c2413b5" />
<img width="605" alt="image" src="https://github.com/user-attachments/assets/943ccca6-4a42-456e-90aa-615c00352cfd" />

## License

This project is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

use std::collections::HashMap;
use std::env;
use std::fmt;

use anyhow::{anyhow, bail, Context, Result};

#[derive(Debug)]
pub struct Config {
    /// The configured GitHub auth token.
    ///
    /// This will be used if there is no organization / user specific token.
    pub token: Option<String>,

    /// The per owner configured GitHub auth tokens.
    ///
    /// This is represented as a map of organization / user (owner) name to
    /// GitHub auth token
    pub tokens: HashMap<String, String>,

    /// List of available commands based on the configuration.
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub enum Command {
    /// List repositories for a user
    UserRepos { name: String, user: String },
    /// List repositories for an organization
    OrgRepos { name: String, org: String },
    /// List pull requests for a repository
    Pulls { name: String, repo: Repo },
}

#[derive(Debug)]
pub struct Repo {
    /// The owner of the repository (organization / user)
    pub owner: String,
    /// The name of the repository
    pub name: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut token = None;
        let mut tokens = HashMap::new();
        let mut commands = Vec::new();

        for (k, v) in env::vars() {
            if v.is_empty() {
                continue;
            }
            if k == "GITHUB_TOKEN" {
                token = Some(v);
            } else if let Some(owner) = k.strip_prefix("GITHUB_TOKEN_") {
                tokens.insert(owner.to_owned(), v);
            } else if let Some(name) = k.strip_prefix("GITHUB_REPOS_") {
                if let Some(user) = v.strip_prefix("user:") {
                    commands.push(Command::UserRepos {
                        name: name.to_lowercase().to_owned(),
                        user: user.to_owned(),
                    });
                } else if let Some(org) = v.strip_prefix("org:") {
                    commands.push(Command::OrgRepos {
                        name: name.to_lowercase().to_owned(),
                        org: org.to_owned(),
                    });
                } else {
                    bail!("invalid value '{}', expected 'user:' or 'org:' prefix", v)
                }
            } else if let Some(name) = k.strip_prefix("GITHUB_PULLS_") {
                let repo = Repo::parse(&v).with_context(|| format!("invalid repo: {}", v))?;
                commands.push(Command::Pulls {
                    name: name.to_owned(),
                    repo,
                });
            }
        }

        Ok(Self {
            token,
            tokens,
            commands,
        })
    }

    pub fn get_token(&self, owner: &str) -> Result<&str> {
        self.tokens
            .get(owner)
            .or(self.token.as_ref())
            .map(String::as_str)
            .ok_or_else(|| anyhow!("no token for {owner}"))
    }
}

impl Command {
    pub fn name(&self) -> &str {
        match self {
            Self::UserRepos { name, .. } => name,
            Self::OrgRepos { name, .. } => name,
            Self::Pulls { name, .. } => name,
        }
    }
}

impl Repo {
    fn parse(name: &str) -> Option<Self> {
        let mut parts = name.split('/').map(str::to_owned);
        let owner = parts.next()?;
        let name = parts.next()?;
        Some(Self { owner, name })
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

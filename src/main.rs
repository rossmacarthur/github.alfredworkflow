mod config;
mod github;
mod ord_float;

use std::cmp::Reverse;
use std::env;
use std::io;
use std::time::Duration;

use anyhow::Result;
use constcat::concat;
use itermore::IterSorted;
use powerpack::logger;
use powerpack::Item;

use crate::config::{Command, Config, Repo};
use crate::ord_float::OrdFloat;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const LOG_FILENAME: &str = concat!(PKG_NAME, "-", PKG_VERSION, ".log");

#[derive(Debug)]
pub struct Repository {
    // owner: String,
    name: String,
    description: Option<String>,
    url: String,
    is_fork: bool,
    is_archived: bool,
    is_private: bool,
    updated_at: jiff::Timestamp,
}

#[derive(Debug)]
struct PullRequest {
    number: u64,
    title: String,
    url: String,
    author: String,
    updated_at: jiff::Timestamp,
}

impl Repository {
    fn cmp_key(&self, query: &str) -> impl Ord {
        (
            Reverse(OrdFloat(similar(&self.name, query))),
            self.is_archived,
            self.is_fork,
            Reverse(self.updated_at),
        )
    }

    fn title(&self) -> String {
        let mut title = self.name.clone();
        if self.is_private {
            title.push_str(" 🔒");
        }
        if self.is_archived {
            title.push_str(" 📁");
        }
        title
    }

    fn into_item(self, cmd: &Command) -> Item {
        let autocomplete = format!("{} {}", cmd.name(), &self.name);
        let item = Item::new(self.title())
            .arg(self.url)
            .autocomplete(autocomplete);
        match self.description {
            Some(desc) => item.subtitle(desc),
            None => item,
        }
    }
}

impl PullRequest {
    fn cmp_key(&self, query: &str) -> impl Ord {
        (
            Reverse(OrdFloat(similar(&self.title, query))),
            Reverse(self.updated_at),
        )
    }

    fn into_item(self, cmd: &Command) -> Item {
        Item::new(&self.title)
            .arg(self.url)
            .subtitle(format!("Pull request #{} by {}", self.number, self.author))
            .autocomplete(format!("{} {}", cmd.name(), &self.title))
    }
}

fn similar(value: &str, query: &str) -> f64 {
    if query.trim() == "" {
        return 1.;
    }
    let value = value.to_ascii_lowercase();
    let query = query.to_ascii_lowercase();
    let mut score = strsim::jaro(&value, &query);
    if value.starts_with(&query) {
        score += 0.3;
    }
    (score * 10.0).round() / 10.0
}

fn user_repos(cmd: &Command, config: &Config, user: &str, query: &str) -> Result<Vec<Item>> {
    Ok(github::user_repos(config, user)?
        .into_iter()
        .sorted_by_key(|repo| repo.cmp_key(query))
        .map(|repo| repo.into_item(cmd))
        .collect())
}

fn org_repos(cmd: &Command, config: &Config, org: &str, query: &str) -> Result<Vec<Item>> {
    Ok(github::org_repos(config, org)?
        .into_iter()
        .sorted_by_key(|repo| repo.cmp_key(query))
        .map(|repo| repo.into_item(cmd))
        .collect())
}

fn pulls(cmd: &Command, config: &Config, repo: &Repo, query: &str) -> Result<Vec<Item>> {
    let (query, author) = extract_author(query);
    let pulls = github::pulls(config, &repo.owner, &repo.name)?
        .into_iter()
        .filter(|pull| match author {
            Some(author) => pull.author.starts_with(author),
            None => true,
        })
        .sorted_by_key(|pull| pull.cmp_key(&query))
        .map(|pull| pull.into_item(cmd))
        .collect();
    Ok(pulls)
}

fn extract_author(query: &str) -> (String, Option<&str>) {
    query
        .split_ascii_whitespace()
        .find_map(|word| {
            if word.starts_with('@') && word.len() > 1 {
                let author = word.trim_start_matches("@");
                let query = query.replace(word, "");
                Some((query, author))
            } else {
                None
            }
        })
        .map_or_else(
            || (query.to_string(), None),
            |(query, author)| (query, Some(author)),
        )
}

impl Command {
    fn kind(&self) -> &str {
        match self {
            Command::UserRepos { .. } => "repositories",
            Command::OrgRepos { .. } => "repositories",
            Command::Pulls { .. } => "pull requests",
        }
    }

    fn to_item(&self) -> Item {
        match self {
            Self::UserRepos { name, user } => Item::new(name)
                .subtitle(format!("Search {user}'s repositories"))
                .autocomplete(format!("{name} ")),
            Self::OrgRepos { name, org } => Item::new(name)
                .subtitle(format!("Search {org}'s respositories"))
                .autocomplete(format!("{name} ")),
            Self::Pulls { name, repo } => Item::new(name)
                .subtitle(format!("Search pull requests against {repo}"))
                .arg(format!("https://github.com/{}/pulls", repo))
                .autocomplete(format!("{name} ")),
        }
    }

    fn exec(&self, config: &Config, query: &str) -> Result<Vec<Item>> {
        match self {
            Self::UserRepos { user, .. } => user_repos(self, config, user, query),
            Self::OrgRepos { org, .. } => org_repos(self, config, org, query),
            Self::Pulls { repo, .. } => pulls(self, config, repo, query),
        }
    }
}

fn run() -> Result<()> {
    logger::Builder::new().filename(LOG_FILENAME).try_init()?;

    let config = Config::load()?;

    let arg = env::args()
        .nth(1)
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase);

    if config.commands.is_empty() {
        let item = Item::new("No commands configured yet")
            .subtitle("Configure commands for this workflow using environment variables");
        return output([item]);
    }

    let items = match arg {
        // If no argument is given then just list the available commands
        None => config.commands.iter().map(Command::to_item).collect(),

        // Otherwise process the argument
        Some(arg) => {
            // Get the command and the search query
            let (cmd, query) = arg.split_once(char::is_whitespace).unwrap_or((&arg, ""));

            match config.commands.iter().find(|c| c.name() == cmd) {
                // There is a command that matches this query so execute it
                Some(command) => {
                    let items = command.exec(&config, query)?;
                    if items.is_empty() {
                        let item = Item::new(format!("No {} found", command.kind()));
                        return output([item]);
                    }
                    items
                }

                // No command matches the query exactly, output the commands
                // that start with the half-entered command
                None => {
                    let items: Vec<_> = config
                        .commands
                        .iter()
                        .filter(|c| c.name().starts_with(cmd))
                        .map(Command::to_item)
                        .collect();
                    if items.is_empty() {
                        let item = Item::new("No command found");
                        return output([item]);
                    }
                    items
                }
            }
        }
    };

    output(items)
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        let item = Item::new(format!("Error: {err}")).subtitle(
            "The workflow errored! \
             You might want to try debugging it or checking the logs.",
        );
        output([item])?;
    }
    Ok(())
}

fn output(items: impl IntoIterator<Item = Item>) -> Result<()> {
    powerpack::Output::new()
        .items(items)
        .rerun(Duration::from_secs(2))
        .write(io::stdout())?;
    Ok(())
}

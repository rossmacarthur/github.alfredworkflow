mod cache;
mod github;
mod logger;

use std::cmp::Reverse;
use std::env;
use std::iter;

use anyhow::Result;
use chrono::DateTime;
use powerpack::Item;

#[derive(Debug)]
pub struct Repository {
    owner: String,
    name: String,
    description: Option<String>,
    url: String,
    is_fork: bool,
    is_archived: bool,
    is_private: bool,
    updated_at: DateTime<chrono::Utc>,
}

impl Repository {
    fn into_item(self) -> Item<'static> {
        let mut title = format!("{}/{}", self.owner, self.name);
        if self.is_private {
            title.push_str(" 🔒");
        }
        if self.is_archived {
            title.push_str(" 📁");
        }
        let item = Item::new(title).arg(self.url);
        match self.description {
            Some(desc) => item.subtitle(desc),
            None => item,
        }
    }
}

fn run() -> Result<()> {
    let query = env::args()
        .nth(1)
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase);
    let filter_fn = |repo: &Repository| repo.name.contains(query.as_deref().unwrap_or(""));

    let mut repos = Vec::new();
    if let Some(users) = powerpack::env::var("GITHUB_USERS") {
        for user in users.split(',') {
            repos.extend(github::user_repos(user)?.into_iter().filter(filter_fn));
        }
    }
    if let Some(orgs) = powerpack::env::var("GITHUB_ORGS") {
        for org in orgs.split(',') {
            repos.extend(github::org_repos(org)?.into_iter().filter(filter_fn));
        }
    }

    repos.sort_by_key(|repo| (repo.is_archived, repo.is_fork, Reverse(repo.updated_at)));
    powerpack::output(repos.into_iter().map(Repository::into_item))?;

    Ok(())
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{:#}", err);
        let item = Item::new(format!("Error: {}", err)).subtitle(
            "The workflow errored! \
             You might want to try debugging it or checking the logs.",
        );
        powerpack::output(iter::once(item))?;
    }
    Ok(())
}

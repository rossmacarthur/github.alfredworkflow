mod cache;
mod github;

use std::env;

use anyhow::Result;
use itertools::Itertools;
use powerpack::{Icon, Item};

fn to_item(repo: github::Repo) -> Item<'static> {
    Item::new(format!("{}/{}", repo.owner.login, repo.name))
        .arg(repo.url())
        .icon(Icon::new("icon.png"))
}

fn run(query: Option<&str>) -> Result<()> {
    let repos = cache::repos()?;
    match query {
        Some("") | None => powerpack::output(repos.into_iter().map(to_item)),
        Some(query) => powerpack::output(
            repos
                .into_iter()
                .filter(|repo| repo.owner.login.starts_with(query) || repo.name.starts_with(query))
                .sorted_by(|a, b| (&a.name, &a.owner.login).cmp(&(&b.name, &b.owner.login)))
                .map(to_item),
        ),
    }?;
    Ok(())
}

fn main() -> Result<()> {
    run(env::args().nth(1).as_deref().map(|s| s.trim()))
}

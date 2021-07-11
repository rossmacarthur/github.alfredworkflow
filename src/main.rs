mod cache;
mod github;

use std::env;
use std::iter;
use std::vec;

use anyhow::Result;
use itertools::Itertools;
use powerpack::{Item, String};

const SHORTCUTS: &[(&str, &str)] = &[
    ("/feed", "/"),
    ("/issues", "/issues"),
    ("/notifications", "/notifications"),
    ("/pulls", "/pulls"),
    ("/settings", "/settings"),
];

fn shortcuts() -> vec::IntoIter<(String<'static>, String<'static>)> {
    let mut shortcuts: Vec<_> = SHORTCUTS
        .iter()
        .map(|&(a, b)| (String::from(a), String::from(b)))
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .collect();
    if let Some(user) = powerpack::env::var("GITHUB_USER") {
        shortcuts.insert(0, ("/profile".into(), format!("/{}", user).into()));
    }
    shortcuts.into_iter()
}

fn repo_to_item(repo: github::Repo) -> Item<'static> {
    Item::new(repo.name.clone())
        .subtitle(format!("#{}/{}", repo.owner.login, repo.name))
        .arg(repo.url())
}

fn shortcut_to_item<'a>(shortcut: (String<'a>, String<'a>)) -> Item<'a> {
    Item::new(shortcut.0).arg(format!("https://github.com{}", shortcut.1))
}

fn exact<'a>(query: &'a str) -> Item<'a> {
    Item::new(query).arg(format!("https://github.com/{}", query))
}

fn run(query: Option<&str>) -> Result<()> {
    let repos = cache::repos()?;
    match query {
        Some("") | None => powerpack::output(repos.into_iter().sorted().map(repo_to_item)),
        Some(query) if query.starts_with('/') => powerpack::output(
            shortcuts()
                .filter(|(s, _)| s.starts_with(query))
                .map(shortcut_to_item),
        ),
        Some(query) if query.contains('/') => powerpack::output(iter::once(exact(query))),
        Some(query) => powerpack::output(
            repos
                .into_iter()
                .filter(|repo| repo.owner.login.starts_with(query) || repo.name.starts_with(query))
                .sorted()
                .map(repo_to_item)
                .chain(
                    shortcuts()
                        .filter(|(s, _)| s[1..].starts_with(query))
                        .map(shortcut_to_item),
                ),
        ),
    }?;
    Ok(())
}

fn main() -> Result<()> {
    run(env::args().nth(1).as_deref().map(str::trim))
}

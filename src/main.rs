mod cache;
mod github;

use std::env;
use std::vec;

use anyhow::Result;
use itertools::Itertools;
use powerpack::{Item, String as Str};

const SHORTCUTS: &[(&str, &str)] = &[
    ("/feed", "/"),
    ("/issues", "/issues"),
    ("/notifications", "/notifications"),
    ("/pulls", "/pulls"),
    ("/settings", "/settings"),
];

fn shortcuts() -> vec::IntoIter<(Str<'static>, Str<'static>)> {
    let mut shortcuts: Vec<_> = SHORTCUTS
        .iter()
        .map(|&(a, b)| (Str::from(a), Str::from(b)))
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .collect();
    if let Some(user) = powerpack::env::var("GITHUB_USER") {
        shortcuts.insert(0, ("/profile".into(), format!("/{}", user).into()));
    }
    shortcuts.into_iter()
}

fn repo_to_item(repo: github::Repo) -> Item<'static> {
    Item::new(format!("{}/{}", repo.owner.login, repo.name)).arg(repo.url())
}

fn shortcut_to_item<'a>(shortcut: (Str<'a>, Str<'a>)) -> Item<'a> {
    Item::new(shortcut.0).arg(format!("https://github.com{}", shortcut.1))
}

fn run(query: Option<&str>) -> Result<()> {
    let repos = cache::repos()?;
    match query {
        Some("") | None => powerpack::output(repos.into_iter().map(repo_to_item)),
        Some(query) if query.starts_with('/') => powerpack::output(
            shortcuts()
                .filter(|(s, _)| s.starts_with(query))
                .map(shortcut_to_item),
        ),
        Some(query) => powerpack::output(
            repos
                .into_iter()
                .filter(|repo| repo.owner.login.starts_with(query) || repo.name.starts_with(query))
                .sorted_by(|a, b| (&a.name, &a.owner.login).cmp(&(&b.name, &b.owner.login)))
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

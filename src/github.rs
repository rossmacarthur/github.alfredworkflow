use anyhow::Result;
use isahc::prelude::*;
use once_cell::sync::Lazy;
use powerpack::env;
use regex_macro::regex;
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

static TOKEN: Lazy<String> =
    Lazy::new(|| env::var("GITHUB_TOKEN").expect("`GITHUB_TOKEN` must be set"));

#[derive(Debug, Deserialize, Serialize)]
pub struct Owner {
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Repo {
    pub owner: Owner,
    pub name: String,
}

impl Repo {
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner.login, self.name)
    }
}

pub fn repos() -> Result<Vec<Repo>> {
    let mut repos = Vec::new();
    let mut url = String::from("https://api.github.com/user/repos");

    let client = isahc::HttpClient::builder()
        .default_headers(&[
            ("Accept", "application/vnd.github.v3+json"),
            ("Authorization", &format!("token {}", &*TOKEN)),
            ("User-Agent", USER_AGENT),
        ])
        .build()?;

    loop {
        let mut resp = client.get(url)?;
        repos.extend(resp.json::<Vec<_>>()?.into_iter());

        let link = resp.headers().get("link").unwrap().to_str().unwrap();
        let next = regex!(r#".*<(.*)>; rel="next".*"#)
            .captures(link)
            .map(|caps| caps[1].into());

        if let Some(u) = next {
            url = u;
        } else {
            break;
        }
    }
    Ok(repos)
}

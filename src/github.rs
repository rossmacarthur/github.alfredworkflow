use std::io::prelude::*;

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json as json;

use crate::Repository;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn fetch(query: &str, token: &str) -> Result<json::Value> {
    #[derive(Debug, Serialize)]
    struct Query<'a> {
        query: &'a str,
    }

    let mut buf = Vec::new();
    let mut easy = curl::easy::Easy::new();
    let mut data = &*serde_json::to_vec(&Query { query })?;

    eprintln!("{}", query);

    easy.fail_on_error(true)?;
    easy.follow_location(true)?;
    easy.http_headers({
        let mut hl = curl::easy::List::new();
        hl.append(&format!("Authorization: Bearer {}", token))?;
        hl.append(&format!("User-Agent: {}", USER_AGENT))?;
        hl
    })?;
    easy.post(true)?;
    easy.url("https://api.github.com/graphql")?;

    {
        let mut t = easy.transfer();
        t.read_function(|into| Ok(data.read(into).unwrap()))?;
        t.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        })?;
        t.perform()?;
    }

    Ok(serde_json::from_slice(&buf)?)
}

fn fetch_and_parse<T>(
    name: &str,
    query: &str,
    checksum: [u8; 20],
    ptr: &str,
    parse_fn: fn(json::Value) -> Result<T>,
) -> Result<Vec<T>> {
    let token = powerpack::env::var("GITHUB_TOKEN")
        .ok_or_else(|| anyhow!("GITHUB_TOKEN environment variable is not set!"))?;
    let resp = crate::cache::load(name, checksum, || fetch(query, &token))?;
    let nodes: Vec<json::Value> = lookup(&resp, ptr)?;
    nodes.into_iter().map(parse_fn).collect()
}

pub fn user_repos(user: &str) -> Result<Vec<Repository>> {
    repos("user", user)
}

pub fn org_repos(org: &str) -> Result<Vec<Repository>> {
    repos("organization", org)
}

fn repos(typ: &str, login: &str) -> Result<Vec<Repository>> {
    let query = r#"
query {
    <type>(login: "<login>") {
        repositories(first: 100) {
            nodes {
                owner {
                    login
                }
                name
                description
                url
                isFork
                isArchived
                isPrivate
                pushedAt
            }
        }
    }
}"#
    .replace("<type>", typ)
    .replace("<login>", login);
    let ptr = format!("/data/{}/repositories/nodes", typ);
    let checksum = checksum(&query);
    fetch_and_parse(
        &format!("{}_repos", login),
        &query,
        checksum,
        &ptr,
        parse_repository,
    )
}

fn parse_repository(value: json::Value) -> Result<Repository> {
    let owner = lookup(&value, "/owner/login")?;
    let name = lookup(&value, "/name")?;
    let description = lookup(&value, "/description")?;
    let url = lookup(&value, "/url")?;
    let is_fork = lookup(&value, "/isFork")?;
    let is_archived = lookup(&value, "/isArchived")?;
    let is_private = lookup(&value, "/isPrivate")?;
    let updated_at: DateTime<chrono::Utc> = lookup::<String>(&value, "/pushedAt")?.parse()?;
    Ok(Repository {
        owner,
        name,
        description,
        url,
        is_fork,
        is_archived,
        is_private,
        updated_at,
    })
}

fn lookup<T>(value: &json::Value, ptr: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let v = value
        .pointer(ptr)
        .with_context(|| format!("failed to lookup `{}`", ptr))?;
    Ok(json::from_value(v.clone())?)
}

fn checksum(query: &str) -> [u8; 20] {
    use sha1::*;
    let mut hasher = Sha1::new();
    hasher.update(query.as_bytes());
    hasher.finalize().try_into().unwrap()
}

use std::io::prelude::*;

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::Repository;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

type ParseFn = fn(json::Value) -> Result<Repository>;

struct Query<'a> {
    name: String,
    login: &'a str,
    query: String,
    page_info_ptr: &'a str,
    nodes_ptr: &'a str,
    parse_fn: ParseFn,
}

#[derive(Debug, Serialize)]
struct Variables<'a> {
    login: &'a str,
    after: Option<String>,
}

#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "endCursor")]
    cursor: String,
    #[serde(rename = "hasNextPage")]
    has_next: bool,
}

impl Query<'_> {
    fn checksum(&self) -> [u8; 20] {
        use sha1::*;
        let mut hasher = Sha1::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.login.as_bytes());
        hasher.update(self.query.as_bytes());
        hasher.finalize().try_into().unwrap()
    }
}

fn fetch_and_parse(q: Query<'_>) -> Result<Vec<Repository>> {
    let token = powerpack::env::var("GITHUB_TOKEN")
        .ok_or_else(|| anyhow!("GITHUB_TOKEN environment variable is not set!"))?;

    let mut r = crate::cache::load(&q.name, q.checksum(), || fetch_all(&q, &token))?;
    let resps = r
        .as_array_mut()
        .context("cache value is not an array")?
        .drain(..);

    let mut nodes = Vec::new();
    for resp in resps {
        let ns: Vec<json::Value> = lookup(&resp, q.nodes_ptr)?;
        nodes.extend(ns);
    }
    nodes.into_iter().map(q.parse_fn).collect()
}

fn fetch_all(q: &Query<'_>, token: &str) -> Result<json::Value> {
    let mut array = Vec::new();
    let mut variables = Variables {
        login: q.login,
        after: None,
    };

    loop {
        let resp = fetch(&q.query, &variables, token)?;
        let page_info: PageInfo = lookup(&resp, q.page_info_ptr)?;
        array.push(resp);
        if !page_info.has_next {
            break Ok(json::Value::Array(array));
        }
        variables.after = Some(page_info.cursor);
    }
}

fn fetch(query: &str, variables: &Variables, token: &str) -> Result<json::Value> {
    #[derive(Debug, Serialize)]
    struct Query<'a> {
        query: &'a str,
        variables: &'a Variables<'a>,
    }

    let mut buf = Vec::new();
    let mut easy = curl::easy::Easy::new();
    let mut data = &*serde_json::to_vec(&Query { query, variables })?;

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

pub fn user_repos(user: &str) -> Result<Vec<Repository>> {
    repos("user", user)
}

pub fn org_repos(org: &str) -> Result<Vec<Repository>> {
    repos("organization", org)
}

fn repos(kind: &str, login: &str) -> Result<Vec<Repository>> {
    let template = r#"
query($login: String!, $after: String) {
    <kind>(login: $login) {
        repositories(first: 100, after: $after) {
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
            pageInfo {
                endCursor
                hasNextPage
            }
        }
    }
}"#;
    let query = template.replace("<kind>", kind);
    fetch_and_parse(Query {
        name: format!("{}_repos", login),
        login,
        query,
        page_info_ptr: &format!("/data/{}/repositories/pageInfo", kind),
        nodes_ptr: &format!("/data/{}/repositories/nodes", kind),
        parse_fn: parse_repository,
    })
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

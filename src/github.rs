use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use constcat::concat;
use powerpack::cache;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::config::Config;
use crate::{PullRequest, Repository};

const USER_AGENT: &str = concat!(crate::PKG_NAME, "/", crate::PKG_VERSION);

static CACHE: LazyLock<cache::Cache> = LazyLock::new(|| {
    cache::Builder::new()
        .ttl(Duration::from_secs(60))
        .initial_poll(Duration::from_millis(500))
        .build()
});

type ParseFn<T> = fn(json::Value) -> Result<T>;

struct Query<'a, T> {
    /// The name of the query, used for the cache key
    name: String,
    /// The GraphQL query
    query: &'a str,
    /// The GraphQL query variables
    variables: HashMap<&'static str, json::Value>,
    /// A JSON pointer to the page info in the result of the query
    page_info_ptr: &'a str,
    /// A JSON pointer to the queried data in the result of the query
    nodes_ptr: &'a str,
    /// A function to parse the queried data into T
    parse_fn: ParseFn<T>,
}

#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next: bool,
    #[serde(rename = "endCursor")]
    cursor: Option<String>,
}

impl<T> Query<'_, T> {
    fn checksum(&self) -> [u8; 20] {
        use sha1::*;
        let mut hasher = Sha1::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.query.as_bytes());
        hasher.finalize().into()
    }
}

fn fetch_and_parse<T>(token: &str, q: Query<'_, T>) -> Result<Vec<T>> {
    let mut r = CACHE.query(
        cache::Query::new(&q.name)
            .checksum(q.checksum())
            .update_fn(|| fetch_all(&q, token)),
    )?;
    let resps = r
        .as_array_mut()
        .context("cache value is not an array")?
        .drain(..);

    let mut nodes = Vec::new();
    for resp in resps {
        let ns: Vec<json::Value> = lookup(&resp, q.nodes_ptr).context("failed to lookup nodes")?;
        nodes.extend(ns);
    }
    nodes.into_iter().map(q.parse_fn).collect()
}

fn fetch_all<T>(q: &Query<'_, T>, token: &str) -> Result<json::Value> {
    let mut array = Vec::new();
    let mut variables = q.variables.clone();

    loop {
        let resp = fetch(q.query, &variables, token)?;
        let page_info: PageInfo =
            lookup(&resp, q.page_info_ptr).context("failed to lookup page info")?;
        array.push(resp);
        if !page_info.has_next {
            break Ok(json::Value::Array(array));
        }
        let after = json::Value::from(page_info.cursor.context("expected cursor in page info")?);
        variables.insert("after", after);
    }
}

fn fetch(
    query: &str,
    variables: &HashMap<&'static str, json::Value>,
    token: &str,
) -> Result<json::Value> {
    #[derive(Debug, Serialize)]
    struct Query<'a> {
        query: &'a str,
        variables: &'a HashMap<&'static str, json::Value>,
    }

    let agent = ureq::config::Config::builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        .build()
        .new_agent();

    let data: json::Value = agent
        .post("https://api.github.com/graphql")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", USER_AGENT)
        .send_json(&Query { query, variables })?
        .body_mut()
        .read_json()?;

    // GitHub can return a 200 OK with an error message in the body
    if let Some(errs) = data.pointer("/errors") {
        if let Some(errs) = errs.as_array() {
            for err in errs {
                if let Some(json::Value::String(msg)) = err.pointer("/message") {
                    return Err(anyhow!("GitHub error: {}", msg));
                }
            }
        }
        return Err(anyhow!("GitHub error: {}", errs));
    }

    Ok(data)
}

pub fn user_repos(config: &Config, user: &str) -> Result<Vec<Repository>> {
    repos(config, "user", user)
}

pub fn org_repos(config: &Config, org: &str) -> Result<Vec<Repository>> {
    repos(config, "organization", org)
}

fn repos(config: &Config, kind: &str, login: &str) -> Result<Vec<Repository>> {
    let token = config.get_token(login)?;

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

    fetch_and_parse(
        token,
        Query {
            name: format!("repos_{login}"),
            query: &query,
            variables: HashMap::from_iter([("login", json::Value::from(login))]),
            page_info_ptr: &format!("/data/{kind}/repositories/pageInfo"),
            nodes_ptr: &format!("/data/{kind}/repositories/nodes"),
            parse_fn: parse_repository,
        },
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
    let updated_at: jiff::Timestamp = lookup::<String>(&value, "/pushedAt")?.parse()?;
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

pub fn pulls(config: &Config, login: &str, name: &str) -> Result<Vec<PullRequest>> {
    let token = config.get_token(login)?;

    let query = r#"
query($login: String!, $name: String!, $after: String) {
    repository(owner: $login, name: $name) {
        pullRequests(first: 100, after: $after, states: [OPEN]) {
            nodes {
                number
                title
                url
                author {
                    login
                }
                updatedAt
            }
            pageInfo {
                endCursor
                hasNextPage
            }
        }
    }
}
"#;

    fetch_and_parse(
        token,
        Query {
            name: format!("pulls_{login}_{name}"),
            query,
            variables: HashMap::from_iter([
                ("login", json::Value::from(login)),
                ("name", json::Value::from(name)),
            ]),
            page_info_ptr: "/data/repository/pullRequests/pageInfo",
            nodes_ptr: "/data/repository/pullRequests/nodes",
            parse_fn: parse_pull_request,
        },
    )
}

fn parse_pull_request(value: json::Value) -> Result<PullRequest> {
    let number = lookup(&value, "/number")?;
    let title = lookup(&value, "/title")?;
    let url = lookup(&value, "/url")?;
    let author = lookup(&value, "/author/login")?;
    let updated_at: jiff::Timestamp = lookup::<String>(&value, "/updatedAt")?.parse()?;
    Ok(PullRequest {
        number,
        title,
        url,
        author,
        updated_at,
    })
}

fn lookup<T>(value: &json::Value, ptr: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let v = value
        .pointer(ptr)
        .with_context(|| format!("failed to lookup `{ptr}` in `{value:?}`"))?;
    Ok(json::from_value(v.clone())?)
}

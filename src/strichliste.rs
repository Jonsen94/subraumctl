//! The deployed 0.11.0 API: values are cents, including fractional cents.
//! Only selected account fields escape this module; transaction responses are ignored.

pub mod cli;

use std::time::Duration;

use eyre::{Result, ensure, eyre};
use serde_json::{Value, json};

use crate::cli::GlobalArgs;

pub const MCP_COMMENT: &str = "triggered by mcp";

#[derive(Clone, Debug)]
pub struct Options {
    pub url: Option<String>,
    pub user: Option<String>,
    pub user_id: Option<u64>,
    pub timeout: Duration,
}

impl Options {
    pub fn from_global(global: &GlobalArgs) -> Self {
        Self {
            url: global.strichliste_url.clone(),
            user: global.strichliste_user.clone(),
            user_id: global.strichliste_user_id,
            timeout: Duration::from_secs(global.strichliste_timeout),
        }
    }
}

/// This intentionally has no transfer, user lookup, or user selector variant.
#[derive(Clone, Debug)]
pub enum Request {
    Balance,
    Rank,
    Pay(f64),
    Add(f64),
}

impl Request {
    pub fn value(&self) -> Result<Option<f64>> {
        match self {
            Self::Pay(euros) => cents(*euros).map(|value| Some(-value)),
            Self::Add(euros) => cents(*euros).map(Some),
            _ => Ok(None),
        }
    }

    pub fn preview(&self) -> Result<Value> {
        Ok(match self.value()? {
            Some(value) => json!({"dry_run": true, "submitted": false, "value_cents": value}),
            None => json!({"dry_run": true}),
        })
    }
}

pub fn parse_amount(value: &str) -> std::result::Result<f64, String> {
    let amount = value
        .parse::<f64>()
        .map_err(|_| "expected euros as a number".to_string())?;
    cents(amount).map_err(|error| error.to_string())?;
    Ok(amount)
}

pub fn cents(euros: f64) -> Result<f64> {
    let value = euros * 100.0;
    ensure!(
        euros.is_finite() && euros > 0.0 && value.is_finite(),
        "amount must be a positive finite number of euros"
    );
    Ok(value)
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub name: String,
    balance: Option<f64>,
    weighted_count: Option<f64>,
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|n| n.is_finite())
}

impl User {
    fn parse(value: &Value) -> Result<Self> {
        Ok(Self {
            id: value["id"]
                .as_u64()
                .ok_or_else(|| eyre!("invalid Strichliste user id"))?,
            name: value["name"]
                .as_str()
                .ok_or_else(|| eyre!("invalid Strichliste user name"))?
                .to_owned(),
            balance: number(&value["balance"]),
            weighted_count: number(&value["weightedCountOfPurchases"]),
        })
    }

    fn balance_json(&self) -> Result<Value> {
        let balance = self
            .balance
            .ok_or_else(|| eyre!("Strichliste balance missing or invalid"))?;
        Ok(json!({"balance_eur": balance / 100.0}))
    }
}

pub struct Client {
    http: reqwest::Client,
    base: reqwest::Url,
    options: Options,
}

impl Client {
    pub fn new(options: Options) -> Result<Self> {
        ensure!(
            options.user_id.is_some() ^ options.user.is_some(),
            "configure --strichliste-user or --strichliste-user-id"
        );
        ensure!(
            options
                .user
                .as_ref()
                .is_none_or(|name| !name.trim().is_empty()),
            "Strichliste user name must not be empty"
        );
        let base = options
            .url
            .as_deref()
            .ok_or_else(|| eyre!("configure --strichliste-url"))?;
        let mut base =
            reqwest::Url::parse(base).map_err(|_| eyre!("invalid Strichliste API URL"))?;
        ensure!(
            matches!(base.scheme(), "http" | "https")
                && base.host_str().is_some()
                && base.query().is_none()
                && base.fragment().is_none(),
            "Strichliste API URL must be HTTP(S), without query or fragment"
        );
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let http = reqwest::Client::builder()
            .timeout(options.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()?;
        Ok(Self {
            http,
            base,
            options,
        })
    }

    fn endpoint(&self, path: &str) -> Result<reqwest::Url> {
        self.base
            .join(path)
            .map_err(|_| eyre!("invalid Strichliste endpoint"))
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let response = self
            .http
            .get(self.endpoint(path)?)
            .send()
            .await
            .map_err(|_| eyre!("Strichliste read failed"))?;
        ensure!(response.status().is_success(), "Strichliste read rejected");
        // Bound list responses as well as individual accounts.
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| eyre!("Strichliste read failed"))?
        {
            ensure!(
                bytes.len() + chunk.len() <= 8 * 1024 * 1024,
                "Strichliste response too large"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| eyre!("invalid Strichliste JSON"))
    }

    pub async fn users(&self) -> Result<Vec<User>> {
        let value = self.get("user").await?;
        let users = value
            .as_array()
            .ok_or_else(|| eyre!("expected Strichliste user array"))?;
        let users = users.iter().map(User::parse).collect::<Result<Vec<_>>>()?;
        let mut ids = std::collections::HashSet::new();
        ensure!(
            users.iter().all(|user| ids.insert(user.id)),
            "duplicate Strichliste user ids"
        );
        Ok(users)
    }

    fn select<'a>(&self, users: &'a [User]) -> Result<&'a User> {
        let mut matching = users.iter().filter(|user| {
            if let Some(id) = self.options.user_id {
                user.id == id
            } else {
                user.name.to_lowercase()
                    == self.options.user.as_deref().unwrap_or("").to_lowercase()
            }
        });
        let user = matching
            .next()
            .ok_or_else(|| eyre!("configured Strichliste user not found"))?;
        ensure!(
            matching.next().is_none(),
            "ambiguous Strichliste name; configure a user id"
        );
        Ok(user)
    }

    pub async fn current_user(&self) -> Result<User> {
        if let Some(id) = self.options.user_id {
            let user = User::parse(&self.get(&format!("user/{id}")).await?)?;
            ensure!(user.id == id, "Strichliste returned an unexpected user");
            Ok(user)
        } else {
            Ok(self.select(&self.users().await?)?.clone())
        }
    }

    pub async fn execute(&self, request: &Request, comment: &str) -> Result<Value> {
        // Validate before any IO.
        let value = request.value()?;
        if matches!(request, Request::Rank) {
            let users = self.users().await?;
            let current = self.select(&users)?;
            let score = current
                .weighted_count
                .ok_or_else(|| eyre!("weightedCountOfPurchases missing or invalid"))?;
            let mut rank = 1;
            for user in &users {
                let other = user
                    .weighted_count
                    .ok_or_else(|| eyre!("weightedCountOfPurchases missing or invalid"))?;
                if other > score {
                    rank += 1;
                }
            }
            return Ok(json!({"rank": rank, "weighted_count_of_purchases": score}));
        }
        let user = self.current_user().await?;
        match value {
            Some(value) => {
                self.post(user.id, json!({"value": value, "comment": comment}))
                    .await
            }
            None => user.balance_json(),
        }
    }

    async fn post(&self, user_id: u64, body: Value) -> Result<Value> {
        let response = self
            .http
            .post(self.endpoint(&format!("user/{user_id}/transaction"))?)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&body)?)
            .send()
            .await
            .map_err(|_| {
                eyre!("Strichliste transaction outcome unknown; check your balance before retrying")
            })?;
        ensure!(
            response.status().is_success(),
            "Strichliste transaction was not confirmed; check your balance before retrying"
        );
        // Never expose the raw response: it may include other users or transaction history.
        Ok(json!({"submitted": true, "value_cents": body["value"]}))
    }

    pub async fn transfer(&self, recipient: &User, euros: f64, comment: &str) -> Result<Value> {
        let value = -cents(euros)?;
        let sender = self.current_user().await?;
        ensure!(sender.id != recipient.id, "cannot send to yourself");
        self.post(
            sender.id,
            json!({
                "value": value,
                "comment": format!("to {}: {}", recipient.name, comment),
                "toUserId": recipient.id,
                "toUserComment": format!("from {}: {}", sender.name, comment),
            }),
        )
        .await
    }
}

/// MCP's boundary fixes the identity and audit comment independently of tool arguments.
pub async fn execute_mcp(options: Options, request: &Request, dry_run: bool) -> Result<Value> {
    if dry_run {
        let mut preview = request.preview()?;
        if request.value()?.is_some() {
            preview["comment"] = json!(MCP_COMMENT);
        }
        return Ok(preview);
    }
    Client::new(options)?.execute(request, MCP_COMMENT).await
}

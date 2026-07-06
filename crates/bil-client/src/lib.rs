use bil_axle::{
    CheckResponse, DisproveResponse, ExtractDeclsResponse, ExtractTheoremsResponse,
    Have2LemmaResponse, Have2SorryResponse, MergeResponse, NormalizeResponse, RenameResponse,
    RepairProofsResponse, SimplifyTheoremsResponse, Sorry2LemmaResponse, Theorem2LemmaResponse,
    Theorem2SorryResponse, VerifyProofResponse,
};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, LOCATION};
use reqwest::{Method, StatusCode, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::env;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Semaphore;

pub use bil_axle;

pub const DEFAULT_URL: &str = "https://axle.axiommath.ai";
pub const BASE_TIMEOUT_SECONDS: f64 = 1_800.0;
pub const MAX_CONCURRENCY: usize = 20;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AxleError {
    #[error("Invalid AXLE client configuration: {0}")]
    Config(String),
    #[error("Retryable error at {url}: {details}")]
    IsUnavailable { url: String, details: String },
    #[error(
        "{message}\n\nCongratulations, you found a bug! Please file an issue at https://github.com/AxiomMath/axiom-lean-engine/issues"
    )]
    InternalError { message: String },
    #[error("{message}\n\nPlease check your input and try again.")]
    InvalidArgument { message: String },
    #[error("Error 40-something: {0}")]
    RuntimeError(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    RateLimited(String),
    #[error("{message}")]
    BrowserLoginRequired {
        api_base_url: String,
        message: String,
    },
}

impl AxleError {
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Forbidden(_) => Some(403),
            Self::NotFound(_) => Some(404),
            Self::Conflict(_) => Some(409),
            Self::RateLimited(_) => Some(429),
            Self::BrowserLoginRequired { .. } => Some(302),
            _ => None,
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AxleClient {
    url: String,
    http: reqwest::Client,
    base_timeout_seconds: f64,
    max_concurrency: usize,
    semaphore: Arc<Semaphore>,
}

impl AxleClient {
    pub fn new(
        url: Option<String>,
        max_concurrency: Option<usize>,
        base_timeout_seconds: Option<f64>,
        api_key: Option<String>,
    ) -> Result<Self, AxleError> {
        let url = url
            .or_else(|| env::var("AXLE_API_URL").ok())
            .unwrap_or_else(|| DEFAULT_URL.to_string());
        let url = url.trim_end_matches('/').to_string();

        let max_concurrency = match max_concurrency {
            Some(value) => value,
            None => env_or_default("AXLE_MAX_CONCURRENCY", MAX_CONCURRENCY)?,
        };
        let base_timeout_seconds = match base_timeout_seconds {
            Some(value) => value,
            None => env_or_default("AXLE_TIMEOUT_SECONDS", BASE_TIMEOUT_SECONDS)?,
        };
        let api_key = api_key.or_else(|| env::var("AXLE_API_KEY").ok());

        let mut headers = HeaderMap::new();
        let request_source = env::var("AXLE_REQUEST_SOURCE").unwrap_or_else(|_| "sdk".to_string());
        headers.insert(
            "X-Request-Source",
            HeaderValue::from_str(&request_source)
                .map_err(|err| AxleError::Config(err.to_string()))?,
        );
        if let Some(api_key) = api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .map_err(|err| AxleError::Config(err.to_string()))?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .pool_max_idle_per_host(max_concurrency)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| AxleError::Config(err.to_string()))?;

        Ok(Self {
            url,
            http,
            base_timeout_seconds,
            max_concurrency,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    pub fn base_timeout_seconds(&self) -> f64 {
        self.base_timeout_seconds
    }

    pub async fn check_status(&self, timeout_seconds: f64) -> Result<Value, AxleError> {
        let response_text = self
            .call("v1/status", Some(timeout_seconds), Method::GET, None)
            .await?;
        let status: Value = serde_json::from_str(&response_text)
            .map_err(|err| AxleError::internal(err.to_string()))?;
        if status.get("status").and_then(Value::as_str) != Some("healthy") {
            return Err(AxleError::internal(format!(
                "Server is not healthy: {status}"
            )));
        }
        Ok(status)
    }

    pub async fn run_one(&self, method: &str, request: Value) -> Result<Value, AxleError> {
        let request_timeout_seconds = request.get("timeout_seconds").and_then(Value::as_f64);
        let response_text = self
            .call(
                &format!("api/v1/{method}"),
                request_timeout_seconds,
                Method::POST,
                Some(request),
            )
            .await?;

        let stripped_response = response_text.trim_end();
        let lines: Vec<&str> = if stripped_response.is_empty() {
            Vec::new()
        } else {
            stripped_response.lines().collect()
        };

        if lines.len() != 1 {
            return Err(AxleError::internal(format!(
                "Expected 1 response, got {}",
                lines.len()
            )));
        }

        let response: Value = serde_json::from_str(lines[0])
            .map_err(|err| AxleError::internal(format!("Invalid JSON response: {err}")))?;

        if let Some(message) = response.get("internal_error").and_then(Value::as_str) {
            return Err(AxleError::internal(message));
        }
        if let Some(message) = response.get("user_error").and_then(Value::as_str) {
            return Err(AxleError::invalid_argument(message));
        }
        if let Some(message) = response.get("error").and_then(Value::as_str) {
            return Err(AxleError::RuntimeError(message.to_string()));
        }

        Ok(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn verify_proof(
        &self,
        formal_statement: String,
        content: String,
        environment: String,
        permitted_sorries: Option<Vec<String>>,
        mathlib_options: Option<bool>,
        use_def_eq: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<VerifyProofResponse, AxleError> {
        let response = self
            .run_one(
                "verify_proof",
                object_request(|map| {
                    insert_required(map, "formal_statement", formal_statement)?;
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "permitted_sorries", permitted_sorries)?;
                    insert_optional(map, "mathlib_options", mathlib_options)?;
                    insert_optional(map, "use_def_eq", use_def_eq)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    pub async fn extract_theorems(
        &self,
        content: String,
        environment: String,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<ExtractTheoremsResponse, AxleError> {
        let response = self
            .run_one(
                "extract_theorems",
                basic_content_request(content, environment, ignore_imports, timeout_seconds)?,
            )
            .await?;
        from_value(response)
    }

    pub async fn extract_decls(
        &self,
        content: String,
        environment: String,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<ExtractDeclsResponse, AxleError> {
        let response = self
            .run_one(
                "extract_decls",
                basic_content_request(content, environment, ignore_imports, timeout_seconds)?,
            )
            .await?;
        from_value(response)
    }

    pub async fn merge(
        &self,
        documents: Vec<String>,
        environment: String,
        use_def_eq: Option<bool>,
        include_alts_as_comments: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<MergeResponse, AxleError> {
        let response = self
            .run_one(
                "merge",
                object_request(|map| {
                    insert_required(map, "documents", documents)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "use_def_eq", use_def_eq)?;
                    insert_optional(map, "include_alts_as_comments", include_alts_as_comments)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn theorem2sorry(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<Theorem2SorryResponse, AxleError> {
        let response = self
            .run_one(
                "theorem2sorry",
                theorem_request(
                    content,
                    environment,
                    names,
                    indices,
                    None::<String>,
                    None::<Vec<String>>,
                    theorems_only,
                    ignore_imports,
                    timeout_seconds,
                )?,
            )
            .await?;
        from_value(response)
    }

    pub async fn rename(
        &self,
        content: String,
        declarations: Map<String, Value>,
        environment: String,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<RenameResponse, AxleError> {
        let response = self
            .run_one(
                "rename",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    map.insert("declarations".to_string(), Value::Object(declarations));
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn theorem2lemma(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        target: Option<String>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<Theorem2LemmaResponse, AxleError> {
        let response = self
            .run_one(
                "theorem2lemma",
                theorem_request(
                    content,
                    environment,
                    names,
                    indices,
                    target,
                    None::<Vec<String>>,
                    theorems_only,
                    ignore_imports,
                    timeout_seconds,
                )?,
            )
            .await?;
        from_value(response)
    }

    pub async fn check(
        &self,
        content: String,
        environment: String,
        mathlib_options: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<CheckResponse, AxleError> {
        let response = self
            .run_one(
                "check",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "mathlib_options", mathlib_options)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn simplify_theorems(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        simplifications: Option<Vec<String>>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<SimplifyTheoremsResponse, AxleError> {
        let response = self
            .run_one(
                "simplify_theorems",
                theorem_request(
                    content,
                    environment,
                    names,
                    indices,
                    None::<String>,
                    simplifications,
                    theorems_only,
                    ignore_imports,
                    timeout_seconds,
                )?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn repair_proofs(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        repairs: Option<Vec<String>>,
        terminal_tactics: Option<Vec<String>>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<RepairProofsResponse, AxleError> {
        let response = self
            .run_one(
                "repair_proofs",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "names", names)?;
                    insert_optional(map, "indices", indices)?;
                    insert_optional(map, "repairs", repairs)?;
                    insert_optional(map, "terminal_tactics", terminal_tactics)?;
                    insert_optional(map, "theorems_only", theorems_only)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn have2lemma(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        include_have_body: Option<bool>,
        include_whole_context: Option<bool>,
        reconstruct_callsite: Option<bool>,
        verbosity: Option<u64>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<Have2LemmaResponse, AxleError> {
        let response = self
            .run_one(
                "have2lemma",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "names", names)?;
                    insert_optional(map, "indices", indices)?;
                    insert_optional(map, "include_have_body", include_have_body)?;
                    insert_optional(map, "include_whole_context", include_whole_context)?;
                    insert_optional(map, "reconstruct_callsite", reconstruct_callsite)?;
                    insert_optional(map, "verbosity", verbosity)?;
                    insert_optional(map, "theorems_only", theorems_only)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn have2sorry(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<Have2SorryResponse, AxleError> {
        let response = self
            .run_one(
                "have2sorry",
                theorem_request(
                    content,
                    environment,
                    names,
                    indices,
                    None::<String>,
                    None::<Vec<String>>,
                    theorems_only,
                    ignore_imports,
                    timeout_seconds,
                )?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn sorry2lemma(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        extract_sorries: Option<bool>,
        extract_errors: Option<bool>,
        include_whole_context: Option<bool>,
        reconstruct_callsite: Option<bool>,
        merge_duplicates: Option<bool>,
        theorems_only: Option<bool>,
        verbosity: Option<u64>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<Sorry2LemmaResponse, AxleError> {
        let response = self
            .run_one(
                "sorry2lemma",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "names", names)?;
                    insert_optional(map, "indices", indices)?;
                    insert_optional(map, "extract_sorries", extract_sorries)?;
                    insert_optional(map, "extract_errors", extract_errors)?;
                    insert_optional(map, "include_whole_context", include_whole_context)?;
                    insert_optional(map, "reconstruct_callsite", reconstruct_callsite)?;
                    insert_optional(map, "merge_duplicates", merge_duplicates)?;
                    insert_optional(map, "theorems_only", theorems_only)?;
                    insert_optional(map, "verbosity", verbosity)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn disprove(
        &self,
        content: String,
        environment: String,
        names: Option<Vec<String>>,
        indices: Option<Vec<i64>>,
        terminal_tactics: Option<Vec<String>>,
        theorems_only: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<DisproveResponse, AxleError> {
        let response = self
            .run_one(
                "disprove",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "names", names)?;
                    insert_optional(map, "indices", indices)?;
                    insert_optional(map, "terminal_tactics", terminal_tactics)?;
                    insert_optional(map, "theorems_only", theorems_only)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    pub async fn normalize(
        &self,
        content: String,
        environment: String,
        normalizations: Option<Vec<String>>,
        failsafe: Option<bool>,
        ignore_imports: Option<bool>,
        timeout_seconds: Option<f64>,
    ) -> Result<NormalizeResponse, AxleError> {
        let response = self
            .run_one(
                "normalize",
                object_request(|map| {
                    insert_required(map, "content", content)?;
                    insert_required(map, "environment", environment)?;
                    insert_optional(map, "normalizations", normalizations)?;
                    insert_optional(map, "failsafe", failsafe)?;
                    insert_optional(map, "ignore_imports", ignore_imports)?;
                    insert_optional(map, "timeout_seconds", timeout_seconds)?;
                    Ok(())
                })?,
            )
            .await?;
        from_value(response)
    }

    pub async fn environments(
        &self,
        timeout_seconds: Option<f64>,
    ) -> Result<Vec<Value>, AxleError> {
        let response_text = self
            .call("v1/environments", timeout_seconds, Method::GET, None)
            .await?;
        serde_json::from_str(&response_text)
            .map_err(|err| AxleError::internal(format!("Invalid JSON response: {err}")))
    }

    pub async fn prove_riemann(&self, timeout_seconds: Option<f64>) -> Result<Value, AxleError> {
        let response_text = self
            .call("v1/prove_riemann", timeout_seconds, Method::GET, None)
            .await?;
        serde_json::from_str(&response_text)
            .map_err(|err| AxleError::internal(format!("Invalid JSON response: {err}")))
    }

    async fn call(
        &self,
        method: &str,
        request_timeout_seconds: Option<f64>,
        http_method: Method,
        body: Option<Value>,
    ) -> Result<String, AxleError> {
        let call_timeout_seconds =
            self.base_timeout_seconds + request_timeout_seconds.unwrap_or_default();
        let request_url = format!("{}/{}", self.url, method);
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AxleError::internal("AXLE concurrency semaphore was closed"))?;

        let mut request = self
            .http
            .request(http_method, &request_url)
            .timeout(Duration::from_secs_f64(call_timeout_seconds));
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|err| {
            if err.is_timeout() {
                AxleError::IsUnavailable {
                    url: self.url.clone(),
                    details: format!(
                        "client timeout: server did not respond within {call_timeout_seconds}s"
                    ),
                }
            } else if err.is_connect() || err.is_request() {
                AxleError::IsUnavailable {
                    url: self.url.clone(),
                    details: err.to_string(),
                }
            } else {
                AxleError::internal(err.to_string())
            }
        })?;

        self.raise_for_status(response, &request_url).await
    }

    async fn raise_for_status(
        &self,
        response: reqwest::Response,
        request_url: &str,
    ) -> Result<String, AxleError> {
        if response.status() == StatusCode::OK {
            return response
                .text()
                .await
                .map_err(|err| AxleError::internal(err.to_string()));
        }

        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let error_message = response
            .text()
            .await
            .map_err(|err| AxleError::internal(err.to_string()))?;

        Err(self.raise_for_status_code(status, &error_message, location.as_deref(), request_url))
    }

    fn raise_for_status_code(
        &self,
        status: u16,
        error_message: &str,
        location: Option<&str>,
        request_url: &str,
    ) -> AxleError {
        if let Some(error) = maybe_google_oidc_302_exc(status, location, request_url, &self.url) {
            return error;
        }

        match status {
            429 => AxleError::RateLimited(format!("Rate limited: {error_message}")),
            503 => AxleError::IsUnavailable {
                url: self.url.clone(),
                details: format!("Service unavailable: {error_message}"),
            },
            400 => AxleError::invalid_argument(format!("Bad request: {error_message}")),
            403 => AxleError::Forbidden(format!("Forbidden: {error_message}")),
            404 => AxleError::NotFound(format!("Not found: {error_message}")),
            409 => AxleError::Conflict(format!("Conflict: {error_message}")),
            500 => AxleError::internal(format!("Internal server error: {error_message}")),
            401 | 402 | 405..=499 => {
                AxleError::invalid_argument(format!("Client error {status}: {error_message}"))
            }
            _ => AxleError::internal(format!("Server error {status}: {error_message}")),
        }
    }
}

fn env_or_default<T>(name: &str, default: T) -> Result<T, AxleError>
where
    T: std::str::FromStr + Copy,
    T::Err: Display,
{
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|err| AxleError::Config(format!("invalid {name}: {err}"))),
        Err(_) => Ok(default),
    }
}

fn object_request<F>(builder: F) -> Result<Value, AxleError>
where
    F: FnOnce(&mut Map<String, Value>) -> Result<(), AxleError>,
{
    let mut map = Map::new();
    builder(&mut map)?;
    Ok(Value::Object(map))
}

fn insert_required<T>(map: &mut Map<String, Value>, key: &str, value: T) -> Result<(), AxleError>
where
    T: Serialize,
{
    map.insert(
        key.to_string(),
        serde_json::to_value(value).map_err(|err| AxleError::internal(err.to_string()))?,
    );
    Ok(())
}

fn insert_optional<T>(
    map: &mut Map<String, Value>,
    key: &str,
    value: Option<T>,
) -> Result<(), AxleError>
where
    T: Serialize,
{
    if let Some(value) = value {
        insert_required(map, key, value)?;
    }
    Ok(())
}

fn from_value<T>(value: Value) -> Result<T, AxleError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|err| AxleError::internal(format!("Failed to deserialize response: {err}")))
}

fn basic_content_request(
    content: String,
    environment: String,
    ignore_imports: Option<bool>,
    timeout_seconds: Option<f64>,
) -> Result<Value, AxleError> {
    object_request(|map| {
        insert_required(map, "content", content)?;
        insert_required(map, "environment", environment)?;
        insert_optional(map, "ignore_imports", ignore_imports)?;
        insert_optional(map, "timeout_seconds", timeout_seconds)?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
fn theorem_request(
    content: String,
    environment: String,
    names: Option<Vec<String>>,
    indices: Option<Vec<i64>>,
    target: Option<String>,
    string_list_field: Option<Vec<String>>,
    theorems_only: Option<bool>,
    ignore_imports: Option<bool>,
    timeout_seconds: Option<f64>,
) -> Result<Value, AxleError> {
    object_request(|map| {
        insert_required(map, "content", content)?;
        insert_required(map, "environment", environment)?;
        insert_optional(map, "names", names)?;
        insert_optional(map, "indices", indices)?;
        insert_optional(map, "target", target)?;
        insert_optional(map, "simplifications", string_list_field.clone())?;
        insert_optional(map, "theorems_only", theorems_only)?;
        insert_optional(map, "ignore_imports", ignore_imports)?;
        insert_optional(map, "timeout_seconds", timeout_seconds)?;
        Ok(())
    })
}

fn maybe_google_oidc_302_exc(
    status: u16,
    location: Option<&str>,
    request_url: &str,
    api_base_url: &str,
) -> Option<AxleError> {
    let location = location?.trim();
    if status != 302 || location.is_empty() {
        return None;
    }

    let base = Url::parse(request_url).ok()?;
    let parsed = base.join(location).ok()?;
    let hostname_matches = parsed
        .host_str()
        .map(|host| host.eq_ignore_ascii_case("accounts.google.com"))
        .unwrap_or(false);
    let path_matches = parsed.path().trim_end_matches('/') == "/o/oauth2/v2/auth";

    if parsed.scheme().eq_ignore_ascii_case("https") && hostname_matches && path_matches {
        let message = format!(
            "Endpoint {request_url:?} requires interactive browser sign-in and is not available from the CLI."
        );
        return Some(AxleError::BrowserLoginRequired {
            api_base_url: api_base_url.to_string(),
            message,
        });
    }

    None
}

//! Thin blocking HTTP client for the `formsy.server` API.
//!
//! Mirrors the request/error style of `scripts/e2e_server_compile_query.py::post_json`:
//! sends `Authorization: Bearer <api_key>` when a key is set, and surfaces non-2xx
//! responses as `failed with HTTP {status}: {body}`.

use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::models::{
    CompileRequest, CompileResponse, GetNeighborsRequest, GetNeighborsResponse,
    GetNodeDetailRequest, GetNodeDetailResponse, QueryRequest, QueryResponse,
    SearchNodesRequest, SearchNodesResponse,
};

pub struct FormsyClient {
    base_url: Url,
    http: Client,
}

impl FormsyClient {
    pub fn new(base_url: String, api_key: Option<String>, timeout: Duration) -> Result<Self> {
        let base_url = Url::parse(&base_url)
            .map_err(|e| anyhow!("invalid --base-url {base_url:?}: {e}"))?;

        let mut headers = HeaderMap::new();
        if let Some(key) = &api_key {
            let value = HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|e| anyhow!("invalid api key: {e}"))?;
            headers.insert(AUTHORIZATION, value);
        }

        let http = Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .build()?;

        Ok(Self { base_url, http })
    }

    /// POST a JSON body to `{base_url}{path}` and deserialize the JSON response.
    fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|e| anyhow!("bad endpoint path {path:?}: {e}"))?;

        let resp = self.http.post(url).json(body).send()?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(anyhow!("request to {path} failed with HTTP {status}: {text}"));
        }
        let value: serde_json::Value = resp.json()?;
        serde_json::from_value(value).map_err(|e| {
            anyhow!("could not decode response from {path} as JSON: {e}")
        })
    }

    pub fn compile(&self, request: &CompileRequest) -> Result<CompileResponse> {
        self.post("/api/v1/compile", request)
    }

    pub fn query(&self, request: &QueryRequest) -> Result<QueryResponse> {
        self.post("/api/v1/query", request)
    }

    /// Raw JSON helper: same POST, but returns the untouched `serde_json::Value`.
    /// Used by `--json` output so the CLI prints exactly what the server sent.
    pub fn compile_json(&self, request: &CompileRequest) -> Result<serde_json::Value> {
        self.post("/api/v1/compile", request)
    }

    pub fn query_json(&self, request: &QueryRequest) -> Result<serde_json::Value> {
        self.post("/api/v1/query", request)
    }

    pub fn search_nodes(&self, request: &SearchNodesRequest) -> Result<SearchNodesResponse> {
        self.post("/api/v1/search_nodes", request)
    }

    pub fn search_nodes_json(&self, request: &SearchNodesRequest) -> Result<serde_json::Value> {
        self.post("/api/v1/search_nodes", request)
    }

    pub fn get_neighbors(&self, request: &GetNeighborsRequest) -> Result<GetNeighborsResponse> {
        self.post("/api/v1/get_neighbors", request)
    }

    pub fn get_neighbors_json(&self, request: &GetNeighborsRequest) -> Result<serde_json::Value> {
        self.post("/api/v1/get_neighbors", request)
    }

    pub fn get_node_detail(
        &self,
        request: &GetNodeDetailRequest,
    ) -> Result<GetNodeDetailResponse> {
        self.post("/api/v1/get_node_detail", request)
    }

    pub fn get_node_detail_json(
        &self,
        request: &GetNodeDetailRequest,
    ) -> Result<serde_json::Value> {
        self.post("/api/v1/get_node_detail", request)
    }
}

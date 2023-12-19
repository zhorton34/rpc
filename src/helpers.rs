use reqwest::{Client, Response, ErrorKind, StatusCode};
use serde_json::Value;
use jsonrpsee::{RpcError, RpcResult};
pub async fn fetch_url(url: &str) -> Result<RpcResult, RpcError> {
    let response = reqwest::get(url).await?;

    if response.status() == StatusCode::OK {
        let content = response.text().await?;
        Ok(content)
    } else {
        Err(jsonrpsee::core::Error::Custom(e.to_string())),
    }
}

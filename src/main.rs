use hyper::Method;
use jsonrpsee::server::{RpcModule, Server, IntoResponse};
use jsonrpsee_core::params::{ArrayParams, ObjectParams};
use jsonrpsee::core::EmptyServerParams;
use jsonrpsee::core::{server::*, RpcResult};
use jsonrpsee::types::error::{ErrorCode, ErrorObject, SERVER_ERROR_MSG, INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE, INVALID_PARAMS_MSG, PARSE_ERROR_CODE};
use jsonrpsee::types::{ErrorObjectOwned, Params, Response, ResponsePayload};
use jsonrpsee::SubscriptionMessage;
use jsonrpsee_core::traits::ToRpcParams;
use tower::hedge::Future;

use std::f32::consts::E;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use std::hash::{Hash};
use serde::{Serialize, Deserialize};
use serde_json::{self, Error, Value};
mod helpers;
mod errors;
use helpers::Property;
use errors::PdfExtractError;
use helpers::{extract_pdf_text, parse_southlaw_properties};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");

	// Start up a JSON-RPC server that allows cross origin requests.
	let server_addr = run_server().await?;
    println!("Server running at: {}", server_addr);
	// Print instructions for testing CORS from a browser.
	println!("Run the following snippet in the developer console in any Website.");

	futures::future::pending().await
}


async fn run_server() -> anyhow::Result<SocketAddr> {
	// Add a CORS middleware for handling HTTP requests.
	// This middleware does affect the response, including appropriate
	// headers to satisfy CORS. Because any origins are allowed, the
	// "Access-Control-Allow-Origin: *" header is appended to the response.
	let cors = CorsLayer::new()
		// Allow `POST` when accessing the resource
		.allow_methods([Method::POST])
		// Allow requests from any origin
		.allow_origin(Any)
		.allow_headers([hyper::header::CONTENT_TYPE]);
	let middleware = tower::ServiceBuilder::new().layer(cors);

	// The RPC exposes the access control for filtering and the middleware for
	// modifying requests / responses. These features are independent of one another
	// and can also be used separately.
	// In this example, we use both features.
	let server = Server::builder().set_http_middleware(middleware).build("127.0.0.1:0".parse::<SocketAddr>()?).await?;

	let mut module = RpcModule::new(());


    module.register_method::<RpcResult<u64>, _>("echo_call", |params: jsonrpsee_types::Params<'_>, _| {
        params.one::<u64>().map_err(Into::into)
    }).unwrap();
	
    
    module.register_async_method(
        "get_southlaw_properties",
        |params: Params, _| async move {
            let params: Value = match params.parse() {
                Ok(params) => params,
                Err(_) => {
                    return RpcResult::Err(ErrorObject::owned(
                        INVALID_PARAMS_CODE,
                        "Invalid parameters".to_string(),
                        Some(json!({"reason": "Expected an object with 'url' field"}))
                    ));
                }
            };    
    
            let url = match params.get("url").and_then(Value::as_str) {
                Some(url) => url.to_string(),
                None => {
                    return RpcResult::Err(ErrorObject::owned(
                        INVALID_PARAMS_CODE,
                        "Invalid URL parameter".to_string(),
                        Some(json!({"reason": "'url' field is missing or not a string"}))
                    ));
                }
            };

            println!("Extracting text from PDF at: {}", url);
            
            match extract_pdf_text(&url).await {
                Ok(raw_text) => {
                    match parse_southlaw_properties(raw_text).await {
                        Ok(properties) => {
                            // Return successful result as JSON RPC response
                            RpcResult::Ok(json!({
                                "result": properties,
                                "jsonrpc": "2.0",
                                "id": "2"
                            }))
                        },
                        Err(e) => {
                            // Handle error during properties parsing
                            RpcResult::Err(ErrorObject::owned(
                                INTERNAL_ERROR_CODE,
                                format!("Error parsing SouthLaw properties: {}", e),
                                Some(json!({
                                    "error": "Error parsing SouthLaw properties"
                                }))
                            ))
                        }
                    }
                },
                Err(PdfExtractError::DownloadError(e)) => {
                    // Handle network error
                    RpcResult::Err(ErrorObject::owned(
                        INTERNAL_ERROR_CODE,
                        format!("Network error: {}", e),
                        Some(json!({
                            "error": "Network error"
                        }))
                    ))
                },
                Err(PdfExtractError::ExtractionError(e)) => {
                    // Handle PDF processing error
                    RpcResult::Err(ErrorObject::owned(
                        INTERNAL_ERROR_CODE,
                        format!("PDF processing error: {}", e),
                        Some(json!({
                            "error": "PDF processing error"
                        }))
                    ))
                }
            }
        }).unwrap();
    
    
	let addr = server.local_addr()?;
	let handle = server.start(module);

	// In this example we don't care about doing shutdown so let's it run forever.
	// You may use the `ServerHandle` to shut it down or manage it yourself.
	tokio::spawn(handle.stopped());

	Ok(addr)
}
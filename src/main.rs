// use hyper::Method;
// use jsonrpsee::server::{RpcModule, Server};
// use std::net::SocketAddr;
// use tower_http::cors::{Any, CorsLayer};
// mod helpers;
// use lopdf::Document;
// use std::io::Cursor;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};
use serde_json;
use sha2::{Sha256, Digest};
use std::collections::hash_map::DefaultHasher;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
// 	tracing_subscriber::FmtSubscriber::builder()
// 		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
// 		.try_init()
// 		.expect("setting default subscriber failed");

// 	// Start up a JSON-RPC server that allows cross origin requests.
// 	let server_addr = run_server().await?;

// 	// Print instructions for testing CORS from a browser.
// 	println!("Run the following snippet in the developer console in any Website.");
// 	println!(
// 		r#"
//         fetch("http://{}", {{
//             method: 'POST',
//             mode: 'cors',
//             headers: {{ 'Content-Type': 'application/json' }},
//             body: JSON.stringify({{
//                 jsonrpc: '2.0',
//                 method: 'say_hello',
//                 id: 1
//             }})
//         }}).then(res => {{
//             console.log("Response:", res);
//             return res.text()
//         }}).then(body => {{
//             console.log("Response Body:", body)
//         }});
//     "#,
// 		server_addr
// 	);

// 	futures::future::pending().await
// }

// async fn run_server() -> anyhow::Result<SocketAddr> {
// 	// Add a CORS middleware for handling HTTP requests.
// 	// This middleware does affect the response, including appropriate
// 	// headers to satisfy CORS. Because any origins are allowed, the
// 	// "Access-Control-Allow-Origin: *" header is appended to the response.
// 	let cors = CorsLayer::new()
// 		// Allow `POST` when accessing the resource
// 		.allow_methods([Method::POST])
// 		// Allow requests from any origin
// 		.allow_origin(Any)
// 		.allow_headers([hyper::header::CONTENT_TYPE]);
// 	let middleware = tower::ServiceBuilder::new().layer(cors);

// 	// The RPC exposes the access control for filtering and the middleware for
// 	// modifying requests / responses. These features are independent of one another
// 	// and can also be used separately.
// 	// In this example, we use both features.
// 	let server = Server::builder().set_http_middleware(middleware).build("127.0.0.1:0".parse::<SocketAddr>()?).await?;

// 	let mut module = RpcModule::new(());
// 	module.register_method("say_hello", |_, _| {
// 		println!("say_hello method called!");
// 		"Hello there!!"
// 	})?;

//     module.register_method("extract_pdf", |_, _| {
//         let url = "https://https://www.southlaw.com/report/Sales_Report_MO.pdf";

//         helpers.fetch_url(url);
//     })?;

// 	let addr = server.local_addr()?;
// 	let handle = server.start(module);

// 	// In this example we don't care about doing shutdown so let's it run forever.
// 	// You may use the `ServerHandle` to shut it down or manage it yourself.
// 	tokio::spawn(handle.stopped());

// 	Ok(addr)
// }
use hyper::{Body, Client, Request};
use std::net::SocketAddr;
use std::time::Duration;

use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use jsonrpsee::server::middleware::http::ProxyGetRequestLayer;
use jsonrpsee::server::{RpcModule, Server};
use lopdf::Document;
use std::io::Cursor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");

	let addr = run_server().await?;
	let url = format!("http://{}", addr);

	// Use RPC client to get the response of `say_hello` method.
	let client = HttpClientBuilder::default().build(&url)?;
	let response: String = client.request("say_hello", rpc_params![]).await?;
	println!("[main]: response: {:?}", response);

	// Use hyper client to manually submit a `GET /health` request.
	let http_client = Client::new();
	let uri = format!("http://{}/health", addr);

    // Use RPC client to get the response of`'extract_pdf`` method
    let pdf_text: String = client.request("get_southlaw_properties", rpc_params![]).await?;
    println!("[main]: get_southlaw_properties: {:?}", pdf_text);


	let req = Request::builder().method("GET").uri(&uri).body(Body::empty())?;
	println!("[main]: Submit proxy request: {:?}", req);
	let res = http_client.request(req).await?;
	println!("[main]: Received proxy response: {:?}", res);

	// Interpret the response as String.
	let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
	let out = String::from_utf8(bytes.to_vec()).unwrap();
	println!("[main]: Interpret proxy response: {:?}", out);
	assert_eq!(out.as_str(), "{\"health\":true}");

	Ok(())
}

async fn run_server() -> anyhow::Result<SocketAddr> {
	// Custom tower service to handle the RPC requests
	let service_builder = tower::ServiceBuilder::new()
		// Proxy `GET /health` requests to internal `system_health` method.
		.layer(ProxyGetRequestLayer::new("/health", "system_health")?)
		.timeout(Duration::from_secs(2));

	let server =
		Server::builder().set_http_middleware(service_builder).build("127.0.0.1:0".parse::<SocketAddr>()?).await?;

	let addr = server.local_addr()?;

	let mut module = RpcModule::new(());
	module.register_method("say_hello", |_, _| "lo").unwrap();
	module.register_method("system_health", |_, _| serde_json::json!({ "health": true })).unwrap();
    
    module.register_async_method(
        "get_southlaw_properties",
        |_, _| async move {
            let url = "https://www.southlaw.com/report/Sales_Report_MO.pdf";
            let client = reqwest::Client::new();
            let res = client.get(url).send().await.unwrap();
            let bytes = res.bytes().await.unwrap();
            let mut cursor = Cursor::new(bytes);
            let doc = Document::load_from(&mut cursor).unwrap();
            
            let total_pages = doc.get_pages().len() as u32;
            let page_numbers: Vec<u32> = (1..=total_pages).collect();

            let text = doc.extract_text(&page_numbers).unwrap();
            
            let lines = text.split("\n").into_iter()
            .filter(|line| !line.contains("Foreclosure Sales")
                && !line.contains("Information Reported as of:")
                && !line.contains("Property Address")
                && !line.contains("Property City")
                && !line.contains("Sale Date")
                && !line.contains("Sale Time")
                && !line.contains("Continued Date/Time")
                && !line.contains("Opening Bid")
                && !line.contains("Sale Location(City)")
                && !line.contains("Civil Case No.")
                && !line.contains("Firm File#")
                && !line.contains("Property Zip")
                && !line.contains("13160 Foster, Ste. 100")
                && !line.eq(&"1")
                && !line.eq(&"2")
                && !line.eq(&"3")
                && !line.eq(&"4")
                && !line.eq(&"5")
                && !line.eq(&"6")
                && !line.eq(&"7")
                && !line.eq(&"8")
                && !line.eq(&"9")
            )
            .map(|line| line.trim())
            .collect::<Vec<&str>>();

            
            let data = lines.to_owned().into_iter().map(|line| line.to_owned()).collect::<Vec<String>>().join("|");
            fn is_street_address(segment: &str) -> bool {
                let parts: Vec<&str> = segment.split_whitespace().collect();
            
                // Check if the first part is a number and there are additional parts for the street name
                parts.first().map_or(false, |first_part| first_part.chars().all(char::is_numeric)) &&
                parts.len() > 1
            }

            let mut entries = data.split('|').collect::<Vec<_>>().to_vec();
            entries.pop();
            let mut idx: usize = 0;
            let mut county: String = "".to_string();

            let mut properties: Vec<Property> = Vec::new();

            #[derive(Debug, Clone, Hash, Serialize, Deserialize)]
            struct Property {
                pub id: String,
                pub state: String,
                pub county: String,
                pub street: String,
                pub city: String,
                pub zip: String,
                pub sale_date: String,
                pub sale_time: String,
                pub continued_date_time: String,
                pub opening_bid: String,
                pub sale_location_city: String,
                pub firm_file_number: String,
            }
            
            impl Property {
                // Constructor to create a new empty Property
                pub fn new() -> Self {
                    Property {
                        id: String::new(),
                        county: String::new(),
                        street: String::new(),
                        city: String::new(),
                        state: String::new(),
                        zip: String::new(),
                        sale_date: String::new(),
                        sale_time: String::new(),
                        continued_date_time: String::new(),
                        opening_bid: String::new(),
                        sale_location_city: String::new(),
                        firm_file_number: String::new(),
                    }
                }

                // Setters
                pub fn set(&mut self, idx: String, value: String) {
                    if idx == "0" {
                        self.county = value;
                    } else if idx == "1" {
                        self.street = value;
                    } else if idx == "2" {
                        self.city = value;
                    } else if idx == "3" {
                        self.zip = value;
                    } else if idx == "4" {
                        self.sale_date = value;
                    } else if idx == "5" {
                        self.sale_time = value;
                    } else if idx == "6" {
                        self.continued_date_time = value;
                    } else if idx == "7" {
                        self.opening_bid = value;
                    } else if idx == "8" {
                        self.sale_location_city = value;
                    } else if idx == "9" {
                        self.firm_file_number = value;
                    } else if idx == "10" {
                        self.state = value;
                    } else if idx == "11" {
                        self.id = value;
                    }
                }
            }

            for value in entries.iter() {
                if Some(value).is_none() {
                    continue;
                }
                
                if idx == 0 {
                    properties.push(Property::new());
                    properties.last_mut().unwrap().set("10".to_string(), "MO".to_string());
                    if is_street_address(value) {
                        // Skip the county and move to the next element
                        println!("{:?}: {:?}", idx, county);
                        properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
                        
                        idx += 1; // Fast forward by one increment
                        println!("{:?}: {:?}", idx, value);
                        properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
                    } else {
                        // This is a county
                        county = value.to_string();
                        println!("{:?}: {:?}", idx, county);
                        properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
                    }
                } else {
                    println!("{:?}: {:?}", idx, value);
                    properties.last_mut().unwrap().set(idx.to_string(), value.to_string());
                }
        
                // Increment idx and reset if it reaches 10
                idx = (idx + 1) % 10;
            }

            let properties_with_id = properties.to_owned().into_iter().map(|property| {
                let concatenated = format!("{}{}{}{}{}{}{}{}{}{}{}", 
                property.county, property.street, property.city, property.state, property.zip, 
                property.sale_date, property.sale_time, property.continued_date_time, 
                property.opening_bid, property.sale_location_city, property.firm_file_number);
    
                let mut hasher = Sha256::new();
                hasher.update(concatenated);
                let result = hasher.finalize();

                let mut new_property = Property::new();
                new_property.set("0".to_string(), property.county);
                new_property.set("1".to_string(), property.street);
                new_property.set("2".to_string(), property.city);
                new_property.set("3".to_string(), property.zip);
                new_property.set("4".to_string(), property.sale_date);
                new_property.set("5".to_string(), property.sale_time);
                new_property.set("6".to_string(), property.continued_date_time);
                new_property.set("7".to_string(), property.opening_bid);
                new_property.set("8".to_string(), property.sale_location_city);
                new_property.set("9".to_string(), property.firm_file_number);
                new_property.set("10".to_string(), property.state);
                new_property.set("11".to_string(), format!("{:x}", result));
                
                new_property
            }).collect::<Vec<Property>>();
            
            serde_json::json!(serde_json::to_string(&properties_with_id).unwrap())
        },
    ).unwrap();


    
    let handle = server.start(module);

	// In this example we don't care about doing shutdown so let's it run forever.
	// You may use the `ServerHandle` to shut it down or manage it yourself.
	tokio::spawn(handle.stopped());

	Ok(addr)
}
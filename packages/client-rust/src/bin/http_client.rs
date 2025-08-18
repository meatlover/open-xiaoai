use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use open_xiaoai::services::connect::data::{Event, Request, Response};

pub struct HttpClient {
    client: Client,
    base_url: String,
    headers: HashMap<String, String>,
    client_id: String,
}

impl HttpClient {
    pub fn new(base_url: &str) -> Self {
        let mut headers = HashMap::new();
        
        // Add Cloudflare Access Service Token headers if available
        if let Ok(client_id) = std::env::var("CF_ACCESS_CLIENT_ID") {
            headers.insert("CF-Access-Client-Id".to_string(), client_id);
        }
        if let Ok(client_secret) = std::env::var("CF_ACCESS_CLIENT_SECRET") {
            headers.insert("CF-Access-Client-Secret".to_string(), client_secret);
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = if base_url.ends_with('/') {
            base_url.trim_end_matches('/').to_string()
        } else {
            base_url.to_string()
        };

        Self {
            client,
            base_url,
            headers,
            client_id: Uuid::new_v4().to_string(),
        }
    }

    async fn send_request(&self, method: &str, endpoint: &str, body: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err("Unsupported HTTP method".into()),
        };

        // Add headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Add body if provided
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let json: Value = response.json().await?;
        Ok(json)
    }

    pub async fn register(&self) -> Result<(), Box<dyn std::error::Error>> {
        let body = serde_json::json!({
            "clientId": self.client_id,
            "action": "register"
        });

        let _response = self.send_request("POST", "/register", Some(body)).await?;
        println!("✅ Client registered: {}", self.client_id);
        Ok(())
    }

    pub async fn send_event(&self, event: &Event) -> Result<(), Box<dyn std::error::Error>> {
        let body = serde_json::to_value(event)?;
        let _response = self.send_request("POST", "/events", Some(body)).await?;
        println!("📤 Event sent: {}", event.name);
        Ok(())
    }

    pub async fn poll_commands(&self) -> Result<Vec<Response>, Box<dyn std::error::Error>> {
        let response = self.send_request("GET", &format!("/commands/{}", self.client_id), None).await?;
        
        if let Some(commands) = response.get("commands").and_then(|c| c.as_array()) {
            let mut responses = Vec::new();
            for command in commands {
                if let Ok(response) = serde_json::from_value::<Response>(command.clone()) {
                    responses.push(response);
                }
            }
            if !responses.is_empty() {
                println!("📥 Received {} commands", responses.len());
            }
            Ok(responses)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn send_rpc(&self, request: &Request) -> Result<Option<Response>, Box<dyn std::error::Error>> {
        let body = serde_json::to_value(request)?;
        let response = self.send_request("POST", "/rpc", Some(body)).await?;
        
        if let Ok(response) = serde_json::from_value::<Response>(response) {
            println!("🔄 RPC response: {}", response.id);
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    pub async fn run_main_loop(&self) {
        if let Err(e) = self.register().await {
            eprintln!("❌ Failed to register: {}", e);
            return;
        }

        println!("🔄 Starting main loop...");
        
        loop {
            // Poll for commands every 5 seconds
            match self.poll_commands().await {
                Ok(commands) => {
                    for command in commands {
                        println!("📋 Command received: {:?}", command);
                        // Process command here
                        // You can add specific command handling logic
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to poll commands: {}", e);
                }
            }

            // Send a heartbeat event
            let heartbeat = Event::new("heartbeat", serde_json::json!({
                "timestamp": chrono::Utc::now().timestamp(),
                "clientId": self.client_id
            }));

            if let Err(e) = self.send_event(&heartbeat).await {
                eprintln!("❌ Failed to send heartbeat: {}", e);
            }

            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let url = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("SERVER_URL").ok())
        .unwrap_or_else(|| "http://localhost:4399".to_string());

    println!("🚀 HTTP Client starting...");
    println!("🔗 Server URL: {}", url);

    let client = HttpClient::new(&url);
    client.run_main_loop().await;
}

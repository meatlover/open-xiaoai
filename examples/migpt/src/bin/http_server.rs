use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use warp::{http::StatusCode, Filter, Reply};

use open_xiaoai::services::connect::data::{Event, Request, Response};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct LlmService {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LlmService {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    pub async fn call_llm(&self, instruction: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let messages = vec![
            json!({
                "role": "system",
                "content": "你是一个智能助手，请根据用户的问题给出回答。"
            }),
            json!({
                "role": "user", 
                "content": instruction
            })
        ];

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 1000
        });

        println!("🤖 Sending LLM request: {}", instruction);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("LLM API error {}: {}", status, text).into());
        }

        let json: Value = response.json().await?;
        
        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            println!("🎯 LLM response: {}", content);
            Ok(content.to_string())
        } else {
            Err("No content in LLM response".into())
        }
    }
}

pub fn extract_instruction_text(text: &str) -> Option<String> {
    // Try to extract instruction from various formats
    if let Some(caps) = Regex::new(r#""instruction":\s*"([^"]+)""#).unwrap().captures(text) {
        return Some(caps[1].to_string());
    }
    
    if let Some(caps) = Regex::new(r#"instruction[："]\s*([^，。\n]+)"#).unwrap().captures(text) {
        return Some(caps[1].to_string());
    }
    
    // If it's already a plain instruction, return as-is
    if !text.contains("{") && !text.contains(":") && text.len() > 2 {
        return Some(text.to_string());
    }
    
    None
}

fn read_config() -> LlmConfig {
    // Try multiple config file locations
    let config_paths = [
        "config.json",
        "config.ts", 
        "../../examples/migpt/config.ts"
    ];
    
    for config_path in &config_paths {
        match fs::read_to_string(config_path) {
            Ok(content) => {
                println!("📄 Reading config from: {}", config_path);
                
                // Try parsing as JSON first
                if config_path.ends_with(".json") {
                    if let Ok(json_value) = serde_json::from_str::<Value>(&content) {
                        let base_url = json_value["openai"]["baseURL"].as_str()
                            .unwrap_or("https://api.openai.com/v1").to_string();
                        let api_key = json_value["openai"]["apiKey"].as_str()
                            .unwrap_or("").to_string();
                        let model = json_value["openai"]["model"].as_str()
                            .unwrap_or("gpt-4o-mini").to_string();
                        
                        println!("⚙️  LLM Config - URL: {}, Model: {}, Key: {}...", base_url, model, &api_key.chars().take(10).collect::<String>());
                        
                        return LlmConfig {
                            base_url,
                            api_key,
                            model,
                        };
                    }
                }
                
                // Fall back to regex parsing for TypeScript files
                let base_url = if let Some(caps) = Regex::new(r#"baseURL[:\s]*"([^"]+)""#).unwrap().captures(&content) {
                    caps[1].to_string()
                } else {
                    "https://api.openai.com/v1".to_string()
                };
                
                let api_key = if let Some(caps) = Regex::new(r#"apiKey[:\s]*"([^"]+)""#).unwrap().captures(&content) {
                    caps[1].to_string()
                } else {
                    "".to_string()
                };
                
                let model = if let Some(caps) = Regex::new(r#"model[:\s]*"([^"]+)""#).unwrap().captures(&content) {
                    caps[1].to_string()
                } else {
                    "gpt-4o-mini".to_string()
                };
                
                println!("⚙️  LLM Config - URL: {}, Model: {}, Key: {}...", base_url, model, &api_key.chars().take(10).collect::<String>());
                
                return LlmConfig {
                    base_url,
                    api_key,
                    model,
                };
            }
            Err(_) => continue,
        }
    }
    
    println!("⚠️  No config file found, using defaults");
    LlmConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: "".to_string(),
        model: "gpt-4o-mini".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub events: Arc<Mutex<Vec<Event>>>,
    pub commands: Arc<Mutex<HashMap<String, Vec<Response>>>>,
    pub llm_service: LlmService,
}

impl ServerState {
    pub fn new() -> Self {
        let config = read_config();
        let llm_service = LlmService::new(config);
        
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            commands: Arc::new(Mutex::new(HashMap::new())),
            llm_service,
        }
    }
}

async fn handle_register(
    body: Value,
    state: Arc<ServerState>,
) -> Result<impl Reply, Infallible> {
    if let Some(client_id) = body.get("clientId").and_then(|v| v.as_str()) {
        println!("📝 Client registered: {}", client_id);
        let mut commands = state.commands.lock().unwrap();
        commands.insert(client_id.to_string(), Vec::new());
        Ok(warp::reply::with_status(
            warp::reply::json(&json!({"status": "registered", "clientId": client_id})),
            StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Missing clientId"})),
            StatusCode::BAD_REQUEST,
        ))
    }
}

async fn handle_events(
    event: Event,
    state: Arc<ServerState>,
) -> Result<impl Reply, Infallible> {
    println!("📨 Event received: {} - {}", event.name, event.id);
    
    // Store the event
    {
        let mut events = state.events.lock().unwrap();
        events.push(event.clone());
    }
    
    // Process text instruction events
    if event.name == "instruction" {
        if let Some(text) = event.data.get("text").and_then(|v| v.as_str()) {
            if let Some(instruction) = extract_instruction_text(text) {
                println!("🎯 Processing instruction: {}", instruction);
                
                // Call LLM service
                match state.llm_service.call_llm(&instruction).await {
                    Ok(response_text) => {
                        println!("✅ LLM response: {}", response_text);
                        
                        // Create TTS command
                        let tts_command = Response {
                            id: Uuid::new_v4().to_string(),
                            data: json!({
                                "action": "tts",
                                "text": response_text,
                                "instruction": instruction
                            }),
                        };
                        
                        // Store command for client polling
                        if let Some(client_id) = event.data.get("clientId").and_then(|v| v.as_str()) {
                            let mut commands = state.commands.lock().unwrap();
                            commands.entry(client_id.to_string())
                                .or_insert_with(Vec::new)
                                .push(tts_command.clone());
                            println!("🔊 TTS Command stored for client {}: {}", client_id, tts_command.id);
                        } else {
                            println!("🔊 TTS Command created (no client_id): {}", tts_command.id);
                        }
                    }
                    Err(e) => {
                        println!("❌ LLM call failed: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({"status": "received", "eventId": event.id})),
        StatusCode::OK,
    ))
}

async fn handle_get_commands(
    client_id: String,
    state: Arc<ServerState>,
) -> Result<impl Reply, Infallible> {
    let commands = {
        let commands_map = state.commands.lock().unwrap();
        commands_map.get(&client_id).cloned().unwrap_or_default()
    };
    
    if !commands.is_empty() {
        println!("📋 Sending {} commands to client: {}", commands.len(), client_id);
        // Clear commands after sending
        let mut commands_map = state.commands.lock().unwrap();
        commands_map.insert(client_id, Vec::new());
    }
    
    Ok(warp::reply::json(&json!({"commands": commands})))
}

async fn handle_rpc(
    request: Request,
    _state: Arc<ServerState>,
) -> Result<impl Reply, Infallible> {
    println!("🔧 RPC request: {} - {}", request.method, request.id);
    
    let response = Response {
        id: request.id,
        data: json!({
            "status": "processed",
            "method": request.method,
            "result": "OK"
        }),
    };
    
    Ok(warp::reply::json(&response))
}

async fn handle_test() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::json(&json!({
        "status": "ok",
        "message": "HTTP Server is running",
        "timestamp": chrono::Utc::now().timestamp()
    })))
}

pub fn with_state(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (Arc<ServerState>,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

#[tokio::main]
async fn main() {
    println!("🚀 HTTP Server starting...");
    
    let state = Arc::new(ServerState::new());
    
    // Test endpoint
    let test = warp::path("test")
        .and(warp::get())
        .and_then(handle_test);
    
    // Register endpoint
    let register = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_register);
    
    // Events endpoint
    let events = warp::path("events")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_events);
    
    // Commands endpoint (GET /commands/{client_id})
    let commands = warp::path!("commands" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_get_commands);
    
    // RPC endpoint
    let rpc = warp::path("rpc")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_rpc);
    
    // CORS headers
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization", "cf-access-client-id", "cf-access-client-secret"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);
    
    let routes = test
        .or(register)
        .or(events)
        .or(commands)
        .or(rpc)
        .with(cors);
    
    println!("🌐 Server listening on http://0.0.0.0:4399");
    println!("🔗 Test endpoint: http://localhost:4399/test");
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 4399))
        .await;
}

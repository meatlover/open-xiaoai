use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use open_xiaoai::services::connect::data::{Event, Response};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    mode: String, // "direct" or "proxy"
    openai: OpenAIConfig,
    #[serde(rename = "serverProxy")]
    server_proxy: Option<ServerProxyConfig>,
    prompt: PromptConfig,
    audio: Option<AudioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIConfig {
    #[serde(rename = "baseURL")]
    base_url: String,
    #[serde(rename = "apiKey")]
    api_key: String,
    model: String,
    timeout: Option<u64>,
    #[serde(rename = "maxTokens")]
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerProxyConfig {
    #[serde(rename = "baseURL")]
    base_url: String,
    timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptConfig {
    system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AudioConfig {
    #[serde(rename = "sampleRate")]
    sample_rate: u32,
    channels: u32,
    format: String,
}

pub enum LLMService {
    Direct(DirectLLMService),
    Server(ServerProxyService),
}

impl LLMService {
    async fn call_llm(&self, instruction: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            LLMService::Direct(service) => service.call_llm(instruction).await,
            LLMService::Server(service) => service.call_llm(instruction).await,
        }
    }
}

pub struct DirectLLMService {
    config: OpenAIConfig,
    client: Client,
    system_prompt: String,
}

impl DirectLLMService {
    pub fn new(config: OpenAIConfig, system_prompt: String) -> Self {
        let timeout = Duration::from_secs(config.timeout.unwrap_or(30));
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            system_prompt,
        }
    }

    async fn call_llm(&self, instruction: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let messages = vec![
            json!({
                "role": "system",
                "content": self.system_prompt
            }),
            json!({
                "role": "user", 
                "content": instruction
            })
        ];

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature.unwrap_or(0.7),
            "max_tokens": self.config.max_tokens.unwrap_or(1000)
        });

        println!("🤖 [DIRECT] Calling LLM: {}", instruction);
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("LLM API error: {}", error_text).into());
        }

        let response_json: Value = response.json().await?;
        
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(message) = first_choice.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        println!("✅ [DIRECT] LLM response: {}", content);
                        return Ok(content.to_string());
                    }
                }
            }
        }

        Err("Invalid LLM response format".into())
    }
}

pub struct ServerProxyService {
    config: ServerProxyConfig,
    client: Client,
    client_id: String,
    headers: HashMap<String, String>,
}

impl ServerProxyService {
    pub fn new(config: ServerProxyConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout.unwrap_or(30));
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        let mut headers = HashMap::new();
        
        // Add Cloudflare Access Service Token headers if available
        if let Ok(client_id) = std::env::var("CF_ACCESS_CLIENT_ID") {
            headers.insert("CF-Access-Client-Id".to_string(), client_id);
        }
        if let Ok(client_secret) = std::env::var("CF_ACCESS_CLIENT_SECRET") {
            headers.insert("CF-Access-Client-Secret".to_string(), client_secret);
        }

        Self {
            config,
            client,
            client_id: Uuid::new_v4().to_string(),
            headers,
        }
    }

    async fn send_request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}{}", self.config.base_url, path);
        
        let mut request = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            _ => return Err("Unsupported HTTP method".into()),
        };

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        if let Some(body) = body {
            request = request.header("Content-Type", "application/json").json(&body);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {} error: {}", status, error_text).into());
        }

        let result: Value = response.json().await?;
        Ok(result)
    }

    pub async fn register(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!({
            "clientId": self.client_id
        });

        let _response = self.send_request("POST", "/register", Some(body)).await?;
        println!("✅ [PROXY] Client registered: {}", self.client_id);
        Ok(())
    }

    pub async fn send_event(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = json!(event);
        let _response = self.send_request("POST", "/events", Some(body)).await?;
        Ok(())
    }

    pub async fn poll_commands(&self) -> Result<Vec<Response>, Box<dyn std::error::Error + Send + Sync>> {
        let path = format!("/commands/{}", self.client_id);
        let response = self.send_request("GET", &path, None).await?;
        
        if let Some(commands) = response.get("commands") {
            let commands: Vec<Response> = serde_json::from_value(commands.clone())?;
            if !commands.is_empty() {
                println!("📨 [PROXY] Received {} commands", commands.len());
            }
            Ok(commands)
        } else {
            Ok(vec![])
        }
    }

    async fn call_llm(&self, instruction: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("🌐 [PROXY] Sending instruction to server: {}", instruction);
        
        // Send instruction event to server
        let event = Event::new("instruction", json!({
            "text": instruction,
            "clientId": self.client_id
        }));
        
        self.send_event(&event).await?;
        println!("✅ [PROXY] Instruction sent, waiting for response...");
        
        // Poll for response (try for up to 30 seconds)
        for attempt in 1..=30 {
            sleep(Duration::from_secs(1)).await;
            
            match self.poll_commands().await {
                Ok(commands) => {
                    for command in commands {
                        if let Some(action) = command.data.get("action") {
                            if action == "tts" {
                                if let Some(text) = command.data.get("text").and_then(|v| v.as_str()) {
                                    println!("🎯 [PROXY] Received response: {}", text);
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  [PROXY] Poll attempt {} failed: {}", attempt, e);
                }
            }
        }
        
        Err("Timeout waiting for server response".into())
    }

    pub async fn run_proxy_mode(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Register with server
        if let Err(e) = self.register().await {
            eprintln!("❌ [PROXY] Failed to register: {}", e);
            return Err(e);
        }

        println!("🔄 [PROXY] Starting main loop...");
        
        loop {
            // Poll for commands every 5 seconds
            match self.poll_commands().await {
                Ok(commands) => {
                    for command in commands {
                        println!("📋 [PROXY] Processing command: {:?}", command.data);
                        // Process commands here if needed
                    }
                }
                Err(e) => {
                    eprintln!("❌ [PROXY] Failed to poll commands: {}", e);
                }
            }

            // Send a heartbeat event
            let heartbeat = Event::new("heartbeat", json!({
                "timestamp": chrono::Utc::now().timestamp(),
                "clientId": self.client_id
            }));

            if let Err(e) = self.send_event(&heartbeat).await {
                eprintln!("❌ [PROXY] Failed to send heartbeat: {}", e);
            }

            sleep(Duration::from_secs(5)).await;
        }
    }
}

pub struct MultiModeClient {
    llm_service: LLMService,
    config: Config,
}

impl MultiModeClient {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_content = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;

        let llm_service = match config.mode.as_str() {
            "direct" => {
                LLMService::Direct(DirectLLMService::new(
                    config.openai.clone(), 
                    config.prompt.system.clone()
                ))
            }
            "proxy" => {
                let server_config = config.server_proxy.as_ref()
                    .ok_or("Server proxy config missing for proxy mode")?;
                LLMService::Server(ServerProxyService::new(server_config.clone()))
            }
            _ => return Err(format!("Unknown mode: {}. Valid options: 'direct', 'proxy'", config.mode).into()),
        };

        Ok(Self {
            llm_service,
            config,
        })
    }

    pub async fn process_instruction(&self, text: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Extract instruction from text (simplified)
        let instruction = if text.starts_with("请") || text.starts_with("你") {
            text
        } else {
            return Ok("Not a valid instruction".to_string());
        };

        self.llm_service.call_llm(instruction).await
    }

    pub async fn run_test_loop(&self) {
        println!("🚀 Multi-Mode Client starting in {} mode", self.config.mode);

        let test_instruction = "你好，请介绍一下自己";
        println!("\n📝 Testing: {}", test_instruction);
        
        match self.process_instruction(test_instruction).await {
            Ok(response) => {
                println!("📄 Response: {}", response);
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
            }
        }
    }

    pub async fn run_production_mode(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &self.llm_service {
            LLMService::Server(proxy_service) => {
                println!("🚀 Starting proxy mode production client...");
                proxy_service.run_proxy_mode().await?;
            }
            LLMService::Direct(_) => {
                println!("🚀 Starting direct mode production client...");
                // For direct mode, we could implement audio processing loop here
                // For now, just run the test
                self.run_test_loop().await;
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    
    let config_path = args.get(1)
        .cloned()
        .unwrap_or_else(|| "config.json".to_string());

    let test_mode = args.get(2).map(|s| s == "--test").unwrap_or(false);

    println!("📋 Loading config from: {}", config_path);

    let client = MultiModeClient::new(&config_path)?;
    
    if test_mode {
        println!("🧪 Running in test mode");
        client.run_test_loop().await;
    } else {
        println!("🏭 Running in production mode");
        client.run_production_mode().await?;
    }

    Ok(())
}

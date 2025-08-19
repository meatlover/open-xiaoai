use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

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
}

impl ServerProxyService {
    pub fn new(config: ServerProxyConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout.unwrap_or(30));
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            client_id: Uuid::new_v4().to_string(),
        }
    }

    async fn send_event(&self, text: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/events", self.config.base_url);
        
        let payload = json!({
            "id": format!("instruction-{}", Uuid::new_v4()),
            "name": "instruction",
            "data": {
                "text": text
            }
        });

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Server proxy error: {}", error_text).into());
        }

        let result: Value = response.json().await?;
        Ok(result)
    }

    async fn call_llm(&self, instruction: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        println!("🌐 [PROXY] Sending to server: {}", instruction);
        
        let _response = self.send_event(instruction).await?;
        
        // For proxy mode, we'd need to implement polling or WebSocket to get the response
        // For this demo, we'll simulate the server processing
        println!("✅ [PROXY] Event sent to server for processing");
        
        // In a real implementation, you'd poll the server for the response
        Ok("Server response would be retrieved via polling or streaming".to_string())
    }
}

pub struct MultiModeClient {
    llm_service: LLMService,
    config: Config,
}

impl MultiModeClient {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());

    println!("📋 Loading config from: {}", config_path);

    let client = MultiModeClient::new(&config_path)?;
    client.run_test_loop().await;

    Ok(())
}

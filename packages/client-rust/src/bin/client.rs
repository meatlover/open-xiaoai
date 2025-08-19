use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::Mutex;
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

#[derive(Clone)]
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
        self.run_production_mode_with_debug(false).await
    }
    
    pub async fn run_production_mode_with_debug(&self, debug: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &self.llm_service {
            LLMService::Server(proxy_service) => {
                println!("🚀 Starting proxy mode production client...");
                if debug {
                    println!("🐛 Debug: Proxy service configuration loaded");
                }
                proxy_service.run_proxy_mode().await?;
            }
            LLMService::Direct(direct_service) => {
                println!("🚀 Starting direct mode production client...");
                if debug {
                    println!("🐛 Debug: Direct LLM service configuration loaded");
                }
                self.run_direct_mode_production_with_debug(direct_service, debug).await?;
            }
        }
        Ok(())
    }

    async fn run_direct_mode_production(&self, direct_service: &DirectLLMService) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.run_direct_mode_production_with_debug(direct_service, false).await
    }

    async fn run_direct_mode_production_with_debug(&self, direct_service: &DirectLLMService, debug: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use open_xiaoai::services::monitor::instruction::{InstructionMonitor, LogMessage, Payload};
        use open_xiaoai::services::monitor::kws::{KwsMonitor, KwsMonitorEvent};
        use open_xiaoai::services::monitor::file::FileMonitorEvent;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        println!("🎤 Direct mode: Integrating with XiaoAi device audio system");
        
        if debug {
            println!("🐛 Debug: Starting XiaoAi device integration with detailed logging");
        }
        
        // Create wake word log file if it doesn't exist
        let kws_dir = "/tmp/open-xiaoai";
        let kws_file = "/tmp/open-xiaoai/kws.log";
        
        if let Err(_) = std::fs::metadata(kws_dir) {
            std::fs::create_dir_all(kws_dir)?;
            println!("📁 Created wake word directory: {}", kws_dir);
        }
        
        if let Err(_) = std::fs::metadata(kws_file) {
            std::fs::write(kws_file, "")?;
            println!("📄 Created wake word log file: {}", kws_file);
        }
        
        println!("👂 Monitoring wake words at: {}", kws_file);
        println!("📢 Monitoring voice instructions at: /tmp/mico_aivs_lab/instruction.log");
        
        if debug {
            println!("🐛 Debug: File monitoring setup complete");
            println!("🐛 Debug: Setting up wake word detection state");
        }
        
        // Shared state to track if wake word was detected
        let wake_detected = Arc::new(AtomicBool::new(false));
        let wake_detected_clone = Arc::clone(&wake_detected);
        
        // Start monitoring wake words and instructions in parallel
        let direct_service_clone = Arc::new(direct_service.clone());
        let wake_detected_for_instruction = Arc::clone(&wake_detected);
        
        // Spawn wake word monitoring in background
        let wake_task = {
            let wake_detected = Arc::clone(&wake_detected_clone);
            let debug_flag = debug;
            tokio::spawn(async move {
                if debug_flag {
                    println!("🐛 Debug: Wake word monitoring task starting");
                }
                
                // Make wake word monitoring more resilient
                loop {
                    if debug_flag {
                        println!("🐛 Debug: Starting wake word monitor");
                    }
                    
                    let wake_detected = Arc::clone(&wake_detected);
                    let debug_flag = debug_flag;
                    
                    KwsMonitor::start(move |event| {
                        let wake_detected = Arc::clone(&wake_detected);
                        let debug_flag = debug_flag;
                        async move {
                            if debug_flag {
                                println!("🐛 Debug: Wake word event: {:?}", event);
                            }
                            
                            match event {
                                KwsMonitorEvent::Keyword(keyword) => {
                                    println!("🎯 Wake word detected: {}", keyword);
                                    wake_detected.store(true, Ordering::Relaxed);
                                    
                                    // Reset wake word detection after 10 seconds
                                    let wake_detected_reset = Arc::clone(&wake_detected);
                                    tokio::spawn(async move {
                                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                                        wake_detected_reset.store(false, Ordering::Relaxed);
                                        if debug_flag {
                                            println!("🐛 Debug: Wake word detection reset after timeout");
                                        }
                                    });
                                }
                                KwsMonitorEvent::Started => {
                                    println!("🎤 Wake word monitoring started");
                                }
                            }
                            Ok(())
                        }
                    }).await;
                    
                    if debug_flag {
                        println!("🐛 Debug: Wake word monitoring stopped, restarting in 30 seconds...");
                    }
                    
                    // Wait before restarting
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            })
        };
        
        // Spawn instruction monitoring in background
        let instruction_task = {
            let direct_service = Arc::clone(&direct_service_clone);
            let wake_detected = Arc::clone(&wake_detected_for_instruction);
            let debug_flag = debug;
            
            tokio::spawn(async move {
                if debug_flag {
                    println!("🐛 Debug: Instruction monitoring task starting");
                }
                
                // Make instruction monitoring more resilient
                loop {
                    if debug_flag {
                        println!("🐛 Debug: Starting instruction monitor");
                    }
                    
                    let direct_service = Arc::clone(&direct_service);
                    let wake_detected = Arc::clone(&wake_detected);
                    let debug_flag = debug_flag;
                    
                    InstructionMonitor::start(move |event| {
                        let direct_service = Arc::clone(&direct_service);
                        let wake_detected = Arc::clone(&wake_detected);
                        let debug_flag = debug_flag;
                        
                        async move {
                            if let FileMonitorEvent::NewLine(content) = event {
                                if debug_flag {
                                    println!("🐛 Debug: New instruction line: {}", content);
                                }
                                
                                // Parse the instruction log line
                                if let Ok(log_message) = serde_json::from_str::<LogMessage>(&content) {
                                    if let Payload::RecognizeResultPayload { is_final, results, .. } = log_message.payload {
                                        if is_final && !results.is_empty() {
                                            let text = &results[0].text;
                                            // Lower the confidence threshold and accept any non-empty text
                                            if !text.trim().is_empty() {
                                                // Deduplication: Check if we've processed this instruction recently
                                                static LAST_INSTRUCTION: std::sync::OnceLock<Arc<Mutex<Option<(String, u64)>>>> = std::sync::OnceLock::new();
                                                let last_instruction = LAST_INSTRUCTION.get_or_init(|| Arc::new(Mutex::new(None)));
                                                
                                                let current_time = std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_secs();
                                                
                                                let mut last_processed = last_instruction.lock().await;
                                                let should_skip = if let Some((last_text, last_time)) = &*last_processed {
                                                    last_text == text && current_time - last_time < 3
                                                } else {
                                                    false
                                                };
                                                
                                                if should_skip {
                                                    if debug_flag {
                                                        println!("🐛 Debug: Skipping duplicate instruction '{}' (processed recently)", text);
                                                    }
                                                    return Ok(());
                                                }
                                                
                                                // Update the last processed instruction
                                                *last_processed = Some((text.to_string(), current_time));
                                                drop(last_processed);
                                                
                                                println!("🎤 Voice instruction: '{}' (confidence: {})", text, results[0].confidence);
                                                
                                                if debug_flag {
                                                    println!("🐛 Debug: Processing voice instruction (confidence threshold relaxed)");
                                                }
                                                
                                                // For now, process ALL voice instructions to bypass wake word requirement
                                                // TODO: Add proper wake word detection later
                                                let should_process = true; // wake_detected.load(Ordering::Relaxed) || true;
                                                
                                                if should_process {
                                                    println!("✅ Processing voice instruction");
                                                    
                                                    // First, interrupt XiaoAi's default processing
                                                    if debug_flag {
                                                        println!("🐛 Debug: Interrupting XiaoAi default processing");
                                                    }
                                                    
                                                    if let Err(e) = Self::interrupt_xiaoai().await {
                                                        if debug_flag {
                                                            println!("🐛 Debug: Failed to interrupt XiaoAi: {}", e);
                                                        }
                                                    }
                                                    
                                                    if debug_flag {
                                                        println!("🐛 Debug: Calling LLM with text: '{}'", text);
                                                    }
                                                    
                                                    // Process the instruction with LLM
                                                    match direct_service.call_llm(text).await {
                                                        Ok(response) => {
                                                            println!("🤖 LLM Response: {}", response);
                                                            
                                                            if debug_flag {
                                                                println!("🐛 Debug: Sending TTS response: '{}'", response);
                                                            }
                                                            
                                                            // Send response to device TTS
                                                            if let Err(e) = Self::send_tts_response(&response).await {
                                                                eprintln!("❌ Failed to send TTS response: {}", e);
                                                                if debug_flag {
                                                                    eprintln!("🐛 Debug: TTS error details: {:?}", e);
                                                                }
                                                            } else if debug_flag {
                                                                println!("🐛 Debug: TTS response sent successfully");
                                                            }
                                                            
                                                            // Reset wake word detection after processing
                                                            wake_detected.store(false, Ordering::Relaxed);
                                                            if debug_flag {
                                                                println!("🐛 Debug: Wake word detection reset after processing");
                                                            }
                                                        }
                                                        Err(e) => {
                                                            eprintln!("❌ LLM call failed: {}", e);
                                                            if debug_flag {
                                                                eprintln!("🐛 Debug: LLM error details: {:?}", e);
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    println!("⏭️  Ignoring instruction (no recent wake word detected)");
                                                    if debug_flag {
                                                        println!("🐛 Debug: Instruction ignored - wake word not detected recently");
                                                    }
                                                }
                                            } else if debug_flag {
                                                println!("🐛 Debug: Skipping empty text");
                                            }
                                        }
                                    }
                                } else if debug_flag {
                                    println!("🐛 Debug: Failed to parse JSON: {}", content);
                                }
                            }
                            Ok(())
                        }
                    }).await;
                    
                    if debug_flag {
                        println!("🐛 Debug: Instruction monitoring stopped, restarting in 30 seconds...");
                    }
                    
                    // Wait before restarting
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            })
        };        println!("✅ Audio monitoring started - waiting for wake words and instructions");
        
        if debug {
            println!("🐛 Debug: Both monitoring tasks spawned");
            println!("🐛 Debug: Entering main service loop - the client will run until manually stopped");
        }
        
        // Keep the service running - the tasks run in background
        // We use a simple infinite loop with periodic heartbeat
        let mut heartbeat_counter = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            heartbeat_counter += 1;
            
            if debug {
                println!("🐛 Debug: Heartbeat #{} - service running normally", heartbeat_counter);
            }
            
            // Check if tasks are still running
            if wake_task.is_finished() {
                eprintln!("❌ Wake word monitoring task has stopped unexpectedly");
                return Err("Wake word monitoring failed".into());
            }
            
            if instruction_task.is_finished() {
                eprintln!("❌ Instruction monitoring task has stopped unexpectedly");
                return Err("Instruction monitoring failed".into());
            }
        }
    }

    async fn interrupt_xiaoai() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        
        // Interrupt XiaoAi's default processing by restarting the mico service
        let output = Command::new("sh")
            .arg("-c")
            .arg("/etc/init.d/mico_aivs_lab restart >/dev/null 2>&1")
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("🛑 XiaoAi default processing interrupted");
                } else {
                    println!("⚠️  Failed to interrupt XiaoAi service");
                }
            }
            Err(e) => {
                println!("⚠️  XiaoAi interruption command failed: {}", e);
            }
        }
        
        // Small delay to allow service restart
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        Ok(())
    }

    async fn send_tts_response(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        
        // Use device TTS system
        let output = Command::new("sh")
            .arg("-c")
            .arg(&format!("/usr/sbin/tts_play.sh '{}'", text.replace("'", "'\\''")))
            .output();
            
        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("🔊 TTS response sent successfully");
                } else {
                    println!("⚠️  TTS command failed, using fallback");
                    // Fallback: write to file for other processes
                    std::fs::write("/tmp/xiaoai_output.txt", text)?;
                }
            }
            Err(_) => {
                // Fallback: write to file for other processes
                std::fs::write("/tmp/xiaoai_output.txt", text)?;
                println!("🔊 TTS response written to file");
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    
    // Parse arguments
    let mut config_path = None;
    let mut test_mode = false;
    let mut debug_mode = false;
    
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--test" => test_mode = true,
            "--debug" => debug_mode = true,
            _ if i == 1 => config_path = Some(arg.clone()),
            _ => {}
        }
    }
    
    // Show usage if no config file provided
    if config_path.is_none() {
        print_usage();
        return Ok(());
    }
    
    let config_path = config_path.unwrap();
    
    if debug_mode {
        println!("🐛 Debug mode enabled");
        std::env::set_var("RUST_LOG", "debug");
    }

    println!("📋 Loading config from: {}", config_path);
    if debug_mode {
        println!("🔧 Arguments: test_mode={}, debug_mode={}", test_mode, debug_mode);
    }

    let client = MultiModeClient::new(&config_path)?;
    
    if test_mode {
        println!("🧪 Running in test mode");
        client.run_test_loop().await;
    } else {
        println!("🏭 Running in production mode");
        if debug_mode {
            println!("🐛 Debug: Starting production mode with detailed logging");
        }
        client.run_production_mode_with_debug(debug_mode).await?;
    }

    Ok(())
}

fn print_usage() {
    println!("🤖 Open-XiaoAi Unified Client");
    println!();
    println!("Usage:");
    println!("  ./client <config.json> [--test] [--debug]");
    println!();
    println!("Options:");
    println!("  --test    Run in test mode (quick functionality test)");
    println!("  --debug   Enable debug mode (verbose logging)");
    println!();
    println!("Examples:");
    println!("  ./client config.json              # Run in production mode");
    println!("  ./client config.json --test       # Run in test mode");
    println!("  ./client config.json --debug      # Run with debug logging");
    println!("  ./client config.json --test --debug  # Test mode with debug");
    println!();
    println!("📖 Configuration Setup:");
    println!();
    println!("🔗 Proxy Mode (via HTTP server):");
    println!("{{");
    println!("  \"mode\": \"proxy\",");
    println!("  \"serverProxy\": {{");
    println!("    \"baseURL\": \"http://your-server:4399\",");
    println!("    \"timeout\": 30");
    println!("  }},");
    println!("  \"prompt\": {{");
    println!("    \"system\": \"你是一个智能助手。\"");
    println!("  }}");
    println!("}}");
    println!();
    println!("🤖 Direct Mode (direct LLM API):");
    println!("{{");
    println!("  \"mode\": \"direct\",");
    println!("  \"openai\": {{");
    println!("    \"baseURL\": \"https://api.openai.com/v1\",");
    println!("    \"apiKey\": \"your-api-key\",");
    println!("    \"model\": \"gpt-4\",");
    println!("    \"timeout\": 30,");
    println!("    \"maxTokens\": 1000,");
    println!("    \"temperature\": 0.7");
    println!("  }},");
    println!("  \"prompt\": {{");
    println!("    \"system\": \"你是一个智能助手。\"");
    println!("  }}");
    println!("}}");
    println!();
    println!("💡 Tips:");
    println!("  • Use proxy mode for centralized server management");
    println!("  • Use direct mode for standalone operation");
    println!("  • Copy config.template.json and modify for your setup");
    println!("  • Test your config with --test flag first");
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::base::AppError;
use crate::services::connect::message::MessageManager;
use tokio_tungstenite::tungstenite::Message;

/// AI-Brain protocol message types
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInputMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    pub text: String,
}

impl UserInputMessage {
    pub fn new(session_id: String, text: String) -> Self {
        Self {
            msg_type: "user_input".to_string(),
            session_id,
            text,
        }
    }
}

/// Parse instruction event and extract ASR final text
pub fn extract_asr_text(instruction_data: &Value) -> Option<String> {
    // Parse the NewLine JSON structure
    let new_line = instruction_data.get("NewLine")?.as_str()?;
    let line: Value = serde_json::from_str(new_line).ok()?;
    
    // Check if this is a SpeechRecognizer RecognizeResult
    let header = line.get("header")?;
    if header.get("namespace")?.as_str()? != "SpeechRecognizer" {
        return None;
    }
    if header.get("name")?.as_str()? != "RecognizeResult" {
        return None;
    }
    
    // Check if it's a final result
    let payload = line.get("payload")?;
    if !payload.get("is_final")?.as_bool()? {
        return None;
    }
    
    // Extract the text
    let results = payload.get("results")?.as_array()?;
    let text = results.get(0)?.get("text")?.as_str()?;
    
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Send user_input message to AI-Brain server
pub async fn send_user_input(session_id: String, text: String) -> Result<(), AppError> {
    let msg = UserInputMessage::new(session_id, text);
    let json_str = serde_json::to_string(&msg)?;
    
    MessageManager::instance()
        .send(Message::Text(json_str))
        .await
}

/// Generate a session ID (simple UUID for Phase 1)
pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_asr_text_final() {
        let instruction_data = json!({
            "NewLine": r#"{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"123","id":"456"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"Hello world","confidence":0.95}]}}"#
        });
        
        let result = extract_asr_text(&instruction_data);
        assert_eq!(result, Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_asr_text_not_final() {
        let instruction_data = json!({
            "NewLine": r#"{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"123","id":"456"},"payload":{"is_final":false,"is_vad_begin":false,"results":[{"text":"Hello","confidence":0.8}]}}"#
        });
        
        let result = extract_asr_text(&instruction_data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_asr_text_empty() {
        let instruction_data = json!({
            "NewLine": r#"{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"123","id":"456"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"","confidence":0.0}]}}"#
        });
        
        let result = extract_asr_text(&instruction_data);
        assert_eq!(result, None);
    }
}

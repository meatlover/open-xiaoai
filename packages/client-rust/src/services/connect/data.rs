use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum AppMessage {
    Request(Request),
    Response(Response),
    Event(Event),
    Stream(Stream),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub id: String,
    pub tag: String,
    pub bytes: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<Value>,
}

impl Stream {
    pub fn new(tag: &str, bytes: Vec<u8>, data: Option<Value>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tag: tag.to_string(),
            bytes,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub data: Value,
}

impl Event {
    pub fn new(name: &str, data: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub params: Value,
}

impl Request {
    pub fn new(method: &str, params: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub data: Value,
}

impl Response {
    pub fn new(id: &str, data: Value) -> Self {
        Self {
            id: id.to_string(),
            data,
        }
    }

    pub fn success() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data: serde_json::json!({"status": "success"}),
        }
    }

    pub fn from_data(data: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data,
        }
    }

    pub fn from_error(id: &str, e: impl std::fmt::Display) -> Self {
        Self {
            id: id.to_string(),
            data: serde_json::json!({
                "error": e.to_string(),
                "status": "error"
            }),
        }
    }
}

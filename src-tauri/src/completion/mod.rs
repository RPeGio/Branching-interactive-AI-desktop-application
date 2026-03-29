// use std::path::PathBuf;
// use std::env;
use futures::StreamExt;
use serde;
use serde_json::{self};
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct MessageContext {
    pub content: String,
    pub role: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BalanceMessage {
    pub available: bool,
    pub balance: String,
    pub currency: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UserConfig {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub frequency_penalty: f32,
}

#[tauri::command]
pub async fn stream_chat(
    app: AppHandle,
    key: String,
    contexts: Vec<MessageContext>,
    model_config: UserConfig
) -> Result<(), String> {
    if key.is_empty() { return Err("Bearer token is required!".to_string()); }

    let client = reqwest::Client::new();
    
    println!("Sending request to DeepSeek API...");
    
    let response = client
        .post("https://api.deepseek.com/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "messages": &contexts
                .iter()
                .map(|ctx| {
                    serde_json::json!({
                        "content": ctx.content,
                        "role": ctx.role,
                    })
                }).collect::<Vec<_>>(),
            "model": "deepseek-chat",
            "stream": true,
            "max_tokens": model_config.max_tokens,
            "temperature": model_config.temperature,
            "top_p": model_config.top_p,
            "frequency_penalty": model_config.frequency_penalty,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status().to_string() != String::from("200 OK") {
        return Err(format!("{}", response.status().to_string()));
    }
    app.emit("completion-status", response.status().to_string()).unwrap();

    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        // 将二进制chunk转换为字符串（SSE格式）
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            // 解析 SSE 数据 (data: {...}\n\n)
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        println!("Stream completed");
                        app.emit("completion-end", "Stream completed").unwrap();
                        break;
                    } else {
                        let parsed_data: serde_json::Value = serde_json::from_str(data).unwrap();
                        if let Some(content) = parsed_data["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                // print!("{}", content);
                                // std::io::stdout().flush().unwrap();
                                app.emit("completion-chunk", content.to_string()).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn balance(app: AppHandle, key: String) -> Result<(), String> {
    if key.is_empty() { return Err("Bearer token is required!".to_string()); }
    
    let client = reqwest::Client::new();

    println!("Requesting for balance...");

    let response = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {}", key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if let Ok(res) = response.text().await {
        // println!("{:?}", res);
        let body: serde_json::Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
        
        if let None = body["error"].as_null() {
            return Err(format!("{}.\nError type: {}", body["error"]["message"], body["error"]["type"]));
        }

        let [mut balance, mut currency] = [String::new(), String::new()];
        let mut available: bool = false;
        
        if let Some(str) = body["is_available"].as_bool() {
            available = str;
        }
        if let Some(str) = body["balance_infos"][0]["total_balance"].as_str() {
            balance = str.to_string();
        }
        if let Some(str) = body["balance_infos"][0]["currency"].as_str() {
            currency = str.to_string();
        }

        let response = BalanceMessage {
            available,
            balance,
            currency,
        };

        app.emit("balance", response).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn title_genetation(key: String, contexts: Vec<MessageContext>) -> Result<String, String> {
    if key.is_empty() { return Err("Bearer token is required!".to_string()); }
    let client = reqwest::Client::new();
    
    println!("Sending title generation request to DeepSeek API...");
    
    let response = client
        .post("https://api.deepseek.com/chat/completions")
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "messages": &contexts
                .iter()
                .map(|ctx| {
                    serde_json::json!({
                        "content": ctx.content,
                        "role": ctx.role,
                    })
                }).collect::<Vec<_>>(),
            "model": "deepseek-chat",
            "stream": false,
            "max_tokens": 2000,
            "temperature": 0.7
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status().to_string() != String::from("200 OK") {
        return Err(format!("{}", response.status().to_string()));
    }

    // println!("Response: {:?}", response);
    if let Ok(res) = response.text().await {
        // println!("{:?}", res);
        let body: serde_json::Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
        
        if let None = body["error"].as_null() {
            return Err(format!("{}.\nError type: {}", body["error"]["message"], body["error"]["type"]));
        }

        if let Some(title) = body["choices"][0]["message"]["content"].as_str() {
            return Ok(title.to_string());
        }
    }

    Err("Failed to generate title".to_string())
}
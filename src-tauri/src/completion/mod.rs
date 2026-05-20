// use std::path::PathBuf;
// use std::env;
use futures::StreamExt;
use serde;
use serde_json::{self};
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
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
    model_config: UserConfig,
    model: String,
) -> Result<(), String> {
    if key.is_empty() {
        return Err("Bearer token is required!".to_string());
    }

    let client = reqwest::Client::new();

    println!("Sending request to DeepSeek API...");
    //     app.emit("completion-chunk", "
    // # 标题1
    // ## 标题2

    // 这是一个段落，包含**粗体**和*斜体*文本。

    // - 无序列表项1
    // - 无序列表项2

    // 1. 有序列表项1
    // 2. 有序列表项2

    // > 这是一个引用块

    // `行内代码`

    // # 数学公式测试

    // 行内公式：$E = mc^2$

    // 块级公式：
    // $$
    // \\int_{0}^{\\infty} e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
    // $$

    // 转义括号公式：\\(a^2 + b^2 = c^2\\) 和 \\[x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}\\]

    // # GFM扩展语法测试

    // ## 表格

    // | 姓名 | 年龄 | 城市 |
    // | ---- | ---- | ---- |
    // | 张三 | 25   | 北京 |
    // | 李四 | 30   | 上海 |

    // ## 删除线

    // ~~这是删除的文本~~

    // ## 任务列表

    // - [x] 已完成任务
    // - [ ] 未完成任务
    //     ").unwrap();

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
            "model": format!("deepseek-v4-{}", model),
            "thinking": { "type": "disabled" },
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
    let _ = app.emit("completion-status", response.status().to_string());

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            buffer.push_str(&text);

            loop {
                let boundary = match buffer.find("\n\n") {
                    Some(pos) => pos,
                    None => break,
                };

                let event = buffer[..boundary].to_string();
                buffer = buffer[boundary + 2..].to_string();

                for line in event.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            println!("Stream completed");
                            let _ = app.emit("completion-end", "Stream completed");
                            return Ok(());
                        }

                        match serde_json::from_str::<serde_json::Value>(data) {
                            Ok(parsed) => {
                                if let Some(content) =
                                    parsed["choices"][0]["delta"]["content"].as_str()
                                {
                                    if !content.is_empty() {
                                        let _ = app.emit("completion-chunk", content.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse SSE data: {} -- raw: {}", e, data);
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit("completion-end", "Stream completed");
    Ok(())
}

#[tauri::command]
pub async fn balance(app: AppHandle, key: String) -> Result<(), String> {
    if key.is_empty() {
        return Err("Bearer token is required!".to_string());
    }

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
            return Err(format!(
                "{}.\nError type: {}",
                body["error"]["message"], body["error"]["type"]
            ));
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
pub async fn title_generation(
    key: String,
    contexts: Vec<MessageContext>,
) -> Result<String, String> {
    if key.is_empty() {
        return Err("Bearer token is required!".to_string());
    }
    let client = reqwest::Client::new();

    // println!("{:#?}", contexts);

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
            "model": "deepseek-v4-flash",
            "stream": false,
            "max_tokens": 100,
            "temperature": 0.5
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
            return Err(format!(
                "{}.\nError type: {}",
                body["error"]["message"], body["error"]["type"]
            ));
        }

        if let Some(title) = body["choices"][0]["message"]["content"].as_str() {
            // println!("OK!");
            return Ok(title.to_string());
        }
    }

    Err("Failed to generate title".to_string())
}

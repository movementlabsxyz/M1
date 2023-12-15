use axum::response::{IntoResponse, Response as AxumResponse};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::set_headers;

const URL: &'static str =
    "http://127.0.0.1:9650/ext/bc/28oE37BKazkbVp2EYrm7obGN6PES8pEj5uWnqSHch5YdNdeiHu/rpc";
static mut COUNTER: u64 = 0;

pub async fn request_to_subnet_ext<T: Serialize>(
    method: &'static str,
    _params: Option<T>,
) -> Result<Value, reqwest::Error> {
    unsafe { COUNTER += 1 };
    let client = reqwest::Client::new();
    let params = match _params {
        Some(t) => vec![t],
        _ => vec![],
    };
    let response = client
        .post(URL)
        .json(&serde_json::json!({
          "jsonrpc": "2.0",
          "method": method,
          "params": params,
          "id": 0
        }))
        .send()
        .await?;

    let json_data = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::Value::default());
    Ok(json_data)
}

pub fn make_response(data: &Value) -> impl IntoResponse {
    match data["result"]["data"].as_str() {
        Some(v) => {
          let mut response = AxumResponse::new(v.to_string());
          let header_data = &data["result"]["header"];
          set_headers(&mut response, header_data);
          response
        },
        _ => {
            let response = AxumResponse::new(
                serde_json::json!({
                  "error_code": "account_not_found",
                  "message": "A message describing the error",
                })
                .to_string(),
            );
            response
        }
    }
}

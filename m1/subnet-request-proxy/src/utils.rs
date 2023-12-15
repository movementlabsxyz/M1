use axum::http::{HeaderMap, HeaderValue, Response};

fn set_header_value(
    headers: &mut HeaderMap,
    header_key: &'static str,
    header_data: &serde_json::Value,
    data_key: &'static str,
    default_value: Option<&str>,
) {
    if let Some(v) = header_data[data_key].as_str() {
        headers.insert(header_key, HeaderValue::from_str(v).unwrap());
    } else {
        if let Some(def_v) = default_value {
            headers.insert(header_key, HeaderValue::from_str(def_v).unwrap());
        }
    }
}

pub fn set_headers<T>(response: &mut Response<T>, header_data: &serde_json::Value) {
    let headers = response.headers_mut();
    set_header_value(
        headers,
        "X-APTOS-BLOCK-HEIGHT",
        header_data,
        "block_height",
        Some("0"),
    );
    set_header_value(
        headers,
        "X-APTOS-CHAIN-ID",
        header_data,
        "chain_id",
        Some("0"),
    );
    set_header_value(headers, "X-APTOS-EPOCH", header_data, "epoch", Some("0"));
    set_header_value(
        headers,
        "X-APTOS-LEDGER-OLDEST-VERSION",
        header_data,
        "ledger_oldest_version",
        Some("0"),
    );
    set_header_value(
        headers,
        "X-APTOS-LEDGER-TIMESTAMPUSEC",
        header_data,
        "ledger_timestamp_usec",
        Some("0"),
    );
    set_header_value(
        headers,
        "X-APTOS-LEDGER-VERSION",
        header_data,
        "ledger_version",
        Some("0"),
    );
    set_header_value(
        headers,
        "X-APTOS-OLDEST-BLOCK-HEIGHT",
        header_data,
        "oldest_block_height",
        Some("0"),
    );
    set_header_value(headers, "X-APTOS-CURSOR", header_data, "cursor", None);
}

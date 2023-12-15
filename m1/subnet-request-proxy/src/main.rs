use axum::{
    body::Body,
    extract::{Path, Query, Request},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, Method, StatusCode},
    response::Html,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::rate::RateLimitLayer;


use http_body_util::BodyExt;

mod apis;
mod subnet;
mod types;
mod utils;
mod rate;
use crate::apis::*;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // built app with a route
    let app = Router::new()
        //
        .route("/v1/", get(get_ledger_info))
        .route("/v1/mint", post(faucet))
        .route("/v1/-/healthy", get(healthy))
        .route("/v1/view", get(view_request))
        .route(
            "/v1/transactions",
            get(transactions).post(submit_transactions),
        )
        .route("/v1/transactions/batch", post(submit_transaction_batch))
        .route(
            "/v1/transactions/by_hash/:txn_hash",
            get(get_transaction_by_hash),
        )
        .route(
            "/v1/transactions/by_version/:txn_version",
            get(get_transaction_by_version),
        )
        .route("/v1/transactions/simulate", post(simulate_transaction))
        .route(
            "/v1/accounts/:address/transactions",
            get(get_accounts_transactions),
        )
        .route("/v1/accounts/:address", get(get_account))
        .route(
            "/v1/accounts/:address/resources",
            get(get_account_resources),
        )
        .route("/v1/accounts/:address/modules", get(get_account_modules))
        .route(
            "/v1/accounts/:address/resource/:resource_type",
            get(get_account_resources_state),
        )
        .route(
            "/v1/accounts/:address/module/:module_name",
            get(get_account_modules_state),
        )
        .route(
            "/v1/accounts/:address/events/:creation_number",
            post(get_events_by_creation_number),
        )
        .route(
            "/v1/accounts/:address/events/:event_handle/:field_name",
            post(get_events_by_event_handle),
        )
        //
        .route("/v1/blocks/by_height/:height", get(get_block_by_height))
        //
        .route("/v1/blocks/by_version/:version", get(get_block_by_version))
        // .route("/v1/tables/:table_handle/item", post(get_table_item))
        // .route("/v1/tables/:table_handle/raw_item", post(get_raw_table_item))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([CONTENT_TYPE])
        )
        .layer(
            RateLimitLayer::new(
                1, core::time::Duration::from_secs(10)
            )
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3011")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

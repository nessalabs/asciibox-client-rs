#![allow(dead_code)]
pub mod logs;
use box_client::{BoxApi, Configuration};
use serde_json::Value;
use wiremock::MockServer;
pub const BOX_ID: &str = "bx_23456789";
pub fn client(server: &MockServer) -> BoxApi {
    BoxApi::new(
        Configuration::new("fixture-only")
            .unwrap()
            .with_base_path(server.uri())
            .unwrap()
            .with_org("configured-org"),
    )
    .unwrap()
}
pub fn sdk_case(operation: &str) -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/sdk_operations.json")).unwrap();
    fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["operation"] == operation)
        .unwrap()
        .clone()
}

//! Generate JSON Schema for Fresh configuration
//!
//! This binary generates a JSON Schema file from the Config struct,
//! which can be used by the config editor plugin.
//!
//! Usage: cargo run --bin generate_config_schema > plugins/config-schema.json

use fresh::config::Config;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(Config);
    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");
    println!("{}", json);
}

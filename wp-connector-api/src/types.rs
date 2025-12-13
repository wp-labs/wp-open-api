use std::collections::BTreeMap;

// Core param map type used across connector config/build APIs
pub type ParamMap = BTreeMap<String, serde_json::Value>;

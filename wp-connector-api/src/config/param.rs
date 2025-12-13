use crate::types::ParamMap;

// Helpers: convert from TOML to ParamMap (serde_json)

pub fn parammap_from_toml_table(t: toml::value::Table) -> ParamMap {
    table_to_param_map(&t)
}

pub fn parammap_from_toml_map(t: toml::map::Map<String, toml::Value>) -> ParamMap {
    table_to_param_map(&toml::value::Table::from(t))
}

fn table_to_param_map(t: &toml::value::Table) -> ParamMap {
    fn conv(v: &toml::Value) -> serde_json::Value {
        match v {
            toml::Value::String(s) => serde_json::Value::String(s.clone()),
            toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
            toml::Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
            toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
            toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(conv).collect()),
            toml::Value::Table(tab) => serde_json::Value::Object(
                tab.iter()
                    .map(|(k, v)| (k.clone(), conv(v)))
                    .collect::<serde_json::Map<_, _>>(),
            ),
        }
    }
    let mut out = ParamMap::new();
    for (k, v) in t.iter() {
        out.insert(k.clone(), conv(v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parammap_from_toml_map, parammap_from_toml_table};
    use serde_json::json;
    use std::collections::BTreeMap;
    use toml::value::{Datetime, Table, Value};

    fn build_sample_table() -> Table {
        let mut nested = Table::new();
        nested.insert("inner".into(), Value::Integer(99));

        let mut table = Table::new();
        table.insert("string".into(), Value::String("warp".into()));
        table.insert("int".into(), Value::Integer(42));
        table.insert("float".into(), Value::Float(3.10));
        table.insert("bool".into(), Value::Boolean(true));
        table.insert(
            "datetime".into(),
            Value::Datetime("2024-05-19T10:15:30Z".parse::<Datetime>().unwrap()),
        );
        table.insert(
            "array".into(),
            Value::Array(vec![Value::Integer(1), Value::Boolean(false)]),
        );
        table.insert("obj".into(), Value::Table(nested));
        table
    }

    #[test]
    fn table_to_param_map_handles_scalar_and_nested_types() {
        let table = build_sample_table();
        let map = parammap_from_toml_table(table);

        let mut expected = BTreeMap::new();
        expected.insert("string".into(), json!("warp"));
        expected.insert("int".into(), json!(42));
        expected.insert("float".into(), json!(3.10));
        expected.insert("bool".into(), json!(true));
        expected.insert("datetime".into(), json!("2024-05-19T10:15:30Z"));
        expected.insert("array".into(), json!([1, false]));
        expected.insert("obj".into(), json!({"inner": 99}));

        assert_eq!(map, expected);
    }

    #[test]
    fn parammap_from_toml_map_alias_matches_table_conversion() {
        let table = build_sample_table();
        let map_version = parammap_from_toml_map(table.clone().into_iter().collect());
        let table_version = parammap_from_toml_table(table);
        assert_eq!(map_version, table_version);
    }
}

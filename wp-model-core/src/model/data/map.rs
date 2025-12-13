use std::collections::BTreeMap;

use crate::model::Value;

use super::field::Field;

pub type ObjectMap = BTreeMap<String, Field<Value>>;

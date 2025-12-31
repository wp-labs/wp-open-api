use std::collections::BTreeMap;

use crate::model::Value;
use arcstr::ArcStr;

use super::field::Field;

pub type ObjectMap = BTreeMap<ArcStr, Field<Value>>;

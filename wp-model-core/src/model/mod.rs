use crate::model::data::record::Record;
pub use crate::model::types::value::Value;
use data::field::Field;
use smallvec::SmallVec;
use std::sync::Arc;
pub mod error;
pub mod fmt_def;
pub mod format;
mod macros;

//pub mod array;
// compare impls moved to orion_exp adapters
//mod conv;
pub mod data;
pub mod types;
// conditions impls moved out; core remains pure types + format

pub use types::meta::{DataType, MetaErr};
pub use types::value::{DateTimeValue, DomainT, EmailT, IdCardT, Maker, MobilePhoneT, UrlValue};
pub use types::value::{DigitValue, FloatValue, HexT, IgnoreT, IpNetValue};

pub type DataField = Field<Value>;
//VBean<Value>
pub type DataRecord = Record<DataField>;
//pub type SharedField = Field<Arc<Value>>;
pub type SharedRecord = Record<Arc<DataField>>;
//oml proc using

//pub use conv::to_shared_field_vec;
//pub use conv::to_value_field_vec;

const TAGSET_INLINE_CAPACITY: usize = 16;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TagSet {
    pub item: SmallVec<[(String, String); TAGSET_INLINE_CAPACITY]>,
}

impl TagSet {
    pub fn new() -> Self {
        Self {
            item: SmallVec::new(),
        }
    }

    pub fn append(&mut self, key: &str, value: &str) {
        self.insert_or_update(key.into(), value.into());
    }

    pub fn to_tdos(&self) -> Vec<DataField> {
        self.item
            .iter()
            .map(|(k, v)| DataField::from_chars(k.clone(), v.clone()))
            .collect()
    }

    pub fn set_tag(&mut self, key: &str, val: String) {
        self.insert_or_update(key.to_string(), val);
    }

    pub fn get_tag(&self, key: &str) -> String {
        self.get(key).map(|v| v.to_string()).unwrap_or_default()
    }

    /// Borrowing getter to avoid unnecessary allocations on hot paths.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key))
            .ok()
            .and_then(|idx| self.item.get(idx))
            .map(|(_, v)| v.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.item.is_empty()
    }

    pub fn len(&self) -> usize {
        self.item.len()
    }

    fn insert_or_update(&mut self, key: String, value: String) {
        match self
            .item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key.as_str()))
        {
            Ok(idx) => self.item[idx].1 = value,
            Err(idx) => self.item.insert(idx, (key, value)),
        }
    }
}

pub trait OrDefault<T> {
    fn or_default(self) -> T;
}

impl<T, E> OrDefault<T> for Result<T, E>
where
    T: Default,
{
    fn or_default(self) -> T {
        self.unwrap_or_else(|_| T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{DataField, TagSet};

    #[test]
    fn tagset_keeps_sorted_order_and_updates() {
        let mut tags = TagSet::default();
        tags.append("beta", "2");
        tags.append("alpha", "1");
        tags.set_tag("gamma", "3".to_string());

        let ordered_keys: Vec<_> = tags.item.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(ordered_keys, vec!["alpha", "beta", "gamma"]);
        assert_eq!(tags.len(), 3);
        assert_eq!(tags.get("beta"), Some("2"));

        tags.set_tag("beta", "22".to_string());
        assert_eq!(tags.get_tag("beta"), "22");
        assert_eq!(tags.get("beta"), Some("22"));
    }

    #[test]
    fn tagset_to_tdos_preserves_sorted_order() {
        let mut tags = TagSet::default();
        tags.append("stage", "sink");
        tags.append("env", "prod");

        let tdos = tags.to_tdos();
        assert_eq!(tdos.len(), 2);
        assert_eq!(tdos[0], DataField::from_chars("env", "prod"));
        assert_eq!(tdos[1], DataField::from_chars("stage", "sink"));
    }
}

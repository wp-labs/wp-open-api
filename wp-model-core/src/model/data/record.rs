use crate::model::Maker;
use crate::model::format::LevelFormatAble;
use crate::model::{DataType, Value};
use crate::traits::AsValueRef;
use serde_derive::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr};

use super::field::Field;
pub const WP_EVENT_ID: &str = "wp_event_id";
/// 记录中每一项需要暴露的行为
pub trait RecordItem {
    fn get_name(&self) -> &str;
    fn get_meta(&self) -> &DataType;
    fn get_value(&self) -> &Value;
    fn get_value_mut(&mut self) -> &mut Value;
}

/// 为 Record 生成字段所需的工厂方法
pub trait RecordItemFactory {
    fn from_digit<S: Into<String>>(name: S, val: i64) -> Self;
    fn from_ip<S: Into<String>>(name: S, ip: IpAddr) -> Self;
    fn from_chars<S: Into<String>>(name: S, val: S) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record<T> {
    pub items: Vec<T>,
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self {
            items: Vec::with_capacity(10),
        }
    }
}

impl<T> From<Vec<T>> for Record<T> {
    fn from(value: Vec<T>) -> Self {
        Self { items: value }
    }
}

impl<T> Display for Record<T>
where
    T: RecordItem + LevelFormatAble,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for (i, o) in self.items.iter().enumerate() {
            if *o.get_meta() != DataType::Ignore {
                write!(f, "NO:{:<5}", i + 1)?;
                o.level_fmt(f, 1)?;
            }
        }
        Ok(())
    }
}

impl<T> Record<T>
where
    T: RecordItem + RecordItemFactory,
{
    pub fn set_id(&mut self, id: u64) {
        // 如果已存在 wp_msg_id 字段，避免重复追加
        if self.items.iter().any(|f| f.get_name() == WP_EVENT_ID) {
            return;
        }
        let Ok(id_i64) = i64::try_from(id) else {
            // 事件 ID 超出 i64 无法表示，保持记录原状
            return;
        };
        self.items.insert(0, T::from_digit(WP_EVENT_ID, id_i64));
    }
    pub fn test_value() -> Self {
        let data = vec![
            T::from_ip("ip", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            T::from_chars("chars", "test"),
        ];
        Self { items: data }
    }
}

impl<T> Record<T>
where
    T: RecordItem,
{
    pub fn get_value(&self, key: &str) -> Option<&Value> {
        self.items
            .iter()
            .find(|x| x.get_name() == key)
            .map(|x| x.get_value())
    }
}

impl<T> Record<T> {
    pub fn append(&mut self, data: T) {
        self.items.push(data);
    }
    pub fn merge(&mut self, mut other: Self) {
        self.items.append(&mut other.items);
    }
}

impl<T> Record<T>
where
    T: RecordItem,
{
    // 存在同名字段时取第一个字段返回值
    pub fn field(&self, key: &str) -> Option<&T> {
        self.items.iter().find(|item| item.get_name() == key)
    }

    pub fn get2(&self, name: &str) -> Option<&T> {
        self.items.iter().find(|x| x.get_name() == name)
    }
    pub fn get_value_mut(&mut self, name: &str) -> Option<&mut T> {
        self.items.iter_mut().find(|x| x.get_name() == name)
    }
    pub fn remove_field(&mut self, name: &str) -> bool {
        let pos = self.items.iter().position(|x| x.get_name() == name);
        if let Some(pos) = pos {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }
}

impl<V> RecordItem for Field<V>
where
    V: AsValueRef<Value>,
{
    fn get_name(&self) -> &str {
        Field::get_name(self)
    }

    fn get_meta(&self) -> &DataType {
        Field::get_meta(self)
    }

    fn get_value(&self) -> &Value {
        Field::get_value(self)
    }

    fn get_value_mut(&mut self) -> &mut Value {
        Field::get_value_mut(self)
    }
}

impl<V> RecordItemFactory for Field<V>
where
    V: Maker<i64> + Maker<String> + Maker<IpAddr>,
{
    fn from_digit<S: Into<String>>(name: S, val: i64) -> Self {
        Field::from_digit(name, val)
    }

    fn from_ip<S: Into<String>>(name: S, ip: IpAddr) -> Self {
        Field::from_ip(name, ip)
    }

    fn from_chars<S: Into<String>>(name: S, val: S) -> Self {
        Field::from_chars(name, val)
    }
}

// ValueGetter impl removed from core; use function-style adapters in extension crates.

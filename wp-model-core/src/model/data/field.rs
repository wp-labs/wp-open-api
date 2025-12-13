use crate::model::DataType;
use crate::model::format::LevelFormatAble;

use crate::model::Value;
use crate::traits::AsValueRef;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Field<T> {
    pub meta: DataType,
    pub name: String,
    pub value: T,
}

/// A trait for getting an immutable reference to the inner value of a Field
pub trait ValueRef<T> {
    /// Returns an immutable reference to the inner value
    fn value_ref(&self) -> &T;
}

impl<T> ValueRef<T> for Field<T> {
    fn value_ref(&self) -> &T {
        &self.value
    }
}

impl<T> From<Field<T>> for Field<Rc<T>> {
    fn from(other: Field<T>) -> Self {
        let value = Rc::<T>::from(other.value);
        Self {
            meta: other.meta,
            name: other.name,
            value,
        }
    }
}

impl<T> From<Field<T>> for Field<Arc<T>> {
    fn from(other: Field<T>) -> Self {
        let value = Arc::<T>::from(other.value);
        Self {
            meta: other.meta,
            name: other.name,
            value,
        }
    }
}

impl<T> Field<T> {
    pub fn new<S: Into<String>, V: Into<T>>(meta: DataType, name: S, value: V) -> Self {
        Self {
            meta,
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn new_opt(meta: DataType, name: Option<String>, value: T) -> Self {
        let name = if let Some(name) = name {
            name
        } else {
            From::from(&meta)
        };
        Field { meta, name, value }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
    pub fn clone_name(&self) -> String {
        self.name.as_str().to_string()
    }
    pub fn get_meta(&self) -> &DataType {
        &self.meta
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name
    }
}
impl<T> Field<T>
where
    T: AsValueRef<Value>,
{
    pub fn get_value(&self) -> &Value {
        self.value.as_value_ref()
        //&self.value
    }

    pub fn get_value_mut(&mut self) -> &mut Value {
        self.value.as_value_mutref()
    }
}

impl<T> LevelFormatAble for Field<T>
where
    T: Display,
{
    fn level_fmt(&self, f: &mut Formatter<'_>, level: usize) -> std::fmt::Result {
        let meta: String = From::from(&self.meta);
        writeln!(
            f,
            "{:width$}[{:<16}] {:<20} : {}",
            "",
            meta,
            self.name,
            self.value,
            width = level * 6
        )?;
        Ok(())
    }
}

impl<T> Display for Field<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.meta, self.value)
    }
}

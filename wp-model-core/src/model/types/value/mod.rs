mod composite;
mod custom;
mod network;
mod primitive;
mod traits;
use crate::model::DataField;
use crate::model::data::field::Field;
use crate::traits::AsValueRef;
use std::fmt::{Debug, Display, Formatter};
use std::net::IpAddr;
use std::rc::Rc;
use std::sync::Arc;

pub use composite::{IgnoreT, ObjectValue};
pub use custom::{IdCardT, MobilePhoneT};
pub use network::{DomainT, EmailT, IpNetValue, UrlValue};
pub use primitive::{DateTimeValue, DigitValue, FloatValue, HexT};
use serde::{Deserialize, Serialize};
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Value {
    // 基本类型
    Null,
    Bool(bool),
    Chars(String),
    Float(FloatValue),
    Digit(DigitValue),

    Time(DateTimeValue),

    IpNet(IpNetValue),
    IpAddr(IpAddr),
    Domain(DomainT),
    Url(UrlValue),
    // 自定义验证类型
    Email(EmailT),
    IdCard(IdCardT),
    MobilePhone(MobilePhoneT),
    Hex(HexT),
    // 复合类型
    //Obj(BTreeMap<String, Field<Value>>),
    Obj(ObjectValue),
    Array(Vec<Field<Value>>),
    Symbol(String),
    Ignore(IgnoreT),
}

impl AsValueRef<Value> for Value {
    fn as_value_ref(&self) -> &Value {
        self
    }

    fn as_value_mutref(&mut self) -> &mut Value {
        self
    }
}
impl AsValueRef<Value> for Rc<Value> {
    fn as_value_ref(&self) -> &Value {
        self.as_ref()
    }

    fn as_value_mutref(&mut self) -> &mut Value {
        panic!("Cannot mutably borrow Value stored in Rc; use owned Value instead");
    }
}

impl AsValueRef<Value> for Arc<Value> {
    fn as_value_ref(&self) -> &Value {
        self.as_ref()
    }

    fn as_value_mutref(&mut self) -> &mut Value {
        panic!("Cannot mutably borrow Value stored in Arc; use owned Value instead");
    }
}
pub trait Maker<T> {
    fn make(value: T) -> Self;
}

pub trait InnerFrom<T>: Sized {
    fn inner_from(value: T) -> Self;
}
impl<T> Maker<T> for Value
where
    Value: From<T>,
{
    fn make(value: T) -> Self {
        Value::from(value)
    }
}

impl<T> Maker<T> for Rc<Value>
where
    Value: From<T>,
{
    fn make(value: T) -> Self {
        Rc::new(Value::from(value))
    }
}
impl<T> InnerFrom<T> for Value
where
    T: Into<Value>,
{
    fn inner_from(value: T) -> Self {
        value.into()
    }
}

impl From<IgnoreT> for Value {
    fn from(value: IgnoreT) -> Self {
        Value::Ignore(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Chars(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Chars(value.to_string())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Digit(value)
    }
}
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<IpAddr> for Value {
    fn from(value: IpAddr) -> Self {
        Self::IpAddr(value)
    }
}
impl From<HexT> for Value {
    fn from(value: HexT) -> Self {
        Self::Hex(value)
    }
}
impl From<DateTimeValue> for Value {
    fn from(value: DateTimeValue) -> Self {
        Self::Time(value)
    }
}
impl From<IpNetValue> for Value {
    fn from(value: IpNetValue) -> Self {
        Self::IpNet(value)
    }
}

impl From<Vec<DataField>> for Value {
    fn from(value: Vec<DataField>) -> Self {
        Self::Array(value)
    }
}

impl From<ObjectValue> for Value {
    fn from(value: ObjectValue) -> Self {
        Self::Obj(value)
    }
}

impl From<DomainT> for Value {
    fn from(value: DomainT) -> Self {
        Self::Domain(value)
    }
}

impl From<UrlValue> for Value {
    fn from(value: UrlValue) -> Self {
        Self::Url(value)
    }
}

impl From<EmailT> for Value {
    fn from(value: EmailT) -> Self {
        Self::Email(value)
    }
}

impl From<IdCardT> for Value {
    fn from(value: IdCardT) -> Self {
        Self::IdCard(value)
    }
}

impl From<MobilePhoneT> for Value {
    fn from(value: MobilePhoneT) -> Self {
        Self::MobilePhone(value)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Time(time) => {
                write!(f, "{}", time)
            }
            Value::IpNet(ip) => {
                write!(f, "{}", ip)
            }
            Value::IpAddr(addr) => {
                write!(f, "{}", addr)
            }
            Value::Float(float) => {
                write!(f, "{}", float)
            }
            Value::Digit(digit) => {
                write!(f, "{}", digit)
            }
            Value::Bool(bool) => {
                write!(f, "{}", bool)
            }
            Value::Chars(chars) => {
                write!(f, "{}", chars)
            }
            Value::Hex(hex) => {
                write!(f, "{}", hex)
            }
            Value::Obj(obj) => {
                write!(f, "{:?}", obj)
            }
            Value::Array(array) => {
                write!(f, "{:?}", array)
            }
            //arr_fmt(array, f),
            Value::Symbol(v) => {
                write!(f, "{}", v)
            }
            Value::Domain(v) => write!(f, "{}", v.0),
            Value::Url(v) => write!(f, "{}", v.0),
            Value::Email(v) => write!(f, "{}", v.0),
            Value::IdCard(v) => write!(f, "{}", v.0),
            Value::MobilePhone(v) => write!(f, "{}", v.0),
            Value::Ignore(_) => {
                write!(f, "")
            }
            Value::Null => {
                write!(f, "NULL")
            }
        }
    }
}

// Comparison impls moved to orion_exp adapters to decouple core from orion_exp.

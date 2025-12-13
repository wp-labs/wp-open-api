use std::net::IpAddr;

use crate::model::{
    DataType, DateTimeValue, DomainT, EmailT, HexT, IdCardT, IgnoreT, Maker, MobilePhoneT,
    UrlValue, Value, types::value::ObjectValue,
};

use super::Field;

impl<T> Field<T>
where
    T: Maker<bool>,
{
    pub fn from_bool<S: Into<String>>(name: S, val: bool) -> Self {
        Self::new(DataType::Bool, name.into(), T::make(val))
    }
}

impl<T> Field<T>
where
    T: Maker<String>,
{
    pub fn from_chars<S: Into<String>>(name: S, val: S) -> Self {
        Self::new(DataType::Chars, name.into(), T::make(val.into()))
    }
}
impl<T> Field<T>
where
    T: Maker<String>,
{
    pub fn from_symbol<S: Into<String>>(name: S, val: S) -> Self {
        Self::new(DataType::Symbol, name.into(), T::make(val.into()))
    }
}

impl<T> Field<T>
where
    T: Maker<i64>,
{
    pub fn from_digit<S: Into<String>>(name: S, val: i64) -> Self {
        Self::new(DataType::Digit, name.into(), T::make(val))
    }
}
impl<T> Field<T>
where
    T: Maker<HexT>,
{
    pub fn from_hex<S: Into<String>>(name: S, val: HexT) -> Self {
        Self::new(DataType::Hex, name.into(), T::make(val))
    }
}
impl<T> Field<T>
where
    T: Maker<f64>,
{
    pub fn from_float<S: Into<String>>(name: S, val: f64) -> Self {
        Self::new(DataType::Float, name.into(), T::make(val))
    }
}
impl<T> Field<T>
where
    T: Maker<IpAddr>,
{
    pub fn from_ip<S: Into<String>>(name: S, ip: IpAddr) -> Self {
        Self::new(DataType::IP, name.into(), T::make(ip))
    }
}

impl<T> Field<T>
where
    T: Maker<DomainT>,
{
    pub fn from_domain<S: Into<String>, V: Into<String>>(name: S, domain: V) -> Self {
        Self::new(
            DataType::Domain,
            name.into(),
            T::make(DomainT(domain.into())),
        )
    }
}

impl<T> Field<T>
where
    T: Maker<UrlValue>,
{
    pub fn from_url<S: Into<String>, V: Into<String>>(name: S, url: V) -> Self {
        Self::new(DataType::Url, name.into(), T::make(UrlValue(url.into())))
    }
}

impl<T> Field<T>
where
    T: Maker<EmailT>,
{
    pub fn from_email<S: Into<String>, V: Into<String>>(name: S, email: V) -> Self {
        Self::new(DataType::Email, name.into(), T::make(EmailT(email.into())))
    }
}

impl<T> Field<T>
where
    T: Maker<IdCardT>,
{
    pub fn from_id_card<S: Into<String>, V: Into<String>>(name: S, id_card: V) -> Self {
        Self::new(
            DataType::IdCard,
            name.into(),
            T::make(IdCardT(id_card.into())),
        )
    }
}

impl<T> Field<T>
where
    T: Maker<MobilePhoneT>,
{
    pub fn from_mobile_phone<S: Into<String>, V: Into<String>>(name: S, mobile_phone: V) -> Self {
        Self::new(
            DataType::MobilePhone,
            name.into(),
            T::make(MobilePhoneT(mobile_phone.into())),
        )
    }
}

impl<T> Field<T>
where
    T: Maker<IgnoreT>,
{
    pub fn from_ignore<S: Into<String>>(name: S) -> Self {
        Self::new(DataType::Ignore, name.into(), T::make(IgnoreT::default()))
    }
}

impl<T> Field<T>
where
    T: Maker<DateTimeValue>,
{
    pub fn from_time<S: Into<String>>(name: S, val: DateTimeValue) -> Self {
        Self::new(DataType::Time, name.into(), T::make(val))
    }
}
impl<T> Field<T>
where
    T: Maker<Vec<Field<Value>>>,
{
    pub fn from_arr<S: Into<String>>(name: S, val: Vec<Field<Value>>) -> Self {
        if let Some(f) = val.first() {
            let meta = f.get_meta().to_string();
            Self::new(DataType::Array(meta), name.into(), T::make(val))
        } else {
            Self::new(DataType::Array("auto".into()), name.into(), T::make(val))
            //unreachable!("arr is empty");
        }
    }
}

impl<T> Field<T>
where
    T: Maker<ObjectValue>,
{
    pub fn from_obj<S: Into<String>>(name: S, val: ObjectValue) -> Self {
        Self::new(DataType::Obj, name.into(), T::make(val))
    }
}

impl Value {
    pub fn tag(&self) -> &str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Chars(_) => "Chars",
            Value::Symbol(_) => "Symbol",
            Value::Digit(_) => "Digit",
            Value::Time(_) => "Time",
            Value::Hex(_) => "Hex",
            Value::Float(_) => "Float",
            Value::IpNet(_) => "IpNet",
            Value::IpAddr(_) => "IpAddr",
            Value::Ignore(_) => "Ignore",
            Value::Obj(_) => "Map",
            Value::Array(_) => "Array",
            Value::Domain(_) => "Domain",
            Value::Url(_) => "Url",
            Value::Email(_) => "Email",
            Value::IdCard(_) => "IdCard",
            Value::MobilePhone(_) => "MobilePhone",
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Value::Time(_)
            | Value::IpNet(_)
            | Value::IpAddr(_)
            | Value::Float(_)
            | Value::Digit(_)
            | Value::Bool(_)
            | Value::Hex(_) => false,
            Value::Domain(v) => v.0.is_empty(),
            Value::Url(v) => v.0.is_empty(),
            Value::Email(v) => v.0.is_empty(),
            Value::IdCard(v) => v.0.is_empty(),
            Value::MobilePhone(v) => v.0.is_empty(),
            Value::Chars(v) => v.is_empty(),
            Value::Obj(v) => v.is_empty(),
            Value::Array(v) => v.is_empty(),
            Value::Symbol(v) => v.is_empty(),
            Value::Ignore(_) => true,
            Value::Null => true,
        }
    }
}

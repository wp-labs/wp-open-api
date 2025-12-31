use arcstr::ArcStr;
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct IdCardT(pub ArcStr);
impl Display for IdCardT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct MobilePhoneT(pub ArcStr);
impl Display for MobilePhoneT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
use serde::{Deserialize, Serialize};

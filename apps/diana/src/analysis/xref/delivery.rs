use super::{QueryError, XrefRow};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Issue {
    pub context: String,
    pub kind: String,
    pub reason: String,
    pub range_bytes: Option<[usize; 2]>,
}

impl Issue {
    pub fn from_request(context: String, error: &diana_decomp::RequestError) -> Self {
        Self {
            context,
            kind: format!("{:?}", error.kind()),
            reason: error.to_string(),
            range_bytes: None,
        }
    }
    pub fn unknown(context: String, reason: &str) -> Self {
        Self {
            context,
            kind: "Unknown".to_owned(),
            reason: reason.to_owned(),
            range_bytes: None,
        }
    }
    pub fn new(context: String, error: &QueryError, range_bytes: Option<[usize; 2]>) -> Self {
        Self {
            context,
            kind: format!("{:?}", error.kind()),
            reason: error.to_string(),
            range_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Row(XrefRow),
    Issue(Issue),
}

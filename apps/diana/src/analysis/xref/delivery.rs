use super::{QueryError, XrefRow};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Issue {
    pub context: String,
    pub kind: String,
    pub reason: String,
    pub range_bytes: Option<[usize; 2]>,
}

impl Issue {
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

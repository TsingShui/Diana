pub enum Outcome<T> {
    Ready(T),
    Declined(crate::OutputReason),
}

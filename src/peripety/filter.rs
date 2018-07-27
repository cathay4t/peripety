#[derive(Debug, Clone)]
pub struct StorageEventFilter {
    pub filter_type: StorageEventFilterType,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum StorageEventFilterType {
    Wwid, // Also match on owners' wwid.
    EventType,
    Severity, // Equal or higher severity will match.
    SubSystem,
    Since,
    EventId,
}

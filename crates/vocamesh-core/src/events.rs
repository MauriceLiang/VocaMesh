use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPhase {
    Scanning,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolScanEvent {
    pub phase: ScanPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<&'static str>,
}

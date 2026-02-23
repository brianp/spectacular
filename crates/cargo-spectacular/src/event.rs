use serde::Deserialize;

/// Top-level libtest JSON event, discriminated by `"type"`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "suite")]
    Suite(SuiteEvent),
    #[serde(rename = "test")]
    Test(TestEvent),
}

/// Suite-level events: started, ok, failed.
#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum SuiteEvent {
    #[serde(rename = "started")]
    Started { test_count: usize },
    #[serde(rename = "ok")]
    Ok(SuiteResult),
    #[serde(rename = "failed")]
    Failed(SuiteResult),
}

/// Aggregated suite results emitted at the end.
#[derive(Debug, Deserialize)]
pub struct SuiteResult {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    #[serde(default)]
    pub exec_time: Option<f64>,
}

/// Individual test events: started, ok, failed, ignored.
#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum TestEvent {
    #[serde(rename = "started")]
    Started { name: String },
    #[serde(rename = "ok")]
    Ok {
        name: String,
        #[serde(default)]
        exec_time: Option<f64>,
    },
    #[serde(rename = "failed")]
    Failed {
        name: String,
        #[serde(default)]
        exec_time: Option<f64>,
        #[serde(default)]
        stdout: Option<String>,
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(rename = "ignored")]
    Ignored { name: String },
}

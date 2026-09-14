use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One probe call.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Probe<T> {
    Ok { data: T },
    Auth { message: String },
    Unavailable { message: String },
    Error { message: String },
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Env {
    pub name: String,
    pub subscription: String,
    pub resource_group: String,
    pub cosmos_account: String,
    pub cosmos_db: String,
    pub store_account: String,
    pub app_insights: String,
    pub extract_job: String,
    pub sb_namespace: String,
    pub landing_retention_days: i64,
    pub pending_max_days: i64,
    /// Blob containers that actually exist in the account.
    pub containers: Vec<String>,
}

impl Env {
    pub fn has_container(&self, name: &str) -> bool {
        self.containers.iter().any(|c| c == name)
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Location {
    pub container: String,
    pub path: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Doc {
    pub id: String,
    pub document_id: String,
    pub revision_id: String,
    pub business_key: String,
    pub source_system: String,
    pub source_document_id: String,
    pub document_type: String,
    pub state: String,
    pub file_name: String,
    pub file_guid: String,
    pub mime_type: String,
    pub sniffed_mime: String,
    pub size_bytes: Option<u64>,
    pub actual_size_bytes: Option<u64>,
    pub registered_at: Option<String>,
    pub uploaded_at: Option<String>,
    pub promoted_at: Option<String>,
    pub dispatched_at: Option<String>,
    pub produced_at: Option<String>,
    pub pending_since: Option<String>,
    pub pending_attempts: Option<u32>,
    pub pending_key: Option<String>,
    pub pending_reason: Option<String>,
    pub quarantine_reason: Option<String>,
    pub correlation_id: String,
    pub root_file_guid: String,
    pub root_document_id: String,
    pub parent_document_id: Option<String>,
    pub path_from_root: Option<String>,
    pub derivation_step: Option<String>,
    pub archive_extraction_state: Option<String>,
    pub archive_extracted_at: Option<String>,
    pub archive_member_count: Option<u64>,
    pub archive_extraction_lease_expires_at: Option<String>,
    pub reference_data_as_of: Option<String>,
    pub landing: Option<Location>,
    pub raw: Option<Location>,
    pub curated: Option<Location>,
    pub attributes: BTreeMap<String, Value>,
    pub written_at: Option<String>,
}

impl Doc {
    /// The identifier an operator types: the business key when there is one.
    pub fn display_key(&self) -> &str {
        if self.business_key.is_empty() { &self.document_id } else { &self.business_key }
    }

    pub fn is_member(&self) -> bool {
        !self.root_file_guid.is_empty() && self.root_file_guid != self.file_guid
    }

    pub fn attribute(&self, key: &str) -> Option<String> {
        match self.attributes.get(key)? {
            Value::Null => None,
            Value::String(s) if s.is_empty() => None,
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditRow {
    pub id: String,
    pub document_id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub correlation_id: String,
    pub detail: Value,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BlobCheck {
    pub zone: String,
    pub container: String,
    pub container_exists: bool,
    /// None when the catalog names no address to look at.
    pub path: Option<String>,
    /// None when the check could not run (see `error`).
    pub exists: Option<bool>,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
    pub error: Option<String>,
}

/// One entry the archive holds, whether or not the walk reached it.
#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExpectedEntry {
    pub path: String,
    pub depth: u32,
    pub container: bool,
    pub size: Option<u64>,
    pub document_id: String,
    /// The archive policy's reason when it refuses this entry: the walk stops here.
    pub rejected: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Expected {
    pub entries: Vec<ExpectedEntry>,
    /// Why the archive could not be enumerated, when it could not.
    pub error: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LogRow {
    pub at: String,
    /// `functions` (App Insights) or `job` (Container Apps console).
    pub source: String,
    pub level: String,
    pub category: String,
    pub message: String,
    pub replica: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Logs {
    pub available: bool,
    pub reason: Option<String>,
    /// Ingestion lag of the newest rows, in seconds, as App Insights reports it.
    pub lag_seconds: Option<f64>,
    pub checked_at: Option<String>,
    pub entries: Vec<LogRow>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceCfg {
    pub source_system: String,
    pub reference_set: Option<String>,
    pub pending_max_days: Option<i64>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RefPointer {
    pub reference_set_id: String,
    pub generation: Option<i64>,
    pub row_count: Option<u64>,
    pub built_at: Option<String>,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StuckRoot {
    pub root: Doc,
    pub members_written: u64,
    pub last_progress: Option<String>,
    pub deepest: Option<u32>,
    pub expected: Option<Expected>,
    pub replica: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Overview {
    pub env: Env,
    pub checked_at: String,
    pub catalog_rows: u64,
    pub audit_rows: u64,
    pub parked: Vec<Doc>,
    pub stuck: Vec<StuckRoot>,
    pub sources: Vec<SourceCfg>,
    pub references: Vec<RefPointer>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Searched {
    pub label: String,
    pub result: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Trace {
    pub env: Env,
    pub checked_at: String,
    pub query: String,
    pub matched_on: Option<String>,
    /// The revision being traced: the root of an archive family, or the document.
    pub root: Option<Doc>,
    /// Every catalog row sharing the root, members included.
    pub family: Vec<Doc>,
    pub is_archive: bool,
    /// Other revisions or documents the query also matched.
    pub candidates: Vec<Doc>,
    pub audit: Vec<AuditRow>,
    pub blobs: Vec<BlobCheck>,
    pub expected: Option<Expected>,
    pub logs: Logs,
    pub source: Option<SourceCfg>,
    pub reference: Option<RefPointer>,
    pub searched: Vec<Searched>,
    pub catalog_rows: u64,
    pub audit_rows: u64,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DeadMessage {
    pub queue: String,
    pub sequence: Option<i64>,
    pub enqueued_at: Option<String>,
    pub delivery_count: Option<u32>,
    pub reason: Option<String>,
    pub description: Option<String>,
    pub document_id: Option<String>,
    pub subject: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueState {
    pub name: String,
    pub dead_letter_count: Option<u64>,
    pub active_count: Option<u64>,
    pub error: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DeadLetters {
    pub env: Env,
    pub checked_at: String,
    pub queues: Vec<QueueState>,
    pub messages: Vec<DeadMessage>,
    /// Event Grid deliveries that never reached a queue, by blob name.
    pub event_grid: Vec<BlobCheck>,
    /// Catalog rows for the document ids the messages name.
    pub docs: Vec<Doc>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Timing {
    pub env: Env,
    pub checked_at: String,
    pub docs: Vec<Doc>,
    pub truncated: bool,
}

/// The probe's key spelling is the contract.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_answer_arrives_with_every_key_the_probe_writes() {
        let json = r#"{"status":"ok","data":{
            "env":{"name":"dev","landingRetentionDays":8,"pendingMaxDays":5,"containers":["landing","raw"]},
            "checkedAt":"2026-08-14T09:04:11+00:00","query":"k","matchedOn":"businessKey","isArchive":true,
            "root":{"documentId":"D","revisionId":"00","businessKey":"k","state":"Promoted","fileGuid":"g",
                    "sniffedMime":"application/zip","pendingAttempts":3,"archiveExtractionState":"Extracting",
                    "archiveExtractionLeaseExpiresAt":null,"landing":{"container":"landing","path":"p"},
                    "attributes":{"reference.sbmProjectNumber":"SM1000"},"writtenAt":"2026-08-14T09:00:00+00:00",
                    "eTag":"\"0x1\"","ttl":-1},
            "family":[{"documentId":"M","fileGuid":"m","rootFileGuid":"g","pathFromRoot":"a/b.pdf",
                       "raw":{"container":"raw","path":"x"}}],
            "candidates":[],
            "audit":[{"id":"i","documentId":"D","eventType":"ArchiveExtracted","occurredAt":"t","correlationId":"c","detail":{"members":4}}],
            "blobs":[{"zone":"raw","container":"raw","containerExists":true,"path":"x","exists":true,"size":10,"lastModified":"t","error":null}],
            "expected":{"entries":[{"path":"link","depth":0,"container":false,"size":null,"rejected":"MEMBER_ENTRY_TYPE_FORBIDDEN","documentId":"E"}],"error":"stops"},
            "logs":{"available":true,"reason":null,"lagSeconds":109.0,"checkedAt":"t",
                    "entries":[{"at":"t","source":"job","level":"Info","category":"extract","message":"m","replica":"r1"}]},
            "source":{"sourceSystem":"PACE","referenceSet":"mdr","pendingMaxDays":5},
            "reference":{"referenceSetId":"mdr","path":"reference/mdr/current.json","generation":2,"rowCount":9,"builtAt":"t"},
            "searched":[{"label":"Cosmos catalog","result":"1 row"}],"catalogRows":0,"auditRows":0}}"#;
        let Probe::Ok { data: t } = serde_json::from_str::<Probe<Trace>>(json).unwrap() else { panic!("not ok") };
        let root = t.root.as_ref().unwrap();
        assert!(t.is_archive);
        assert_eq!(t.env.landing_retention_days, 8);
        assert!(t.env.has_container("raw"));
        assert_eq!((root.pending_attempts, root.written_at.is_some()), (Some(3), true));
        assert_eq!(root.archive_extraction_state.as_deref(), Some("Extracting"));
        assert_eq!(root.landing.as_ref().unwrap().path, "p");
        assert_eq!(root.attribute("reference.sbmProjectNumber").as_deref(), Some("SM1000"));
        assert!(t.family[0].is_member());
        assert_eq!(t.family[0].path_from_root.as_deref(), Some("a/b.pdf"));
        assert_eq!(t.audit[0].event_type, "ArchiveExtracted");
        assert!(t.blobs[0].container_exists && t.blobs[0].exists == Some(true));
        assert_eq!(t.blobs[0].last_modified.as_deref(), Some("t"));
        assert_eq!(t.expected.as_ref().unwrap().entries[0].rejected.as_deref(), Some("MEMBER_ENTRY_TYPE_FORBIDDEN"));
        assert_eq!((t.logs.lag_seconds, t.logs.entries[0].replica.as_deref()), (Some(109.0), Some("r1")));
        assert_eq!(t.source.as_ref().unwrap().pending_max_days, Some(5));
        assert_eq!((t.reference.as_ref().unwrap().generation, t.reference.as_ref().unwrap().row_count), (Some(2), Some(9)));
        assert_eq!(t.matched_on.as_deref(), Some("businessKey"));
    }

    #[test]
    fn overview_and_dead_letter_answers_arrive_with_their_keys() {
        let overview = r#"{"status":"ok","data":{"env":{},"checkedAt":"t","catalogRows":42,"auditRows":311,
            "parked":[{"documentId":"P","pendingSince":"t","pendingKey":"NEO/1"}],
            "stuck":[{"root":{"documentId":"R"},"membersWritten":1,"lastProgress":"t","deepest":0,
                      "expected":{"entries":[],"error":null},"replica":"r"}],
            "sources":[{"sourceSystem":"NEO","referenceSet":null,"pendingMaxDays":null}],
            "references":[{"referenceSetId":"mdr","error":"403"}],"warnings":["w"]}}"#;
        let Probe::Ok { data: o } = serde_json::from_str::<Probe<Overview>>(overview).unwrap() else { panic!("not ok") };
        assert_eq!((o.catalog_rows, o.audit_rows), (42, 311));
        assert_eq!(o.parked[0].pending_key.as_deref(), Some("NEO/1"));
        assert_eq!((o.stuck[0].members_written, o.stuck[0].last_progress.as_deref()), (1, Some("t")));
        assert_eq!(o.sources[0].pending_max_days, None);
        assert_eq!(o.references[0].error.as_deref(), Some("403"));

        let dead = r#"{"status":"ok","data":{"env":{},"checkedAt":"t",
            "queues":[{"name":"landing-created","deadLetterCount":1,"activeCount":0,"error":null}],
            "messages":[{"queue":"landing-created","sequence":17,"enqueuedAt":"t","deliveryCount":10,
                         "reason":"MaxDeliveryCountExceeded","description":"d","subject":"s","documentId":"D"}],
            "eventGrid":[{"zone":"eventgrid-deadletter","container":"c","containerExists":true,"path":"b","exists":true,"size":1}],
            "docs":[]}}"#;
        let Probe::Ok { data: d } = serde_json::from_str::<Probe<DeadLetters>>(dead).unwrap() else { panic!("not ok") };
        assert_eq!(d.queues[0].dead_letter_count, Some(1));
        assert_eq!((d.messages[0].delivery_count, d.messages[0].enqueued_at.as_deref()), (Some(10), Some("t")));
        assert_eq!(d.event_grid[0].path.as_deref(), Some("b"));
    }
}

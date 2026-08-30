//! Rules-neutral M2.5.C authority contracts and persisted identity primitives.
//!
//! This module owns fixed V1 array preimages and provenance bindings. It does
//! not resolve sources, validate Magic semantics, classify C candidates, or
//! derive interaction classes.

use crate::{cbor, envelope, PersistenceDecodeErrorV1};

pub const AUTHORITY_SCHEMA_V1: &str = "manafold.m2.5.c.interaction-review-authority.v1";
pub const ACCEPTANCE_EVENT_SCHEMA_V1: &str = "manafold.m2.5.c.review-acceptance-event.v1";
pub const REVIEWER_ROSTER_SCHEMA_V1: &str = "manafold.m2.5.c.reviewer-roster.v1";
pub const ACCEPTANCE_CHECKLIST_V1: &str = "interaction-authority-review-checklist.v1";
pub const SUPERSESSION_RECORD_SCHEMA_V1: &str = "manafold.m2.5.c.supersession-record.v1";

const RAW_REV3_PATHS: [&str; 4] = [
    "derived/Pair_Interaction_Census_REV3.csv",
    "inputs/deck_row_source_resolution_REV3.csv",
    "source/raw/source_record_index_REV3.csv",
    "source/raw/oracle_cards_selected_REV3.jsonl",
];

const ARTIFACT_ROLES: [&str; 10] = [
    "declared_model",
    "rev3_source",
    "b2_catalog",
    "b2_classifications",
    "b2_closure",
    "b1_final_citations",
    "b1_final_closure",
    "candidate_universe",
    "acceptance_event_leaf",
    "reviewer_roster_leaf",
];

const AUTHORITY_KINDS: [&str; 7] = [
    "model",
    "rev3",
    "b2",
    "b1_final",
    "c_candidate",
    "reviewer_roster",
    "acceptance_event",
];

pub const REVIEWER_ROLES: [&str; 5] = [
    "project_owner",
    "architecture_maintainer",
    "rules_authority_maintainer",
    "information_safety_reviewer",
    "conformance_maintainer",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityIdentityKind {
    RelationTheorem,
    RelationTheoremRecord,
    RelationApplication,
    RelationApplicationRecord,
    RelationSupersession,
    DomainTheorem,
    DomainTheoremRecord,
    DomainApplication,
    DomainApplicationRecord,
    DomainSupersession,
    ContextTheorem,
    ContextTheoremRecord,
    ContextApplication,
    ContextApplicationRecord,
    ContextSupersession,
    AcceptanceSubject,
    ReviewAcceptanceEvent,
}

impl AuthorityIdentityKind {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::RelationTheorem => "rp.v1/",
            Self::RelationTheoremRecord => "rpr.v1/",
            Self::RelationApplication => "rpa.v1/",
            Self::RelationApplicationRecord => "rpar.v1/",
            Self::RelationSupersession => "rps.v1/",
            Self::DomainTheorem => "dp.v1/",
            Self::DomainTheoremRecord => "dpr.v1/",
            Self::DomainApplication => "dpa.v1/",
            Self::DomainApplicationRecord => "dpar.v1/",
            Self::DomainSupersession => "dps.v1/",
            Self::ContextTheorem => "cp.v1/",
            Self::ContextTheoremRecord => "cpr.v1/",
            Self::ContextApplication => "cpa.v1/",
            Self::ContextApplicationRecord => "cpar.v1/",
            Self::ContextSupersession => "cps.v1/",
            Self::AcceptanceSubject => "asp.v1/",
            Self::ReviewAcceptanceEvent => "ae.v1/",
        }
    }

    pub const fn semantic_domain(self) -> &'static str {
        match self {
            Self::RelationTheorem => "manafold.m2.5.c.relation-proof.v1",
            Self::RelationTheoremRecord => "manafold.m2.5.c.relation-proof-record.v1",
            Self::RelationApplication => "manafold.m2.5.c.relation-application.v1",
            Self::RelationApplicationRecord => "manafold.m2.5.c.relation-application-record.v1",
            Self::RelationSupersession => "manafold.m2.5.c.relation-supersession.v1",
            Self::DomainTheorem => "manafold.m2.5.c.domain-proof.v1",
            Self::DomainTheoremRecord => "manafold.m2.5.c.domain-proof-record.v1",
            Self::DomainApplication => "manafold.m2.5.c.domain-application.v1",
            Self::DomainApplicationRecord => "manafold.m2.5.c.domain-application-record.v1",
            Self::DomainSupersession => "manafold.m2.5.c.domain-supersession.v1",
            Self::ContextTheorem => "manafold.m2.5.c.context-proof.v1",
            Self::ContextTheoremRecord => "manafold.m2.5.c.context-proof-record.v1",
            Self::ContextApplication => "manafold.m2.5.c.context-application.v1",
            Self::ContextApplicationRecord => "manafold.m2.5.c.context-application-record.v1",
            Self::ContextSupersession => "manafold.m2.5.c.context-supersession.v1",
            Self::AcceptanceSubject => "manafold.m2.5.c.acceptance-subject-payload.v1",
            Self::ReviewAcceptanceEvent => "manafold.m2.5.c.review-acceptance-event.v1",
        }
    }

    pub const fn input_schema_id(self) -> &'static str {
        match self {
            Self::RelationTheorem => "manafold.m2.5.c.relation-proof-input.v1",
            Self::RelationTheoremRecord => "manafold.m2.5.c.relation-proof-record-input.v1",
            Self::RelationApplication => "manafold.m2.5.c.relation-application-input.v1",
            Self::RelationApplicationRecord => {
                "manafold.m2.5.c.relation-application-record-input.v1"
            }
            Self::RelationSupersession => "manafold.m2.5.c.relation-supersession-input.v1",
            Self::DomainTheorem => "manafold.m2.5.c.domain-proof-input.v1",
            Self::DomainTheoremRecord => "manafold.m2.5.c.domain-proof-record-input.v1",
            Self::DomainApplication => "manafold.m2.5.c.domain-application-input.v1",
            Self::DomainApplicationRecord => "manafold.m2.5.c.domain-application-record-input.v1",
            Self::DomainSupersession => "manafold.m2.5.c.domain-supersession-input.v1",
            Self::ContextTheorem => "manafold.m2.5.c.context-proof-input.v1",
            Self::ContextTheoremRecord => "manafold.m2.5.c.context-proof-record-input.v1",
            Self::ContextApplication => "manafold.m2.5.c.context-application-input.v1",
            Self::ContextApplicationRecord => "manafold.m2.5.c.context-application-record-input.v1",
            Self::ContextSupersession => "manafold.m2.5.c.context-supersession-input.v1",
            Self::AcceptanceSubject => "manafold.m2.5.c.acceptance-subject-payload-input.v1",
            Self::ReviewAcceptanceEvent => "manafold.m2.5.c.review-acceptance-event-input.v1",
        }
    }

    pub const fn input_arity(self) -> usize {
        match self {
            Self::RelationTheorem => 12,
            Self::RelationTheoremRecord => 5,
            Self::RelationApplication => 4,
            Self::RelationApplicationRecord => 3,
            Self::RelationSupersession => 8,
            Self::DomainTheorem => 8,
            Self::DomainTheoremRecord => 5,
            Self::DomainApplication => 5,
            Self::DomainApplicationRecord => 3,
            Self::DomainSupersession => 8,
            Self::ContextTheorem => 8,
            Self::ContextTheoremRecord => 5,
            Self::ContextApplication => 3,
            Self::ContextApplicationRecord => 3,
            Self::ContextSupersession => 8,
            Self::AcceptanceSubject => 3,
            Self::ReviewAcceptanceEvent => 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityIdentityV1 {
    kind: AuthorityIdentityKind,
    digest_bytes: [u8; 32],
}

impl AuthorityIdentityV1 {
    pub fn from_digest_bytes(kind: AuthorityIdentityKind, digest_bytes: [u8; 32]) -> Self {
        Self { kind, digest_bytes }
    }

    pub fn compute(
        kind: AuthorityIdentityKind,
        payload: cbor::Value,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let payload = canonical_identity_input(kind, payload)?;
        let canonical_payload = cbor::encode_canonical(&payload)?;
        let envelope = envelope::encode_envelope(
            kind.semantic_domain(),
            kind.input_schema_id(),
            &canonical_payload,
        )?;
        Ok(Self {
            kind,
            digest_bytes: envelope::hash_envelope(&envelope),
        })
    }

    pub const fn kind(&self) -> AuthorityIdentityKind {
        self.kind
    }

    pub const fn digest_bytes(&self) -> [u8; 32] {
        self.digest_bytes
    }

    pub const fn semantic_domain(&self) -> &'static str {
        self.kind.semantic_domain()
    }

    pub const fn input_schema_id(&self) -> &'static str {
        self.kind.input_schema_id()
    }

    pub fn as_text(&self) -> String {
        format!("{}{}", self.kind.prefix(), hex_encode(&self.digest_bytes))
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(envelope::DIGEST_ENVELOPE_ID.to_owned()),
            cbor::Value::Text(envelope::SHA256_ID.to_owned()),
            cbor::Value::Text(self.semantic_domain().to_owned()),
            cbor::Value::Text(envelope::CANONICAL_CBOR_ID.to_owned()),
            cbor::Value::Text(self.input_schema_id().to_owned()),
            cbor::Value::Bytes(self.digest_bytes.to_vec()),
        ])
    }
}

pub fn compute_authority_identity(
    kind: AuthorityIdentityKind,
    payload: cbor::Value,
) -> Result<AuthorityIdentityV1, PersistenceDecodeErrorV1> {
    AuthorityIdentityV1::compute(kind, payload)
}

pub fn canonical_identity_input(
    kind: AuthorityIdentityKind,
    payload: cbor::Value,
) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
    let values = match payload {
        cbor::Value::Array(values) => values,
        _ => return Err(PersistenceDecodeErrorV1::SemanticValidation),
    };
    if values.len() != kind.input_arity() {
        return Err(PersistenceDecodeErrorV1::WrongRecordLength);
    }
    if values.first() != Some(&cbor::Value::Text(kind.input_schema_id().to_owned())) {
        return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
    }
    validate_identity_payload(kind, &values)?;
    Ok(cbor::Value::Array(values))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBindingDigestV1 {
    pub artifact_role: String,
    pub path: String,
    pub schema_or_null: Option<String>,
    pub raw_sha256: [u8; 32],
}

impl SourceBindingDigestV1 {
    pub fn new(
        artifact_role: impl Into<String>,
        path: impl Into<String>,
        schema_or_null: Option<&str>,
        raw_sha256: [u8; 32],
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let artifact_role = artifact_role.into();
        let path = path.into();
        validate_member(&ARTIFACT_ROLES, &artifact_role)?;
        validate_repo_relative_path(&path)?;
        let schema_or_null = schema_or_null.map(str::to_owned);
        if RAW_REV3_PATHS.contains(&path.as_str()) {
            if artifact_role != "rev3_source" || schema_or_null.is_some() {
                return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
            }
        } else if artifact_role == "rev3_source" {
            if path != "inputs/interaction_model_v1.json"
                || schema_or_null.as_deref() != Some("interaction-model.v1")
            {
                return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
            }
        } else if schema_or_null.as_deref() != expected_schema_for_artifact_role(&artifact_role) {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        Ok(Self {
            artifact_role,
            path,
            schema_or_null,
            raw_sha256,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.artifact_role.clone()),
            cbor::Value::Text(self.path.clone()),
            self.schema_or_null
                .as_ref()
                .map_or(cbor::Value::Null, |value| cbor::Value::Text(value.clone())),
            cbor::Value::Bytes(self.raw_sha256.to_vec()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceLocatorV1 {
    WholeArtifact,
    JsonPointer(String),
    ArchiveMember(String),
    EventId(String),
}

impl EvidenceLocatorV1 {
    fn validate(&self, acceptance: bool) -> Result<(), PersistenceDecodeErrorV1> {
        match self {
            Self::WholeArtifact => Ok(()),
            Self::JsonPointer(pointer) => {
                if is_valid_json_pointer(pointer) {
                    Ok(())
                } else {
                    Err(PersistenceDecodeErrorV1::SemanticValidation)
                }
            }
            Self::ArchiveMember(path) => validate_repo_relative_path(path),
            Self::EventId(event_id) if !acceptance => {
                if is_namespaced_digest(event_id, "ae.v1/") {
                    Ok(())
                } else {
                    Err(PersistenceDecodeErrorV1::SemanticValidation)
                }
            }
            Self::EventId(_) => Err(PersistenceDecodeErrorV1::SemanticValidation),
        }
    }

    fn to_cbor(&self) -> cbor::Value {
        match self {
            Self::WholeArtifact => cbor::Value::Array(vec![
                cbor::Value::Text("whole_artifact".to_owned()),
                cbor::Value::Null,
            ]),
            Self::JsonPointer(pointer) => cbor::Value::Array(vec![
                cbor::Value::Text("json_pointer".to_owned()),
                cbor::Value::Text(pointer.clone()),
            ]),
            Self::ArchiveMember(path) => cbor::Value::Array(vec![
                cbor::Value::Text("archive_member".to_owned()),
                cbor::Value::Text(path.clone()),
            ]),
            Self::EventId(event_id) => cbor::Value::Array(vec![
                cbor::Value::Text("event_id".to_owned()),
                cbor::Value::Text(event_id.clone()),
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRefV1 {
    pub authority_kind: String,
    pub path: String,
    pub locator: EvidenceLocatorV1,
    pub raw_sha256: [u8; 32],
}

impl EvidenceRefV1 {
    pub fn new(
        authority_kind: impl Into<String>,
        path: impl Into<String>,
        locator: EvidenceLocatorV1,
        raw_sha256: [u8; 32],
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let authority_kind = authority_kind.into();
        let path = path.into();
        validate_member(&AUTHORITY_KINDS, &authority_kind)?;
        validate_repo_relative_path(&path)?;
        locator.validate(false)?;
        Ok(Self {
            authority_kind,
            path,
            locator,
            raw_sha256,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.authority_kind.clone()),
            cbor::Value::Text(self.path.clone()),
            self.locator.to_cbor(),
            cbor::Value::Bytes(self.raw_sha256.to_vec()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceEvidenceRefV1 {
    pub path: String,
    pub raw_sha256: [u8; 32],
    pub locator: EvidenceLocatorV1,
}

impl AcceptanceEvidenceRefV1 {
    pub fn new(
        path: impl Into<String>,
        raw_sha256: [u8; 32],
        locator: EvidenceLocatorV1,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let path = path.into();
        validate_repo_relative_path(&path)?;
        if matches!(&locator, EvidenceLocatorV1::EventId(_)) {
            return Err(PersistenceDecodeErrorV1::UnknownVariant);
        }
        locator.validate(true)?;
        Ok(Self {
            path,
            raw_sha256,
            locator,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.path.clone()),
            cbor::Value::Bytes(self.raw_sha256.to_vec()),
            self.locator.to_cbor(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewEventRefV1 {
    pub path: String,
    pub raw_sha256: [u8; 32],
    pub event_id: String,
}

impl ReviewEventRefV1 {
    pub fn new(
        path: impl Into<String>,
        raw_sha256: [u8; 32],
        event_id: impl Into<String>,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let path = path.into();
        let event_id = event_id.into();
        validate_repo_relative_path(&path)?;
        if !is_namespaced_digest(&event_id, "ae.v1/") {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        let expected_path = format!(
            "sources/m2_5/authorities/review_acceptance_events/v1/{}.json",
            &event_id["ae.v1/".len()..]
        );
        if path != expected_path {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        Ok(Self {
            path,
            raw_sha256,
            event_id,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.path.clone()),
            cbor::Value::Bytes(self.raw_sha256.to_vec()),
            cbor::Value::Array(vec![
                cbor::Value::Text("event_id".to_owned()),
                cbor::Value::Text(self.event_id.clone()),
            ]),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceV1 {
    pub review_event_ref: ReviewEventRefV1,
}

impl AcceptanceV1 {
    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text("human_accepted".to_owned()),
            self.review_event_ref.to_cbor(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerRosterRefV1 {
    pub path: String,
    pub schema: String,
    pub raw_sha256: [u8; 32],
}

impl ReviewerRosterRefV1 {
    pub fn new(
        path: impl Into<String>,
        schema: impl Into<String>,
        raw_sha256: [u8; 32],
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let path = path.into();
        let schema = schema.into();
        validate_repo_relative_path(&path)?;
        if schema != REVIEWER_ROSTER_SCHEMA_V1 {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        let expected_path = format!(
            "sources/m2_5/authorities/reviewer_rosters/v1/{}.json",
            hex_encode(&raw_sha256)
        );
        if path != expected_path {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        Ok(Self {
            path,
            schema,
            raw_sha256,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.path.clone()),
            cbor::Value::Text(self.schema.clone()),
            cbor::Value::Bytes(self.raw_sha256.to_vec()),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceSubjectKind {
    RelationTheoremRecord,
    DomainTheoremRecord,
    ContextTheoremRecord,
    RelationApplicationRecord,
    DomainApplicationRecord,
    ContextApplicationRecord,
    SupersessionRecord,
}

impl AcceptanceSubjectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelationTheoremRecord => "relation_theorem_record",
            Self::DomainTheoremRecord => "domain_theorem_record",
            Self::ContextTheoremRecord => "context_theorem_record",
            Self::RelationApplicationRecord => "relation_application_record",
            Self::DomainApplicationRecord => "domain_application_record",
            Self::ContextApplicationRecord => "context_application_record",
            Self::SupersessionRecord => "supersession_record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    MultiReviewer,
    SoloSeparateSelfReview,
}

impl ReviewMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MultiReviewer => "multi_reviewer",
            Self::SoloSeparateSelfReview => "solo_separate_self_review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceSubjectPayloadV1 {
    pub subject_kind: AcceptanceSubjectKind,
    pub subject_payload: cbor::Value,
}

impl AcceptanceSubjectPayloadV1 {
    pub fn new(
        subject_kind: AcceptanceSubjectKind,
        subject_payload: cbor::Value,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        if !matches!(subject_payload, cbor::Value::Array(_)) {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        Ok(Self {
            subject_kind,
            subject_payload,
        })
    }

    pub fn semantic_input(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(
                AuthorityIdentityKind::AcceptanceSubject
                    .input_schema_id()
                    .to_owned(),
            ),
            cbor::Value::Text(self.subject_kind.as_str().to_owned()),
            self.subject_payload.clone(),
        ])
    }

    pub fn identity(&self) -> Result<AuthorityIdentityV1, PersistenceDecodeErrorV1> {
        AuthorityIdentityV1::compute(
            AuthorityIdentityKind::AcceptanceSubject,
            self.semantic_input(),
        )
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.subject_kind.as_str().to_owned()),
            self.subject_payload.clone(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerRoleBindingV1 {
    pub reviewer_id: String,
    pub roles: Vec<String>,
}

impl ReviewerRoleBindingV1 {
    pub fn new(
        reviewer_id: impl Into<String>,
        roles: Vec<String>,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let reviewer_id = reviewer_id.into();
        if reviewer_id.is_empty() {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        if roles
            .iter()
            .any(|role| !REVIEWER_ROLES.contains(&role.as_str()))
        {
            return Err(PersistenceDecodeErrorV1::UnknownVariant);
        }
        if roles.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        Ok(Self { reviewer_id, roles })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.reviewer_id.clone()),
            cbor::Value::Array(
                self.roles
                    .iter()
                    .map(|role| cbor::Value::Text(role.clone()))
                    .collect(),
            ),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerV1 {
    pub reviewer_id: String,
    pub roles: Vec<String>,
}

impl ReviewerV1 {
    pub fn new(
        reviewer_id: impl Into<String>,
        roles: Vec<String>,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let binding = ReviewerRoleBindingV1::new(reviewer_id, roles)?;
        Ok(Self {
            reviewer_id: binding.reviewer_id,
            roles: binding.roles,
        })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(self.reviewer_id.clone()),
            cbor::Value::Array(
                self.roles
                    .iter()
                    .map(|role| cbor::Value::Text(role.clone()))
                    .collect(),
            ),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerRosterV1 {
    pub reviewers: Vec<ReviewerV1>,
}

impl ReviewerRosterV1 {
    pub fn new(reviewers: Vec<ReviewerV1>) -> Result<Self, PersistenceDecodeErrorV1> {
        if reviewers.is_empty() {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        if reviewers
            .windows(2)
            .any(|pair| pair[0].reviewer_id >= pair[1].reviewer_id)
        {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        Ok(Self { reviewers })
    }

    pub fn to_cbor(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(REVIEWER_ROSTER_SCHEMA_V1.to_owned()),
            cbor::Value::Array(self.reviewers.iter().map(ReviewerV1::to_cbor).collect()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewAcceptanceEventInputV1 {
    pub subject_kind: AcceptanceSubjectKind,
    pub subject_payload_digest: [u8; 32],
    pub reviewer_roster_ref: ReviewerRosterRefV1,
    pub reviewer_role_bindings: Vec<ReviewerRoleBindingV1>,
    pub review_mode: ReviewMode,
    pub source_binding_digests: Vec<SourceBindingDigestV1>,
    pub review_evidence_refs: Vec<AcceptanceEvidenceRefV1>,
}

impl ReviewAcceptanceEventInputV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject_kind: AcceptanceSubjectKind,
        subject_payload_digest: [u8; 32],
        reviewer_roster_ref: ReviewerRosterRefV1,
        reviewer_role_bindings: Vec<ReviewerRoleBindingV1>,
        review_mode: ReviewMode,
        source_binding_digests: Vec<SourceBindingDigestV1>,
        review_evidence_refs: Vec<AcceptanceEvidenceRefV1>,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        if reviewer_role_bindings.is_empty()
            || reviewer_role_bindings
                .windows(2)
                .any(|pair| pair[0].reviewer_id >= pair[1].reviewer_id)
        {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        if source_binding_digests.is_empty() || review_evidence_refs.is_empty() {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        if source_binding_digests
            .iter()
            .any(|binding| binding.artifact_role == "acceptance_event_leaf")
        {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        if !source_binding_digests
            .iter()
            .any(|binding| binding.artifact_role == "declared_model")
        {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        if !source_binding_digests.iter().any(|binding| {
            binding.artifact_role == "reviewer_roster_leaf"
                && binding.path == reviewer_roster_ref.path
                && binding.schema_or_null.as_deref() == Some(reviewer_roster_ref.schema.as_str())
                && binding.raw_sha256 == reviewer_roster_ref.raw_sha256
        }) {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        let source_values: Vec<cbor::Value> = source_binding_digests
            .iter()
            .map(SourceBindingDigestV1::to_cbor)
            .collect();
        let evidence_values: Vec<cbor::Value> = review_evidence_refs
            .iter()
            .map(AcceptanceEvidenceRefV1::to_cbor)
            .collect();
        validate_canonical_order(&source_values)?;
        validate_canonical_order(&evidence_values)?;
        Ok(Self {
            subject_kind,
            subject_payload_digest,
            reviewer_roster_ref,
            reviewer_role_bindings,
            review_mode,
            source_binding_digests,
            review_evidence_refs,
        })
    }

    pub fn semantic_input(&self) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
        Ok(cbor::Value::Array(vec![
            cbor::Value::Text(
                AuthorityIdentityKind::ReviewAcceptanceEvent
                    .input_schema_id()
                    .to_owned(),
            ),
            cbor::Value::Text(self.subject_kind.as_str().to_owned()),
            cbor::Value::Bytes(self.subject_payload_digest.to_vec()),
            cbor::Value::Text("human_accepted".to_owned()),
            self.reviewer_roster_ref.to_cbor(),
            cbor::Value::Array(
                self.reviewer_role_bindings
                    .iter()
                    .map(ReviewerRoleBindingV1::to_cbor)
                    .collect(),
            ),
            cbor::Value::Text(self.review_mode.as_str().to_owned()),
            cbor::Value::Text(ACCEPTANCE_CHECKLIST_V1.to_owned()),
            cbor::Value::Array(
                self.source_binding_digests
                    .iter()
                    .map(SourceBindingDigestV1::to_cbor)
                    .collect(),
            ),
            cbor::Value::Array(
                self.review_evidence_refs
                    .iter()
                    .map(AcceptanceEvidenceRefV1::to_cbor)
                    .collect(),
            ),
        ]))
    }

    pub fn identity(&self) -> Result<AuthorityIdentityV1, PersistenceDecodeErrorV1> {
        AuthorityIdentityV1::compute(
            AuthorityIdentityKind::ReviewAcceptanceEvent,
            self.semantic_input()?,
        )
    }

    pub fn to_cbor(&self) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
        self.semantic_input()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewAcceptanceEventLeafV1 {
    pub event_id: AuthorityIdentityV1,
    pub subject_kind: AcceptanceSubjectKind,
    pub subject_payload_digest: [u8; 32],
    pub decision: String,
    pub reviewer_roster_ref: ReviewerRosterRefV1,
    pub reviewer_role_bindings: Vec<ReviewerRoleBindingV1>,
    pub review_mode: ReviewMode,
    pub checklist_id: String,
    pub source_binding_digests: Vec<SourceBindingDigestV1>,
    pub review_evidence_refs: Vec<AcceptanceEvidenceRefV1>,
}

impl ReviewAcceptanceEventLeafV1 {
    pub fn from_input(
        input: ReviewAcceptanceEventInputV1,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        let event_id = input.identity()?;
        Ok(Self {
            event_id,
            subject_kind: input.subject_kind,
            subject_payload_digest: input.subject_payload_digest,
            decision: "human_accepted".to_owned(),
            reviewer_roster_ref: input.reviewer_roster_ref,
            reviewer_role_bindings: input.reviewer_role_bindings,
            review_mode: input.review_mode,
            checklist_id: ACCEPTANCE_CHECKLIST_V1.to_owned(),
            source_binding_digests: input.source_binding_digests,
            review_evidence_refs: input.review_evidence_refs,
        })
    }

    pub fn as_input(&self) -> Result<ReviewAcceptanceEventInputV1, PersistenceDecodeErrorV1> {
        if self.event_id.kind() != AuthorityIdentityKind::ReviewAcceptanceEvent {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        if self.decision != "human_accepted" || self.checklist_id != ACCEPTANCE_CHECKLIST_V1 {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        let input = ReviewAcceptanceEventInputV1::new(
            self.subject_kind,
            self.subject_payload_digest,
            self.reviewer_roster_ref.clone(),
            self.reviewer_role_bindings.clone(),
            self.review_mode,
            self.source_binding_digests.clone(),
            self.review_evidence_refs.clone(),
        )?;
        if input.identity()? != self.event_id {
            return Err(PersistenceDecodeErrorV1::DigestMismatch);
        }
        Ok(input)
    }

    pub fn to_cbor(&self) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
        self.as_input()?.semantic_input()
    }

    pub fn to_wire(&self) -> Result<serde_json::Value, PersistenceDecodeErrorV1> {
        self.as_input()?;
        Ok(serde_json::json!({
            "event_id": self.event_id.as_text(),
            "schema": ACCEPTANCE_EVENT_SCHEMA_V1,
            "subject_kind": self.subject_kind.as_str(),
            "subject_payload_digest": hex_encode(&self.subject_payload_digest),
            "decision": self.decision,
            "reviewer_roster_ref": reviewer_roster_ref_to_wire(&self.reviewer_roster_ref),
            "reviewer_role_bindings": self.reviewer_role_bindings.iter()
                .map(reviewer_role_binding_to_wire)
                .collect::<Vec<_>>(),
            "review_mode": self.review_mode.as_str(),
            "checklist_id": self.checklist_id,
            "source_binding_digests": self.source_binding_digests.iter()
                .map(source_binding_to_wire)
                .collect::<Vec<_>>(),
            "review_evidence_refs": self.review_evidence_refs.iter()
                .map(acceptance_evidence_to_wire)
                .collect::<Vec<_>>(),
        }))
    }
}

fn locator_to_wire(locator: &EvidenceLocatorV1) -> serde_json::Value {
    match locator {
        EvidenceLocatorV1::WholeArtifact => serde_json::json!({"kind": "whole_artifact"}),
        EvidenceLocatorV1::JsonPointer(pointer) => {
            serde_json::json!({"kind": "json_pointer", "value": pointer})
        }
        EvidenceLocatorV1::ArchiveMember(path) => {
            serde_json::json!({"kind": "archive_member", "value": path})
        }
        EvidenceLocatorV1::EventId(event_id) => {
            serde_json::json!({"kind": "event_id", "value": event_id})
        }
    }
}

fn source_binding_to_wire(binding: &SourceBindingDigestV1) -> serde_json::Value {
    serde_json::json!({
        "artifact_role": binding.artifact_role,
        "path": binding.path,
        "schema_or_null": binding.schema_or_null,
        "raw_sha256": hex_encode(&binding.raw_sha256),
    })
}

fn acceptance_evidence_to_wire(evidence: &AcceptanceEvidenceRefV1) -> serde_json::Value {
    serde_json::json!({
        "path": evidence.path,
        "raw_sha256": hex_encode(&evidence.raw_sha256),
        "locator": locator_to_wire(&evidence.locator),
    })
}

fn reviewer_roster_ref_to_wire(reference: &ReviewerRosterRefV1) -> serde_json::Value {
    serde_json::json!({
        "path": reference.path,
        "schema": reference.schema,
        "raw_sha256": hex_encode(&reference.raw_sha256),
    })
}

fn reviewer_role_binding_to_wire(binding: &ReviewerRoleBindingV1) -> serde_json::Value {
    serde_json::json!({
        "reviewer_id": binding.reviewer_id,
        "roles": binding.roles,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    RelationTheoremRecord,
    RelationApplicationRecord,
    DomainTheoremRecord,
    DomainApplicationRecord,
    ContextTheoremRecord,
    ContextApplicationRecord,
}

impl RecordKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelationTheoremRecord => "relation_theorem_record",
            Self::RelationApplicationRecord => "relation_application_record",
            Self::DomainTheoremRecord => "domain_theorem_record",
            Self::DomainApplicationRecord => "domain_application_record",
            Self::ContextTheoremRecord => "context_theorem_record",
            Self::ContextApplicationRecord => "context_application_record",
        }
    }

    const fn identity_kind(self) -> AuthorityIdentityKind {
        match self {
            Self::RelationTheoremRecord => AuthorityIdentityKind::RelationTheoremRecord,
            Self::RelationApplicationRecord => AuthorityIdentityKind::RelationApplicationRecord,
            Self::DomainTheoremRecord => AuthorityIdentityKind::DomainTheoremRecord,
            Self::DomainApplicationRecord => AuthorityIdentityKind::DomainApplicationRecord,
            Self::ContextTheoremRecord => AuthorityIdentityKind::ContextTheoremRecord,
            Self::ContextApplicationRecord => AuthorityIdentityKind::ContextApplicationRecord,
        }
    }

    const fn supersession_kind(self) -> AuthorityIdentityKind {
        match self {
            Self::RelationTheoremRecord | Self::RelationApplicationRecord => {
                AuthorityIdentityKind::RelationSupersession
            }
            Self::DomainTheoremRecord | Self::DomainApplicationRecord => {
                AuthorityIdentityKind::DomainSupersession
            }
            Self::ContextTheoremRecord | Self::ContextApplicationRecord => {
                AuthorityIdentityKind::ContextSupersession
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupersessionReason {
    SemanticCorrection,
    SourceRevision,
    ModelRevision,
    AuthorityRevocation,
}

impl SupersessionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SemanticCorrection => "semantic_correction",
            Self::SourceRevision => "source_revision",
            Self::ModelRevision => "model_revision",
            Self::AuthorityRevocation => "authority_revocation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersessionRecordV1 {
    pub superseded_record_id: AuthorityIdentityV1,
    pub replacement_record_id: Option<AuthorityIdentityV1>,
    pub superseded_record_kind: RecordKind,
    pub replacement_record_kind: Option<RecordKind>,
    pub reason_code: SupersessionReason,
    pub source_evidence_refs: Vec<EvidenceRefV1>,
    pub review_event_ref: ReviewEventRefV1,
}

impl SupersessionRecordV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        superseded_record_id: AuthorityIdentityV1,
        replacement_record_id: Option<AuthorityIdentityV1>,
        superseded_record_kind: RecordKind,
        replacement_record_kind: Option<RecordKind>,
        reason_code: SupersessionReason,
        source_evidence_refs: Vec<EvidenceRefV1>,
        review_event_ref: ReviewEventRefV1,
    ) -> Result<Self, PersistenceDecodeErrorV1> {
        if superseded_record_id.kind() != superseded_record_kind.identity_kind() {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
        match (&replacement_record_id, &replacement_record_kind) {
            (None, None) if reason_code == SupersessionReason::AuthorityRevocation => {}
            (None, None) => return Err(PersistenceDecodeErrorV1::SemanticValidation),
            (Some(replacement), Some(kind))
                if *kind == superseded_record_kind
                    && replacement.kind() == kind.identity_kind()
                    && reason_code != SupersessionReason::AuthorityRevocation => {}
            _ => return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch),
        }
        if source_evidence_refs.is_empty() {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        let evidence_values: Vec<cbor::Value> = source_evidence_refs
            .iter()
            .map(EvidenceRefV1::to_cbor)
            .collect();
        validate_canonical_order(&evidence_values)?;
        Ok(Self {
            superseded_record_id,
            replacement_record_id,
            superseded_record_kind,
            replacement_record_kind,
            reason_code,
            source_evidence_refs,
            review_event_ref,
        })
    }

    pub fn semantic_input(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::Value::Text(
                self.superseded_record_kind
                    .supersession_kind()
                    .input_schema_id()
                    .to_owned(),
            ),
            cbor::Value::Bytes(self.superseded_record_id.digest_bytes().to_vec()),
            self.replacement_record_id
                .as_ref()
                .map_or(cbor::Value::Null, |identity| {
                    cbor::Value::Bytes(identity.digest_bytes().to_vec())
                }),
            cbor::Value::Text(self.superseded_record_kind.as_str().to_owned()),
            self.replacement_record_kind
                .as_ref()
                .map_or(cbor::Value::Null, |kind| {
                    cbor::Value::Text(kind.as_str().to_owned())
                }),
            cbor::Value::Text(self.reason_code.as_str().to_owned()),
            cbor::Value::Array(
                self.source_evidence_refs
                    .iter()
                    .map(EvidenceRefV1::to_cbor)
                    .collect(),
            ),
            self.review_event_ref.to_cbor(),
        ])
    }

    pub fn identity(&self) -> Result<AuthorityIdentityV1, PersistenceDecodeErrorV1> {
        AuthorityIdentityV1::compute(
            self.superseded_record_kind.supersession_kind(),
            self.semantic_input(),
        )
    }

    pub fn to_cbor(&self) -> cbor::Value {
        self.semantic_input()
    }
}

fn validate_member(allowed: &[&str], value: &str) -> Result<(), PersistenceDecodeErrorV1> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(PersistenceDecodeErrorV1::UnknownVariant)
    }
}

fn expected_schema_for_artifact_role(role: &str) -> Option<&'static str> {
    match role {
        "declared_model" => Some("manafold.m2.5.c.declared-interaction-model.v2"),
        "b2_catalog" => Some("manafold.m2.5.b2.requirement-family-catalog.v1"),
        "b2_classifications" => Some("manafold.m2.5.b2.card-semantic-classifications.v1"),
        "b2_closure" => Some("manafold.m2.5.b2.classification-closure.v1"),
        "b1_final_citations" => Some("manafold.m2.5.b1.official-authority-citations.v3"),
        "b1_final_closure" => Some("manafold.m2.5.b1.official-authority-citation-closure.v2"),
        "candidate_universe" => Some("manafold.m2.5.c.interaction-candidate-universe.v2"),
        "acceptance_event_leaf" => Some(ACCEPTANCE_EVENT_SCHEMA_V1),
        "reviewer_roster_leaf" => Some(REVIEWER_ROSTER_SCHEMA_V1),
        "rev3_source" => None,
        _ => unreachable!("artifact role was checked against the closed vocabulary"),
    }
}

fn is_valid_json_pointer(pointer: &str) -> bool {
    if pointer.is_empty() {
        return true;
    }
    if !pointer.starts_with('/') {
        return false;
    }
    let bytes = pointer.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            if index + 1 >= bytes.len() || !matches!(bytes[index + 1], b'0' | b'1') {
                return false;
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    true
}

fn validate_canonical_order(values: &[cbor::Value]) -> Result<(), PersistenceDecodeErrorV1> {
    let mut previous: Option<Vec<u8>> = None;
    for value in values {
        let encoded = cbor::encode_canonical(value)?;
        if let Some(previous) = &previous {
            if encoded <= *previous {
                return Err(if encoded == *previous {
                    PersistenceDecodeErrorV1::DuplicateSemanticKey
                } else {
                    PersistenceDecodeErrorV1::NoncanonicalOrder
                });
            }
        }
        previous = Some(encoded);
    }
    Ok(())
}

fn validate_repo_relative_path(value: &str) -> Result<(), PersistenceDecodeErrorV1> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains("://")
        || value
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
        || value
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}

fn is_namespaced_digest(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(is_lower_hex))
}

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

const RELATION_CHANNELS: [&str; 11] = [
    "participant_boundary",
    "event_or_effect_causality",
    "target_or_choice",
    "zone_or_object_identity",
    "control_or_ownership",
    "replacement_or_layer",
    "trigger_or_lki",
    "information_or_visibility",
    "ordering_or_temporal",
    "decision_actor",
    "format_and_declared_scope",
];
const ARITIES: [&str; 4] = [
    "unary",
    "unordered_binary",
    "directional_binary",
    "higher_order",
];
const DIRECTIONALITIES: [&str; 2] = ["unordered", "directional"];
const OPERATIONS: [&str; 13] = [
    "reads",
    "changes_characteristic",
    "changes_eligibility",
    "changes_target_legality",
    "changes_controller",
    "changes_ownership",
    "changes_zone",
    "creates_object",
    "copies_value",
    "replaces_event",
    "triggers_ability",
    "orders_event",
    "supplies_choice",
];
const PROOF_KINDS: [&str; 3] = [
    "positive_interaction",
    "positive_separation",
    "model_bound_scope",
];
const SCOPE_REASONS: [&str; 4] = [
    "unbounded_n_way_not_representable",
    "undeclared_participant_kind",
    "undeclared_relation_shape",
    "undeclared_outcome_surface",
];
const REVIEW_DOMAINS: [&str; 11] = [
    "triggers_and_lki",
    "replacement_layers_and_dependency",
    "copy_and_token_creation",
    "target_legality_protection_and_identity",
    "control_and_ownership",
    "commander_and_format",
    "hidden_information_and_visibility",
    "ordering_and_temporal_dependencies",
    "source_versus_affected_identity",
    "controller_owner_and_decision_actor",
    "higher_order_interactions",
];
const APPLICABILITY: [&str; 2] = ["applicable", "not_applicable"];
const TERMINAL_DISPOSITIONS: [&str; 4] = [
    "required_interaction",
    "not_an_interaction_with_proof",
    "out_of_declared_scope_with_reason",
    "unresolved",
];
const PRECONDITION_KINDS: [&str; 6] = [
    "candidate_relation_shape",
    "participant_binding",
    "b2_boundary",
    "source_context",
    "temporal_semantic",
    "class_projection",
];
const SEPARATION_KINDS: [&str; 3] = [
    "boundary_disjointness",
    "closed_channel_exclusion",
    "independent_effect_separation",
];
const REQUIRED_CONCLUSIONS: [&str; 2] = ["separated", "not_relevant"];
const SLOT_KINDS: [&str; 2] = ["context_dimension", "temporal_semantic"];
const RECORD_KINDS: [&str; 6] = [
    "relation_theorem_record",
    "relation_application_record",
    "domain_theorem_record",
    "domain_application_record",
    "context_theorem_record",
    "context_application_record",
];
const SUBJECT_KINDS: [&str; 7] = [
    "relation_theorem_record",
    "domain_theorem_record",
    "context_theorem_record",
    "relation_application_record",
    "domain_application_record",
    "context_application_record",
    "supersession_record",
];

fn value_array(
    value: &cbor::Value,
    expected_len: Option<usize>,
) -> Result<&[cbor::Value], PersistenceDecodeErrorV1> {
    let values = match value {
        cbor::Value::Array(values) => values.as_slice(),
        _ => return Err(PersistenceDecodeErrorV1::SemanticValidation),
    };
    if expected_len.is_some_and(|expected| values.len() != expected) {
        return Err(PersistenceDecodeErrorV1::WrongRecordLength);
    }
    Ok(values)
}

fn value_text(value: &cbor::Value) -> Result<&str, PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Text(value) if !value.is_empty() => Ok(value),
        _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
    }
}

fn value_uint32(value: &cbor::Value) -> Result<u32, PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Unsigned(value) if *value <= u64::from(u32::MAX) => Ok(*value as u32),
        _ => Err(PersistenceDecodeErrorV1::ValueOutOfRange),
    }
}

fn value_bytes32(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Bytes(value) if value.len() == 32 => Ok(()),
        _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
    }
}

fn enum_text<'a>(
    value: &'a cbor::Value,
    allowed: &[&str],
) -> Result<&'a str, PersistenceDecodeErrorV1> {
    let value = value_text(value)?;
    validate_member(allowed, value)?;
    Ok(value)
}

fn optional_uint32(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    if !matches!(value, cbor::Value::Null) {
        value_uint32(value)?;
    }
    Ok(())
}

fn validate_cbor_value(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Null
        | cbor::Value::Bool(_)
        | cbor::Value::Unsigned(_)
        | cbor::Value::Signed(_)
        | cbor::Value::Bytes(_)
        | cbor::Value::Text(_) => Ok(()),
        cbor::Value::Array(values) => {
            for value in values {
                validate_cbor_value(value)?;
            }
            Ok(())
        }
    }
}

fn validate_canonical_values(
    values: &[cbor::Value],
    validator: fn(&cbor::Value) -> Result<(), PersistenceDecodeErrorV1>,
) -> Result<(), PersistenceDecodeErrorV1> {
    let mut previous: Option<Vec<u8>> = None;
    for value in values {
        validator(value)?;
        let encoded = cbor::encode_canonical(value)?;
        if let Some(previous) = &previous {
            match encoded.as_slice().cmp(previous.as_slice()) {
                std::cmp::Ordering::Less => {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalOrder)
                }
                std::cmp::Ordering::Equal => {
                    return Err(PersistenceDecodeErrorV1::DuplicateSemanticKey)
                }
                std::cmp::Ordering::Greater => {}
            }
        }
        previous = Some(encoded);
    }
    Ok(())
}

fn validate_ordered_enum(
    values: &[cbor::Value],
    allowed: &[&str],
) -> Result<(), PersistenceDecodeErrorV1> {
    let mut seen = Vec::with_capacity(values.len());
    for value in values {
        seen.push(enum_text(value, allowed)?.to_owned());
    }
    if seen.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
    }
    let expected: Vec<&str> = allowed
        .iter()
        .copied()
        .filter(|value| seen.iter().any(|seen| seen == value))
        .collect();
    if seen.iter().map(String::as_str).collect::<Vec<_>>() != expected {
        return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
    }
    Ok(())
}

fn validate_digest_reference(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(6))?;
    for field in &fields[..5] {
        value_text(field)?;
    }
    value_bytes32(&fields[5])
}

fn validate_locator_array(
    value: &cbor::Value,
    acceptance: bool,
) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    let kind = value_text(&fields[0])?;
    match kind {
        "whole_artifact" if matches!(fields[1], cbor::Value::Null) => Ok(()),
        "json_pointer" => match &fields[1] {
            cbor::Value::Text(pointer) if is_valid_json_pointer(pointer) => Ok(()),
            _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
        },
        "archive_member" => match &fields[1] {
            cbor::Value::Text(path) => validate_repo_relative_path(path),
            _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
        },
        "event_id" if !acceptance => match &fields[1] {
            cbor::Value::Text(event_id) if is_namespaced_digest(event_id, "ae.v1/") => Ok(()),
            _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
        },
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_source_binding_array(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(4))?;
    let role = value_text(&fields[0])?;
    let path = value_text(&fields[1])?;
    let schema = match &fields[2] {
        cbor::Value::Null => None,
        cbor::Value::Text(value) => Some(value.as_str()),
        _ => return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch),
    };
    let raw_sha256 = match &fields[3] {
        cbor::Value::Bytes(value) if value.len() == 32 => {
            let mut digest = [0u8; 32];
            digest.copy_from_slice(value);
            digest
        }
        _ => return Err(PersistenceDecodeErrorV1::SemanticValidation),
    };
    SourceBindingDigestV1::new(role, path, schema, raw_sha256).map(|_| ())
}

fn validate_evidence_ref_array(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(4))?;
    validate_member(&AUTHORITY_KINDS, value_text(&fields[0])?)?;
    validate_repo_relative_path(value_text(&fields[1])?)?;
    validate_locator_array(&fields[2], false)?;
    value_bytes32(&fields[3])
}

fn validate_evidence_refs(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let refs = value_array(value, None)?;
    validate_canonical_values(refs, validate_evidence_ref_array)
}

fn validate_acceptance_evidence_ref_array(
    value: &cbor::Value,
) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(3))?;
    validate_repo_relative_path(value_text(&fields[0])?)?;
    value_bytes32(&fields[1])?;
    validate_locator_array(&fields[2], true)
}

fn validate_acceptance_evidence_refs(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let refs = value_array(value, None)?;
    if refs.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    validate_canonical_values(refs, validate_acceptance_evidence_ref_array)
}

fn validate_review_event_ref_array(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(3))?;
    let path = value_text(&fields[0])?;
    value_bytes32(&fields[1])?;
    let locator = value_array(&fields[2], Some(2))?;
    if value_text(&locator[0])? != "event_id" {
        return Err(PersistenceDecodeErrorV1::UnknownVariant);
    }
    let event_id = value_text(&locator[1])?;
    ReviewEventRefV1::new(path, [0u8; 32], event_id).map(|_| ())
}

fn validate_roster_ref_array(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(3))?;
    let digest = match &fields[2] {
        cbor::Value::Bytes(value) if value.len() == 32 => {
            let mut digest = [0u8; 32];
            digest.copy_from_slice(value);
            digest
        }
        _ => return Err(PersistenceDecodeErrorV1::SemanticValidation),
    };
    ReviewerRosterRefV1::new(value_text(&fields[0])?, value_text(&fields[1])?, digest).map(|_| ())
}

fn validate_reviewer_roles(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let roles = value_array(value, None)?;
    let mut previous: Option<&str> = None;
    for role in roles {
        let role = enum_text(role, &REVIEWER_ROLES)?;
        if previous.is_some_and(|previous| previous >= role) {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        previous = Some(role);
    }
    Ok(())
}

fn validate_participant_role(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(4))?;
    value_uint32(&fields[0])?;
    for field in &fields[1..] {
        value_text(field)?;
    }
    Ok(())
}

fn validate_participant_roles(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let roles = value_array(value, None)?;
    if roles.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    for (index, role) in roles.iter().enumerate() {
        let fields = value_array(role, Some(4))?;
        if value_uint32(&fields[0])? != index as u32 {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        validate_participant_role(role)?;
    }
    Ok(())
}

fn validate_b2_boundary_ref(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    value_text(&fields[0])?;
    value_text(&fields[1])?;
    Ok(())
}

fn validate_b2_boundary_refs(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let refs = value_array(value, None)?;
    validate_canonical_values(refs, validate_b2_boundary_ref)
}

fn validate_b1_citation_ref(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    value_text(&fields[0])?;
    value_text(&fields[1])?;
    Ok(())
}

fn validate_b1_citation_refs(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let refs = value_array(value, None)?;
    validate_canonical_values(refs, validate_b1_citation_ref)
}

fn validate_model_boundary_locator(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    match value_text(&fields[0])? {
        "coverage_scope" if matches!(fields[1], cbor::Value::Null) => Ok(()),
        "excluded_claim" => {
            value_uint32(&fields[1])?;
            Ok(())
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_positive_boundary_fact(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let tagged = value_array(value, Some(2))?;
    let payload = value_array(&tagged[1], None)?;
    match value_text(&tagged[0])? {
        "b2_boundary" => {
            if payload.len() != 4 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            for field in payload {
                value_text(field)?;
            }
            Ok(())
        }
        "rev3_locator" | "b2_locator" => {
            if payload.len() != 3 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            value_text(&payload[0])?;
            value_bytes32(&payload[1])?;
            validate_locator_array(&payload[2], false)
        }
        "b1_citation" => validate_b1_citation_ref(&tagged[1]),
        "context_slot" => {
            if payload.len() != 3 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            value_text(&payload[0])?;
            value_text(&payload[1])?;
            validate_cbor_value(&payload[2])
        }
        "model_boundary" => {
            if payload.len() != 3 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            value_text(&payload[0])?;
            value_text(&payload[1])?;
            validate_model_boundary_locator(&payload[2])
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_positive_boundary_facts(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let facts = value_array(value, None)?;
    if facts.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    validate_canonical_values(facts, validate_positive_boundary_fact)
}

fn validate_class_projection(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(9))?;
    enum_text(&fields[0], &ARITIES)?;
    enum_text(&fields[1], &DIRECTIONALITIES)?;
    validate_participant_roles(&fields[2])?;
    value_text(&fields[3])?;
    for field in &fields[4..7] {
        let values = value_array(field, None)?;
        for value in values {
            value_text(value)?;
        }
    }
    validate_b2_boundary_refs(&fields[7])?;
    validate_b1_citation_refs(&fields[8])
}

fn validate_candidate_shape(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(5))?;
    for field in &fields[..4] {
        value_text(field)?;
    }
    value_uint32(&fields[4])?;
    Ok(())
}

fn validate_model_boundary_ref(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(4))?;
    value_text(&fields[0])?;
    if value_text(&fields[1])? != "manafold.m2.5.c.declared-interaction-model.v2" {
        return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
    }
    value_bytes32(&fields[2])?;
    validate_model_boundary_locator(&fields[3])
}

fn validate_precondition_payload(
    kind: &str,
    payload: &cbor::Value,
) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(payload, None)?;
    match kind {
        "candidate_relation_shape" => {
            if fields.len() != 4 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            for field in fields {
                value_text(field)?;
            }
            Ok(())
        }
        "participant_binding" => {
            if fields.len() != 4 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            value_uint32(&fields[0])?;
            for field in &fields[1..] {
                value_text(field)?;
            }
            Ok(())
        }
        "b2_boundary" => {
            if fields.len() != 4 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            for field in fields {
                value_text(field)?;
            }
            Ok(())
        }
        "source_context" | "temporal_semantic" => {
            if fields.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            value_text(&fields[0])?;
            validate_cbor_value(&fields[1])
        }
        "class_projection" => validate_class_projection(payload),
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_preconditions(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let preconditions = value_array(value, None)?;
    let mut ids = std::collections::BTreeSet::new();
    for precondition in preconditions {
        let fields = value_array(precondition, Some(2))?;
        let id = value_text(&fields[0])?;
        if !ids.insert(id) {
            return Err(PersistenceDecodeErrorV1::DuplicateSemanticKey);
        }
        let tagged = value_array(&fields[1], Some(2))?;
        let kind = enum_text(&tagged[0], &PRECONDITION_KINDS)?;
        validate_precondition_payload(kind, &tagged[1])?;
    }
    Ok(())
}

fn validate_causal_chain(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let chain = value_array(value, None)?;
    if chain.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    for (ordinal, edge) in chain.iter().enumerate() {
        let fields = value_array(edge, Some(7))?;
        if value_uint32(&fields[0])? != ordinal as u32 {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        value_uint32(&fields[1])?;
        enum_text(&fields[2], &OPERATIONS)?;
        validate_b2_boundary_refs(&fields[3])?;
        optional_uint32(&fields[4])?;
        optional_uint32(&fields[5])?;
        validate_b1_citation_refs(&fields[6])?;
    }
    Ok(())
}

fn validate_required_channels(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, None)?;
    validate_ordered_enum(fields, &RELATION_CHANNELS)
}

fn validate_separation_obligations(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let obligations = value_array(value, Some(RELATION_CHANNELS.len()))?;
    for (index, obligation) in obligations.iter().enumerate() {
        let fields = value_array(obligation, Some(2))?;
        if enum_text(&fields[0], &RELATION_CHANNELS)? != RELATION_CHANNELS[index] {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
        enum_text(&fields[1], &REQUIRED_CONCLUSIONS)?;
    }
    Ok(())
}

fn validate_relation_proof_payload(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let tagged = value_array(value, Some(2))?;
    let kind = enum_text(&tagged[0], &PROOF_KINDS)?;
    let fields = value_array(&tagged[1], None)?;
    match kind {
        "positive_interaction" => {
            if fields.len() != 3 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_causal_chain(&fields[0])?;
            validate_required_channels(&fields[1])?;
            if !matches!(fields[2], cbor::Value::Null) {
                validate_class_projection(&fields[2])?;
            }
            Ok(())
        }
        "positive_separation" => {
            if fields.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            enum_text(&fields[0], &SEPARATION_KINDS)?;
            validate_separation_obligations(&fields[1])
        }
        "model_bound_scope" => {
            if fields.len() != 4 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_model_boundary_ref(&fields[0])?;
            enum_text(&fields[1], &SCOPE_REASONS)?;
            validate_candidate_shape(&fields[2])?;
            let evidence = value_array(&fields[3], None)?;
            if evidence.is_empty() {
                return Err(PersistenceDecodeErrorV1::SemanticValidation);
            }
            validate_evidence_refs(&fields[3])
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_relation_binding(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(5))?;
    value_text(&fields[0])?;
    value_text(&fields[1])?;
    enum_text(&fields[2], &DIRECTIONALITIES)?;
    value_text(&fields[3])?;
    validate_participant_roles(&fields[4])
}

fn validate_candidate_universe_binding(
    value: &cbor::Value,
) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(3))?;
    value_text(&fields[0])?;
    value_text(&fields[1])?;
    value_bytes32(&fields[2])
}

fn validate_domain_binding(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    enum_text(&fields[0], &REVIEW_DOMAINS)?;
    enum_text(&fields[1], &APPLICABILITY)?;
    Ok(())
}

fn validate_context_binding(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(4))?;
    enum_text(&fields[0], &ARITIES)?;
    enum_text(&fields[1], &DIRECTIONALITIES)?;
    validate_participant_roles(&fields[2])?;
    value_text(&fields[3])?;
    Ok(())
}

fn validate_precondition_attestations(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let attestations = value_array(value, None)?;
    for attestation in attestations {
        let fields = value_array(attestation, Some(4))?;
        value_text(&fields[0])?;
        validate_cbor_value(&fields[1])?;
        validate_evidence_refs(&fields[2])?;
        value_text(&fields[3])?;
    }
    Ok(())
}

fn validate_class_projection_equivalence(
    value: &cbor::Value,
) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(6))?;
    validate_class_projection(&fields[0])?;
    validate_class_projection(&fields[1])?;
    let equal_positions = value_array(&fields[2], None)?;
    for position in equal_positions {
        value_text(position)?;
    }
    if value_text(&fields[3])? != "same_theorem_semantic_id" {
        return Err(PersistenceDecodeErrorV1::UnknownVariant);
    }
    validate_evidence_refs(&fields[4])?;
    value_text(&fields[5])?;
    Ok(())
}

fn validate_channel_coverage(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(6))?;
    enum_text(&fields[0], &RELATION_CHANNELS)?;
    enum_text(&fields[1], &REQUIRED_CONCLUSIONS)?;
    validate_positive_boundary_facts(&fields[2])?;
    validate_evidence_refs(&fields[3])?;
    validate_b1_citation_refs(&fields[4])?;
    value_text(&fields[5])?;
    Ok(())
}

fn validate_channel_coverages(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let coverages = value_array(value, Some(RELATION_CHANNELS.len()))?;
    for (index, coverage) in coverages.iter().enumerate() {
        validate_channel_coverage(coverage)?;
        let channel = value_array(coverage, Some(6))?;
        if enum_text(&channel[0], &RELATION_CHANNELS)? != RELATION_CHANNELS[index] {
            return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
        }
    }
    Ok(())
}

fn validate_scope_attestation(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(6))?;
    value_text(&fields[0])?;
    value_text(&fields[1])?;
    validate_model_boundary_ref(&fields[2])?;
    enum_text(&fields[3], &SCOPE_REASONS)?;
    validate_candidate_shape(&fields[4])?;
    let evidence = value_array(&fields[5], None)?;
    if evidence.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    validate_evidence_refs(&fields[5])
}

fn validate_member_proof_attestation(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let tagged = value_array(value, Some(2))?;
    let kind = enum_text(&tagged[0], &PROOF_KINDS)?;
    let fields = value_array(&tagged[1], None)?;
    match kind {
        "positive_interaction" => {
            if fields.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            let ordinals = value_array(&fields[0], None)?;
            for (index, ordinal) in ordinals.iter().enumerate() {
                if value_uint32(ordinal)? != index as u32 {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
                }
            }
            if !matches!(fields[1], cbor::Value::Null) {
                validate_class_projection_equivalence(&fields[1])?;
            }
            Ok(())
        }
        "positive_separation" => {
            if fields.len() != 1 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_channel_coverages(&fields[0])
        }
        "model_bound_scope" => {
            if fields.len() != 1 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_scope_attestation(&fields[0])
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_relation_member(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(8))?;
    value_text(&fields[0])?;
    validate_digest_reference(&fields[1])?;
    value_text(&fields[2])?;
    validate_candidate_universe_binding(&fields[3])?;
    validate_relation_binding(&fields[4])?;
    validate_precondition_attestations(&fields[5])?;
    validate_evidence_refs(&fields[6])?;
    validate_member_proof_attestation(&fields[7])
}

fn validate_domain_criterion(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    let kind = value_text(&fields[0])?;
    let payload = value_array(&fields[1], None)?;
    match kind {
        "channel_implicated" | "channel_excluded" => {
            if payload.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            enum_text(&payload[0], &RELATION_CHANNELS)?;
            validate_positive_boundary_fact(&payload[1])
        }
        "rule_domain_required" => {
            if payload.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_b1_citation_ref(&payload[0])?;
            validate_boundary_field_names(&payload[1])
        }
        "rule_domain_excluded" => {
            if payload.len() != 2 {
                return Err(PersistenceDecodeErrorV1::WrongRecordLength);
            }
            validate_excluded_rule_domain_id(&payload[0])?;
            validate_positive_boundary_fact(&payload[1])
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_domain_criteria(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let criteria = value_array(value, None)?;
    for criterion in criteria {
        validate_domain_criterion(criterion)?;
    }
    Ok(())
}

fn validate_boundary_field_names(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    const FIELDS: [&str; 12] = [
        "includes",
        "excludes",
        "objects",
        "action_or_event",
        "timing",
        "zone_visibility",
        "eligibility_condition_duration",
        "targets_choices",
        "ownership_control",
        "numeric_scaling_counters",
        "information_identity_effect",
        "rule_dependency",
    ];
    let values = value_array(value, None)?;
    if values.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    validate_ordered_enum(values, &FIELDS)
}

fn validate_excluded_rule_domain_id(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(2))?;
    match value_text(&fields[0])? {
        "b1_final_citation" => validate_b1_citation_ref(&fields[1]),
        "review_domain" => {
            enum_text(&fields[1], &REVIEW_DOMAINS)?;
            Ok(())
        }
        _ => Err(PersistenceDecodeErrorV1::UnknownVariant),
    }
}

fn validate_domain_member(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(8))?;
    value_text(&fields[0])?;
    validate_digest_reference(&fields[1])?;
    value_text(&fields[2])?;
    validate_candidate_universe_binding(&fields[3])?;
    validate_domain_binding(&fields[4])?;
    validate_precondition_attestations(&fields[5])?;
    validate_evidence_refs(&fields[6])?;
    let attestation = value_array(&fields[7], Some(1))?;
    let criteria = value_array(&attestation[0], None)?;
    for criterion in criteria {
        let fields = value_array(criterion, Some(4))?;
        value_uint32(&fields[0])?;
        validate_domain_criterion(&fields[1])?;
        validate_evidence_refs(&fields[2])?;
        value_text(&fields[3])?;
    }
    Ok(())
}

fn validate_context_member(value: &cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let fields = value_array(value, Some(8))?;
    value_text(&fields[0])?;
    validate_digest_reference(&fields[1])?;
    value_text(&fields[2])?;
    validate_candidate_universe_binding(&fields[3])?;
    validate_context_binding(&fields[4])?;
    validate_precondition_attestations(&fields[5])?;
    validate_evidence_refs(&fields[6])?;
    let attestation = value_array(&fields[7], Some(1))?;
    let slots = value_array(&attestation[0], Some(14))?;
    for slot in slots {
        let fields = value_array(slot, Some(5))?;
        enum_text(&fields[0], &SLOT_KINDS)?;
        value_text(&fields[1])?;
        validate_cbor_value(&fields[2])?;
        validate_evidence_refs(&fields[3])?;
        value_text(&fields[4])?;
    }
    Ok(())
}

fn validate_record_input(
    fields: &[cbor::Value],
    application: bool,
) -> Result<(), PersistenceDecodeErrorV1> {
    if application {
        if fields.len() != 3 {
            return Err(PersistenceDecodeErrorV1::WrongRecordLength);
        }
        value_text(&fields[0])?;
        value_bytes32(&fields[1])?;
        validate_review_event_ref_array(&fields[2])
    } else {
        if fields.len() != 5 {
            return Err(PersistenceDecodeErrorV1::WrongRecordLength);
        }
        value_text(&fields[0])?;
        value_bytes32(&fields[1])?;
        validate_evidence_refs(&fields[2])?;
        validate_review_event_ref_array(&fields[3])?;
        value_text(&fields[4])?;
        Ok(())
    }
}

fn validate_supersession_input(fields: &[cbor::Value]) -> Result<(), PersistenceDecodeErrorV1> {
    if fields.len() != 8 {
        return Err(PersistenceDecodeErrorV1::WrongRecordLength);
    }
    value_text(&fields[0])?;
    value_bytes32(&fields[1])?;
    let replacement = !matches!(fields[2], cbor::Value::Null);
    if replacement {
        value_bytes32(&fields[2])?;
    }
    let superseded_kind = enum_text(&fields[3], &RECORD_KINDS)?;
    let replacement_kind_is_null = matches!(fields[4], cbor::Value::Null);
    if replacement {
        if replacement_kind_is_null
            || enum_text(&fields[4], &RECORD_KINDS)? != superseded_kind
            || value_text(&fields[5])? == "authority_revocation"
        {
            return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
        }
    } else if !replacement_kind_is_null || value_text(&fields[5])? != "authority_revocation" {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    enum_text(
        &fields[5],
        &[
            "semantic_correction",
            "source_revision",
            "model_revision",
            "authority_revocation",
        ],
    )?;
    validate_evidence_refs(&fields[6])?;
    validate_review_event_ref_array(&fields[7])
}

fn validate_acceptance_subject_input(
    fields: &[cbor::Value],
) -> Result<(), PersistenceDecodeErrorV1> {
    if fields.len() != 3 {
        return Err(PersistenceDecodeErrorV1::WrongRecordLength);
    }
    value_text(&fields[0])?;
    let kind = enum_text(&fields[1], &SUBJECT_KINDS)?;
    let payload = value_array(&fields[2], None)?;
    if kind.ends_with("_theorem_record") {
        if payload.len() != 3 {
            return Err(PersistenceDecodeErrorV1::WrongRecordLength);
        }
        value_bytes32(&payload[0])?;
        validate_evidence_refs(&payload[1])?;
        value_text(&payload[2])?;
    } else if kind.ends_with("_application_record") {
        if payload.len() != 1 {
            return Err(PersistenceDecodeErrorV1::WrongRecordLength);
        }
        value_bytes32(&payload[0])?;
    } else {
        if payload.len() != 6 {
            return Err(PersistenceDecodeErrorV1::WrongRecordLength);
        }
        value_bytes32(&payload[0])?;
        let replacement = !matches!(payload[1], cbor::Value::Null);
        if replacement {
            value_bytes32(&payload[1])?;
        }
        let superseded_kind = enum_text(&payload[2], &RECORD_KINDS)?;
        if replacement {
            if enum_text(&payload[3], &RECORD_KINDS)? != superseded_kind {
                return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
            }
        } else if !matches!(payload[3], cbor::Value::Null) {
            return Err(PersistenceDecodeErrorV1::SemanticValidation);
        }
        enum_text(
            &payload[4],
            &[
                "semantic_correction",
                "source_revision",
                "model_revision",
                "authority_revocation",
            ],
        )?;
        validate_evidence_refs(&payload[5])?;
    }
    Ok(())
}

fn validate_acceptance_event_input(fields: &[cbor::Value]) -> Result<(), PersistenceDecodeErrorV1> {
    if fields.len() != 10 {
        return Err(PersistenceDecodeErrorV1::WrongRecordLength);
    }
    value_text(&fields[0])?;
    enum_text(&fields[1], &SUBJECT_KINDS)?;
    value_bytes32(&fields[2])?;
    if value_text(&fields[3])? != "human_accepted" {
        return Err(PersistenceDecodeErrorV1::UnknownVariant);
    }
    validate_roster_ref_array(&fields[4])?;
    let bindings = value_array(&fields[5], None)?;
    if bindings.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    let mut reviewer_ids = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let fields = value_array(binding, Some(2))?;
        let reviewer_id = value_text(&fields[0])?;
        validate_reviewer_roles(&fields[1])?;
        reviewer_ids.push(reviewer_id);
    }
    if reviewer_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PersistenceDecodeErrorV1::NoncanonicalOrder);
    }
    enum_text(&fields[6], &["multi_reviewer", "solo_separate_self_review"])?;
    if value_text(&fields[7])? != ACCEPTANCE_CHECKLIST_V1 {
        return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
    }
    let source_bindings = value_array(&fields[8], None)?;
    if source_bindings.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    for binding in source_bindings {
        validate_source_binding_array(binding)?;
    }
    if source_bindings.iter().any(|binding| {
        value_array(binding, Some(4))
            .ok()
            .and_then(|fields| value_text(&fields[0]).ok())
            == Some("acceptance_event_leaf")
    }) {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    if !source_bindings.iter().any(|binding| {
        value_array(binding, Some(4))
            .ok()
            .and_then(|fields| value_text(&fields[0]).ok())
            == Some("declared_model")
    }) {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    let roster = value_array(&fields[4], Some(3))?;
    let has_roster = source_bindings.iter().any(|binding| {
        value_array(binding, Some(4)).is_ok_and(|binding| {
            binding[0] == cbor::Value::Text("reviewer_roster_leaf".to_owned())
                && binding[1] == roster[0]
                && binding[2] == roster[1]
                && binding[3] == roster[2]
        })
    });
    if !has_roster {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    validate_canonical_values(source_bindings, validate_source_binding_array)?;
    validate_acceptance_evidence_refs(&fields[9])
}

fn validate_identity_payload(
    kind: AuthorityIdentityKind,
    fields: &[cbor::Value],
) -> Result<(), PersistenceDecodeErrorV1> {
    match kind {
        AuthorityIdentityKind::RelationTheorem => {
            value_text(&fields[1])?;
            enum_text(&fields[2], &PROOF_KINDS)?;
            enum_text(&fields[3], &ARITIES)?;
            value_text(&fields[4])?;
            enum_text(&fields[5], &DIRECTIONALITIES)?;
            value_text(&fields[6])?;
            validate_participant_roles(&fields[7])?;
            validate_preconditions(&fields[8])?;
            validate_relation_proof_payload(&fields[9])?;
            validate_b2_boundary_refs(&fields[10])?;
            validate_b1_citation_refs(&fields[11])
        }
        AuthorityIdentityKind::DomainTheorem => {
            value_text(&fields[1])?;
            enum_text(&fields[2], &REVIEW_DOMAINS)?;
            enum_text(&fields[3], &APPLICABILITY)?;
            validate_domain_criteria(&fields[4])?;
            validate_preconditions(&fields[5])?;
            validate_b2_boundary_refs(&fields[6])?;
            validate_b1_citation_refs(&fields[7])
        }
        AuthorityIdentityKind::ContextTheorem => {
            value_text(&fields[1])?;
            validate_context_binding(&fields[2])?;
            let dimensions = value_array(&fields[3], Some(10))?;
            let temporal = value_array(&fields[4], Some(4))?;
            for value in dimensions.iter().chain(temporal.iter()) {
                value_text(value)?;
            }
            validate_preconditions(&fields[5])?;
            validate_b2_boundary_refs(&fields[6])?;
            validate_b1_citation_refs(&fields[7])
        }
        AuthorityIdentityKind::RelationTheoremRecord
        | AuthorityIdentityKind::DomainTheoremRecord
        | AuthorityIdentityKind::ContextTheoremRecord => validate_record_input(fields, false),
        AuthorityIdentityKind::RelationApplication => {
            value_bytes32(&fields[1])?;
            enum_text(&fields[2], &TERMINAL_DISPOSITIONS)?;
            for member in value_array(&fields[3], None)? {
                validate_relation_member(member)?;
            }
            Ok(())
        }
        AuthorityIdentityKind::DomainApplication => {
            value_bytes32(&fields[1])?;
            enum_text(&fields[2], &REVIEW_DOMAINS)?;
            enum_text(&fields[3], &APPLICABILITY)?;
            for member in value_array(&fields[4], None)? {
                validate_domain_member(member)?;
            }
            Ok(())
        }
        AuthorityIdentityKind::ContextApplication => {
            value_bytes32(&fields[1])?;
            for member in value_array(&fields[2], None)? {
                validate_context_member(member)?;
            }
            Ok(())
        }
        AuthorityIdentityKind::RelationApplicationRecord
        | AuthorityIdentityKind::DomainApplicationRecord
        | AuthorityIdentityKind::ContextApplicationRecord => validate_record_input(fields, true),
        AuthorityIdentityKind::RelationSupersession
        | AuthorityIdentityKind::DomainSupersession
        | AuthorityIdentityKind::ContextSupersession => validate_supersession_input(fields),
        AuthorityIdentityKind::AcceptanceSubject => validate_acceptance_subject_input(fields),
        AuthorityIdentityKind::ReviewAcceptanceEvent => validate_acceptance_event_input(fields),
    }
}

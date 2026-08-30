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
pub struct ReviewAcceptanceEventV1 {
    pub subject_kind: AcceptanceSubjectKind,
    pub subject_payload_digest: [u8; 32],
    pub reviewer_roster_ref: ReviewerRosterRefV1,
    pub reviewer_role_bindings: Vec<ReviewerRoleBindingV1>,
    pub review_mode: ReviewMode,
    pub source_binding_digests: Vec<SourceBindingDigestV1>,
    pub review_evidence_refs: Vec<AcceptanceEvidenceRefV1>,
}

impl ReviewAcceptanceEventV1 {
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

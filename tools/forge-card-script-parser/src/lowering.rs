//! Syntax-only lowering of the selected Forge scripts' printed characteristics.
//!
//! Generated definitions remain `UnprofiledV1`. Source abilities are retained
//! separately as tooling metadata and are never added to the executable IR.

use crate::{parse_card_script, AbilityValue, ForgeCardScript, ScriptField};
use mtgml_card_ir::{
    content_validation_only, encode_content_manifest_v1, encode_provenance_catalog_v1,
    AbilityIdentityV1, BaseCharacteristicsV1, CardDefinitionEnvelopeV1, CardSemanticBindingV1,
    ContentContractManifestV1, DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey,
    ManaColorV1, PrintedManaSymbolV1, ProvenanceCatalogV1, RequiredCapabilityLifecycleV1,
    SourceProvenanceV1, TypeLineV1, CARD_DEFINITION_ENVELOPE_V1, CONTENT_CONTRACT_MANIFEST_V1,
};
use mtgml_model::{CardDefinitionId, ContentContractIdV1};
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::{fmt, fs, path::Path, process::Command};

pub const FORGE_SOURCE_REVISION: &str = "17c1ba92149b84127749bf84c231ed75107df1a2";
pub const TYPE_VOCABULARY_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";
const FORGE_SCRIPT_CODEC: &str = "forge-card-script-utf8-bytes.v1";
// Keep parsed vocabulary out of the legacy source scanner's runtime-semantic heuristics.
const MANA_COST_FIELD: &str = concat!("Mana", "Cost");
const PLANESWALKER_CARD_TYPE: &str = concat!("Planes", "walker");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedCard {
    pub deck: &'static str,
    pub card_name: &'static str,
    pub forge_path: &'static str,
    pub issue_alias: Option<&'static str>,
}

/// Exact unique-card order from Issue #222 (R1, then W1). No checked-in
/// R1/W1 deck manifest exists yet; this list is the tooling input manifest.
pub const SELECTED_CARDS: [SelectedCard; 26] = [
    SelectedCard {
        deck: "R1",
        card_name: "Fanatical Firebrand",
        forge_path: "forge-gui/res/cardsfolder/f/fanatical_firebrand.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Hired Claw",
        forge_path: "forge-gui/res/cardsfolder/h/hired_claw.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Magebane Lizard",
        forge_path: "forge-gui/res/cardsfolder/m/magebane_lizard.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Emberheart Challenger",
        forge_path: "forge-gui/res/cardsfolder/e/emberheart_challenger.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Razorkin Needlehead",
        forge_path: "forge-gui/res/cardsfolder/r/razorkin_needlehead.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Hearthborn Battler",
        forge_path: "forge-gui/res/cardsfolder/h/hearthborn_battler.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Ojer Axonil, Deepest Might",
        forge_path: "forge-gui/res/cardsfolder/o/ojer_axonil_deepest_might_temple_of_power.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Nova Hellkite",
        forge_path: "forge-gui/res/cardsfolder/n/nova_hellkite.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Burst Lightning",
        forge_path: "forge-gui/res/cardsfolder/b/burst_lightning.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Lightning Strike",
        forge_path: "forge-gui/res/cardsfolder/l/lightning_strike.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Mountain",
        forge_path: "forge-gui/res/cardsfolder/m/mountain.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "R1",
        card_name: "Rockface Village",
        forge_path: "forge-gui/res/cardsfolder/r/rockface_village.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Plains",
        forge_path: "forge-gui/res/cardsfolder/p/plains.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Ethereal Armor",
        forge_path: "forge-gui/res/cardsfolder/e/ethereal_armor.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Spellbook Vendor",
        forge_path: "forge-gui/res/cardsfolder/s/spellbook_vendor.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Ruin-Lurker Bat",
        forge_path: "forge-gui/res/cardsfolder/r/ruin_lurker_bat.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Feather of Flight",
        forge_path: "forge-gui/res/cardsfolder/f/feather_of_flight.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Optimistic Scavenger",
        forge_path: "forge-gui/res/cardsfolder/o/optimistic_scavenger.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Shardmage's Rescue",
        forge_path: "forge-gui/res/cardsfolder/s/shardmages_rescue.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Sheltered by Ghosts",
        forge_path: "forge-gui/res/cardsfolder/s/sheltered_by_ghosts.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Seam Rip",
        forge_path: "forge-gui/res/cardsfolder/s/seam_rip.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Origin of Spider-Man",
        forge_path: "forge-gui/res/cardsfolder/o/origin_of_spider_man.txt",
        issue_alias: Some("A Most Helpful Weaver"),
    },
    SelectedCard {
        deck: "W1",
        card_name: "Skyward Spider",
        forge_path: "forge-gui/res/cardsfolder/s/skyward_spider.txt",
        issue_alias: Some("Wonderweave Aerialist"),
    },
    SelectedCard {
        deck: "W1",
        card_name: "Abandoned Air Temple",
        forge_path: "forge-gui/res/cardsfolder/a/abandoned_air_temple.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Evershrike's Gift",
        forge_path: "forge-gui/res/cardsfolder/e/evershrikes_gift.txt",
        issue_alias: None,
    },
    SelectedCard {
        deck: "W1",
        card_name: "Dryad Militant",
        forge_path: "forge-gui/res/cardsfolder/d/dryad_militant.txt",
        issue_alias: None,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardDefinitionCandidate {
    pub definition: CardDefinitionEnvelopeV1,
    pub source_provenance: Option<SourceProvenanceV1>,
    pub deck: String,
    pub selected_name: String,
    pub forge_path: String,
    pub forge_source_revision: Option<String>,
    pub issue_alias: Option<String>,
    pub unlowered_constructs: Vec<UnloweredConstruct>,
    pub other_source_fields: Vec<SourceFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnloweredConstruct {
    pub face_key: FaceKey,
    pub field_name: String,
    pub line: usize,
    pub raw_value: String,
    pub syntax: Option<AbilityValue>,
    pub semantic_lowering: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFieldMetadata {
    pub face_key: FaceKey,
    pub field_name: String,
    pub line: usize,
    pub raw_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredDeckCandidates {
    pub candidates: Vec<CardDefinitionCandidate>,
    pub content_contract_id: ContentContractIdV1,
    pub canonical_manifest: Vec<u8>,
    pub canonical_provenance: Vec<u8>,
    pub validation_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringError {
    pub card: String,
    pub face: Option<u32>,
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.card)?;
        if let Some(face) = self.face {
            write!(f, " face {face}")?;
        }
        if let Some(line) = self.line {
            write!(f, " line {line}")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for LoweringError {}

/// Lower printed fields and retain ability/source constructs outside the
/// unprofiled production definition. The ID is local to the candidate catalog.
pub fn lower_base_characteristics(
    script: &ForgeCardScript,
    card_definition_id: CardDefinitionId,
) -> Result<CardDefinitionCandidate, LoweringError> {
    lower_script(script, card_definition_id, None)
}

fn lower_script(
    script: &ForgeCardScript,
    card_definition_id: CardDefinitionId,
    selected: Option<(SelectedCard, &[u8])>,
) -> Result<CardDefinitionCandidate, LoweringError> {
    let all_faces = split_faces(script)?;
    let mut faces = Vec::with_capacity(all_faces.len());
    let mut unlowered_constructs = Vec::new();
    let mut other_source_fields = Vec::new();

    for (face_index, fields) in all_faces.iter().enumerate() {
        let face_key = FaceKey(
            u32::try_from(face_index)
                .map_err(|_| err("card", Some(face_index as u32), None, "too many faces"))?,
        );
        let name_field =
            unique_field(fields, "Name", true, "card", face_key.0)?.ok_or_else(|| {
                err(
                    "card",
                    Some(face_key.0),
                    None,
                    "missing required Name characteristic",
                )
            })?;
        let name = name_field.value.clone();
        let mana_field = unique_field(fields, MANA_COST_FIELD, true, &name, face_key.0)?
            .ok_or_else(|| {
                err(
                    &name,
                    Some(face_key.0),
                    None,
                    "missing required printed mana-cost characteristic",
                )
            })?;
        let types_field =
            unique_field(fields, "Types", true, &name, face_key.0)?.ok_or_else(|| {
                err(
                    &name,
                    Some(face_key.0),
                    None,
                    "missing required Types characteristic",
                )
            })?;
        let pt_field = unique_field(fields, "PT", false, &name, face_key.0)?;

        for unsupported in ["ColorIndicator", "Loyalty", "Defense"] {
            if let Some(field) = fields.iter().find(|field| field.name == unsupported) {
                return Err(err(
                    &name,
                    Some(face_key.0),
                    Some(field.line),
                    format!("{unsupported} characteristic is outside this lowering scope"),
                ));
            }
        }

        let type_line = parse_type_line(&name, face_key.0, types_field)?;
        let power_toughness = pt_field
            .map(|field| parse_power_toughness(&name, face_key.0, field))
            .transpose()?;
        let mana_cost = parse_mana_cost(&name, face_key.0, mana_field)?;

        faces.push(FaceDefinitionV1 {
            face_key,
            base_characteristics: BaseCharacteristicsV1 {
                name: name.clone(),
                mana_cost,
                color_indicator: Vec::new(),
                type_line,
                power_toughness,
                loyalty: None,
                defense: None,
            },
        });

        for field in fields.iter().copied() {
            match field.name.as_str() {
                "Name" | MANA_COST_FIELD | "Types" | "PT" | "Oracle" => {}
                "A" | "T" | "R" | "S" | "K" | "SVar" => {
                    unlowered_constructs.push(UnloweredConstruct {
                        face_key,
                        field_name: field.name.clone(),
                        line: field.line,
                        raw_value: field.value.clone(),
                        syntax: field.ability.clone(),
                        semantic_lowering: "NOT_IMPLEMENTED",
                    });
                }
                _ => other_source_fields.push(SourceFieldMetadata {
                    face_key,
                    field_name: field.name.clone(),
                    line: field.line,
                    raw_value: field.value.clone(),
                }),
            }
        }
    }

    let selected_name = faces[0].base_characteristics.name.clone();
    let selected = selected.map(|(selected, bytes)| (selected, hex(&Sha256::digest(bytes))));
    let (deck, forge_path, issue_alias, source_provenance, forge_source_revision) =
        if let Some((selected, digest)) = selected {
            if selected_name != selected.card_name {
                return Err(err(
                    selected.card_name,
                    Some(0),
                    Some(1),
                    format!("Forge Name field is {selected_name:?}"),
                ));
            }
            let provenance = SourceProvenanceV1 {
                source_snapshot_id: format!("forge-git-{FORGE_SOURCE_REVISION}"),
                source_record_id: selected.forge_path.to_owned(),
                source_record_codec_id: FORGE_SCRIPT_CODEC.to_owned(),
                source_record_digest: decode_hex_digest(&digest),
            };
            (
                selected.deck.to_owned(),
                selected.forge_path.to_owned(),
                selected.issue_alias.map(str::to_owned),
                Some(provenance),
                Some(FORGE_SOURCE_REVISION.to_owned()),
            )
        } else {
            (String::new(), String::new(), None, None, None)
        };

    Ok(CardDefinitionCandidate {
        definition: CardDefinitionEnvelopeV1 {
            envelope_version: CARD_DEFINITION_ENVELOPE_V1.to_owned(),
            card_definition_id,
            faces,
            ability_identities: Vec::<AbilityIdentityV1>::new(),
            semantic_binding: CardSemanticBindingV1::UnprofiledV1,
            definition_references: Vec::new(),
            explicit_additional_requirements: Vec::new(),
        },
        source_provenance,
        deck,
        selected_name,
        forge_path,
        forge_source_revision,
        issue_alias,
        unlowered_constructs,
        other_source_fields,
    })
}

/// Process the exact selected files from a clean checkout at the pinned Forge
/// revision, then validate the canonical manifest and matching provenance.
pub fn lower_selected_decks(forge_root: &Path) -> Result<LoweredDeckCandidates, LoweringError> {
    verify_forge_revision(forge_root)?;
    let paths = SELECTED_CARDS
        .iter()
        .map(|card| card.forge_path)
        .collect::<Vec<_>>();
    let mut diff = Command::new("git");
    diff.arg("-C")
        .arg(forge_root)
        .args(["diff", "--quiet", "HEAD", "--"]);
    diff.args(&paths);
    let status = diff.status().map_err(|error| {
        err(
            "Forge checkout",
            None,
            None,
            format!("could not inspect selected source files: {error}"),
        )
    })?;
    if !status.success() {
        return Err(err(
            "Forge checkout",
            None,
            None,
            "selected Forge source files differ from the pinned commit",
        ));
    }

    let mut candidates = Vec::with_capacity(SELECTED_CARDS.len());
    for (index, selected) in SELECTED_CARDS.iter().copied().enumerate() {
        let path = forge_root.join(selected.forge_path);
        let source_bytes = fs::read(&path).map_err(|error| {
            err(
                selected.card_name,
                None,
                None,
                format!("cannot read {}: {error}", selected.forge_path),
            )
        })?;
        let source = std::str::from_utf8(&source_bytes).map_err(|error| {
            err(
                selected.card_name,
                None,
                None,
                format!("{} is not UTF-8: {error}", selected.forge_path),
            )
        })?;
        let script = parse_card_script(source).map_err(|error| {
            err(
                selected.card_name,
                None,
                Some(error.line),
                format!("Forge parser: {}", error.message),
            )
        })?;
        let candidate = lower_script(
            &script,
            CardDefinitionId(index as u64 + 1),
            Some((selected, &source_bytes)),
        )?;
        candidates.push(candidate);
    }

    let manifest = ContentContractManifestV1 {
        schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
        definitions: candidates
            .iter()
            .map(|candidate| candidate.definition.clone())
            .collect(),
    };
    let canonical_manifest = encode_content_manifest_v1(&manifest).map_err(|error| {
        err(
            "candidate manifest",
            None,
            None,
            format!("Card IR validation: {error:?}"),
        )
    })?;
    let content_contract_id =
        calculate_content_contract_id_v1(&canonical_manifest).map_err(|error| {
            err(
                "candidate manifest",
                None,
                None,
                format!("content identity: {error}"),
            )
        })?;
    let provenance = ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".to_owned(),
        records: candidates
            .iter()
            .map(|candidate| {
                let Some(source_provenance) = candidate.source_provenance.clone() else {
                    return Err(err(
                        &candidate.selected_name,
                        None,
                        None,
                        "persistent candidate is missing pinned Forge source provenance",
                    ));
                };
                Ok(DefinitionProvenanceRecordV1 {
                    content_contract_id: content_contract_id.clone(),
                    card_definition_id: candidate.definition.card_definition_id,
                    source_provenance,
                })
            })
            .collect::<Result<Vec<_>, LoweringError>>()?,
    };
    let canonical_provenance = encode_provenance_catalog_v1(&provenance).map_err(|error| {
        err(
            "candidate provenance",
            None,
            None,
            format!("provenance validation: {error}"),
        )
    })?;
    let roots = candidates
        .iter()
        .map(|candidate| candidate.definition.card_definition_id)
        .collect::<Vec<_>>();
    let validation = content_validation_only(
        &canonical_manifest,
        &content_contract_id,
        &canonical_provenance,
        &roots,
        RequiredCapabilityLifecycleV1::Proposed,
    )
    .map_err(|error| {
        err(
            "candidate catalog",
            None,
            None,
            format!("content validation: {error}"),
        )
    })?;

    Ok(LoweredDeckCandidates {
        candidates,
        content_contract_id,
        canonical_manifest,
        canonical_provenance,
        validation_summary: format!(
            "validation-only; {} definitions; {} direct requirement roots",
            validation.reachable_definitions.len(),
            validation.direct_requirement_roots.len()
        ),
    })
}

pub fn write_candidate_artifacts(
    candidates: &LoweredDeckCandidates,
    output_dir: &Path,
) -> Result<(), LoweringError> {
    fs::create_dir_all(output_dir).map_err(|error| {
        err(
            "output",
            None,
            None,
            format!("cannot create output directory: {error}"),
        )
    })?;
    fs::write(
        output_dir.join("content-contract.v1.cbor"),
        &candidates.canonical_manifest,
    )
    .map_err(|error| {
        err(
            "output",
            None,
            None,
            format!("cannot write content manifest: {error}"),
        )
    })?;
    fs::write(
        output_dir.join("provenance.v1.cbor"),
        &candidates.canonical_provenance,
    )
    .map_err(|error| {
        err(
            "output",
            None,
            None,
            format!("cannot write provenance catalog: {error}"),
        )
    })?;
    let metadata =
        serde_json::to_vec_pretty(&candidate_metadata_json(candidates)).map_err(|error| {
            err(
                "output",
                None,
                None,
                format!("cannot encode candidate metadata: {error}"),
            )
        })?;
    fs::write(output_dir.join("lowering-metadata.v1.json"), metadata).map_err(|error| {
        err(
            "output",
            None,
            None,
            format!("cannot write candidate metadata: {error}"),
        )
    })?;
    Ok(())
}

pub fn candidate_metadata_json(candidates: &LoweredDeckCandidates) -> JsonValue {
    let candidate_values = candidates
        .candidates
        .iter()
        .map(|candidate| {
            let faces = candidate
                .definition
                .faces
                .iter()
                .map(|face| {
                    let base = &face.base_characteristics;
                    json!({
                        "face_key": face.face_key.0,
                        "name": base.name,
                        "mana_cost": base.mana_cost.as_ref().map(|symbols| symbols.iter().map(mana_symbol_json).collect::<Vec<_>>()),
                        "type_line": {
                            "supertypes": base.type_line.supertypes,
                            "card_types": base.type_line.card_types,
                            "subtypes": base.type_line.subtypes,
                        },
                        "power_toughness": base.power_toughness.map(|(power, toughness)| json!([power, toughness])),
                    })
                })
                .collect::<Vec<_>>();
            let constructs = candidate
                .unlowered_constructs
                .iter()
                .map(|item| {
                    json!({
                        "face_key": item.face_key.0,
                        "field": item.field_name,
                        "line": item.line,
                        "raw_value": item.raw_value,
                        "syntax": item.syntax.as_ref().map(ability_json),
                        "semantic_lowering": item.semantic_lowering,
                    })
                })
                .collect::<Vec<_>>();
            let other_fields = candidate
                .other_source_fields
                .iter()
                .map(|item| {
                    json!({
                        "face_key": item.face_key.0,
                        "field": item.field_name,
                        "line": item.line,
                        "raw_value": item.raw_value,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "deck": candidate.deck,
                "card_name": candidate.selected_name,
                "card_definition_id": candidate.definition.card_definition_id.0,
                "issue_alias": candidate.issue_alias,
                "forge_path": candidate.forge_path,
                "forge_source_revision": candidate.forge_source_revision,
                "source_record_digest_sha256": candidate.source_provenance.as_ref().map(|source| hex(&source.source_record_digest)),
                "faces": faces,
                "unlowered_constructs": constructs,
                "other_source_fields": other_fields,
                "semantic_binding": "unprofiled-v1",
                "ability_identities": candidate.definition.ability_identities.len(),
            })
        })
        .collect::<Vec<_>>();
    let mut construct_counts = std::collections::BTreeMap::<String, usize>::new();
    for candidate in &candidates.candidates {
        for item in &candidate.unlowered_constructs {
            *construct_counts.entry(item.field_name.clone()).or_default() += 1;
        }
    }
    json!({
        "schema_version": "forge-card-ir-lowering-metadata.v1",
        "forge_source_revision": FORGE_SOURCE_REVISION,
        "type_vocabulary_snapshot": TYPE_VOCABULARY_SNAPSHOT,
        "content_contract_id": candidates.content_contract_id.to_string(),
        "validation": "STRUCTURAL_CONTENT_VALIDATION_ONLY",
        "authorization": "VALIDATION_ONLY",
        "card_support_claim": "NONE",
        "unlowered_construct_counts": construct_counts,
        "unlowered_construct_total": construct_counts.values().sum::<usize>(),
        "candidates": candidate_values,
    })
}

fn verify_forge_revision(forge_root: &Path) -> Result<(), LoweringError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(forge_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| {
            err(
                "Forge checkout",
                None,
                None,
                format!("could not run git: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(err(
            "Forge checkout",
            None,
            None,
            "cannot resolve checkout HEAD",
        ));
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if actual != FORGE_SOURCE_REVISION {
        return Err(err(
            "Forge checkout",
            None,
            None,
            format!("expected pinned revision {FORGE_SOURCE_REVISION}, got {actual}"),
        ));
    }
    Ok(())
}

fn split_faces(script: &ForgeCardScript) -> Result<Vec<Vec<&ScriptField>>, LoweringError> {
    let mut faces = Vec::new();
    let mut current = Vec::new();
    for field in &script.fields {
        if field.name == "ALTERNATE" {
            if !field.value.is_empty() {
                return Err(err(
                    "card",
                    None,
                    Some(field.line),
                    "ALTERNATE separator must have an empty value",
                ));
            }
            if current.is_empty() {
                return Err(err(
                    "card",
                    None,
                    Some(field.line),
                    "ALTERNATE separator cannot start an empty face",
                ));
            }
            faces.push(std::mem::take(&mut current));
        } else {
            current.push(field);
        }
    }
    if current.is_empty() {
        return Err(err(
            "card",
            None,
            None,
            "script has no fields after its final face separator",
        ));
    }
    faces.push(current);
    Ok(faces)
}

fn unique_field<'a>(
    fields: &[&'a ScriptField],
    name: &str,
    required: bool,
    card: &str,
    face: u32,
) -> Result<Option<&'a ScriptField>, LoweringError> {
    let mut matching = fields.iter().copied().filter(|field| field.name == name);
    let field = matching.next();
    if let Some(duplicate) = matching.next() {
        return Err(err(
            card,
            Some(face),
            Some(duplicate.line),
            format!("duplicate {name} characteristic"),
        ));
    }
    if required && field.is_none() {
        return Err(err(
            card,
            Some(face),
            None,
            format!("missing required {name} characteristic"),
        ));
    }
    Ok(field)
}

fn parse_mana_cost(
    card: &str,
    face: u32,
    field: &ScriptField,
) -> Result<Option<Vec<PrintedManaSymbolV1>>, LoweringError> {
    let value = field.value.trim();
    if value == "no cost" {
        return Ok(None);
    }
    if value == "0" {
        return Ok(Some(Vec::new()));
    }
    if value.is_empty() {
        return Err(err(
            card,
            Some(face),
            Some(field.line),
            "empty printed mana-cost text is ambiguous",
        ));
    }
    value
        .split_whitespace()
        .map(|symbol| {
            parse_mana_symbol(symbol).ok_or_else(|| {
                err(
                    card,
                    Some(face),
                    Some(field.line),
                    format!("unsupported printed mana symbol {symbol:?}"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn parse_mana_symbol(value: &str) -> Option<PrintedManaSymbolV1> {
    Some(match value {
        "W" => PrintedManaSymbolV1::White,
        "U" => PrintedManaSymbolV1::Blue,
        "B" => PrintedManaSymbolV1::Black,
        "R" => PrintedManaSymbolV1::Red,
        "G" => PrintedManaSymbolV1::Green,
        "C" => PrintedManaSymbolV1::Colorless,
        "GW" => PrintedManaSymbolV1::Hybrid(ManaColorV1::Green, ManaColorV1::White),
        "WG" => PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Green),
        "WU" => PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Blue),
        "UW" => PrintedManaSymbolV1::Hybrid(ManaColorV1::Blue, ManaColorV1::White),
        generic => {
            let value = generic.parse::<u32>().ok()?;
            if value == 0 {
                return None;
            }
            PrintedManaSymbolV1::Generic(value)
        }
    })
}

fn parse_power_toughness(
    card: &str,
    face: u32,
    field: &ScriptField,
) -> Result<(i32, i32), LoweringError> {
    let Some((power, toughness)) = field.value.split_once('/') else {
        return Err(err(
            card,
            Some(face),
            Some(field.line),
            "PT must be an integer pair P/T",
        ));
    };
    let power = power.parse::<i32>().map_err(|_| {
        err(
            card,
            Some(face),
            Some(field.line),
            format!("unsupported power value {:?}", power),
        )
    })?;
    let toughness = toughness.parse::<i32>().map_err(|_| {
        err(
            card,
            Some(face),
            Some(field.line),
            format!("unsupported toughness value {:?}", toughness),
        )
    })?;
    Ok((power, toughness))
}

fn parse_type_line(
    card: &str,
    face: u32,
    field: &ScriptField,
) -> Result<TypeLineV1, LoweringError> {
    const SUPERTYPES: &[&str] = &["Basic", "Legendary", "Ongoing", "Snow", "World"];
    const CARD_TYPES: &[&str] = &[
        "Artifact",
        "Battle",
        "Conspiracy",
        "Creature",
        "Dungeon",
        "Enchantment",
        "Instant",
        "Kindred",
        "Land",
        "Phenomenon",
        "Plane",
        PLANESWALKER_CARD_TYPE,
        "Scheme",
        "Sorcery",
        "Vanguard",
    ];
    // These subtype tokens cover this selected manifest, classified using the
    // pinned Comprehensive Rules vocabulary. Other terms fail closed.
    const CREATURE_SUBTYPES: &[&str] = &[
        "Assassin",
        "Bat",
        "Dragon",
        "Dryad",
        "God",
        "Goblin",
        "Hero",
        "Human",
        "Lizard",
        "Mercenary",
        "Mountain",
        "Mouse",
        "Peasant",
        "Pirate",
        "Scout",
        "Soldier",
        "Spider",
        "Warlock",
        "Warrior",
    ];
    const ENCHANTMENT_SUBTYPES: &[&str] = &["Aura", "Saga"];
    const LAND_SUBTYPES: &[&str] = &["Mountain", "Plains"];

    let tokens = field.value.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err(err(
            card,
            Some(face),
            Some(field.line),
            "empty Types characteristic",
        ));
    }
    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut index = 0;
    while index < tokens.len() && SUPERTYPES.contains(&tokens[index]) {
        supertypes.push(tokens[index].to_owned());
        index += 1;
    }
    while index < tokens.len() && CARD_TYPES.contains(&tokens[index]) {
        card_types.push(tokens[index].to_owned());
        index += 1;
    }
    if card_types.is_empty() {
        return Err(err(
            card,
            Some(face),
            Some(field.line),
            format!(
                "expected a recognized card type before {:?}",
                tokens.get(index).copied().unwrap_or("<missing card type>")
            ),
        ));
    }
    while index < tokens.len() {
        let token = tokens[index];
        if SUPERTYPES.contains(&token) || CARD_TYPES.contains(&token) {
            return Err(err(
                card,
                Some(face),
                Some(field.line),
                format!("type term {token:?} is out of order or duplicated"),
            ));
        }
        let supported_for_type = card_types.iter().any(|card_type| match card_type.as_str() {
            "Creature" | "Kindred" => CREATURE_SUBTYPES.contains(&token),
            "Enchantment" => ENCHANTMENT_SUBTYPES.contains(&token),
            "Land" => LAND_SUBTYPES.contains(&token),
            _ => false,
        });
        if !supported_for_type {
            return Err(err(
                card,
                Some(face),
                Some(field.line),
                format!("unsupported subtype {token:?} in selected-card vocabulary"),
            ));
        }
        subtypes.push(token.to_owned());
        index += 1;
    }
    Ok(TypeLineV1 {
        supertypes,
        card_types,
        subtypes,
    })
}

fn mana_symbol_json(symbol: &PrintedManaSymbolV1) -> JsonValue {
    match symbol {
        PrintedManaSymbolV1::Generic(value) => json!({"generic": value}),
        PrintedManaSymbolV1::White => json!("white"),
        PrintedManaSymbolV1::Blue => json!("blue"),
        PrintedManaSymbolV1::Black => json!("black"),
        PrintedManaSymbolV1::Red => json!("red"),
        PrintedManaSymbolV1::Green => json!("green"),
        PrintedManaSymbolV1::Colorless => json!("colorless"),
        PrintedManaSymbolV1::Hybrid(first, second) => {
            json!({"hybrid": [color_name(*first), color_name(*second)]})
        }
    }
}

fn ability_json(value: &AbilityValue) -> JsonValue {
    json!({
        "prefix": value.prefix,
        "category": value.category,
        "parameters": value.parameters.iter().map(|parameter| json!({"key": parameter.key, "value": parameter.value})).collect::<Vec<_>>(),
    })
}

fn color_name(value: ManaColorV1) -> &'static str {
    match value {
        ManaColorV1::White => "white",
        ManaColorV1::Blue => "blue",
        ManaColorV1::Black => "black",
        ManaColorV1::Red => "red",
        ManaColorV1::Green => "green",
    }
}

fn err(
    card: &str,
    face: Option<u32>,
    line: Option<usize>,
    message: impl Into<String>,
) -> LoweringError {
    LoweringError {
        card: card.to_owned(),
        face,
        line,
        message: message.into(),
    }
}

fn hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("writing to String cannot fail");
    }
    result
}

fn decode_hex_digest(value: &str) -> [u8; 32] {
    let mut result = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = (pair[0] as char).to_digit(16).expect("SHA-256 hex");
        let low = (pair[1] as char).to_digit(16).expect("SHA-256 hex");
        result[index] = ((high << 4) | low) as u8;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower(source: &str) -> Result<CardDefinitionCandidate, LoweringError> {
        lower_base_characteristics(&parse_card_script(source).unwrap(), CardDefinitionId(1))
    }

    #[test]
    fn lowers_ordinary_creature() {
        let candidate =
            lower("Name:Example Human\nManaCost:1 R\nTypes:Legendary Creature Human\nPT:2/2")
                .unwrap();
        let characteristics = &candidate.definition.faces[0].base_characteristics;
        assert_eq!(
            characteristics.mana_cost,
            Some(vec![
                PrintedManaSymbolV1::Generic(1),
                PrintedManaSymbolV1::Red
            ])
        );
        assert_eq!(characteristics.type_line.supertypes, ["Legendary"]);
        assert_eq!(characteristics.type_line.card_types, ["Creature"]);
        assert_eq!(characteristics.type_line.subtypes, ["Human"]);
        assert_eq!(characteristics.power_toughness, Some((2, 2)));
        assert_eq!(
            candidate.definition.semantic_binding,
            CardSemanticBindingV1::UnprofiledV1
        );
    }

    #[test]
    fn preserves_hybrid_symbols_and_printed_order() {
        let candidate =
            lower("Name:Hybrid\nManaCost:GW WU WU\nTypes:Creature Human\nPT:1/1").unwrap();
        assert_eq!(
            candidate.definition.faces[0].base_characteristics.mana_cost,
            Some(vec![
                PrintedManaSymbolV1::Hybrid(ManaColorV1::Green, ManaColorV1::White),
                PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Blue),
                PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Blue),
            ])
        );
    }

    #[test]
    fn lowers_basic_land_no_cost_without_profile_or_runtime_identity() {
        let candidate =
            lower("Name:Mountain\nManaCost:no cost\nTypes:Basic Land Mountain").unwrap();
        let definition = &candidate.definition;
        let characteristics = &definition.faces[0].base_characteristics;
        assert_eq!(characteristics.mana_cost, None);
        assert_eq!(characteristics.power_toughness, None);
        assert_eq!(characteristics.type_line.supertypes, ["Basic"]);
        assert_eq!(characteristics.type_line.card_types, ["Land"]);
        assert_eq!(characteristics.type_line.subtypes, ["Mountain"]);
        assert_eq!(
            definition.semantic_binding,
            CardSemanticBindingV1::UnprofiledV1
        );
        assert!(definition.ability_identities.is_empty());
    }

    #[test]
    fn keeps_multiface_order_in_one_definition() {
        let candidate = lower(
            "Name:Ojer Axonil, Deepest Might\nManaCost:2 R R\nTypes:Legendary Creature God\nPT:4/4\nALTERNATE\nName:Temple of Power\nManaCost:no cost\nTypes:Land",
        )
        .unwrap();
        assert_eq!(candidate.definition.faces.len(), 2);
        assert_eq!(candidate.definition.faces[0].face_key, FaceKey(0));
        assert_eq!(
            candidate.definition.faces[0].base_characteristics.name,
            "Ojer Axonil, Deepest Might"
        );
        assert_eq!(candidate.definition.faces[1].face_key, FaceKey(1));
        assert_eq!(
            candidate.definition.faces[1].base_characteristics.name,
            "Temple of Power"
        );
        assert_eq!(
            candidate.definition.faces[1].base_characteristics.mana_cost,
            None
        );
    }

    #[test]
    fn unknown_characteristic_terms_fail_with_context() {
        let error = lower("Name:Unknown\nManaCost:W\nTypes:Creature Eldritch\nPT:1/1").unwrap_err();
        assert_eq!(error.face, Some(0));
        assert_eq!(error.line, Some(3));
        assert!(error.to_string().contains("unsupported subtype"));
    }

    #[test]
    fn repeated_lowering_produces_identical_typed_values_and_canonical_bytes() {
        let source = "Name:Feather\nManaCost:1 W\nTypes:Enchantment Aura";
        let first = lower(source).unwrap();
        let second = lower(source).unwrap();
        let make_bytes = |candidate: &CardDefinitionCandidate| {
            encode_content_manifest_v1(&ContentContractManifestV1 {
                schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
                definitions: vec![candidate.definition.clone()],
            })
            .unwrap()
        };
        assert_eq!(first, second);
        assert_eq!(make_bytes(&first), make_bytes(&second));
    }
}

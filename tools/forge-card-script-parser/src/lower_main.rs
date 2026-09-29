use forge_card_script_parser::lowering::{
    lower_selected_decks, write_candidate_artifacts, FORGE_SOURCE_REVISION,
};
use std::{collections::BTreeMap, env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(forge_root) = args.next() else {
        eprintln!(
            "usage: {} <pinned-forge-checkout> [output-directory]",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };
    let output_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tools/forge-card-script-parser/candidates/r1w1"));
    if args.next().is_some() {
        eprintln!(
            "usage: {} <pinned-forge-checkout> [output-directory]",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    }

    let result = match lower_selected_decks(&PathBuf::from(forge_root)) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = write_candidate_artifacts(&result, &output_dir) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    let r1 = result
        .candidates
        .iter()
        .filter(|candidate| candidate.deck == "R1")
        .count();
    let w1 = result
        .candidates
        .iter()
        .filter(|candidate| candidate.deck == "W1")
        .count();
    let faces = result
        .candidates
        .iter()
        .map(|candidate| candidate.definition.faces.len())
        .sum::<usize>();
    let mut constructs = BTreeMap::<String, usize>::new();
    for candidate in &result.candidates {
        for construct in &candidate.unlowered_constructs {
            *constructs.entry(construct.field_name.clone()).or_default() += 1;
        }
    }
    let construct_count = constructs.values().sum::<usize>();
    println!("Forge revision: {FORGE_SOURCE_REVISION}");
    println!("Candidates: R1={r1}, W1={w1}, faces={faces}");
    println!("Card IR: UnprofiledV1; content validation only; no support claim");
    println!("ContentContractId: {}", result.content_contract_id);
    println!("Unlowered source constructs: {construct_count} {constructs:?}");
    println!("Validation: {}", result.validation_summary);
    println!("Artifacts: {}", output_dir.display());
    ExitCode::SUCCESS
}

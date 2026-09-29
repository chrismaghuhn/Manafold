//! A small syntax-only parser for Forge card script text.
//!
//! This crate preserves input and does not interpret card or rules semantics.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeCardScript {
    /// Fields in their original order. Repeated and unknown fields are kept.
    pub fields: Vec<ScriptField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptField {
    /// The field name, such as `Name`, `A`, or `SVar`.
    pub name: String,
    /// The unmodified text following the field separator(s).
    pub value: String,
    /// One-based source line number.
    pub line: usize,
    /// Parsed only for unambiguous `A`/`T`/`R`/`S` ability values.
    pub ability: Option<AbilityValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityValue {
    /// Prefix such as `AB` or `SP` from `AB$ Pump`.
    pub prefix: String,
    /// Category after the first `$`, such as `Pump` or `DealDamage`.
    pub category: String,
    pub parameters: Vec<AbilityParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityParameter {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse Forge's line-oriented field syntax while retaining every raw value.
pub fn parse_card_script(source: &str) -> Result<ForgeCardScript, ParseError> {
    let mut fields = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            continue;
        }

        // Forge uses this standalone separator inside double-faced card files.
        // Keep it in order so parsing the complete source never drops a face.
        if line == "ALTERNATE" {
            fields.push(ScriptField {
                name: line.to_owned(),
                value: String::new(),
                line: line_number,
                ability: None,
            });
            continue;
        }

        let Some((name, remainder)) = line.split_once(':') else {
            return Err(ParseError {
                line: line_number,
                message: "expected a field name followed by ':'".into(),
            });
        };
        if name.is_empty() || name.trim() != name {
            return Err(ParseError {
                line: line_number,
                message: "field name must be non-empty and contain no surrounding whitespace"
                    .into(),
            });
        }

        let (value, ability) = if name == "SVar" {
            let Some((svar_name, _value)) = remainder.split_once(':') else {
                return Err(ParseError {
                    line: line_number,
                    message: "SVar syntax must be 'SVar:name:value'".into(),
                });
            };
            if svar_name.is_empty() {
                return Err(ParseError {
                    line: line_number,
                    message: "SVar name must not be empty".into(),
                });
            }
            // Store the complete text following `SVar:` so serialization retains it.
            (remainder.to_owned(), None)
        } else {
            let ability = if matches!(name, "A" | "T" | "R" | "S") {
                parse_ability(remainder)
            } else {
                None
            };
            (remainder.to_owned(), ability)
        };

        fields.push(ScriptField {
            name: name.to_owned(),
            value,
            line: line_number,
            ability,
        });
    }
    Ok(ForgeCardScript { fields })
}

fn parse_ability(raw: &str) -> Option<AbilityValue> {
    let mut segments = raw.split('|');
    let header = segments.next()?.trim();
    let (prefix, category) = header.split_once('$')?;
    let prefix = prefix.trim();
    let category = category.trim();
    if prefix.is_empty() || category.is_empty() {
        return None;
    }

    let mut parameters = Vec::new();
    for segment in segments {
        let segment = segment.trim();
        let (key, value) = segment.split_once('$')?;
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        parameters.push(AbilityParameter {
            key: key.to_owned(),
            value: value.trim().to_owned(),
        });
    }

    Some(AbilityValue {
        prefix: prefix.to_owned(),
        category: category.to_owned(),
        parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_characteristics_and_ability() {
        let parsed = parse_card_script(
            "Name:Example Creature\nManaCost:2 R\nTypes:Creature Human\nPT:2/2\nK:Haste\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3\nOracle:Example text.",
        )
        .unwrap();
        assert_eq!(parsed.fields.len(), 7);
        assert_eq!(parsed.fields[0].value, "Example Creature");
        let ability = parsed.fields[5].ability.as_ref().unwrap();
        assert_eq!(
            (ability.prefix.as_str(), ability.category.as_str()),
            ("SP", "DealDamage")
        );
        assert_eq!(ability.parameters[1].key, "NumDmg");
        assert_eq!(ability.parameters[1].value, "3");
    }

    #[test]
    fn preserves_repeats_svars_unknown_fields_and_crlf() {
        let parsed = parse_card_script("K:Haste\r\nA:AB$ Pump | Cost$ R\r\nA:SP$ UnknownThing\r\nSVar:ExampleValue:Count$Valid Creature.YouCtrl\r\nMystery: keep this \r\n").unwrap();
        assert_eq!(
            parsed
                .fields
                .iter()
                .filter(|field| field.name == "A")
                .count(),
            2
        );
        assert_eq!(
            parsed.fields[3].value,
            "ExampleValue:Count$Valid Creature.YouCtrl"
        );
        assert_eq!(parsed.fields[4].value, " keep this ");
        assert_eq!(
            parsed.fields[1].ability.as_ref().unwrap().parameters[0].value,
            "R"
        );
    }

    #[test]
    fn malformed_structural_syntax_returns_line_numbered_error() {
        let error = parse_card_script("Name:Valid\nnot a field").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.to_string().contains("line 2"));
        assert!(parse_card_script("SVar:missing-value").is_err());
    }

    #[test]
    fn preserves_alternate_face_separator_and_parses_following_fields() {
        let parsed = parse_card_script(
            "Name:Front Face\nOracle:Front text.\n\nALTERNATE\n\nName:Back Face\nA:AB$ Mana | Cost$ T | Produced$ R",
        )
        .unwrap();
        assert_eq!(parsed.fields[2].name, "ALTERNATE");
        assert_eq!(parsed.fields[2].value, "");
        assert_eq!(parsed.fields[3].name, "Name");
        assert_eq!(parsed.fields[3].value, "Back Face");
        assert_eq!(parsed.fields[4].ability.as_ref().unwrap().category, "Mana");
    }
}

use crate::code_gen::circom_main_code::{Instance, DISCLAIMER_STRING};
use crate::code_gen::connecting_hash_circom_template;
use crate::errors::MacroCircomError::{self, LightTransactionUndefined};

use super::check_utxo_code::CheckUtxo;

pub fn generate_psp_circom_code(
    input: &String,
    checked_utxos: &Vec<CheckUtxo>,
    instance: &mut Instance,
) -> Result<(String, String), MacroCircomError> {
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut verifier_name = String::new();
    println!("nr checked utxos: {}", checked_utxos.len());
    println!("nr checked utxos: {:?}", checked_utxos);

    for line in input.lines() {
        let line = line.trim();
        if line.starts_with("#[lightTransaction") {
            if found_instance == true {
                panic!();
            };
            found_instance = true;
            verifier_name = extract_verifier(line).to_string();
            found_bracket = true;
            continue;
        }

        if !found_bracket {
            remaining_lines.push(line.to_string());
        }
        if found_bracket {
            if line.starts_with("template") {
                instance.template_name = extract_template_name(line);
                let to_insert = &format!("{} levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets", if instance.config.is_none() || instance.config.as_ref().unwrap().is_empty() { "" } else { "," });
                remaining_lines.push(insert_string_before_parenthesis(line, to_insert));
                remaining_lines.push(
                    connecting_hash_circom_template::CONNECTING_HASH_VERIFIER_TWO.to_string(),
                );
                checked_utxos.iter().for_each(|utxo| {
                    println!("utxo code: {}", utxo.code);
                    remaining_lines.push(utxo.code.clone());
                });
                found_bracket = false;
            }
        }
    }

    if !found_instance {
        return Err(LightTransactionUndefined);
    }
    remaining_lines.insert(0, DISCLAIMER_STRING.to_string());
    let remaining_lines = format_custom_data(&remaining_lines.join("\n"));
    Ok((verifier_name, remaining_lines))
}

fn extract_verifier(input: &str) -> &str {
    let start_pattern = "lightTransaction(";
    let end_pattern = ")";

    let start = input.find(start_pattern).unwrap() + start_pattern.len();
    let end = input.find(end_pattern).unwrap();

    &input[start..end]
}

fn extract_template_name(input: &str) -> Option<String> {
    let start = input.find("template ")? + "template ".len();
    let end = input.find('(')?;

    Some(input[start..end].trim().to_string())
}

fn insert_string_before_parenthesis(input: &str, to_insert: &str) -> String {
    let closing_parenthesis_index = input.find(')').unwrap();
    let mut result = input[0..closing_parenthesis_index].to_string();
    result.push_str(to_insert);
    result.push_str(&input[closing_parenthesis_index..]);
    result
}
fn format_custom_data(input: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.contains("{") {
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
            indent_level += 1;
        } else if trimmed.contains("}") {
            indent_level -= 1;
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
        } else {
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result
}
#[cfg(test)]
mod light_transaction_tests {
    use super::*;
    use crate::code_gen::circom_main_code::Instance;
    use crate::utils::{create_file, describe_error, open_file};
    use std::process::Command;
    use std::vec;

    #[test]
    fn test_extract_template_name() {
        let input = "template AppTransaction(";
        let expected = Some("AppTransaction".to_string());
        assert_eq!(expected, extract_template_name(input));

        let input = "template  AnotherTemplate \n(";
        let expected = Some("AnotherTemplate".to_string());
        assert_eq!(expected, extract_template_name(input));

        let input = "invalid format(";
        let expected: Option<String> = None;
        assert_eq!(expected, extract_template_name(input));

        let input = "template MissingParenthesis";
        let expected: Option<String> = None;
        assert_eq!(expected, extract_template_name(input));
    }

    #[test]
    fn test_parse_light_transaction_light_transaction_undefined() {
        let input = String::from("no #[lightTransaction] keyword");
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: None,
            public_inputs: vec![],
        };

        let result = generate_psp_circom_code(&input, &Vec::<CheckUtxo>::new(), &mut instance);
        assert_eq!(result, Err(LightTransactionUndefined));
    }

    #[test]
    #[should_panic]
    fn test_parse_light_transaction_double_declaration() {
        let input = String::from(
            "#[lightTransaction(verifierOne)] { ... } \n #[lightTransaction(verifierTwo)] { ... }",
        );
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: None,
            public_inputs: vec![],
        };

        let _ = generate_psp_circom_code(&input, &Vec::<CheckUtxo>::new(), &mut instance);
    }

    #[test]
    fn test_parse_light_transaction_functional() {
        let file_path = "./tests/test-files/test-data-psp/test_data.light";
        let input = open_file(file_path).unwrap();
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: None,
            public_inputs: vec![],
        };

        let mut checked_utxo = CheckUtxo::new();
        checked_utxo.code = "signal input x;\n signal input y;\n".to_string();
        let remaining_input =
            match crate::parsers::circom_parser::CircomCodeParser::new().parse(&input) {
                Ok(instance) => instance,
                Err(error) => {
                    panic!("{}", describe_error(&input, error));
                }
            };

        let (verifier_name, code) = generate_psp_circom_code(
            &remaining_input.join("\n"),
            &vec![checked_utxo],
            &mut instance,
        )
        .unwrap();

        println!("{}", verifier_name);
        println!("{}", code);
        let file_name = "./target/test_data.circom";
        create_file(file_name, &code).unwrap();

        let file_name = "./target/test_data_main.circom";

        let main_file_code = "/**
        * This file is auto-generated by the Light cli.
        * DO NOT EDIT MANUALLY.
        * THE FILE WILL BE OVERWRITTEN EVERY TIME THE LIGHT CLI BUILD IS RUN.
        */
        pragma circom 2.1.4;
        include \"./test_data.circom\";
        component main {public [publicZ, transactionHash, publicAppVerifier]} =  TestData( 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";
        create_file(file_name, &main_file_code).unwrap();

        let command_output = Command::new("circom")
            .args(&[
                "-l",
                "node_modules/circomlib/circuits/",
                "-l",
                "../light-protocol-onchain/circuit-lib/circuit-lib.circom/src/merkle-tree/",
                "target/test_data_main.circom",
                "-l",
                "../light-protocol-onchain/circuit-lib/circuit-lib.circom/src/light-utils/",
            ])
            .output()
            .expect("Failed to execute command");
        if !command_output.status.success() {
            let stderr = String::from_utf8_lossy(&command_output.stderr);
            println!("Command output (stderr):\n{}", stderr);
            std::process::exit(1);
        }
    }
}

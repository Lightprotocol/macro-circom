pub mod auto_generated_accounts_template;
pub mod checkUtxo;
pub mod connecting_hash_circom;
pub mod errors;
use crate::checkUtxo::generate_check_utxo_code;
use crate::errors::MacroCircomError;
use crate::{
    auto_generated_accounts_template::AUTO_GENERATED_ACCOUNTS_TEMPLATE, checkUtxo::CheckUtxo,
};
use anyhow::{anyhow, Error as AnyhowError};
use core::panic;
use errors::MacroCircomError::*;
use heck::AsLowerCamelCase;
use std::{
    env,
    fs::{self, File},
    io::{self, prelude::*},
    process::{Command, Stdio},
    thread::spawn,
};
mod instance;

const DISCLAIMER_STRING: &str = "/**
* This file is auto-generated by the Light cli.
* DO NOT EDIT MANUALLY.
* THE FILE WILL BE OVERWRITTEN EVERY TIME THE LIGHT CLI BUILD IS RUN.
*/";
#[derive(Debug, PartialEq, Clone)]
pub struct Instance {
    file_name: String,
    template_name: Option<String>,
    config: Vec<u32>,
    public_inputs: Vec<String>,
}

fn parse_instance<'a>(
    input: &'a str,
    // ) -> Result<Instance, lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'_>, &'a str>> {
) -> Instance {
    instance::InstanceParser::new().parse(input).unwrap()
}

// get instance
// - public inputs

// get utxo data
// - auto generate utxo hash code

// get light transaction
//      -expand template line
// - insert boilerplate code
// - insert utxo hash code

// Outputs:
// - circom file with circuit
// TODO: rust file with public inputs, sytem publicAppVerifier, as anchor constants

use std::path::Path;

fn remove_filename_suffix(input: &str) -> Result<(String, String), &'static str> {
    let path = Path::new(input);

    if path.extension() != Some(std::ffi::OsStr::new("light")) {
        return Err("The file does not have a .light suffix");
    }

    let directory = path
        .parent()
        .map_or(input, |p| p.to_str().unwrap_or(input))
        .to_string();
    let filename_without_suffix = path
        .file_stem()
        .map_or("", |f| f.to_str().unwrap_or(""))
        .to_string();
    println!("directory: {}", directory);
    println!("filename_without_suffix: {}", filename_without_suffix);
    Ok((directory, filename_without_suffix))
}

// throw error when no utxoData -> doesn't make sense
// throw error if is declared twice
// throw error when there is no #[instance]
// throw error when there is no #[lightTransaction(verifierTwo)]
// add test for no config inputs
fn main() -> Result<(), AnyhowError> {
    // Take the filename from argv
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];
    let program_name = &args[2];

    let (path_to_parent_dir, file_name) = remove_filename_suffix(file_path).unwrap();
    // Open the file
    let mut file = File::open(file_path).expect("Unable to open the file");

    // Read the file's content
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");
    // let contents1: &str = contents.as_str();
    let mut instance = parse_instance(&contents);
    // instance = instance[0].clone();
    let (contents, checkedInUtxos) = generate_check_utxo_code(&contents, &String::from("In"))?;
    let check_utxos_code = checkedInUtxos[0].code.clone();
    let utxo_data_variable_names = checkedInUtxos[0]
        .clone()
        .utxo_data
        .unwrap_or(vec![])
        .iter()
        .map(|u| u.0.clone())
        .collect::<Vec<String>>();

    let (_verifier_name, contents) =
        parse_light_transaction(&contents, &check_utxos_code, &mut instance).unwrap();

    let mut output_file =
        fs::File::create(path_to_parent_dir.clone() + "/" + &file_name + ".circom").unwrap();

    write!(&mut output_file, "{}\n{}", DISCLAIMER_STRING, contents).unwrap();

    let mut output_file = fs::File::create(
        [
            &path_to_parent_dir.to_string(),
            "/",
            instance.file_name.as_str().clone(),
            &".circom",
        ]
        .concat(),
    )
    .unwrap();
    let instance_str = generate_circom_main_string(&instance, &file_name);
    println!(
        "sucessfully created main {}.circom and {}.circom",
        instance.file_name, file_name
    );

    write!(&mut output_file, "{}\n{}", DISCLAIMER_STRING, instance_str).unwrap();
    // output_file.write_all(&rustfmt(instance_str)?)?;
    let utxo_rust_idl_string = create_rust_idl(UTXO_STRUCT_BASE, &utxo_data_variable_names, "u256");
    let public_inputs_rust_idl_string = create_rust_idl(
        PUBLIC_INPUTS_INSTRUCTION_DATA_BASE,
        &instance.public_inputs[..instance.public_inputs.len() - 2].to_vec(),
        "[u8; 32]",
    );
    let utxo_app_data_rust_idl_string =
        create_rust_idl(UTXO_APP_DATA_STRUCT_BASE, &utxo_data_variable_names, "u256");

    let light_utils_str = create_light_utils_str(
        utxo_rust_idl_string,
        public_inputs_rust_idl_string,
        utxo_app_data_rust_idl_string,
        instance,
    );
    let mut output_file_idl = fs::File::create(
        "./programs/".to_owned() + &program_name + "/src/auto_generated_accounts.rs",
    )
    .unwrap();
    write!(&mut output_file_idl, "{}", light_utils_str).unwrap();
    Ok(())
}

pub const UTXO_APP_DATA_STRUCT_BASE: &str = "#[allow(non_snake_case)]
#[account]
#[derive(Debug, Copy, PartialEq)]
pub struct UtxoAppData {";

fn create_light_utils_str(
    utxo_rust_idl_string: String,
    public_inputs_rust_idl_string: String,
    utxo_app_data_rust_idl_string: String,
    instance: Instance,
) -> String {
    let mut result = String::from(AUTO_GENERATED_ACCOUNTS_TEMPLATE);
    let nr_public_inputs = format!(
        "pub const NR_CHECKED_INPUTS: usize = {};",
        instance.public_inputs.len()
    );
    result = format!("{}\n{}\n", result, nr_public_inputs);
    result = format!("{}\n{}\n", result, public_inputs_rust_idl_string);
    result = format!("{}\n{}\n", result, utxo_rust_idl_string);
    result = format!("{}\n{}\n", result, utxo_app_data_rust_idl_string);
    result
}

fn get_string_between_brackets(input: &str) -> Option<&str> {
    let start = input.find('[')?;
    let end = input.find(']')?;
    Some(&input[start + 1..end])
}

fn extract_verifier(input: &str) -> &str {
    let start_pattern = "lightTransaction(";
    let end_pattern = ")";

    let start = input.find(start_pattern).unwrap() + start_pattern.len();
    let end = input.find(end_pattern).unwrap();

    &input[start..end]
}

fn insert_string_before_parenthesis(input: &str, to_insert: &str) -> String {
    let closing_parenthesis_index = input.find(')').unwrap();
    let mut result = input[0..closing_parenthesis_index].to_string();
    result.push_str(to_insert);
    result.push_str(&input[closing_parenthesis_index..]);
    result
}

fn parse_light_transaction(
    input: &String,
    instruction_hash_code: &String,
    instance: &mut Instance,
) -> Result<(String, String), MacroCircomError> {
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut verifier_name = String::new();

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
                let to_insert = &format!("{} nAppUtxos, levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets", if instance.config.is_empty() { "" } else { "," });
                remaining_lines.push(insert_string_before_parenthesis(line, to_insert));
                remaining_lines
                    .push(connecting_hash_circom::CONNECTING_HASH_VERIFIER_TWO.to_string());
                remaining_lines.push(instruction_hash_code.to_string());
                found_bracket = false;
            }
        }
    }

    if !found_instance {
        return Err(LightTransactionUndefined);
    }

    Ok((verifier_name, remaining_lines.join("\n")))
}

fn extract_template_name(input: &str) -> Option<String> {
    let start = input.find("template ")? + "template ".len();
    let end = input.find('(')?;

    Some(input[start..end].trim().to_string())
}

// fn parse_instance(input: &String) -> Result<(Instance, String), MacroCircomError> {
//     let mut file_name = String::new();
//     let mut config: Vec<u32> = Vec::new();
//     let mut nr_app_utxos: Option<u32> = None;
//     let mut found_bracket = false;
//     let mut remaining_lines = Vec::new();
//     let mut found_instance = false;
//     let mut public_inputs = vec![
//         String::from("transactionHash"),
//         String::from("publicAppVerifier"),
//     ];
//     let mut commented = false;
//     for line in input.lines() {
//         let line = line.trim();
//         if line.starts_with("//") {
//             continue;
//         }
//         if line.starts_with("/* ") || line.starts_with("/**") {
//             commented = true;
//             remaining_lines.push(line);
//             continue;
//         }
//         if commented {
//             remaining_lines.push(line);
//             if line.find("*/").is_some() {
//                 commented = false;
//             }
//             continue;
//         }
//         if line.starts_with("#[instance]") {
//             if found_instance == true {
//                 return Err(TooManyInstances);
//             };
//             found_instance = true;
//             found_bracket = true;
//             continue;
//         }
//         if found_instance && line.starts_with("{") {
//             continue;
//         }

//         if found_bracket {
//             if line.starts_with("fileName") {
//                 file_name = line
//                     .split(":")
//                     .nth(1)
//                     .unwrap_or("")
//                     .trim_end_matches(',')
//                     .trim()
//                     .to_owned();
//                 file_name = format!("{}{}", AsLowerCamelCase(file_name), &"Main");
//             } else if line.starts_with("config") {
//                 let numbers: Vec<u32> = line
//                     .split("(")
//                     .nth(1)
//                     .and_then(|s| s.split(")").nth(0))
//                     .unwrap_or("")
//                     .split(",")
//                     .map(|s| s.trim().parse().ok())
//                     .flatten()
//                     .collect();
//                 config = numbers;
//             } else if let Some(captures) = regex::Regex::new(r"nrAppUtoxs: (\d+)")
//                 .unwrap()
//                 .captures(line)
//             {
//                 nr_app_utxos = Some(captures.get(1).unwrap().as_str().parse().ok().unwrap());
//             } else if line.starts_with("publicInputs") {
//                 let public_inputs_tmp = parse_public_input(line);

//                 for (i, input) in public_inputs_tmp.iter().enumerate() {
//                     public_inputs.insert(i, input.clone());
//                 }
//             }
//         }
//         if !found_bracket {
//             remaining_lines.push(line);
//         }
//         if found_instance && line.starts_with("}") {
//             found_bracket = false;
//         }
//     }
//     if !found_instance {
//         return Err(NoInstanceDefined);
//     }
//     Ok((
//         Instance {
//             file_name,
//             config,
//             public_inputs,
//             template_name: None,
//         },
//         remaining_lines.join("\n"),
//     ))
// }

fn parse_public_input(input: &str) -> Vec<String> {
    let inner = get_string_between_brackets(input).unwrap();
    inner.split(',').map(|s| s.trim().to_string()).collect()
}

fn generate_circom_main_string(instance: &Instance, file_name: &str) -> String {
    let name = instance.template_name.as_ref().unwrap();
    let config = &instance.config;
    let public_inputs = instance.public_inputs.to_vec();

    let inputs_str = public_inputs.join(", ");
    let config_str = config
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    format!(
        "pragma circom 2.1.4;\n\
include \"./{}.circom\";\n\
component main {{public [{}]}} =  {}({}{} 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);",
         file_name, inputs_str, name, config_str, if config_str.is_empty() { "" } else { "," }
    )
}

const UTXO_STRUCT_BASE: &str = "\
#[allow(non_snake_case)]
#[derive(Debug, Copy, PartialEq)]
#[account]
pub struct Utxo {
    pub amounts: [u64; 2],
    pub spl_asset_index: u64,
    pub verifier_address_index: u64,
    pub blinding: u256,
    pub app_data_hash: u256,
    pub account_shielded_public_key: u256,
    pub account_encryption_public_key: [u8; 32],";

const PUBLIC_INPUTS_INSTRUCTION_DATA_BASE: &str = "#[allow(non_snake_case)]
#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionSecond {";

pub fn create_rust_idl(base: &str, public_inputs: &Vec<String>, input_type: &str) -> String {
    let mut result = String::from(base);

    for input in public_inputs {
        result = format!("{}\n    pub {}: {},", result, input, input_type);
    }

    result.push_str("\n}");
    result
}

#[allow(dead_code)]
fn rustfmt(code: String) -> Result<Vec<u8>, anyhow::Error> {
    let mut cmd = match env::var_os("RUSTFMT") {
        Some(r) => Command::new(r),
        None => Command::new("rustfmt"),
    };

    let mut cmd = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = cmd.stdin.take().unwrap();
    let mut stdout = cmd.stdout.take().unwrap();

    let stdin_handle = spawn(move || {
        stdin.write_all(code.as_bytes()).unwrap();
        // Manually flush and close the stdin handle
        stdin.flush().unwrap();
        drop(stdin);
    });

    let mut formatted_code = vec![];
    io::copy(&mut stdout, &mut formatted_code)?;

    let _ = cmd.wait();
    stdin_handle.join().unwrap();

    Ok(formatted_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instance() {
        let input = String::from(
            r#"
            #[instance]
            {
                fileName: appTransaction,
                config(7, 1, 9, 2),
                nrAppUtoxs: 1
            }
        "#,
        );
        let initial_input = input.clone();
        let expected = Instance {
            file_name: "appTransactionMain".to_owned(),
            template_name: None,
            config: vec![7, 1, 9, 2],
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let result = parse_instance(&input);
        assert_eq!(result, expected);
        // assert_ne!(initial_input, result);
    }

    // #[test]
    // fn test_parse_instance_with_public_input() {
    //     let input = String::from(
    //         r#"
    //         some random line
    //         another random line
    //         #[instance]
    //         {
    //             fileName : appTransaction,
    //             config(7, 1, 9, 2),
    //             nrAppUtoxs: 1,
    //             publicInputs: [inputA,inputB],
    //         }
    //         a final random line
    //     "#,
    //     );
    //     let initial_input = input.clone();
    //     let expected = Instance {
    //         file_name: "appTransactionMain".to_owned(),
    //         config: vec![7, 1, 9, 2],
    //         template_name: None,
    //         public_inputs: vec![
    //             String::from("inputA"),
    //             String::from("inputB"),
    //             String::from("transactionHash"),
    //             String::from("publicAppVerifier"),
    //         ],
    //     };

    //     let result = parse_instance(&input).unwrap();
    //     assert_eq!(result.0, expected);
    //     assert_ne!(initial_input, result.1);
    // }

    #[test]
    fn test_generate_circom_string_pass() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1],
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = "pragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 1, 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_string(&instance, "circuit"),
            expected_string
        );
    }

    #[test]
    fn test_generate_circom_string_pass2() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1, 3, 2],
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = "pragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 1, 3, 2, 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_string(&instance, "circuit"),
            expected_string
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_generate_circom_string_fail() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1],
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let incorrect_expected_string = "pragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 2 ,18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_string(&instance, "circuit"),
            incorrect_expected_string
        );
    }

    // #[test]
    // fn test_generate_instruction_hash_code() {
    //     let input = r#"
    //     {
    //         threshold,
    //         signerPubkeysX[nr],
    //         signerPubkeysY[nr]
    //     }"#;

    //     generate_instruction_hash_code(input.to_string());
    // }
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

    // #[test]
    // fn test_parse_instance_no_instance_defined() {
    //     let input = String::from("no #[instance] keyword");
    //     let result = parse_instance(&input);
    //     assert_eq!(result, Err(NoInstanceDefined));
    // }

    // #[test]
    // fn test_parse_instance_too_many_instances() {
    //     let input = String::from("#[instance] {} \n#[instance] {}");
    //     let result = parse_instance(&input);
    //     assert_eq!(result, Err(TooManyInstances));
    // }

    #[test]
    fn test_parse_light_transaction_light_transaction_undefined() {
        let input = String::from("no #[lightTransaction] keyword");
        let instruction_hash_code = String::from("instruction hash code");
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: vec![],
            public_inputs: vec![],
        };

        let result = parse_light_transaction(&input, &instruction_hash_code, &mut instance);
        assert_eq!(result, Err(LightTransactionUndefined));
    }

    #[test]
    #[should_panic]
    fn test_parse_light_transaction_double_declaration() {
        let input = String::from(
            "#[lightTransaction(verifierOne)] { ... } \n #[lightTransaction(verifierTwo)] { ... }",
        );
        let instruction_hash_code = String::from("instruction hash code");
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: vec![],
            public_inputs: vec![],
        };

        let _ = parse_light_transaction(&input, &instruction_hash_code, &mut instance);
    }

    // doesn't work because the error is on the highest level
    // #[test]
    // fn test_main_invalid_number_app_utxos() {
    //     let input = "#[instance]
    //     {
    //         fileName: MockVerifierTransaction,
    //         config(),
    //         nrAppUtoxs: 1,
    //         publicInputs: [currentSlot]
    //     }

    //     #[lightTransaction(verifierTwo)]
    //     template mockVerifierTransaction() {
    //         /**
    //         * -------------------------- Application starts here --------------------------
    //         */
    //         // defines the data which is saved in the utxo
    //         // this data is defined at utxo creation
    //         // is checked that only utxos with instructionData = hash or 0
    //         // exist in input utxos
    //         // is outside instruction
    //         // could add signal inputs automatically for these
    //         // are private inputs
    //         #[utxoData]
    //         {
    //             releaseSlot
    //         }
    //         signal input currentSlot;
    //         currentSlot === releaseSlot;
    //     }"

    //     let result = main();
    //     assert_eq!(result, Err(anyhow!(InvalidNumberAppUtxos)));
    // }
    #[test]
    fn test_create_utxo_rust_idl_success() {
        let public_inputs = vec![String::from("release_slot"), String::from("other_slot")];
        let result = create_rust_idl(UTXO_STRUCT_BASE, &public_inputs, "u256");

        let expected_output = String::from(
            "#[allow(non_snake_case)]
#[derive(Debug, Copy, PartialEq)]
#[account]
pub struct Utxo {
    pub amounts: [u64; 2],
    pub spl_asset_index: u64,
    pub verifier_address_index: u64,
    pub blinding: u256,
    pub app_data_hash: u256,
    pub account_shielded_public_key: u256,
    pub account_encryption_public_key: [u8; 32],
    pub release_slot: u256,
    pub other_slot: u256,
}",
        );

        assert_eq!(result, expected_output);
    }
    #[test]
    fn test_create_rust_idl() {
        let public_inputs = vec![String::from("current_slot"), String::from("other_slot")];
        let output = create_rust_idl(PUBLIC_INPUTS_INSTRUCTION_DATA_BASE, &public_inputs, "u256");

        let expected_output = "#[allow(non_snake_case)]
#[derive(Debug)]
#[account]
pub struct InstructionDataLightInstructionSecond {
    pub current_slot: u256,
    pub other_slot: u256,
}";
        assert_eq!(output, expected_output);
    }
}

pub mod connecting_hash_circom;
pub mod errors;
pub mod light_utils_string;
use crate::errors::MacroCircomError;
use crate::light_utils_string::{PART_ONE, PART_TWO};
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
#[derive(Debug, PartialEq)]
struct Instance {
    file_name: String,
    template_name: Option<String>,
    config: Vec<u32>,
    nr_app_utoxs: Option<u32>,
    public_inputs: Vec<String>,
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

fn extract_string_between_slash_and_dot_light(input: &str) -> Option<String> {
    let mut parts = input.rsplitn(2, '/');
    let part_after_slash = parts.next()?;
    let dot_light_index = part_after_slash.find(".light")?;

    Some(part_after_slash[..dot_light_index].to_string())
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

    let file_name = extract_string_between_slash_and_dot_light(file_path).unwrap();
    // Open the file
    let mut file = File::open(file_path).expect("Unable to open the file");

    // Read the file's content
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");

    let (mut instance, contents) = parse_instance(&contents)?;
    if instance.nr_app_utoxs.is_none() {
        return Err(anyhow!(InvalidNumberAppUtxos));
    }

    let (instruction_hash_code, contents, utxo_data_variable_names) = parse_general(
        &contents,
        &String::from("#[utxoData]"),
        generate_instruction_hash_code,
        true,
    )?;

    let (_verifier_name, contents) =
        parse_light_transaction(&contents, &instruction_hash_code, &mut instance).unwrap();

    let mut output_file =
        fs::File::create("./circuit/".to_owned() + &file_name + ".circom").unwrap();

    write!(&mut output_file, "{}", contents).unwrap();

    let mut output_file = fs::File::create(
        [
            &"./circuit/",
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
    write!(&mut output_file, "{}", instance_str).unwrap();
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
    );
    let mut output_file_idl =
        fs::File::create("./programs/".to_owned() + &program_name + "/src/light_utils.rs").unwrap();
    write!(&mut output_file_idl, "{}", light_utils_str).unwrap();
    Ok(())
}

pub const UTXO_APP_DATA_STRUCT_BASE: &str = "#[allow(non_snake_case)]
#[account]
pub struct UtxoAppData {";

fn create_light_utils_str(
    utxo_rust_idl_string: String,
    public_inputs_rust_idl_string: String,
    utxo_app_data_rust_idl_string: String,
) -> String {
    let mut result = String::from(PART_ONE);
    result = format!("{}\n{}\n", result, public_inputs_rust_idl_string);
    result = format!("{}\n{}\n", result, PART_TWO);
    result = format!("{}\n{}\n", result, utxo_rust_idl_string);
    result = format!("{}\n{}\n", result, utxo_app_data_rust_idl_string);
    // result = format!("{}\n{}\n", result, ";");
    result
}

fn parse_general(
    input: &String,
    starting_string: &String,
    parse_between_brackets_fn: fn(String) -> (String, Vec<String>),
    critical: bool,
) -> Result<(String, String, Vec<String>), MacroCircomError> {
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut commented = false;
    let mut bracket_str = Vec::<&str>::new();
    for line in input.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        if line.starts_with("/* ") || line.starts_with("/**") {
            commented = true;
            remaining_lines.push(line);
            continue;
        }
        if commented {
            remaining_lines.push(line);
            if line.find("*/").is_some() {
                commented = false;
            }
            continue;
        }
        if line.starts_with(starting_string) {
            // cannot accept overloads implementations
            if found_instance == true {
                panic!();
            };
            found_instance = true;
            found_bracket = true;
            continue;
        }
        if found_instance && line.starts_with("{") {
            continue;
        }
        if found_bracket && found_instance && line.starts_with("}") {
            found_bracket = false;
            continue;
        }

        if found_bracket {
            bracket_str.push(line);
        }
        if !found_bracket {
            remaining_lines.push(line);
        }
    }
    let (res, variable_vec) = parse_between_brackets_fn(bracket_str.join("\n"));
    if !found_instance && critical {
        return Err(ParseInstanceError(input.to_string()));
    }
    Ok((res, remaining_lines.join("\n"), variable_vec))
}

fn generate_instruction_hash_code(input: String) -> (String, Vec<String>) {
    let variables: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    let mut non_array_variables = 0;
    let mut array_variables = Vec::<String>::new();
    let mut output = String::new();
    let mut output_variable_names = Vec::<String>::new();
    for var in &variables {
        if var.contains('[') && var.contains(']') {
            array_variables.push(get_string_between_brackets(var).unwrap().to_string());
        } else {
            non_array_variables += 1;
        }
        output_variable_names.push(String::from(*var));
        output.push_str(&format!("signal input {};\n", var));
    }
    if non_array_variables != 0 {
        array_variables.insert(0, non_array_variables.to_string());
    }
    output.push_str(
        format!(
            "component instructionHasher = Poseidon({});\n",
            array_variables.join(" + ")
        )
        .as_str(),
    );
    // if non_array_variables != 0 {
    //     array_variables.pop();
    // }
    let mut array_i = 1;
    for (i, var) in variables.iter().enumerate() {
        if var.contains('[') && var.contains(']') {
            let split_var: Vec<&str> = var.split_terminator('[').collect();
            output.push_str(&format!(
                "for ( var i = 0; i < {}; i++) {{\n    instructionHasher.inputs[i + {}] <== {}[i];\n}}\n",
                array_variables[array_i], array_variables[0..array_i].join(" + "), split_var[0]
            ));
            array_i += 1;
        } else {
            output.push_str(&format!("instructionHasher.inputs[{}] <== {};\n", i, var));
        }
    }
    (output, output_variable_names)
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
                let to_insert = &format!("{} levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets", if instance.config.is_empty() { "" } else { "," });
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
fn parse_instance(input: &String) -> Result<(Instance, String), MacroCircomError> {
    let mut file_name = String::new();
    let mut config: Vec<u32> = Vec::new();
    let mut nr_app_utoxs: Option<u32> = None;
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut public_inputs = vec![
        String::from("transactionHash"),
        String::from("publicAppVerifier"),
    ];
    let mut commented = false;
    for line in input.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        if line.starts_with("/* ") || line.starts_with("/**") {
            commented = true;
            remaining_lines.push(line);
            continue;
        }
        if commented {
            remaining_lines.push(line);
            if line.find("*/").is_some() {
                commented = false;
            }
            continue;
        }
        if line.starts_with("#[instance]") {
            if found_instance == true {
                return Err(TooManyInstances);
            };
            found_instance = true;
            found_bracket = true;
            continue;
        }
        if found_instance && line.starts_with("{") {
            continue;
        }

        if found_bracket {
            if line.starts_with("fileName") {
                file_name = line
                    .split(":")
                    .nth(1)
                    .unwrap_or("")
                    .trim_end_matches(',')
                    .trim()
                    .to_owned();
                file_name = format!("{}{}", AsLowerCamelCase(file_name), &"Main");
            } else if line.starts_with("config") {
                let numbers: Vec<u32> = line
                    .split("(")
                    .nth(1)
                    .and_then(|s| s.split(")").nth(0))
                    .unwrap_or("")
                    .split(",")
                    .map(|s| s.trim().parse().ok())
                    .flatten()
                    .collect();
                config = numbers;
            } else if let Some(captures) = regex::Regex::new(r"nrAppUtoxs: (\d+)")
                .unwrap()
                .captures(line)
            {
                nr_app_utoxs = Some(captures.get(1).unwrap().as_str().parse().ok().unwrap());
            } else if line.starts_with("publicInputs") {
                let public_inputs_tmp = parse_public_input(line);

                for (i, input) in public_inputs_tmp.iter().enumerate() {
                    public_inputs.insert(i, input.clone());
                }
            }
        }
        if !found_bracket {
            remaining_lines.push(line);
        }
        if found_instance && line.starts_with("}") {
            found_bracket = false;
        }
    }
    if !found_instance {
        return Err(NoInstanceDefined);
    }
    Ok((
        Instance {
            file_name,
            config,
            nr_app_utoxs,
            public_inputs: public_inputs,
            template_name: None,
        },
        remaining_lines.join("\n"),
    ))
}

fn parse_public_input(input: &str) -> Vec<String> {
    let inner = get_string_between_brackets(input).unwrap();
    inner.split(",").map(|s| s.trim().to_string()).collect()
}

fn generate_circom_main_string(instance: &Instance, file_name: &str) -> String {
    let name = instance.template_name.as_ref().unwrap();
    let config = &instance.config;
    let nr_app_utoxs = instance.nr_app_utoxs.unwrap_or(0);
    let public_inputs = instance.public_inputs.to_vec();

    let inputs_str = public_inputs.join(", ");
    let config_str = config
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    format!(
        "pragma circom 2.0.0;\n\
         include \"./{}.circom\";\n\
         component main {{public [{}]}} =  {}({}{} 18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, {}, 3, 2, 2);",
         file_name, inputs_str, name, config_str, if config_str.is_empty() { "" } else { "," }, nr_app_utoxs
    )
}

const UTXO_STRUCT_BASE: &str = "\
#[allow(non_snake_case)]
#[account]
pub struct Utxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    verifier_address_index: u64,
    blinding: u256,
    app_data_hash: u256,
    account_shielded_public_key: u256,
    account_encryption_public_key: [u8; 32],";

const PUBLIC_INPUTS_INSTRUCTION_DATA_BASE: &str = "#[allow(non_snake_case)]
#[derive(Debug)]
#[account]
pub struct InstructionDataPspInstructionSecond {";

pub fn create_rust_idl(base: &str, public_inputs: &Vec<String>, input_type: &str) -> String {
    let mut result = String::from(base);

    for input in public_inputs {
        result = format!("{}\n    {}: {},", result, input, input_type);
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
            some random line
            another random line
            #[instance]
            {
                fileName : appTransaction,
                config(7, 1, 9, 2),
                nrAppUtoxs: 1,
            }
            a final random line
        "#,
        );
        let initial_input = input.clone();
        let expected = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: None,
            config: vec![7, 1, 9, 2],
            nr_app_utoxs: Some(1),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let result = parse_instance(&input).unwrap();
        assert_eq!(result.0, expected);
        assert_ne!(initial_input, result.1);
    }

    #[test]
    fn test_parse_instance_with_public_input() {
        let input = String::from(
            r#"
            some random line
            another random line
            #[instance]
            {
                fileName : appTransaction,
                config(7, 1, 9, 2),
                nrAppUtoxs: 1,
                publicInputs: [inputA,inputB],
            }
            a final random line
        "#,
        );
        let initial_input = input.clone();
        let expected = Instance {
            file_name: "appTransaction".to_owned(),
            config: vec![7, 1, 9, 2],
            template_name: None,
            nr_app_utoxs: Some(1),
            public_inputs: vec![
                String::from("inputA"),
                String::from("inputB"),
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let result = parse_instance(&input).unwrap();
        assert_eq!(result.0, expected);
        assert_ne!(initial_input, result.1);
    }

    #[test]
    fn test_generate_circom_string_pass() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1],
            nr_app_utoxs: Some(1),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 1, 18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

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
            nr_app_utoxs: Some(1),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 1, 3, 2, 18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

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
            nr_app_utoxs: Some(1),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let incorrect_expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 2 ,18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_string(&instance, "circuit"),
            incorrect_expected_string
        );
    }

    #[test]
    fn test_generate_instruction_hash_code() {
        let input = r#"
        {
            threshold,
            signerPubkeysX[nr],
            signerPubkeysY[nr]
        }"#;

        generate_instruction_hash_code(input.to_string());
    }
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
    fn test_parse_instance_no_instance_defined() {
        let input = String::from("no #[instance] keyword");
        let result = parse_instance(&input);
        assert_eq!(result, Err(NoInstanceDefined));
    }

    #[test]
    fn test_parse_instance_too_many_instances() {
        let input = String::from("#[instance] {} \n#[instance] {}");
        let result = parse_instance(&input);
        assert_eq!(result, Err(TooManyInstances));
    }

    #[test]
    fn test_parse_light_transaction_light_transaction_undefined() {
        let input = String::from("no #[lightTransaction] keyword");
        let instruction_hash_code = String::from("instruction hash code");
        let mut instance = Instance {
            file_name: String::from("file_name"),
            template_name: None,
            config: vec![],
            nr_app_utoxs: None,
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
            nr_app_utoxs: None,
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
#[account]
pub struct Utxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    verifier_address_index: u64,
    blinding: u256,
    app_data_hash: u256,
    account_shielded_public_key: u256,
    account_encryption_public_key: [u8; 32],
    release_slot: [u8; 32],
    other_slot: [u8; 32],
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
pub struct InstructionDataPspInstructionSecond {
    current_slot: [u8; 32],
    other_slot: [u8; 32],
}";

        assert_eq!(output, expected_output);
    }
}

pub mod connecting_hash_circom;

use core::panic;
// create circom file
// create function that wr
use std::{
    fs::{self, File},
    io::{Read, Write},
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
// TODO: rust file with public inputs, sytem verifier, as anchor constants

fn main() {
    // TODO: take filename from argv
    let mut file = File::open("./circuit/circuit.light").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let (mut instance, contents) = parse_instance(&contents).unwrap();

    let (instruction_hash_code, contents) = parse_general(
        &contents,
        &String::from("#[utxoData]"),
        generate_instruction_hash_code,
    )
    .unwrap();

    let (_verifier_name, contents) =
        parse_light_transaction(&contents, &instruction_hash_code, &mut instance).unwrap();

    let mut output_file = fs::File::create("./circuit/circuit.circom").unwrap();
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
    let instance_str = generate_circom_main_string(&instance);

    write!(&mut output_file, "{}", instance_str).unwrap();
}

fn parse_general(
    input: &String,
    starting_string: &String,
    parse_between_brackets_fn: fn(String) -> String,
) -> Option<(String, String)> {
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
    let res = parse_between_brackets_fn(bracket_str.join("\n"));
    Some((res, remaining_lines.join("\n")))
}

fn generate_instruction_hash_code(input: String) -> String {
    let variables: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    let mut non_array_variables = 0;
    let mut array_variables = Vec::<String>::new();
    let mut output = String::new();

    for var in &variables {
        if var.contains('[') && var.contains(']') {
            array_variables.push(get_string_between_brackets(var).unwrap().to_string());
        } else {
            non_array_variables += 1;
        }
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

    output
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
) -> Option<(String, String)> {
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
                let to_insert = ", levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets";
                remaining_lines.push(insert_string_before_parenthesis(line, to_insert));
                remaining_lines
                    .push(connecting_hash_circom::connecting_hash_verifier_two.to_string());
                remaining_lines.push(instruction_hash_code.to_string());
                found_bracket = false;
            }
        }
    }

    Some((verifier_name, remaining_lines.join("\n")))
}
fn extract_template_name(input: &str) -> Option<String> {
    let start = input.find("template ")? + "template ".len();
    let end = input.find('(')?;

    Some(input[start..end].trim().to_string())
}
fn parse_instance(input: &String) -> Option<(Instance, String)> {
    let mut file_name = String::new();
    let mut config: Vec<u32> = Vec::new();
    let mut nr_app_utoxs: Option<u32> = None;
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut public_inputs = vec![String::from("connectingHash"), String::from("verifier")];
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
                panic!();
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
                nr_app_utoxs = Some(captures.get(1).unwrap().as_str().parse().ok()?);
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

    Some((
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

fn generate_circom_main_string(instance: &Instance) -> String {
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
         include \"./circuit.circom\";\n\
         component main {{public [{}]}} =  {}({} ,18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, {}, 3, 2, 2);",
        inputs_str, name, config_str, nr_app_utoxs
    )
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
            public_inputs: vec![String::from("connectingHash"), String::from("verifier")],
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
                String::from("connectingHash"),
                String::from("verifier"),
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
            public_inputs: vec![String::from("connectingHash"), String::from("verifier")],
        };

        let expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [connectingHash, verifier]} =  AppTransaction(7, 1 ,18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

        assert_eq!(generate_circom_main_string(&instance), expected_string);
    }

    #[test]
    fn test_generate_circom_string_pass2() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1, 3, 2],
            nr_app_utoxs: Some(1),
            public_inputs: vec![String::from("connectingHash"), String::from("verifier")],
        };

        let expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [connectingHash, verifier]} =  AppTransaction(7, 1, 3, 2 ,18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

        assert_eq!(generate_circom_main_string(&instance), expected_string);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_generate_circom_string_fail() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: vec![7, 1],
            nr_app_utoxs: Some(1),
            public_inputs: vec![String::from("connectingHash"), String::from("verifier")],
        };

        let incorrect_expected_string = "pragma circom 2.0.0;\n\
            include \"./circuit.circom\";\n\
            component main {public [connectingHash, verifier]} =  AppTransaction(7, 2 ,18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_string(&instance),
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
}

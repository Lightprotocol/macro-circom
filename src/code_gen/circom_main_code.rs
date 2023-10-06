use std::{fs, io::prelude::*};

use heck::ToUpperCamelCase;

pub const DISCLAIMER_STRING: &str = "/**
* This file is auto-generated by the Light cli.
* DO NOT EDIT MANUALLY.
* THE FILE WILL BE OVERWRITTEN EVERY TIME THE LIGHT CLI BUILD IS RUN.
*/";

#[derive(Debug, PartialEq, Clone)]
pub struct Instance {
    pub file_name: String,
    pub template_name: Option<String>, // currently always None
    pub config: Option<Vec<String>>,
    pub public_inputs: Vec<String>,
}

pub fn generate_circom_main_code(instance: &Instance, circuit_name: &str) -> String {
    let name = instance.file_name.clone();
    let config = &instance.config;
    let public_inputs = instance.public_inputs.to_vec();

    let inputs_str = public_inputs.join(", ");
    let config_str = match config {
        Some(config) => config.join(", "),
        None => String::new(),
    };
    format!(
        "{}\npragma circom 2.1.4;\n\
include \"./{}.circom\";\n\
component main {{public [{}]}} =  {}({}{} 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);",
DISCLAIMER_STRING,circuit_name, inputs_str, name.to_upper_camel_case(), config_str, if config_str.is_empty() { "" } else { "," }
    )
}

pub fn generate_circom_main_file(instance: Instance, file_name: &str, path_to_parent_dir: &str) {
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
    let code = generate_circom_main_code(&instance, file_name);
    println!(
        "sucessfully created main {}.circom and {}.circom",
        instance.file_name, file_name
    );

    write!(&mut output_file, "{}", code).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parsers::macro_parser, utils::describe_error};

    #[test]
    fn test_generate_circom_string_pass() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: Some(vec![String::from("7"), String::from("1")]),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = format!("{}\npragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {{public [transactionHash, publicAppVerifier]}} =  AppTransaction(7, 1, 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);", DISCLAIMER_STRING);

        assert_eq!(
            generate_circom_main_code(&instance, "circuit"),
            expected_string
        );
    }

    #[test]
    fn test_generate_circom_string_pass2() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: Some(vec![
                String::from("7"),
                String::from("1"),
                String::from("3"),
                String::from("2"),
            ]),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let expected_string = format!("{}\npragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {{public [transactionHash, publicAppVerifier]}} =  AppTransaction(7, 1, 3, 2, 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);", DISCLAIMER_STRING);

        assert_eq!(
            generate_circom_main_code(&instance, "circuit"),
            expected_string
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_generate_circom_string_fail() {
        let instance = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: Some(String::from("AppTransaction")),
            config: Some(vec![String::from("7"), String::from("1")]),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let incorrect_expected_string = "pragma circom 2.1.4;\n\
            include \"./circuit.circom\";\n\
            component main {public [transactionHash, publicAppVerifier]} =  AppTransaction(7, 2 ,18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";

        assert_eq!(
            generate_circom_main_code(&instance, "circuit"),
            incorrect_expected_string
        );
    }

    #[test]
    fn test_parse_instance() {
        let input = String::from(
            r#"#[instance]
            {
                fileName: appTransactionMain,
                config:(7, 1, 9, 2),
        }"#,
        );
        let expected = Instance {
            file_name: "appTransactionMain".to_owned(),
            template_name: None,
            config: Some(vec![
                String::from("7"),
                String::from("1"),
                String::from("9"),
                String::from("2"),
            ]),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
            ],
        };

        let result = macro_parser::InstanceParser::new().parse(&input);

        match result {
            Ok(result) => assert_eq!(result, expected),
            Err(error) => {
                println!("{}", describe_error(&input, error.clone()));
                panic!("{}", describe_error(&input, error));
            }
        }
    }

    #[test]
    fn test_parse_instance_with_public_input() {
        let input = String::from(
            r#"
        // include "test.circom";
        template sxasSD() {
            signal input inputA;
        }
        #[instance]
        {
            fileName: appTransaction,
            config:(7, 1, 9, 2),
            publicInputs: [inputA,inputB],
        }        
    "#,
        );
        let expected = Instance {
            file_name: "appTransaction".to_owned(),
            template_name: None,
            config: Some(vec![
                String::from("7"),
                String::from("1"),
                String::from("9"),
                String::from("2"),
            ]),
            public_inputs: vec![
                String::from("transactionHash"),
                String::from("publicAppVerifier"),
                String::from("inputA"),
                String::from("inputB"),
            ],
        };

        let result = macro_parser::LightFileParser::new().parse(&input);

        match result {
            Ok(result) => assert_eq!(result.0.unwrap()[0], expected),
            Err(error) => {
                panic!("{}", describe_error(&input, error));
            }
        }
    }
}

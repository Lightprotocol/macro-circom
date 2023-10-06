pub mod checkUtxo;
pub mod code_gen;
pub mod connecting_hash_circom;
pub mod errors;
pub mod ignoredContent;
pub mod instance;
pub mod light_transaction;
pub mod utils;
pub use checkUtxo::*;
use code_gen::auto_generated_accounts::gen_code_auto_generated_accounts;
use code_gen::circom_main::generate_circom_main_string;

use crate::errors::MacroCircomError;
use anyhow::{anyhow, Error as AnyhowError};
use clap::{App, Arg};
use utils::{create_file, describe_error, open_file, write_rust_code_to_file};

#[derive(Debug, PartialEq, Clone)]
pub struct Instance {
    file_name: String,
    template_name: Option<String>,
    config: Option<Vec<String>>,
    public_inputs: Vec<String>,
}

fn parse_instance<'a>(
    input: &'a str,
    // ) -> Result<Instance, lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'_>, &'a str>> {
) -> Instance {
    // instance::InstanceParser::new().parse(input)
    match instance::InstanceParser::new().parse(input) {
        Ok(instance) => instance,
        Err(error) => {
            panic!("{}", describe_error(&input, error));
        }
    }
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
/*
* Structure:
* - get instance (returns struct with file name, template name, config, public inputs)
* - get checkedUtxos (returns structs with code and utxo data)
* - get light transaction body
* - generate circom file
*/
use crate::light_transaction::parse_light_transaction;

fn main() -> Result<(), AnyhowError> {
    let matches = App::new("macro-circom")
        .version("0.1")
        .arg(
            Arg::with_name("file_path")
                .help("Path to the file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("program_name")
                .help("Name of the program")
                .required(true)
                .index(2),
        )
        .get_matches();

    // Get the values of the arguments
    let file_path = matches.value_of("file_path").unwrap();
    let program_name = matches.value_of("program_name").unwrap();

    let (path_to_parent_dir, file_name) = remove_filename_suffix(file_path).unwrap();
    let contents = open_file(file_path)?;

    // parse .light file
    let (mut instance, checked_utxos, remaining_content) = parse(contents.clone())?;

    // start code generation
    let (circom_main_code, circom_code, rust_autogenerated_accounts_code) =
        generate_code(&mut instance, checked_utxos, remaining_content);

    // create files
    create_files(
        circom_main_code,
        circom_code,
        rust_autogenerated_accounts_code,
        path_to_parent_dir,
        file_name,
        program_name.to_string(),
    );
    Ok(())
}

fn parse(contents: String) -> Result<(Instance, Vec<CheckUtxo>, String), AnyhowError> {
    let mut instance = parse_instance(&contents);

    let (remainingContents, checkedInUtxos) = generate_check_utxo_code(&contents)?;
    let (verifier_name, circom_code) =
        parse_light_transaction(&contents, &checkedInUtxos[0].code, &mut instance)?;

    Ok((instance, checkedInUtxos, remainingContents))
}

fn generate_code(
    instance: &mut Instance,
    checked_utxos: Vec<CheckUtxo>,
    content: String,
) -> (String, String, String) {
    let circom_main_code = generate_circom_main_string(&instance, &instance.file_name);
    let rust_autogenerated_accounts_code =
        gen_code_auto_generated_accounts(instance, &checked_utxos);

    // TODO: support vector of checked utxos
    let check_utxos_code = checked_utxos[0].code.clone();
    let (_verifier_name, circom_code) =
        parse_light_transaction(&content, &check_utxos_code, instance).unwrap();
    (
        circom_main_code,
        circom_code,
        rust_autogenerated_accounts_code,
    )
}

fn create_files(
    circom_main_code: String,
    circom_code: String,
    rust_autogenerated_accounts_code: String,
    path_to_parent_dir: String,
    file_name: String,
    program_name: String,
) {
    let circuit_main_file_name = [
        &path_to_parent_dir.to_string(),
        "/",
        file_name.as_str().clone(),
        &".circom",
    ]
    .concat();
    create_file(&circuit_main_file_name, &circom_main_code).unwrap();

    let circuit_file_name = path_to_parent_dir.clone() + "/" + &file_name + ".circom";
    create_file(&circuit_file_name, &circom_code).unwrap();

    let path = "./programs/".to_owned() + &program_name + "/src/auto_generated_accounts.rs";
    write_rust_code_to_file(path, rust_autogenerated_accounts_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instance() {
        let input = String::from(
            r#"#[instance]
            {
                fileName: appTransactionMain,
                config:(7, 1, 9, 2),
        }"#,
        );
        let initial_input = input.clone();
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

        let result = instance::InstanceParser::new().parse(&input);

        match result {
            Ok(result) => assert_eq!(result, expected),
            Err(error) => {
                println!("{}", describe_error(&input, error.clone()));
                panic!("{}", describe_error(&input, error));
            }
        }
        // assert_eq!(result, expected);
        // assert_ne!(initial_input, result);
    }

    #[test]
    fn test_parse_instance_with_public_input() {
        let input = String::from(
            r#"
        // include "test.circom";
        template sxasSD {
            dsaflkjçdsalnf;
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

        let result = instance::InstanceParser::new().parse(&input);

        match result {
            Ok(result) => assert_eq!(result, expected),
            Err(error) => {
                panic!("{}", describe_error(&input, error));
            }
        }
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
    fn test_main() {}
}

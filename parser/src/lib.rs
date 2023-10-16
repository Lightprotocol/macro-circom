extern crate num_bigint_dig as num_bigint;
extern crate num_traits;
extern crate serde;
extern crate serde_derive;
#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub lang);

pub mod errors;
pub mod include_logic;
pub mod parser_logic;
pub mod syntax_sugar_remover;

use include_logic::{FileStack, IncludesGraph};
pub use program_structure::ast::*;
use program_structure::error_code::ReportCode;
use program_structure::error_definition::{Report, ReportCollection};
use program_structure::file_definition::FileLibrary;
use program_structure::program_archive::ProgramArchive;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use syntax_sugar_remover::apply_syntactic_sugar;
#[derive(Debug, PartialEq, Clone)]
pub struct Instance {
    pub file_name: String,
    pub template_name: Option<String>, // currently always None
    pub config: Option<Vec<String>>,
    pub public_inputs: Vec<String>,
}
#[derive(PartialEq, Debug, Clone)]
pub struct CheckUtxo {
    pub code: String,
    pub name: String,
    pub is_in_utxo: bool,
    pub is_out_utxo: bool,
    pub instruction_name: Option<String>,
    pub no_utxos: String,
    // exists, comparator, variable name to compare with
    pub amount_sol: Option<(Comparator, String)>,
    pub amount_spl: Option<(Comparator, String)>,
    pub asset_spl: Option<(Comparator, String)>,
    pub app_data_hash: Option<(Comparator, String)>,
    pub verifier_address: Option<(Comparator, String)>,
    pub blinding: Option<(Comparator, String)>,
    pub tx_version: Option<(Comparator, String)>,
    pub pool_type: Option<(Comparator, String)>,
    // utxo data needs to be defined completely but does not have to be compared
    pub utxo_data: Option<Vec<(String, Option<Comparator>, Option<String>)>>,
}

impl CheckUtxo {
    pub fn new() -> Self {
        CheckUtxo {
            code: String::new(),
            name: String::new(),
            is_in_utxo: false,
            is_out_utxo: false,
            instruction_name: None,
            no_utxos: String::from("0"),
            amount_sol: None,
            amount_spl: None,
            asset_spl: None,
            app_data_hash: None,
            utxo_data: None,
            verifier_address: None,
            blinding: None,
            tx_version: None,
            pool_type: None,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Comparator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqualThan,
    LessEqualThan,
}

impl Comparator {
    pub fn as_str(&self) -> &str {
        match *self {
            Comparator::Equal => "==",
            Comparator::NotEqual => "!=",
            Comparator::GreaterThan => ">",
            Comparator::LessThan => "<",
            Comparator::GreaterEqualThan => ">=",
            Comparator::LessEqualThan => "<=",
        }
    }
}

pub type Version = (usize, usize, usize);

pub fn find_file(
    crr_file: PathBuf,
    ext_link_libraries: Vec<PathBuf>,
) -> (bool, String, String, PathBuf, Vec<Report>) {
    let mut found = false;
    let mut path = "".to_string();
    let mut src = "".to_string();
    let mut crr_str_file = crr_file.clone();
    let mut reports = Vec::new();
    let mut i = 0;
    while !found && i < ext_link_libraries.len() {
        let mut p = PathBuf::new();
        let aux = ext_link_libraries.get(i).unwrap();
        p.push(aux);
        p.push(crr_file.clone());
        crr_str_file = p;
        match open_file(crr_str_file.clone()) {
            Ok((new_path, new_src)) => {
                path = new_path;
                src = new_src;
                found = true;
            }
            Err(e) => {
                reports.push(e);
                i = i + 1;
            }
        }
    }
    (found, path, src, crr_str_file, reports)
}

pub fn run_parser(
    file: String,
    version: &str,
    link_libraries: Vec<PathBuf>,
) -> Result<(ProgramArchive, ReportCollection), (FileLibrary, ReportCollection)> {
    let mut file_library = FileLibrary::new();
    let mut definitions = Vec::new();
    let mut main_components = Vec::new();
    let mut file_stack = FileStack::new(PathBuf::from(file));
    let mut includes_graph = IncludesGraph::new();
    let mut warnings = Vec::new();
    let mut link_libraries2 = link_libraries.clone();
    let mut ext_link_libraries = vec![Path::new("").to_path_buf()];
    ext_link_libraries.append(&mut link_libraries2);
    while let Some(crr_file) = FileStack::take_next(&mut file_stack) {
        let (found, path, src, crr_str_file, reports) =
            find_file(crr_file, ext_link_libraries.clone());
        if !found {
            return Result::Err((file_library.clone(), reports));
        }
        let file_id = file_library.add_file(path.clone(), src.clone());
        let program =
            parser_logic::parse_file(&src, file_id).map_err(|e| (file_library.clone(), vec![e]))?;
        if let Some(main) = program.main_component {
            main_components.push((file_id, main, program.custom_gates));
        }
        includes_graph.add_node(
            crr_str_file,
            program.custom_gates,
            program.custom_gates_declared,
        );
        let includes = program.includes;
        definitions.push((file_id, program.definitions));
        for include in includes {
            let path_include =
                FileStack::add_include(&mut file_stack, include.clone(), &link_libraries.clone())
                    .map_err(|e| (file_library.clone(), vec![e]))?;
            includes_graph
                .add_edge(path_include)
                .map_err(|e| (file_library.clone(), vec![e]))?;
        }
        warnings.append(
            &mut check_number_version(
                path.clone(),
                program.compiler_version,
                parse_number_version(version),
            )
            .map_err(|e| (file_library.clone(), vec![e]))?,
        );
        if program.custom_gates {
            check_custom_gates_version(
                path,
                program.compiler_version,
                parse_number_version(version),
            )
            .map_err(|e| (file_library.clone(), vec![e]))?
        }
    }

    if main_components.len() == 0 {
        let report = errors::NoMainError::produce_report();
        Err((file_library, vec![report]))
    } else if main_components.len() > 1 {
        let report = errors::MultipleMainError::produce_report();
        Err((file_library, vec![report]))
    } else {
        let errors: ReportCollection = includes_graph.get_problematic_paths().iter().map(|path|
            Report::error(
                format!(
                    "Missing custom templates pragma in file {} because of the following chain of includes {}",
                    path.last().unwrap().display(),
                    IncludesGraph::display_path(path)
                ),
                ReportCode::CustomGatesPragmaError
            )
        ).collect();
        if errors.len() > 0 {
            Err((file_library, errors))
        } else {
            let (main_id, main_component, custom_gates) = main_components.pop().unwrap();
            let result_program_archive = ProgramArchive::new(
                file_library,
                main_id,
                main_component,
                definitions,
                custom_gates,
            );
            match result_program_archive {
                Err((lib, rep)) => Err((lib, rep)),
                Ok(mut program_archive) => {
                    let lib = program_archive.get_file_library().clone();
                    let program_archive_result = apply_syntactic_sugar(&mut program_archive);
                    match program_archive_result {
                        Result::Err(v) => Result::Err((lib, vec![v])),
                        Result::Ok(_) => Ok((program_archive, warnings)),
                    }
                }
            }
        }
    }
}

fn open_file(path: PathBuf) -> Result<(String, String), Report> /* path, src */ {
    use errors::FileOsError;
    use std::fs::read_to_string;
    let path_str = format!("{:?}", path);
    read_to_string(path)
        .map(|contents| (path_str.clone(), contents))
        .map_err(|_| FileOsError {
            path: path_str.clone(),
        })
        .map_err(|e| FileOsError::produce_report(e))
}

fn parse_number_version(version: &str) -> Version {
    let version_splitted: Vec<&str> = version.split(".").collect();
    (
        usize::from_str(version_splitted[0]).unwrap(),
        usize::from_str(version_splitted[1]).unwrap(),
        usize::from_str(version_splitted[2]).unwrap(),
    )
}

fn check_number_version(
    file_path: String,
    version_file: Option<Version>,
    version_compiler: Version,
) -> Result<ReportCollection, Report> {
    use errors::{CompilerVersionError, NoCompilerVersionWarning};
    if let Some(required_version) = version_file {
        if required_version.0 == version_compiler.0
            && (required_version.1 < version_compiler.1
                || (required_version.1 == version_compiler.1
                    && required_version.2 <= version_compiler.2))
        {
            Ok(vec![])
        } else {
            let report = CompilerVersionError::produce_report(CompilerVersionError {
                path: file_path,
                required_version,
                version: version_compiler,
            });
            Err(report)
        }
    } else {
        let report = NoCompilerVersionWarning::produce_report(NoCompilerVersionWarning {
            path: file_path,
            version: version_compiler,
        });
        Ok(vec![report])
    }
}

fn check_custom_gates_version(
    file_path: String,
    version_file: Option<Version>,
    version_compiler: Version,
) -> Result<(), Report> {
    let custom_gates_version: Version = (2, 0, 6);
    if let Some(required_version) = version_file {
        if required_version.0 < custom_gates_version.0
            || (required_version.0 == custom_gates_version.0
                && required_version.1 < custom_gates_version.1)
            || (required_version.0 == custom_gates_version.0
                && required_version.1 == custom_gates_version.1
                && required_version.2 < custom_gates_version.2)
        {
            let report = Report::error(
                format!(
                    "File {} requires at least version {:?} to use custom templates (currently {:?})",
                    file_path,
                    custom_gates_version,
                    required_version
                ),
                ReportCode::CustomGatesVersionError
            );
            return Err(report);
        }
    } else {
        if version_compiler.0 < custom_gates_version.0
            || (version_compiler.0 == custom_gates_version.0
                && version_compiler.1 < custom_gates_version.1)
            || (version_compiler.0 == custom_gates_version.0
                && version_compiler.1 == custom_gates_version.1
                && version_compiler.2 < custom_gates_version.2)
        {
            let report = Report::error(
                format!(
                    "File {} does not include pragma version and the compiler version (currently {:?}) should be at least {:?} to use custom templates",
                    file_path,
                    version_compiler,
                    custom_gates_version
                ),
                ReportCode::CustomGatesVersionError
            );
            return Err(report);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use std::{
        fs::{self, File},
        io::{self, prelude::*},
    };

    use super::*;
    pub fn describe_error(
        input: &str,
        error: lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'_>, &'_ str>,
    ) -> String {
        match error {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let start = location.saturating_sub(10);
                let end = std::cmp::min(location + 10, input.len());
                format!(
                    "Invalid token at: `{}`. Full context: `{}`",
                    &input[location..location + 1],
                    &input[start..end]
                )
            }
            lalrpop_util::ParseError::UnrecognizedToken {
                token: (start, token, end),
                expected,
            } => {
                let context_start = start.saturating_sub(50);
                let context_end = std::cmp::min(end + 10, input.len());
                format!(
                    "Unrecognized token `{}` at position {}:{} within context `{}`. Expected one of: {:?}",
                    token,
                    start,
                    end,
                    &input[context_start..context_end],
                    expected
                )
            }
            lalrpop_util::ParseError::ExtraToken {
                token: (start, token, end),
            } => {
                let context_start = start.saturating_sub(10);
                let context_end = std::cmp::min(end + 10, input.len());
                format!(
                    "Extra token `{}` at position {}:{} within context `{}`.",
                    token,
                    start,
                    end,
                    &input[context_start..context_end]
                )
            }
            lalrpop_util::ParseError::User { error } => {
                format!("User-defined error: {}", error)
            }
            _ => format!("Unknown error"),
        }
    }

    #[test]
    fn test_parse_ast_with_instance() {
        let input = String::from(
            r#"#[instance]
            {
                fileName:appTransactionMain,
                config: [7, 1, 9, 2],
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

        let result = lang::ParseInstanceParser::new().parse(&input);

        match result {
            Ok(result) => assert_eq!(result.0.unwrap(), expected),
            Err(error) => {
                println!("{}", describe_error(&input, error.clone()));
                panic!("{}", describe_error(&input, error));
            }
        }
    }

    #[test]
    fn parse_check_utxo() {
        let contents = String::from(
            "#[checkInUtxo(utxoName, 1, instruction)]
       {
            amountSol == sth,
            amountSpl == sth1,
            assetSpl == sth2,
            appDataHash == sth3, 
            utxoData: {
               attribute1,
               attribute2 == testComparison,
               }
           }",
        );
        let parsing_res = match crate::lang::ParseInstanceParser::new().parse(&contents) {
            Ok(instance) => instance,
            Err(error) => {
                println!("Parsing check utxo error.");
                panic!("{}", describe_error(&contents, error));
            }
        };

        let checked_utxos = parsing_res.1;
        let check_utxo = checked_utxos[0].clone();
        assert_eq!(check_utxo.name, "utxoName");
        assert_eq!(check_utxo.no_utxos, "1");
        assert_eq!(
            check_utxo.instruction_name,
            Some(String::from("instruction"))
        );
        assert_eq!(
            check_utxo.amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
        assert_eq!(
            check_utxo.amount_spl,
            Some((Comparator::Equal, String::from("sth1")))
        );
        assert_eq!(
            check_utxo.asset_spl,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            check_utxo.app_data_hash,
            Some((Comparator::Equal, String::from("sth3")))
        );
        assert_eq!(
            check_utxo.utxo_data,
            Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ])
        );
    }

    pub fn open_file(file_path: &str) -> Result<String, std::io::Error> {
        // Open the file
        let mut file = File::open(file_path).expect("Unable to open the file");

        // Read the file's content
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents = match crate::parser_logic::preprocess(&contents, 0usize) {
            Ok(contents) => contents,
            Err(_) => panic!("Error preprocessing file"),
        };
        Ok(contents)
    }

    #[test]
    fn test_light_file() {
        let path = "../macro-circom/tests/test-files/test-data-psp/test_data.light";
        let light_file_str = open_file(path).unwrap();
        let result = lang::ParseInstanceParser::new().parse(&light_file_str);
        match result {
            Ok(result) => {
                println!("{:?}", result);
            }
            Err(error) => {
                println!("{}", describe_error(&light_file_str, error.clone()));
                panic!("{}", describe_error(&light_file_str, error));
            }
        }
    }
} //

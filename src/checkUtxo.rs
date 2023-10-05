use crate::describe_error;
use crate::errors::MacroCircomError;
use crate::errors::MacroCircomError::*;
use crate::{auto_generated_accounts_template::AUTO_GENERATED_ACCOUNTS_TEMPLATE, Instance};
use anyhow::{anyhow, Error as AnyhowError};
use heck::{ToLowerCamelCase, ToUpperCamelCase};
use regex::Regex;
use std::fmt::format;
use std::ops::Deref;
use std::string;

fn generate_input_signal(input: &String) -> String {
    let mut output = String::new();
    output.push_str(&format!("signal input {};\n", input));
    output
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
    // utxo data needs to be defined completely but does not have to be compared
    pub utxo_data: Option<Vec<(String, Option<Comparator>, Option<String>)>>,
}
// get string between curly brackets
// extract the first word of every line as an attribute
// expect a comparison arg as the second word
// expect a variable name as the third word
// expect a comma at the end of the line
// generate code as output string:
// init for loop
// for every attribute create a ForceEqualIfEnabled component if comparsion is ==
/**
 * instructionHash code
 *
* for (var i = 0; i < nIns; i++) {
       checkInUtxo[i] = ForceEqualIfEnabled();
       checkInUtxo[i].in[0] <== inAppDataHash[i];
       checkInUtxo[i].in[1] <== instructionHasher.out;
       checkInUtxo[i].enabled <== isAppInUtxo[i];
   }
*/
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
        }
    }

    pub fn parse_header(header: &str) -> Result<Self, MacroCircomError> {
        let re = Regex::new(r"^#\[\s*checkInUtxo\((?P<name>[a-zA-Z_][a-zA-Z0-9_]*),\s*(?P<no_utxos>\d+)(?:,\s*(?P<instruction_name>[a-zA-Z_][a-zA-Z0-9_]*))?\)\s*\]$").unwrap();
        println!("header raw  {:?}", header);
        println!("header {:?}", re.captures(header));
        if let Some(caps) = re.captures(header) {
            let name = caps["name"].to_string().to_upper_camel_case();
            let no_utxos = caps["no_utxos"]
                .parse()
                .map_err(|_| "Failed to parse number of UTXOs")
                .unwrap();

            // Use the instruction name if it exists; otherwise, default to None
            let instruction_name = caps
                .name("instruction_name")
                .map(|instr| instr.as_str().to_string().to_lower_camel_case());

            Ok(CheckUtxo {
                code: String::new(),
                name,
                is_in_utxo: false,
                is_out_utxo: false,
                instruction_name,
                no_utxos,
                amount_sol: None,
                amount_spl: None,
                asset_spl: None,
                app_data_hash: None,
                utxo_data: None,
            })
        } else {
            Err(MacroCircomError::CheckUtxoInvalidHeaderFormat)
        }
    }

    fn match_comparator(string: &str) -> Result<Comparator, MacroCircomError> {
        match string {
            "==" => Ok(Comparator::Equal),
            // not supported yet
            // "!=" => Ok(Comparator::NotEqual),
            // ">" => Ok(Comparator::GreaterThan),
            // "<" => Ok(Comparator::LessThan),
            // ">=" => Ok(Comparator::GreaterEqualThan),
            // "<=" => Ok(Comparator::LessEqualThan),
            _ => Err(MacroCircomError::InvalidComparator(string.to_string())),
        }
    }

    pub fn process_line(&mut self, line: &str) -> Result<(), MacroCircomError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let re = Regex::new(r"^[, \t\n\r]+|[, \t\n\r]+$").unwrap();

        if parts[0].contains("utxoData") {
            if self.utxo_data.is_none() {
                self.utxo_data = Some(Vec::new());
                return Ok(());
            } else {
                return Err(MacroCircomError::PropertyDefinedMultipleTimes);
            }
        }

        let comparator = CheckUtxo::match_comparator(parts[1])?;
        println!("Processing parts: {:?}", parts);
        match parts[0] {
            "amountSol" => {
                if self.amount_sol.is_none() {
                    self.amount_sol = Some((comparator, re.replace_all(parts[2], "").to_string()));
                    Ok(())
                } else {
                    Err(MacroCircomError::PropertyDefinedMultipleTimes)
                }
            }
            "amountSpl" => {
                if self.amount_spl.is_none() {
                    self.amount_spl = Some((comparator, re.replace_all(parts[2], "").to_string()));
                    Ok(())
                } else {
                    Err(MacroCircomError::PropertyDefinedMultipleTimes)
                }
            }
            "assetSpl" => {
                if self.asset_spl.is_none() {
                    self.asset_spl = Some((comparator, re.replace_all(parts[2], "").to_string()));
                    Ok(())
                } else {
                    Err(MacroCircomError::PropertyDefinedMultipleTimes)
                }
            }
            "appDataHash" => {
                if self.app_data_hash.is_none() {
                    self.app_data_hash =
                        Some((comparator, re.replace_all(parts[2], "").to_string()));
                    Ok(())
                } else {
                    Err(MacroCircomError::PropertyDefinedMultipleTimes)
                }
            }
            _ => Err(MacroCircomError::InvalidProperty),
        }
    }
    pub fn process_utxo_data_line(&mut self, line: &str) -> Result<(i32), MacroCircomError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let re = Regex::new(r"^[, \t\n\r]+|[, \t\n\r]+$").unwrap();
        println!("Processing utxo data parts: {:?}", parts);
        if parts.contains(&"}") {
            return Ok(2);
        }

        if parts.len() == 1 {
            self.utxo_data.as_mut().unwrap().push((
                re.replace_all(parts[0], "").to_string(),
                None,
                None,
            ));
            return Ok(1);
        }

        if parts.len() < 3 {
            return Err(MacroCircomError::PropertyDefinedMultipleTimes);
        }

        let comparator = CheckUtxo::match_comparator(parts[1])?;
        self.utxo_data.as_mut().unwrap().push((
            re.replace_all(parts[0], "").to_string(),
            Some(comparator),
            Some(re.replace_all(parts[2], "").to_string()),
        ));
        Ok((1))
    }
    pub fn from_input(&mut self, input: &Vec<String>) -> Result<(), MacroCircomError> {
        // is 0 when not found, 1 when found, 2 when finished
        let mut found_utxo_data = 0;
        for line in input.iter() {
            println!("{}", line);

            if found_utxo_data == 0 && self.utxo_data.is_some() {
                // found_utxo_data += 1;
                found_utxo_data = self.process_utxo_data_line(line)?;
            } else if found_utxo_data == 0 {
                println!("found utxo data ");
                self.process_line(line)?;
            } else if found_utxo_data == 1 {
                println!("found utxo data");
                found_utxo_data = self.process_utxo_data_line(line)?;
            }
        }
        Ok(())
    }

    pub fn generate_signals(&mut self) {
        // generate the signals
        // only utxo data inputs need signals
        if self.utxo_data.is_some() {
            for utxo in self.utxo_data.as_ref().unwrap() {
                self.code.push_str(generate_input_signal(&utxo.0).as_str());
            }
        }
    }

    pub fn generate_components(&mut self) -> Result<(), MacroCircomError> {
        let utxo_type = if self.is_in_utxo {
            "nIns"
        } else if self.is_out_utxo {
            "nOuts"
        } else {
            return Err(MacroCircomError::CheckUtxoInvalidFormat);
        };
        let utxo_type_prefix = if self.is_in_utxo {
            "In"
        } else if self.is_out_utxo {
            "Out"
        } else {
            return Err(MacroCircomError::CheckUtxoInvalidFormat);
        };
        self.code.push_str(&format!(
            "var {} = {};\n",
            self.name.to_lower_camel_case(),
            self.no_utxos.parse::<u64>().unwrap()
        ));
        if self.amount_sol.is_some() {
            self.code.push_str(&format!(
                "component check{}AmountSol{}[{}][{}];\n",
                utxo_type_prefix,
                self.name,
                self.name.to_lower_camel_case(),
                utxo_type
            ));
        }
        if self.amount_spl.is_some() {
            self.code.push_str(&format!(
                "component check{}AmountSpl{}[{}][{}];\n",
                utxo_type_prefix,
                self.name,
                self.name.to_lower_camel_case(),
                utxo_type
            ));
        }
        if self.asset_spl.is_some() {
            self.code.push_str(&format!(
                "component check{}AssetSpl{}[{}][{}];\n",
                utxo_type_prefix,
                self.name,
                self.name.to_lower_camel_case(),
                utxo_type
            ));
        }
        if self.app_data_hash.is_some() {
            self.code.push_str(&format!(
                "component check{}AppDataHash{}[{}][{}];\n",
                utxo_type_prefix,
                self.name,
                self.name.to_lower_camel_case(),
                utxo_type
            ));
        }
        if self.utxo_data.is_some() {
            for utxo in self.utxo_data.as_ref().unwrap() {
                if utxo.1.is_none() {
                    continue;
                }
                self.code.push_str(&format!(
                    "component check{}UtxoData{}{Name}[{name}][{}];\n",
                    utxo_type_prefix,
                    utxo.0.to_upper_camel_case(),
                    utxo_type,
                    name = self.name.to_lower_camel_case(),
                    Name = self.name,
                ));
            }
            if self.no_utxos.parse::<u64>().unwrap() > 1 {
                self.code.push_str(
                    format!(
                        "component instructionHasher[{}];
component checkInstructionHash[{}][{}];\n",
                        self.name.to_lower_camel_case(),
                        self.name.to_lower_camel_case(),
                        utxo_type
                    )
                    .as_str(),
                );
            } else {
                self.code.push_str(
                    format!(
                        "component instructionHasher{};
component checkInstructionHash{}[{}];\n",
                        self.name, self.name, utxo_type
                    )
                    .as_str(),
                );
            }
        }
        Ok(())
    }

    pub fn generate_comparison_check_code(&mut self) -> Result<(), MacroCircomError> {
        let utxo_type = if self.is_in_utxo {
            "nIns"
        } else if self.is_out_utxo {
            "nOuts"
        } else {
            return Err(MacroCircomError::CheckUtxoInvalidFormat);
        };
        let utxo_type_prefix = if self.is_in_utxo {
            "in"
        } else if self.is_out_utxo {
            "out"
        } else {
            return Err(MacroCircomError::CheckUtxoInvalidFormat);
        };
        let instruction_name = if self.instruction_name.is_some() {
            self.instruction_name.as_ref().unwrap().clone()
        } else {
            String::from("1")
        };
        self.code
            .push_str(format!("for (var i = 0; i < {}; i++) {{\n", utxo_type).as_str());
        let mut generate_equal_code_with_prefix =
            |condition: &Option<(Comparator, String)>, var: String, idx, value, inst| {
                // TODO: add match for other comparators
                if condition.is_some() {
                    let tuple: &(Comparator, String) = condition.as_ref().unwrap();
                    let variable_name = format!("{}{}", utxo_type_prefix, var);
                    self.code.push_str(
                        generate_equal_code(
                            &self.name,
                            &format!("{}{}", value, idx),
                            &variable_name,
                            &tuple.1,
                            inst,
                            utxo_type_prefix,
                        )
                        .as_str(),
                    );
                }
            };

        generate_equal_code_with_prefix(
            &self.amount_sol,
            String::from("AmountSol"),
            ".inputs[0]",
            format!("{}{}", utxo_type_prefix, String::from("AmountsHasher[i]")),
            &instruction_name,
        );

        generate_equal_code_with_prefix(
            &self.app_data_hash,
            String::from("AppDataHash"),
            "",
            format!("{}{}", utxo_type_prefix, String::from("AppDataHash[i].out")),
            &instruction_name,
        );

        generate_equal_code_with_prefix(
            &self.amount_spl,
            String::from("AmountSpl"),
            ".inputs[1]",
            format!("{}{}", utxo_type_prefix, String::from("AmountsHasher[i]")),
            &instruction_name,
        );

        generate_equal_code_with_prefix(
            &self.asset_spl,
            String::from("AssetSpl"),
            ".inputs[4]",
            format!(
                "{}{}",
                utxo_type_prefix,
                String::from("CommitmentHasher[i]")
            ),
            &instruction_name,
        );

        for utxo in self.utxo_data.as_ref().unwrap() {
            if let (name, Some(comp), Some(value)) = utxo {
                let variable_name = format!("UtxoData{}", name.to_upper_camel_case());
                println!("variable name: {}", variable_name);
                generate_equal_code_with_prefix(
                    &(Some(((*comp).clone(), value.clone()))),
                    variable_name,
                    "",
                    name.clone(),
                    &instruction_name,
                );
            }
        }

        self.code.push_str(format!("}}\n").as_str());
        Ok(())
    }

    pub fn generate_instruction_hash_code(&mut self) -> Result<(), MacroCircomError> {
        let utxo_type_prefix = if self.is_in_utxo {
            "in"
        } else if self.is_out_utxo {
            "out"
        } else {
            return Err(MacroCircomError::CheckUtxoInvalidFormat);
        };
        if let Some(utxo_data) = &self.utxo_data {
            if self.no_utxos.parse::<u64>().unwrap() > 1 {
                let loop_code = format!(
                    "for (var appUtxoIndex = 0; appUtxoIndex < nAppUtxos; appUtxoIndex++) {{\n\
                    \tinstructionHasher{}[appUtxoIndex] = Poseidon({});\n",
                    self.name.to_upper_camel_case(),
                    utxo_data.len()
                );
                self.code.push_str(&loop_code);
            } else {
                let hasher_code = format!(
                    "instructionHasher{name} = Poseidon({});\n",
                    utxo_data.len(),
                    name = self.name.to_upper_camel_case()
                );
                self.code.push_str(&hasher_code);
            }

            for (i, var) in utxo_data.iter().enumerate() {
                if var.0.contains('[') && var.0.contains(']') {
                    unimplemented!("arrays not supported yet");
                } else {
                    let input_code = if self.no_utxos.parse::<u64>().unwrap() == 1 {
                        format!(
                            "instructionHasher{name}.inputs[{}] <== {};\n",
                            i,
                            var.0,
                            name = self.name.to_upper_camel_case()
                        )
                    } else {
                        format!(
                            "instructionHasher{name}[appUtxoIndex].inputs[{}] <== {}[appUtxoIndex];\n",
                            i, var.0, name = self.name.to_upper_camel_case()
                        )
                    };
                    self.code.push_str(&input_code);
                }
            }

            let force_equal_code = if self.no_utxos.parse::<u64>().unwrap() > 1 {
                format!(
                    "for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {{\n\
                    \tcheckInstructionHash{name}[appUtxoIndex][inUtxoIndex] = ForceEqualIfEnabled();\n\
                    \tcheckInstructionHash{name}[appUtxoIndex][inUtxoIndex].in[0] <== inAppDataHash[inUtxoIndex];\n\
                    \tcheckInstructionHash{name}[appUtxoIndex][inUtxoIndex].in[1] <== instructionHasher{name}[appUtxoIndex].out;\n\
                    \tcheckInstructionHash{name}[appUtxoIndex][inUtxoIndex].enabled <== is{}AppUtxo{name}[appUtxoIndex][inUtxoIndex];\n\
                    }}\n}}\n",
                    utxo_type_prefix.to_upper_camel_case(),
                    name = self.name.to_upper_camel_case()
                )
            } else {
                format!(
                    "for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {{\n\
                    \tcheckInstructionHash{name}[inUtxoIndex] = ForceEqualIfEnabled();\n\
                    \tcheckInstructionHash{name}[inUtxoIndex].in[0] <== inAppDataHash[inUtxoIndex];\n\
                    \tcheckInstructionHash{name}[inUtxoIndex].in[1] <== instructionHasher{name}.out;\n\
                    \tcheckInstructionHash{name}[inUtxoIndex].enabled <== is{}AppUtxo{name}[inUtxoIndex];\n\
                    }}\n",
                    utxo_type_prefix.to_upper_camel_case(),
                    name = self.name.to_upper_camel_case()
                )
            };

            self.code.push_str(&force_equal_code);
        }

        Ok(())
    }
}

fn generate_equal_code(
    name: &String,
    assigning_variable_name: &String,
    variable_name: &String,
    comparing_variable_name: &String,
    instruction_name: &String,
    type_prefix: &str,
) -> String {
    format!(
        "check{variable_name}{name}[i] = ForceEqualIfEnabled();
        check{variable_name}{name}[i].in[0] <== {assigning_variable_name};
        check{variable_name}{name}[i].in[1] <== {comparing_variable_name};
        check{variable_name}{name}[i].enabled <== is{type_prefix}AppUtxo{name}[i] * {instruction_name};
        ",
        name = name,
        comparing_variable_name = comparing_variable_name,
        instruction_name = instruction_name,
        variable_name = variable_name.to_upper_camel_case(),
        type_prefix = String::from(type_prefix).to_upper_camel_case(),
    )
}

pub fn generate_check_utxo_code(
    contents: &String,
    utxo_type: &String,
    // instance: &Instance,
) -> Result<(String, Vec<CheckUtxo>), MacroCircomError> {
    // let mut checkedUtxos = Vec::<CheckUtxo>::new();
    let mut remaining_contents: String = contents.clone();
    let mut checkedUtxos = match crate::instance::CheckUtxosParser::new().parse(&remaining_contents)
    {
        Ok(instance) => instance,
        Err(error) => {
            panic!("{}", describe_error(&remaining_contents, error));
        }
    };
    for utxo in &mut checkedUtxos {
        utxo.generate_signals();
        utxo.generate_components()?;
        utxo.generate_instruction_hash_code()?;
        utxo.generate_comparison_check_code()?;
    }

    // let mut string_remaining_contents = String::new();
    // for i in 0..4 {
    // starts with #[check{}Utxo( // In or Out
    // let (extractedCheckInUtxos, header_string, _remaining_contents, is_empty) =
    //     parse_general_between_curly_brackets(
    //         &remaining_contents,
    //         &format!("#[check{}Utxo(", utxo_type),
    //         false,
    //         false,
    //     )?;
    // println!("is empty: {}", is_empty);

    // remaining_contents = _remaining_contents;
    // println!("i {}", i);
    // println!("remaining contents: {}", remaining_contents);
    // if is_empty {
    //     return Ok((remaining_contents, checkedUtxos));
    // }
    // let check_utxo_res = crate::instance::CheckUtxoTypeParser::new().parse(&remaining_contents); //CheckUtxo::new().parse(&remaining_contents)?;
    // let mut check_utxo = CheckUtxo::parse_header(&header_string)?;
    // check_utxo.parse_utxo_data(utxoData);
    // got all the info now generate the code
    // generate the input signals
    // generate the components
    // generate the loop
    // generate the components inside the loop
    // close the loop

    Ok((remaining_contents, checkedUtxos))
}

// TODO:
// - add full test
// instruction hasher does not add name
//
// - test with circom
// - put into main.rs
// - test in voting

/*
let (instruction_hash_code, contents, utxo_data_variable_names) = parse_general(
    &contents,
    &String::from("#[utxoData]"),
    generate_instruction_hash_code,
    true,
    &instance,
)?;
*/

pub fn parse_general_between_curly_brackets(
    input: &String,
    starting_string: &String,
    critical: bool,
    multiple: bool,
) -> Result<(Vec<String>, String, String, bool), MacroCircomError> {
    let mut found_bracket = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance: u8 = 0;
    let mut commented = false;
    let mut bracket_str = Vec::<String>::new();
    let mut header_string = String::new();
    for line in input.lines() {
        if found_instance > 1 && !multiple {
            remaining_lines.push(line);
            found_bracket = false;
            continue;
        }
        println!("found instance: {}", found_instance);
        println!("line: {}", line);
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
        if found_instance == 0 && line.starts_with(starting_string) {
            // cannot accept overloads implementations
            // if found_instance == true {
            //     panic!();
            // };
            found_instance += 1;
            found_bracket = true;
            header_string = line.to_string();
            if found_instance > 1 && !multiple {
                bracket_str.push(line.to_string());
                found_bracket = false;
            }
            continue;
        }
        if found_instance > 0 && line.starts_with("{") {
            continue;
        }
        if found_bracket && found_instance > 0 && line.starts_with("}") || line.ends_with("}") {
            found_bracket = false;
            remaining_lines.push(line);
            // if multiple {
            //     found_instance += 1;
            // }
            continue;
        }

        if found_bracket {
            bracket_str.push(line.to_string());
        }
        if !found_bracket {
            remaining_lines.push(line);
        }
    }
    if found_bracket {
        println!("remaining lines {}", remaining_lines.join("\n"));
        println!("bracket str {}", bracket_str.join("\n"));
        println!("header string {}", header_string);
        println!("input {}", input);
        panic!();
        return Err(ParseInstanceError(input.to_string()));
    }
    if found_instance == 0 && critical {
        return Err(ParseInstanceError(input.to_string()));
    }

    Ok((
        bracket_str,
        header_string,
        remaining_lines.join("\n"),
        found_instance == 0,
    ))
}

mod tests {
    use crate::checkUtxo;

    use super::*;

    #[test]
    fn test_parse_header() {
        // Valid input with instruction_name
        let header = "#[checkInUtxo(inputUtxo,1,test)]";
        let result = CheckUtxo::parse_header(header);
        assert!(result.is_ok());
        let utxo = result.unwrap();
        assert_eq!(utxo.name, "InputUtxo");
        assert_eq!(utxo.no_utxos, "1");
        assert_eq!(utxo.instruction_name, Some("test".to_string()));

        // Valid input without instruction_name
        let header2 = "#[checkInUtxo(inputUtxo,2)]";
        let result2 = CheckUtxo::parse_header(header2);
        assert!(result2.is_ok());
        let utxo2 = result2.unwrap();
        assert_eq!(utxo2.name, "InputUtxo");
        assert_eq!(utxo2.no_utxos, "2");
        assert_eq!(utxo2.instruction_name, None);

        // Invalid format
        let header3 = "#[invalidHeader(inputUtxo,1)]";
        let result3 = CheckUtxo::parse_header(header3);
        assert_eq!(result3, Err(MacroCircomError::CheckUtxoInvalidHeaderFormat));
    }

    #[test]
    fn generate_check_in_utxo_code_test() {
        let contents = String::from(
            "#[checkInUtxo(utxoName, 1, instruction)]
        // to append to otherwise duplicate identifiers
       {
            amountSol == sth, // enable comparisons ==, <=, <, =>, >
            amountSpl == sth,
            assetSpl == sth,
            // blinding == sth,
            appDataHash == sth,
            // poolType: sth, // always 0
            // verifierPubkey: // has to be this verifier
            utxoData: {
               attribute1,
               attribute2 == testComparison,
               }
           }",
        );

        let (extractedCheckInUtxos, header_string, remainingContents, is_empty) =
            parse_general_between_curly_brackets(
                &contents,
                &String::from("#[checkInUtxo("),
                false,
                false,
            )
            .unwrap();
        let mut check_utxo = CheckUtxo::parse_header(&header_string).unwrap();
        assert_eq!(check_utxo.name, "UtxoName");
        assert_eq!(check_utxo.no_utxos, "1");
        assert_eq!(
            check_utxo.instruction_name,
            Some(String::from("instruction"))
        );
        check_utxo.from_input(&extractedCheckInUtxos).unwrap();
        assert_eq!(
            check_utxo.amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
    }

    #[test]
    fn generate_signals_test() {
        // Setting up a CheckUtxo instance with mock data
        let mut check_utxo = CheckUtxo {
            code: String::new(),
            name: "UtxoName".to_string(),
            is_in_utxo: false,
            is_out_utxo: false,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: None,
            asset_spl: None,
            app_data_hash: None,
            utxo_data: Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ]),
        };

        // Generating signals
        check_utxo.generate_signals();

        // Expected code output
        let expected_output = format!(
            "{}{}",
            generate_input_signal(&String::from("attribute1")),
            generate_input_signal(&String::from("attribute2"))
        );
        // Asserting that the generated code matches the expected output exactly
        assert_eq!(check_utxo.code, expected_output);
    }

    #[test]
    fn generate_components_test() -> Result<(), MacroCircomError> {
        // Setting up a CheckUtxo instance with mock data
        let mut check_utxo = CheckUtxo {
            code: String::new(),
            name: "UtxoName".to_string(),
            is_in_utxo: true,
            is_out_utxo: false,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: Some((Comparator::Equal, "sth".to_string())),
            asset_spl: Some((Comparator::Equal, "sth".to_string())),
            app_data_hash: Some((Comparator::Equal, "sth".to_string())),
            utxo_data: Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ]),
        };

        // Generating components
        check_utxo.generate_components()?;
        println!("{}", check_utxo.code);
        // Constructing the expected code output
        let expected_output = String::from(
            "var utxoName = 1;\n\
            component checkInAmountSolUtxoName[utxoName][nIns];\n\
            component checkInAmountSplUtxoName[utxoName][nIns];\n\
            component checkInAssetSplUtxoName[utxoName][nIns];\n\
            component checkInAppDataHashUtxoName[utxoName][nIns];\n\
            component checkInUtxoDataAttribute2UtxoName[utxoName][nIns];\n\
            component instructionHasherUtxoName;\n\
            component checkInstructionHashUtxoName[nIns];\n",
        );
        // Asserting that the generated code matches the expected output
        assert_eq!(check_utxo.code, expected_output);

        Ok(())
    }

    #[test]
    fn generate_instruction_hash_code_test() -> Result<(), MacroCircomError> {
        // Setting up a CheckUtxo instance with mock data
        let mut check_utxo = CheckUtxo {
            code: String::new(),
            name: "utxoName".to_string(),
            is_in_utxo: true,
            is_out_utxo: false,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: Some((Comparator::Equal, "sth".to_string())),
            asset_spl: Some((Comparator::Equal, "sth".to_string())),
            app_data_hash: Some((Comparator::Equal, "sth".to_string())),
            utxo_data: Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ]),
        };

        // Calling the generate_instruction_hash_code method
        check_utxo.generate_instruction_hash_code()?;

        // Constructing the expected code output
        let expected_output = r#"instructionHasherUtxoName = Poseidon(2);
instructionHasherUtxoName.inputs[0] <== attribute1;
instructionHasherUtxoName.inputs[1] <== attribute2;
for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {
	checkInstructionHashUtxoName[inUtxoIndex] = ForceEqualIfEnabled();
	checkInstructionHashUtxoName[inUtxoIndex].in[0] <== inAppDataHash[inUtxoIndex];
	checkInstructionHashUtxoName[inUtxoIndex].in[1] <== instructionHasherUtxoName.out;
	checkInstructionHashUtxoName[inUtxoIndex].enabled <== isInAppUtxoUtxoName[inUtxoIndex];
}
"#;

        // Asserting that the generated code matches the expected output
        assert_eq!(check_utxo.code, expected_output);

        Ok(())
    }

    #[test]
    fn generate_comparison_check_code_test() -> Result<(), MacroCircomError> {
        // Setting up a CheckUtxo instance with mock data
        let mut check_utxo = CheckUtxo {
            code: String::new(),
            name: "UtxoName".to_string(),
            is_in_utxo: true,
            is_out_utxo: false,
            instruction_name: Some("instruction".to_string()),
            no_utxos: String::from("1"),
            amount_sol: Some((Comparator::Equal, "sth".to_string())),
            amount_spl: Some((Comparator::Equal, "sth1".to_string())),
            asset_spl: Some((Comparator::Equal, "sth2".to_string())),
            app_data_hash:Some((Comparator::Equal, "sth3".to_string())),
            utxo_data:// None,     //
                                  Some(vec![
                                     ("attribute1".to_string(), None, None),
                                     (
                                         "attribute2".to_string(),
                                         Some(Comparator::Equal),
                                         Some("testComparison".to_string()),
                                     ),
                                 ]),
        };

        // Calling the generate_comparison_check_code method
        check_utxo.generate_comparison_check_code()?;

        // Constructing the expected code output
        let expected_output = r#"for (var i = 0; i < nIns; i++) {
checkInAmountSolUtxoName[i] = ForceEqualIfEnabled();
        checkInAmountSolUtxoName[i].in[0] <== inAmountsHasher[i].inputs[0];
        checkInAmountSolUtxoName[i].in[1] <== sth;
        checkInAmountSolUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;
        checkInAppDataHashUtxoName[i] = ForceEqualIfEnabled();
        checkInAppDataHashUtxoName[i].in[0] <== inAppDataHash[i].out;
        checkInAppDataHashUtxoName[i].in[1] <== sth3;
        checkInAppDataHashUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;
        checkInAmountSplUtxoName[i] = ForceEqualIfEnabled();
        checkInAmountSplUtxoName[i].in[0] <== inAmountsHasher[i].inputs[1];
        checkInAmountSplUtxoName[i].in[1] <== sth1;
        checkInAmountSplUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;
        checkInAssetSplUtxoName[i] = ForceEqualIfEnabled();
        checkInAssetSplUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[4];
        checkInAssetSplUtxoName[i].in[1] <== sth2;
        checkInAssetSplUtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;
        checkInUtxoDataAttribute2UtxoName[i] = ForceEqualIfEnabled();
        checkInUtxoDataAttribute2UtxoName[i].in[0] <== attribute2;
        checkInUtxoDataAttribute2UtxoName[i].in[1] <== testComparison;
        checkInUtxoDataAttribute2UtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;
        }
"#;
        println!("code {}", check_utxo.code);
        // Asserting that the generated code matches the expected output
        assert_eq!(check_utxo.code, expected_output);

        Ok(())
    }

    #[test]
    fn complete_test() {
        let contents = String::from(
            "#[checkInUtxo(utxoName, 1, instruction)]
        // to append to otherwise duplicate identifiers
       {
            amountSol == sth, // enable comparisons ==, <=, <, =>, >
            amountSpl == sth1,
            assetSpl == sth2,
            // blinding == sth,
            appDataHash == sth3,
            // poolType: sth, // always 0
            // verifierPubkey: // has to be this verifier
            utxoData: {
               attribute1,
               attribute2 == testComparison,
               }
           }",
        );
        let (remainingContent, checkedUtxos) =
            generate_check_utxo_code(&contents, &String::from("In")).unwrap();
        let checkUtxo = checkedUtxos[0].clone();
        println!("code {}", checkUtxo.code);
        // assert_eq!(remainingContent, "}\n}");
        assert_eq!(checkUtxo.name, "utxoName");
        assert_eq!(checkUtxo.no_utxos, "1");
        assert_eq!(
            checkUtxo.instruction_name,
            Some(String::from("instruction"))
        );
        assert_eq!(
            checkUtxo.amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
        assert_eq!(
            checkUtxo.amount_spl,
            Some((Comparator::Equal, String::from("sth1")))
        );
        assert_eq!(
            checkUtxo.asset_spl,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            checkUtxo.app_data_hash,
            Some((Comparator::Equal, String::from("sth3")))
        );
        assert_eq!(
            checkUtxo.utxo_data,
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

    #[test]
    fn complete_test_2() {
        let contents = String::from(
            "#[checkInUtxo(utxoName, 1, instruction)]
        // to append to otherwise duplicate identifiers
        {
            amountSol == sth, // enable comparisons ==, <=, <, =>, >
            amountSpl == sth1,
            assetSpl == sth2,
            // blinding == sth,
            appDataHash == sth3,
            // poolType: sth, // always 0
            // verifierPubkey: // has to be this verifier
            utxoData: {
                attribute1,
                attribute2 == testComparison,
                }
            }
           #[checkInUtxo(utxoName1, 1, instruction1)]
        // to append to otherwise duplicate identifiers
       {
            amountSol == sth2, // enable comparisons ==, <=, <, =>, >
            amountSpl == sth12,
            assetSpl == sth22,
            // blinding == sth2,
            appDataHash == sth32,
            // poolType: sth, // always 0
            // verifierPubkey: // has to be this verifier
            utxoData: {
               attribute11,
               attribute21 == testComparison1,
               }
           }",
        );
        let (remainingContent, checkedUtxos) =
            generate_check_utxo_code(&contents, &String::from("In")).unwrap();
        let checkUtxo = checkedUtxos[0].clone();
        assert_eq!(checkedUtxos.len(), 2);
        println!("code {}", checkUtxo.code);
        // assert_eq!(remainingContent, "}\n}\n}\n}");
        assert_eq!(checkUtxo.name, "utxoName");
        assert_eq!(checkUtxo.no_utxos, "1");
        assert_eq!(
            checkUtxo.instruction_name,
            Some(String::from("instruction"))
        );
        assert_eq!(
            checkUtxo.amount_sol,
            Some((Comparator::Equal, String::from("sth")))
        );
        assert_eq!(
            checkUtxo.amount_spl,
            Some((Comparator::Equal, String::from("sth1")))
        );
        assert_eq!(
            checkUtxo.asset_spl,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            checkUtxo.app_data_hash,
            Some((Comparator::Equal, String::from("sth3")))
        );
        assert_eq!(
            checkUtxo.utxo_data,
            Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ])
        );
        // println!("code {}", checkedUtxos[1].code);

        let checkUtxo1 = checkedUtxos[1].clone();
        assert_eq!(checkUtxo1.name, "utxoName1");
        assert_eq!(checkUtxo1.no_utxos, "1");
        assert_eq!(
            checkUtxo1.instruction_name,
            Some(String::from("instruction1"))
        );
        assert_eq!(
            checkUtxo1.amount_sol,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            checkUtxo1.amount_spl,
            Some((Comparator::Equal, String::from("sth12")))
        );
        assert_eq!(
            checkUtxo1.asset_spl,
            Some((Comparator::Equal, String::from("sth22")))
        );
        assert_eq!(
            checkUtxo1.app_data_hash,
            Some((Comparator::Equal, String::from("sth32")))
        );
        assert_eq!(
            checkUtxo1.utxo_data,
            Some(vec![
                ("attribute11".to_string(), None, None),
                (
                    "attribute21".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison1".to_string()),
                ),
            ])
        );
    }
}

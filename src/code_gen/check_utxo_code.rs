use crate::errors::MacroCircomError;
use heck::{ToLowerCamelCase, ToUpperCamelCase};

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

// new approach:
// - do one template for one checked utxo
// - create this template multiple times
// -> reduce complexity

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

    pub fn generate_signals(&mut self) {
        // generate the signals
        // only utxo data inputs need signals
        self.code.push_str(
            format!(
                "signal input is{utxo_type}AppUtxo{name}[n{utxo_type}s];\n",
                utxo_type = if self.is_in_utxo { "In" } else { "Out" },
                name = self.name.to_upper_camel_case()
            )
            .as_str(),
        );
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
                    component checkInstructionHash{}[{}][{}];\n",
                        self.name.to_lower_camel_case(),
                        self.name.to_lower_camel_case(),
                        self.no_utxos.parse::<u64>().unwrap(),
                        utxo_type
                    )
                    .as_str(),
                );
            } else {
                self.code.push_str(
                    format!(
                        "component instructionHasher{};
component checkInstructionHash{}[{}];\n",
                        self.name.to_upper_camel_case(),
                        self.name.to_upper_camel_case(),
                        utxo_type
                    )
                    .as_str(),
                );
            }
        }
        Ok(())
    }

    pub fn generate_comparison_check_code(&mut self) -> Result<(), MacroCircomError> {
        let template = r#"
for (var i = 0; i < {{is_in}}; i++) {
        {{#each comparisons}}
        {{#with this}}
    check{{this.component}}{{../../utxoName}}[i] = ForceEqualIfEnabled();
    check{{this.component}}{{../../utxoName}}[i].in[0] <== {{this.hasher}}[i].{{this.input}};
    check{{this.component}}{{../../utxoName}}[i].in[1] <== {{this.comparison}};
    check{{this.component}}{{../../utxoName}}[i].enabled <== {{../../isInAppUtxo}}{{../../utxoName}}[i] * {{../../instruction}};

{{/with}}{{/each}}
{{#each comparisonsUtxoData}}
        {{#with this}}
    check{{this.component}}{{../../utxoName}}[i] = ForceEqualIfEnabled();
    check{{this.component}}{{../../utxoName}}[i].in[0] <== {{this.input}};
    check{{this.component}}{{../../utxoName}}[i].in[1] <== {{this.comparison}};
    check{{this.component}}{{../../utxoName}}[i].enabled <== {{../../isInAppUtxo}}{{../../utxoName}}[i] * {{../../instruction}};

{{/with}}{{/each}}
    }
"#;
        let mut comparisons = Vec::<handlebars::JsonValue>::new();
        if self.amount_sol.is_some() {
            comparisons.push(serde_json::json!({
                "component": "InAmountSol",
                "hasher": "inAmountsHasher",
                "input": "inputs[0]",
                "comparison": self.amount_sol.as_ref().unwrap().1,
            }));
        }
        if self.app_data_hash.is_some() {
            comparisons.push(serde_json::json!({
                "component": "InAppDataHash",
                "hasher": "inAppDataHash",
                "input": "out",
                "comparison": self.app_data_hash.as_ref().unwrap().1,
            }));
        }
        if self.amount_spl.is_some() {
            comparisons.push(serde_json::json!({
                "component": "InAmountSpl",
                "hasher": "inAmountsHasher",
                "input": "inputs[1]",
                "comparison": self.amount_spl.as_ref().unwrap().1,
            }));
        }
        if self.asset_spl.is_some() {
            comparisons.push(serde_json::json!({
                "component": "InAssetSpl",
                "hasher": "inCommitmentHasher",
                "input": "inputs[4]",
                "comparison": self.asset_spl.as_ref().unwrap().1,
            }));
        }

        let mut comparisons_utxo_data = Vec::<handlebars::JsonValue>::new();

        for triple in self.utxo_data.as_ref().unwrap() {
            if triple.1.is_some() || triple.2.is_some() {
                comparisons_utxo_data.push(serde_json::json!({
                    "component": format!("InUtxoData{}", triple.0.to_upper_camel_case()),
                    "input": triple.0,
                    "comparison": triple.2.as_ref().unwrap(),
                }));
            }
        }

        let handlebars = handlebars::Handlebars::new();
        let data = serde_json::json!({
            "is_in": if self.is_in_utxo { "nIns" } else { "nOuts" },
            "utxoName": "UtxoName",
            "instruction": "instruction",
            "isInAppUtxo": "isInAppUtxo",
            "comparisons": comparisons,
            "comparisonsUtxoData": comparisons_utxo_data,
        });
        let res = handlebars.render_template(template, &data).unwrap();
        self.code.push_str(&res);
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

pub fn generate_check_utxo_code(checked_utxo: &mut Vec<CheckUtxo>) -> Result<(), MacroCircomError> {
    // got all the info now generate the code
    // generate the input signals
    // generate the components
    // generate the loop
    // generate the components inside the loop
    // close the loop
    for utxo in checked_utxo {
        if utxo.no_utxos.parse::<u64>().unwrap() == 0 {
            continue;
        } else if utxo.no_utxos.parse::<u64>().unwrap() > 1 {
            unimplemented!("Multiple utxos not supported yet.");
        }
        utxo.generate_signals();
        utxo.generate_components()?;
        utxo.generate_instruction_hash_code()?;
        utxo.generate_comparison_check_code()?;
    }

    Ok(())
}

// TODO:
// - add full test
// instruction hasher does not add name
//
// - test with circom
// - put into main.rs
// - test in voting

mod tests {
    use handlebars::{Context, Helper, HelperResult, RenderContext};

    use crate::{code_gen::circom_code::format_custom_data, describe_error};

    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::utils::assert_syn_eq;
    /* TODO: rewrite with parser
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
    */
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
            "signal input isOutAppUtxoUtxoName[nOuts];\n{}{}",
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
        // Asserting that the generated code matches the expected output
        assert_eq!(
            remove_formatting(&check_utxo.code),
            remove_formatting(expected_output)
        );

        Ok(())
    }

    #[test]
    fn generate_comparison_check_code_test2() -> Result<(), MacroCircomError> {
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
        // Asserting that the generated code matches the expected output
        // assert_eq!(
        //     format_custom_data(&check_utxo.code),
        //     format_custom_data(expected_output)
        // );

        assert_eq!(
            remove_formatting(&check_utxo.code),
            remove_formatting(expected_output)
        );
        Ok(())
    }

    fn remove_formatting(input: &str) -> String {
        let res: Vec<String> = input
            .split_whitespace()
            .map(|token| {
                token
                    .chars()
                    .filter(|ch| ch.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|token| !token.is_empty())
            .collect();
        res.join("")
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
        let parsing_res = match crate::macro_parser::LightFileParser::new().parse(&contents) {
            Ok(instance) => instance,
            Err(error) => {
                println!("Parsing check utxo error.");
                panic!("{}", describe_error(&contents, error));
            }
        };

        let mut checked_utxos = match parsing_res.1 {
            Some(checked_utxos) => checked_utxos,
            None => Vec::<CheckUtxo>::new(),
        };
        generate_check_utxo_code(&mut checked_utxos).unwrap();
        let check_utxo = checked_utxos[0].clone();
        println!("code {}", check_utxo.code);
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

    #[test]
    fn complete_test_2() {
        let contents = String::from(
            "pragma circom 2.1.4;
            include \"../../node_modules/circomlib/circuits/poseidon.circom\";
            #[checkInUtxo(utxoName, 1, instruction)]
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
        let parsing_res = match crate::macro_parser::LightFileParser::new().parse(&contents) {
            Ok(instance) => instance,
            Err(error) => {
                println!("Parsing check utxo error.");
                panic!("{}", describe_error(&contents, error));
            }
        };

        let mut checked_utxos = match parsing_res.1 {
            Some(checked_utxos) => checked_utxos,
            None => Vec::<CheckUtxo>::new(),
        };
        generate_check_utxo_code(&mut checked_utxos).unwrap();
        let check_utxo = checked_utxos[0].clone();
        assert_eq!(checked_utxos.len(), 2);
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
        // println!("code {}", checked_utxo[1].code);

        let check_utxo1 = checked_utxos[1].clone();
        assert_eq!(check_utxo1.name, "utxoName1");
        assert_eq!(check_utxo1.no_utxos, "1");
        assert_eq!(
            check_utxo1.instruction_name,
            Some(String::from("instruction1"))
        );
        assert_eq!(
            check_utxo1.amount_sol,
            Some((Comparator::Equal, String::from("sth2")))
        );
        assert_eq!(
            check_utxo1.amount_spl,
            Some((Comparator::Equal, String::from("sth12")))
        );
        assert_eq!(
            check_utxo1.asset_spl,
            Some((Comparator::Equal, String::from("sth22")))
        );
        assert_eq!(
            check_utxo1.app_data_hash,
            Some((Comparator::Equal, String::from("sth32")))
        );
        assert_eq!(
            check_utxo1.utxo_data,
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

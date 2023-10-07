use crate::errors::MacroCircomError;
use heck::ToUpperCamelCase;
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

// TODO: enable array support for utxo data
// - add optional array recognition to grammar
// - add 4th option to utxo_data which is a vector of strings which are the array dimension sizes
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
            verifier_address: None,
            blinding: None,
            tx_version: None,
            pool_type: None,
        }
    }

    pub fn generate_code(&mut self) -> Result<(), MacroCircomError> {
        let template = r#"

signal input isInAppUtxo{{utxoName}}[{{is_ins}}];

{{#with this}}{{#each allUtxoData}}
signal input {{this.input}};
{{/each}}{{/with}}



{{#each comparisonsUtxoData}} {{#with this}}
component check{{this.component}}{{../../utxoName}}[{{../../is_ins}}];
{{/with}}{{/each}}

{{#if comparisons}}
{{#each comparisons}}{{#with this}}
component check{{this.component}}{{../../utxoName}}[{{../../is_ins}}];
{{/with}}{{/each}}
for (var i = 0; i < {{is_ins}}; i++) {

    {{#with this}} {{#each comparisons}} 

    check{{is_In}}{{this.component}}{{../../utxoName}}[i] = ForceEqualIfEnabled();
    check{{is_In}}{{this.component}}{{../../utxoName}}[i].in[0] <== {{../../is_in}}{{this.hasher}}[i].{{this.input}};
    check{{is_In}}{{this.component}}{{../../utxoName}}[i].in[1] <== {{this.comparison}};
    check{{is_In}}{{this.component}}{{../../utxoName}}[i].enabled <== {{../../isInAppUtxo}}{{../../utxoName}}[i] * {{../../instruction}};

{{/each}}{{/with}}

{{#each comparisonsUtxoData}}{{#with this}}

    check{{this.component}}{{../../utxoName}}[i] = ForceEqualIfEnabled();
    check{{this.component}}{{../../utxoName}}[i].in[0] <== {{this.input}};
    check{{this.component}}{{../../utxoName}}[i].in[1] <== {{this.comparison}};
    check{{this.component}}{{../../utxoName}}[i].enabled <== isInAppUtxo{{../../utxoName}}[i] * {{../../instruction}};

{{/with}}{{/each}}
}
{{/if}}

{{#if len_utxo_data_non_zero}}
component instructionHasher{{utxoName}} = Poseidon({{utxo_data_length}});

{{#with this}}{{#each allUtxoData}}
instructionHasher{{../../utxoName}}.inputs[{{@index}}] <== {{this.input}};
{{/each}}{{/with}}

component checkInstructionHash{{utxoName}}[{{is_ins}}];
for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {
    checkInstructionHash{{utxoName}}[inUtxoIndex] = ForceEqualIfEnabled();
    checkInstructionHash{{utxoName}}[inUtxoIndex].in[0] <== {{is_in}}AppDataHash[inUtxoIndex];
    checkInstructionHash{{utxoName}}[inUtxoIndex].in[1] <== instructionHasher{{utxoName}}.out;
    checkInstructionHash{{utxoName}}[inUtxoIndex].enabled <== is{{is_In}}AppUtxo{{utxoName}}[inUtxoIndex];
}
{{/if}}
"#;
        let mut comparisons = Vec::<handlebars::JsonValue>::new();
        if self.amount_sol.is_some() {
            comparisons.push(serde_json::json!({
                "component": "AmountSol",
                "hasher": "AmountsHasher",
                "input": "inputs[0]",
                "comparison": self.amount_sol.as_ref().unwrap().1,
            }));
        }
        if self.app_data_hash.is_some() {
            comparisons.push(serde_json::json!({
                "component": "AppDataHash",
                "hasher": "AppDataHash",
                "input": "out",
                "comparison": self.app_data_hash.as_ref().unwrap().1,
            }));
        }
        if self.amount_spl.is_some() {
            comparisons.push(serde_json::json!({
                "component": "AmountSpl",
                "hasher": "AmountsHasher",
                "input": "inputs[1]",
                "comparison": self.amount_spl.as_ref().unwrap().1,
            }));
        }
        if self.asset_spl.is_some() {
            comparisons.push(serde_json::json!({
                "component": "AssetSpl",
                "hasher": "CommitmentHasher",
                "input": "inputs[4]",
                "comparison": self.asset_spl.as_ref().unwrap().1,
            }));
        }

        if self.pool_type.is_some() {
            comparisons.push(serde_json::json!({
                "component": "PoolType",
                "hasher": "CommitmentHasher",
                "input": "inputs[6]",
                "comparison": self.pool_type.as_ref().unwrap().1,
            }));
        }

        if self.verifier_address.is_some() {
            comparisons.push(serde_json::json!({
                "component": "VerifierAddress",
                "hasher": "CommitmentHasher",
                "input": "inputs[7]",
                "comparison": self.verifier_address.as_ref().unwrap().1,
            }));
        }

        if self.blinding.is_some() {
            comparisons.push(serde_json::json!({
                "component": "Blinding",
                "hasher": "CommitmentHasher",
                "input": "inputs[3]",
                "comparison": self.blinding.as_ref().unwrap().1,
            }));
        }

        if self.tx_version.is_some() {
            comparisons.push(serde_json::json!({
                "component": "TxVersion",
                "hasher": "CommitmentHasher",
                "input": "inputs[0]",
                "comparison": self.tx_version.as_ref().unwrap().1,
            }));
        }

        let mut comparisons_utxo_data = Vec::<handlebars::JsonValue>::new();

        for utxo_data in self.utxo_data.as_ref().unwrap() {
            if utxo_data.1.is_some() || utxo_data.2.is_some() {
                comparisons_utxo_data.push(serde_json::json!({
                    "component": format!("UtxoData{}", utxo_data.0.to_upper_camel_case()),
                    "input": utxo_data.0,
                    "comparison": utxo_data.2.as_ref().unwrap(),
                }));
            }
        }

        let mut all_utxo_data = Vec::<handlebars::JsonValue>::new();
        println!("utxo data: {:?}", self.utxo_data);
        for utxo_data in self.utxo_data.as_ref().unwrap() {
            if utxo_data.1.is_some() || utxo_data.2.is_some() {
                all_utxo_data.push(serde_json::json!({
                    "component": format!("UtxoData{}", utxo_data.0.to_upper_camel_case()),
                    "input": utxo_data.0,
                    "comparison": utxo_data.2.as_ref().unwrap(),
                }));
            } else if utxo_data.1.is_none() && utxo_data.2.is_none() {
                all_utxo_data.push(serde_json::json!({
                    "component": format!("UtxoData{}", utxo_data.0.to_upper_camel_case()),
                    "input": utxo_data.0,
                }));
            }
        }

        let handlebars = handlebars::Handlebars::new();

        // This handles the case that we want to check a utxo that is not a program utxo the utxo data is zero
        let len_utxo_data = if self.utxo_data.as_ref().is_some() {
            self.utxo_data.as_ref().unwrap().len()
        } else {
            0
        };
        let data = serde_json::json!({
            "is_ins": if self.is_in_utxo { "nIns" } else { "nOuts" },
            "is_In": if self.is_in_utxo { "In" } else { "Out" },
            "is_in": if self.is_in_utxo { "in" } else { "out" },
            "utxoName": self.name.to_upper_camel_case(),
            "instruction": self.instruction_name,
            "comparisons": comparisons,
            "comparisonsUtxoData": comparisons_utxo_data,
            "allUtxoData": all_utxo_data,
            "utxo_data_length": len_utxo_data,
            "len_utxo_data_non_zero": len_utxo_data > 0,
        });
        println!("data: {:?}", data);
        let res = handlebars.render_template(template, &data).unwrap();
        self.code.push_str(&res);
        Ok(())
    }
}

pub fn generate_check_utxo_code(checked_utxo: &mut Vec<CheckUtxo>) -> Result<(), MacroCircomError> {
    for utxo in checked_utxo {
        if utxo.no_utxos.parse::<u64>().unwrap() == 0 {
            continue;
        } else if utxo.no_utxos.parse::<u64>().unwrap() > 1 {
            unimplemented!("Multiple utxos not supported yet.");
        }
        utxo.generate_code()?;
    }

    Ok(())
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::describe_error;
    #[allow(unused_imports)]
    use crate::utils::assert_syn_eq;
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
            app_data_hash: Some((Comparator::Equal, "sth3".to_string())),
            blinding: Some((Comparator::Equal, "sthB".to_string())),
            tx_version: Some((Comparator::Equal, "sthT".to_string())),
            pool_type: Some((Comparator::Equal, "sthP".to_string())),
            verifier_address: Some((Comparator::Equal, "sthV".to_string())),
            utxo_data: Some(vec![
                ("attribute1".to_string(), None, None),
                (
                    "attribute2".to_string(),
                    Some(Comparator::Equal),
                    Some("testComparison".to_string()),
                ),
            ]),
        };
        check_utxo.generate_code()?;
        println!("code {}", check_utxo.code);
        let expected_output = r#"
signal input isInAppUtxoUtxoName[nIns];

signal input attribute1;

signal input attribute2;

component checkUtxoDataAttribute2UtxoName[nIns];
component checkAmountSolUtxoName[nIns];
component checkAppDataHashUtxoName[nIns];
component checkAmountSplUtxoName[nIns];
component checkAssetSplUtxoName[nIns];
component checkPoolTypeUtxoName[nIns];
component checkVerifierAddressUtxoName[nIns];
component checkBlindingUtxoName[nIns];
component checkTxVersionUtxoName[nIns];

for (var i = 0; i < nIns; i++) {

    checkAmountSolUtxoName[i] = ForceEqualIfEnabled();
    checkAmountSolUtxoName[i].in[0] <== inAmountsHasher[i].inputs[0];
    checkAmountSolUtxoName[i].in[1] <== sth;
    checkAmountSolUtxoName[i].enabled <== UtxoName[i] * instruction; 

    checkAppDataHashUtxoName[i] = ForceEqualIfEnabled();
    checkAppDataHashUtxoName[i].in[0] <== inAppDataHash[i].out;
    checkAppDataHashUtxoName[i].in[1] <== sth3;
    checkAppDataHashUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkAmountSplUtxoName[i] = ForceEqualIfEnabled();
    checkAmountSplUtxoName[i].in[0] <== inAmountsHasher[i].inputs[1];
    checkAmountSplUtxoName[i].in[1] <== sth1;
    checkAmountSplUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkAssetSplUtxoName[i] = ForceEqualIfEnabled();
    checkAssetSplUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[4];
    checkAssetSplUtxoName[i].in[1] <== sth2;
    checkAssetSplUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkPoolTypeUtxoName[i] = ForceEqualIfEnabled();
    checkPoolTypeUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[6];
    checkPoolTypeUtxoName[i].in[1] <== sthP;
    checkPoolTypeUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkVerifierAddressUtxoName[i] = ForceEqualIfEnabled();
    checkVerifierAddressUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[7];
    checkVerifierAddressUtxoName[i].in[1] <== sthV;
    checkVerifierAddressUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkBlindingUtxoName[i] = ForceEqualIfEnabled();
    checkBlindingUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[3];
    checkBlindingUtxoName[i].in[1] <== sthB;
    checkBlindingUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkTxVersionUtxoName[i] = ForceEqualIfEnabled();
    checkTxVersionUtxoName[i].in[0] <== inCommitmentHasher[i].inputs[0];
    checkTxVersionUtxoName[i].in[1] <== sthT;
    checkTxVersionUtxoName[i].enabled <== UtxoName[i] * instruction;

    checkUtxoDataAttribute2UtxoName[i] = ForceEqualIfEnabled();
    checkUtxoDataAttribute2UtxoName[i].in[0] <== attribute2;
    checkUtxoDataAttribute2UtxoName[i].in[1] <== testComparison;
    checkUtxoDataAttribute2UtxoName[i].enabled <== isInAppUtxoUtxoName[i] * instruction;

}

component instructionHasherUtxoName = Poseidon(2);


instructionHasherUtxoName.inputs[0] <== attribute1;
instructionHasherUtxoName.inputs[1] <== attribute2;


component checkInstructionHashUtxoName[nIns];
for (var inUtxoIndex = 0; inUtxoIndex < nIns; inUtxoIndex++) {
    checkInstructionHashUtxoName[inUtxoIndex] = ForceEqualIfEnabled();
    checkInstructionHashUtxoName[inUtxoIndex].in[0] <== inAppDataHash[inUtxoIndex];
    checkInstructionHashUtxoName[inUtxoIndex].in[1] <== instructionHasherUtxoName.out;
    checkInstructionHashUtxoName[inUtxoIndex].enabled <== isInAppUtxoUtxoName[inUtxoIndex];
}
"#;
        // Asserting that the generated code matches the expected output
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

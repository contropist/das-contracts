use crate::util::{error::Error, template_common_cell::*, template_generator::*, template_parser::*};
use das_types_std::{constants::*, packed::*};
use serde_json::json;

fn init(action: &str) -> TemplateGenerator {
    let mut template = TemplateGenerator::new(action, Some(Bytes::from(vec![0])));

    template.push_contract_cell("fake-das-lock", true);
    template.push_contract_cell("fake-secp256k1-blake160-signhash-all", true);
    template.push_contract_cell("balance-cell-type", false);

    template.push_config_cell(DataType::ConfigCellMain, Source::CellDep);

    template
}

#[test]
fn test_balance_only_handle_type_5() {
    let mut template = init("transfer");

    // inputs
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x050000000000000000000000000000000000001111",
    );
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x050000000000000000000000000000000000002222",
    );
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x050000000000000000000000000000000000002222",
    );
    push_input_normal_cell(
        &mut template,
        10_000_000_000,
        "0x0000000000000000000000000000000000003333",
    );

    // outputs
    push_output_balance_cell(
        &mut template,
        20_000_000_000,
        "0x050000000000000000000000000000000000009999",
    );
    push_output_normal_cell(
        &mut template,
        20_000_000_000,
        "0x0000000000000000000000000000000000009999",
    );

    test_tx(template.as_json());
}

#[test]
fn test_balance_only_handletest_balance_skip_all_type_5() {
    let mut template = init("transfer");

    // Simulate skipping das-lock with types other than 05.
    // inputs
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x000000000000000000000000000000000000001111",
    );
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x030000000000000000000000000000000000002222",
    );
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x040000000000000000000000000000000000003333",
    );
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x000000000000000000000000000000000000001111",
    );
    push_input_normal_cell(
        &mut template,
        10_000_000_000,
        "0x0000000000000000000000000000000000003333",
    );

    // outputs
    push_output_balance_cell(
        &mut template,
        20_000_000_000,
        "0x000000000000000000000000000000000000009999",
    );
    push_output_normal_cell(
        &mut template,
        20_000_000_000,
        "0x0000000000000000000000000000000000009999",
    );

    test_tx(template.as_json());
}

#[test]
fn challenge_balance_without_type_in_outputs() {
    let mut template = init("transfer");

    // inputs
    push_input_balance_cell(
        &mut template,
        10_000_000_000,
        "0x000000000000000000000000000000000000001111",
    );

    // outputs
    template.push_output(
        json!({
            "capacity": "10_000_000_000",
            "lock": {
                "code_hash": "{{fake-das-lock}}",
                "args": "0x050000000000000000000000000000000000009999050000000000000000000000000000000000009999"
            }
            // Simulate creating cells with das-lock but no any type script.
        }),
        None,
    );

    challenge_tx(template.as_json(), Error::BalanceCellFoundSomeOutputsLackOfType);
}

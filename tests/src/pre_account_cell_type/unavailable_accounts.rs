use super::common::*;
use crate::util::{self, constants::*, error::Error, template_common_cell::*, template_parser::*};
use das_types_std::constants::*;
use serde_json::json;

#[test]
fn challenge_pre_register_unavailable_accounts() {
    // Simulate registering an unavailable account.
    let account = "thiscantr.bit";
    let mut template = init();
    template.push_config_cell_derived_by_account(account, Source::CellDep);

    push_input_apply_register_cell(
        &mut template,
        json!({
            "data": {
                "account": account,
                "height": HEIGHT - 4,
                "timestamp": TIMESTAMP - 60,
            }
        }),
    );

    push_output_pre_account_cell(
        &mut template,
        json!({
            "capacity": util::gen_register_fee(9, false),
            "witness": {
                "account": account,
                "created_at": TIMESTAMP,
                "price": {
                    "length": 8,
                    "new": ACCOUNT_PRICE_5_CHAR,
                    "renew": ACCOUNT_PRICE_5_CHAR
                }
            }
        }),
    );

    challenge_tx(template.as_json(), Error::AccountIsUnAvailable)
}

#[test]
fn test_pre_register_unavailable_accounts_below_all() {
    // Challenge if the index of ConfigCells will overflow
    let account = "🐭🐂🐯🐰🐲🐍🐎🐑🐒🐔🐶🐷.bit";
    let mut template = init();
    template.push_config_cell_derived_by_account(account, Source::CellDep);

    push_input_apply_register_cell(
        &mut template,
        json!({
            "data": {
                "account": account,
                "height": HEIGHT - 4,
                "timestamp": TIMESTAMP - 60,
            }
        }),
    );

    push_output_pre_account_cell(
        &mut template,
        json!({
            "capacity": util::gen_register_fee(12, false),
            "witness": {
                "account": account,
                "created_at": TIMESTAMP,
                "price": {
                    "length": 8,
                    "new": ACCOUNT_PRICE_5_CHAR,
                    "renew": ACCOUNT_PRICE_5_CHAR
                }
            }
        }),
    );

    test_tx(template.as_json())
}

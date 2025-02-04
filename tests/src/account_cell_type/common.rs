use crate::util::{self, constants::*, template_generator::*};
use das_types_std::{constants::*, packed::*};

pub fn init(action: &str, params_opt: Option<&str>) -> (TemplateGenerator, u64) {
    let mut template = TemplateGenerator::new(action, params_opt.map(|raw| Bytes::from(util::hex_to_bytes(raw))));
    let timestamp = 1611200000u64;

    template.push_contract_cell("always_success", true);
    template.push_contract_cell("fake-das-lock", true);
    template.push_contract_cell("fake-secp256k1-blake160-signhash-all", true);
    template.push_contract_cell("account-cell-type", false);

    template.push_oracle_cell(1, OracleCellType::Time, timestamp);

    template.push_config_cell(DataType::ConfigCellMain, Source::CellDep);
    template.push_config_cell(DataType::ConfigCellAccount, Source::CellDep);

    (template, timestamp)
}

pub fn init_for_renew(action: &str, params_opt: Option<&str>) -> (TemplateGenerator, u64) {
    let (mut template, timestamp) = init(action, params_opt);

    template.push_contract_cell("income-cell-type", false);
    template.push_contract_cell("balance-cell-type", false);

    template.push_oracle_cell(1, OracleCellType::Quote, 1000);
    template.push_config_cell(DataType::ConfigCellPrice, Source::CellDep);
    template.push_config_cell(DataType::ConfigCellIncome, Source::CellDep);

    (template, timestamp)
}

pub fn init_for_sub_account(action: &str, params_opt: Option<&str>) -> TemplateGenerator {
    let (mut template, _) = init(action, params_opt);

    template.push_contract_cell("income-cell-type", false);
    template.push_contract_cell("balance-cell-type", false);
    template.push_contract_cell("sub-account-cell-type", false);

    template.push_config_cell(DataType::ConfigCellIncome, Source::CellDep);
    template.push_config_cell(DataType::ConfigCellSubAccount, Source::CellDep);
    template.push_config_cell(DataType::ConfigCellSubAccountBetaList, Source::CellDep);

    template
}

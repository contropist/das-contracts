use super::util::{constants::*, template_generator::*, template_parser::TemplateParser};
use ckb_testtool::context::Context;
use ckb_tool::ckb_types::bytes;
use das_types::constants::{ConfigID, DataType};

// #[test]
fn gen_wallet_create_test_data() {
    println!("====== Print wallet_create transaction data ======");

    let mut template = TemplateGenerator::new("create_wallet", None);

    let source = Source::Output;
    template.push_wallet_cell("das00001.bit", 9_400_000_000, source);
    template.push_wallet_cell("das00002.bit", 9_400_000_000, source);
    template.push_wallet_cell("das00003.bit", 9_400_000_000, source);

    template.pretty_print();
}

#[test]
fn test_wallet_create() {
    let mut context;
    let mut parser;
    load_template!(&mut context, &mut parser, "wallet_create.json");

    // build transaction
    let tx = parser.build_tx();

    // run in vm
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");

    println!("test_propose: {} cycles", cycles);
}

// #[test]
fn gen_wallet_withdraw_test_data() {
    println!("====== Print wallet_withdraw transaction data ======");

    let mut template = TemplateGenerator::new("withdraw_from_wallet", None);

    template.push_config_cell(
        ConfigID::ConfigCellMain,
        true,
        100_000_000_000,
        Source::CellDep,
    );

    let account = "das00001.bit";

    // Generate RefCells ...
    template.push_ref_cell(
        "0x0000000000000000000000000000000000001111",
        account,
        true,
        10_500_000_000,
        Source::Input,
    );
    template.push_ref_cell(
        "0x0000000000000000000000000000000000001111",
        account,
        true,
        10_500_000_000,
        Source::Output,
    );

    // Generate AccountCells ...
    let (cell_data, entity) = template.gen_account_cell_data(
        account,
        "0x0000000000000000000000000000000000001111",
        "0x0000000000000000000000000000000000001111",
        bytes::Bytes::from(account_to_id_bytes("das00014.bit")),
        1611200000u64,
        1611200000u64 + 31536000,
        None,
    );
    template.push_account_cell(cell_data.clone(), None, 15_800_000_000, Source::Input);
    template.push_account_cell(cell_data.clone(), None, 15_800_000_000, Source::Output);
    template.push_witness(
        DataType::AccountCellData,
        Some((1, 1, entity.clone())),
        Some((1, 1, entity)),
        None,
    );

    // Generate WalletCells ...
    template.push_wallet_cell(account, 1_009_400_000_000, Source::Input);
    template.push_wallet_cell(account, 509_400_000_000, Source::Output);

    template.pretty_print();
}

#[test]
fn test_wallet_withdraw() {
    let mut context;
    let mut parser;
    load_template!(&mut context, &mut parser, "wallet_withdraw.json");

    // build transaction
    let tx = parser.build_tx();

    // run in vm
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");

    println!("test_propose: {} cycles", cycles);
}

fn gen_account_cell(
    template: &mut TemplateGenerator,
    account: &str,
    owner_lock_args: &str,
    manager_lock_args: &str,
    input_index: u32,
    output_index: u32,
) {
    // These fields will not be used by wallet-cell-type script.
    let timestamp = 1611200000u64;
    let registered_at = timestamp - 86400;
    let expired_at = timestamp + 31536000 - 86400;
    let next = bytes::Bytes::from(vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);

    let (cell_data, entity) = template.gen_account_cell_data(
        account,
        owner_lock_args,
        manager_lock_args,
        next.clone(),
        registered_at,
        expired_at,
        None,
    );
    template.push_account_cell(cell_data.clone(), None, 19_400_000_000, Source::Input);
    template.push_account_cell(cell_data, None, 19_400_000_000, Source::Output);
    template.push_witness(
        DataType::AccountCellData,
        Some((1, output_index, entity.clone())),
        Some((1, input_index, entity)),
        None,
    );
}

// #[test]
fn gen_wallet_recycle_test_data() {
    println!("====== Print wallet_recycle transaction data ======");

    let mut template = TemplateGenerator::new("recycle_wallet", None);

    template.push_config_cell(
        ConfigID::ConfigCellMain,
        true,
        100_000_000_000,
        Source::CellDep,
    );

    gen_account_cell(
        &mut template,
        "das00001.bit",
        "0x0000000000000000000000000000000000001111",
        "0x0000000000000000000000000000000000001111",
        0,
        0,
    );
    gen_account_cell(
        &mut template,
        "das00002.bit",
        "0x0000000000000000000000000000000000002222",
        "0x0000000000000000000000000000000000002222",
        1,
        1,
    );
    gen_account_cell(
        &mut template,
        "das00003.bit",
        "0x0000000000000000000000000000000000003333",
        "0x0000000000000000000000000000000000003333",
        2,
        2,
    );

    let source = Source::Input;
    template.push_wallet_cell("das00001.bit", 8_400_000_000, source);
    template.push_wallet_cell("das00002.bit", 14_500_000_000, source);
    template.push_wallet_cell("das00003.bit", 14_400_000_000, source);
    template.push_wallet_cell("das00003.bit", 10_400_000_000, source);

    template.pretty_print();
}

#[test]
fn test_wallet_recycle() {
    let mut context;
    let mut parser;
    load_template!(&mut context, &mut parser, "wallet_recycle.json");

    // build transaction
    let tx = parser.build_tx();

    // run in vm
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");

    println!("test_propose: {} cycles", cycles);
}

// #[test]
fn gen_wallet_deposit_test_data() {
    println!("====== Print wallet deposit transaction data ======");

    let mut template = TemplateGenerator::new("xxx", None);

    let account = "das00001.bit";

    // Generate WalletCells ...
    template.push_wallet_cell(account, 509_400_000_000, Source::Input);
    template.push_wallet_cell(account, 1_009_400_000_000, Source::Output);

    template.pretty_print();
}

#[test]
fn test_wallet_deposit() {
    let mut context;
    let mut parser;
    load_template!(&mut context, &mut parser, "wallet_deposit.json");

    // build transaction
    let tx = parser.build_tx();

    // run in vm
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");

    println!("test_propose: {} cycles", cycles);
}

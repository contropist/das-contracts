use super::{constants::*, util};
use ckb_tool::{
    ckb_hash::blake2b_256,
    ckb_types::{bytes, prelude::Pack},
    faster_hex::hex_string,
};
use das_bloom_filter::BloomFilter;
use das_core::constants::{BLOOM_FILTER_K, BLOOM_FILTER_M};
use das_sorted_list::DasSortedList;
use das_types::{constants::*, packed::*, prelude::*, util as das_util};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::str;

fn gen_always_success_lock(lock_args: &str) -> Script {
    Script::new_builder()
        .code_hash(Hash::try_from(ALWAYS_SUCCESS_CODE_HASH.to_vec()).unwrap())
        .hash_type(Byte::new(1))
        .args(Bytes::from(&util::hex_to_bytes(lock_args).unwrap()[..]))
        .build()
}

fn gen_price_config(length: u8, new_price: u64, renew_price: u64) -> PriceConfig {
    PriceConfig::new_builder()
        .length(Uint8::from(length))
        .new(Uint64::from(new_price))
        .renew(Uint64::from(renew_price))
        .build()
}

fn gen_char_set(name: CharSetType, global: u8, chars: Vec<&str>) -> CharSet {
    let mut builder = CharSet::new_builder()
        .name(Uint32::from(name as u32))
        .global(Uint8::from(global));

    let mut chars_builder = Chars::new_builder();
    for char in chars {
        chars_builder = chars_builder.push(Bytes::from(char.as_bytes()));
    }
    builder = builder.chars(chars_builder.build());

    builder.build()
}

fn gen_char_sets() -> CharSetList {
    CharSetList::new_builder()
        .push(gen_char_set(CharSetType::Emoji, 1, vec!["😂", "👍", "✨"]))
        .push(gen_char_set(
            CharSetType::Digit,
            1,
            vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
        ))
        .push(gen_char_set(
            CharSetType::En,
            0,
            vec![
                "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p",
                "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
            ],
        ))
        .build()
}

fn gen_type_id_table() -> TypeIdTable {
    let mut builder = TypeIdTable::new_builder();
    for (key, val) in TYPE_ID_TABLE.iter() {
        builder = match *key {
            "apply-register-cell-type" => {
                builder.apply_register_cell(util::hex_to_hash(val).unwrap())
            }
            "pre-account-cell-type" => builder.pre_account_cell(util::hex_to_hash(val).unwrap()),
            "account-cell-type" => builder.account_cell(util::hex_to_hash(val).unwrap()),
            "ref-cell-type" => builder.ref_cell(util::hex_to_hash(val).unwrap()),
            "proposal-cell-type" => builder.proposal_cell(util::hex_to_hash(val).unwrap()),
            "wallet-cell-type" => builder.wallet_cell(util::hex_to_hash(val).unwrap()),
            _ => builder,
        }
    }

    builder.build()
}

fn gen_account_char(char: &str, char_set_type: CharSetType) -> AccountChar {
    AccountChar::new_builder()
        .char_set_name(Uint32::from(char_set_type as u32))
        .bytes(Bytes::from(char.as_bytes()))
        .build()
}

fn gen_proposal_item(account: &str, item_type: &ProposalSliceItemType, next: &str) -> ProposalItem {
    let account_id = AccountId::try_from(account_to_id_bytes(account)).unwrap();
    let mut builder = ProposalItem::new_builder()
        .account_id(account_id)
        .item_type(Uint8::from(*item_type as u8));

    if !next.is_empty() {
        let next_account_id = AccountId::try_from(account_to_id_bytes(next)).unwrap();
        builder = builder.next(next_account_id);
    }

    builder.build()
}

fn gen_slices(slices: &Vec<Vec<(&str, ProposalSliceItemType, &str)>>) -> SliceList {
    let mut sl_list = SliceList::new_builder();
    for slice in slices {
        if slice.len() <= 1 {
            panic!("Slice must has more than one item.")
        }

        let mut sl = SL::new_builder();
        let mut next_of_first_item = "";
        for (index, (account, item_type, next)) in slice.iter().enumerate() {
            // When it is the first item, saving its next.
            if index == 0 {
                next_of_first_item = next;
                let (next, _, _) = slice.get(index + 1).unwrap();
                sl = sl.push(gen_proposal_item(account, item_type, next));
            // When it is the last item, use next_of_first_item as its next.
            } else if index == slice.len() - 1 {
                sl = sl.push(gen_proposal_item(account, item_type, next_of_first_item));
            // When it is the items between the first and the last, using its next item's account_id as next.
            } else {
                let (next, _, _) = slice.get(index + 1).unwrap();
                sl = sl.push(gen_proposal_item(account, item_type, next));
            }
        }
        sl_list = sl_list.push(sl.build());
    }
    sl_list.build()
}

pub fn gen_account_record(
    type_: &str,
    label: &str,
    key: &str,
    value: impl AsRef<[u8]>,
    ttl: u32,
) -> Record {
    Record::new_builder()
        .record_type(Bytes::from(type_.as_bytes()))
        .record_label(Bytes::from(label.as_bytes()))
        .record_key(Bytes::from(key.as_bytes()))
        .record_value(Bytes::from(value.as_ref()))
        .record_ttl(Uint32::from(ttl))
        .build()
}

pub fn gen_account_records() -> Records {
    let address_records = [
        (
            "address",
            "Personal",
            "ETH",
            util::hex_to_bytes("0x00000000000000000000").unwrap(),
        ),
        (
            "address",
            "Company",
            "ETH",
            util::hex_to_bytes("0x00000000000000000000").unwrap(),
        ),
        (
            "address",
            "Personal",
            "BTC",
            util::hex_to_bytes("0x00000000000000000000").unwrap(),
        ),
        (
            "address",
            "Company",
            "BTC",
            util::hex_to_bytes("0x00000000000000000000").unwrap(),
        ),
    ];

    let profile_records = [
        (
            "profile",
            "Mars",
            "id",
            "120981203982901389398390".as_bytes(),
        ),
        ("profile", "Company", "email", "xxxxx@mars.bit".as_bytes()),
    ];

    let mut records = Records::new_builder();

    for (type_, label, key, value) in address_records.iter() {
        records = records.push(gen_account_record(type_, label, key, value, 300));
    }
    for (type_, label, key, value) in profile_records.iter() {
        records = records.push(gen_account_record(type_, label, key, value, 300));
    }

    records.build()
}

pub fn gen_account_chars(chars: Vec<impl AsRef<str>>) -> AccountChars {
    let mut builder = AccountChars::new_builder();
    for char in chars {
        let char = char.as_ref();
        // Filter empty chars come from str.split("").
        if char.is_empty() {
            continue;
        }

        if char.len() != 1 {
            builder = builder.push(gen_account_char(char, CharSetType::Emoji))
        } else {
            let raw_char = char.chars().next().unwrap();
            if raw_char.is_digit(10) {
                builder = builder.push(gen_account_char(char, CharSetType::Digit))
            } else {
                builder = builder.push(gen_account_char(char, CharSetType::En))
            }
        }
    }

    builder.build()
}

pub fn gen_account_list() {
    let accounts = vec![
        "das00000.bit",
        "das00001.bit",
        "das00002.bit",
        "das00003.bit",
        "das00004.bit",
        "das00005.bit",
        "das00006.bit",
        "das00007.bit",
        "das00008.bit",
        "das00009.bit",
        "das00010.bit",
        "das00011.bit",
        "das00012.bit",
        "das00013.bit",
        "das00014.bit",
        "das00015.bit",
        "das00016.bit",
        "das00018.bit",
        "das00019.bit",
    ];

    let mut account_id_map = HashMap::new();
    let mut account_id_list = Vec::new();
    for account in accounts.iter() {
        let account_id = bytes::Bytes::from(util::account_to_id(account.as_bytes()));
        account_id_map.insert(account_id.clone(), *account);
        account_id_list.push(account_id);
    }

    let sorted_list = DasSortedList::new(account_id_list);
    println!("====== Accounts list ======");
    for item in sorted_list.items() {
        let account = account_id_map.get(item).unwrap();
        println!("{} => {}", account, item.pack());
    }

    let accounts = vec![
        "das.bit",
        "inviter_01.bit",
        "inviter_02.bit",
        "inviter_03.bit",
        "channel_01.bit",
        "channel_02.bit",
        "channel_03.bit",
        "proposer_01.bit",
        "proposer_02.bit",
        "proposer_03.bit",
    ];
    println!("====== Special accounts list ======");
    for account in accounts.into_iter() {
        let id = bytes::Bytes::from(util::account_to_id(account.as_bytes()));
        println!("{} => {}", account, id.pack());
    }
}

pub fn account_to_id_bytes(account: &str) -> Vec<u8> {
    // Sorted testing accounts, generated from gen_account_list().
    let account_id = match account {
        "das00012.bit" => "0x05d2771e6c0366846677",
        "das00005.bit" => "0x0eb54a48689ce16b1fe8",
        "das00009.bit" => "0x100871e1ff8a4cbde1d4",
        "das00002.bit" => "0x1710cbaf08cf1fa2dcad",
        "das00013.bit" => "0x2ba50252fba902dc5287",
        "das00010.bit" => "0x5998f1666df91f989212",
        "das00004.bit" => "0x5cbc30a5bfd00de7f7c5",
        "das00018.bit" => "0x60e70978fd6456883990",
        "das00008.bit" => "0x6729295b9a936d6c67fd",
        "das00011.bit" => "0x70aa5e4d41c3d557ca84",
        "das00006.bit" => "0x76b1e100d9ff3dc62e75",
        "das00019.bit" => "0x9b992223d5ccd0fca8e1",
        "das00001.bit" => "0xa2e06438859db0449574",
        "das00014.bit" => "0xb209ac25bd48f00b9ae6",
        "das00003.bit" => "0xbfab64fccdb83b3316cf",
        "das00007.bit" => "0xc9804583fc51c64512c0",
        "das00000.bit" => "0xcc4e1b0c31b5f537ad0a",
        "das00016.bit" => "0xeb12a2e3eabe2a20015c",
        "das00015.bit" => "0xed912d8f62ce9815d415",
        // ======
        "das.bit" => "0xb7526803f67ebe70aba6",
        "inviter_01.bit" => "0x12dedc00d4e73e7c7c50",
        "inviter_02.bit" => "0xa3281c84335fc37e8331",
        "inviter_03.bit" => "0x75614a21ee56531bf708",
        "channel_01.bit" => "0x0de06eb8bcaa6b33d9eb",
        "channel_02.bit" => "0xd6d474d85a9edc9ee26a",
        "channel_03.bit" => "0x3781b1590a0ef3b0bbce",
        "proposer_01.bit" => "0xdb352d1d5e245fa09697",
        "proposer_02.bit" => "0xc0d1d7b1b88c584c4c23",
        "proposer_03.bit" => "0x26bcd993ec69922481d2",
        // ======
        _ => panic!("Can not find ID of account."),
    };

    util::hex_to_bytes(account_id).unwrap().to_vec()
}

pub fn gen_record_types() -> RecordTypes {
    let data = vec![
        (
            "profile",
            vec![
                "twitter",
                "facebook",
                "reddit",
                "linkedin",
                "github",
                "telegram",
                "description",
                "avatar",
                "email",
                "phone",
            ],
        ),
        (
            "address",
            vec![
                "btc", "eth", "ckb", "bch", "ltc", "doge", "xrp", "dot", "fil", "trx", "eos",
                "iota", "xmr", "bsc", "heco", "xem", "etc", "dash", "zec", "zil", "flow", "iost",
                "sc", "near", "ksm", "atom", "xtz", "bsv", "sol", "vet", "xlm", "ada",
            ],
        ),
        ("custom_keys", Vec::new()),
    ];

    let mut record_types_builder = RecordTypes::new_builder();
    for (type_name, key_names) in data {
        let mut record_keys_builder = RecordKeys::new_builder();
        for key_name in key_names {
            record_keys_builder = record_keys_builder.push(Bytes::from(key_name.as_bytes()))
        }

        let record_type = RecordType::new_builder()
            .type_name(Bytes::from(type_name.as_bytes()))
            .keys(record_keys_builder.build())
            .build();
        record_types_builder = record_types_builder.push(record_type)
    }

    record_types_builder.build()
}

fn bytes_to_hex(input: Bytes) -> String {
    "0x".to_string() + &hex_string(input.as_reader().raw_data()).unwrap()
}

pub struct TemplateGenerator {
    pub header_deps: Vec<Value>,
    pub cell_deps: Vec<Value>,
    pub inputs: Vec<Value>,
    pub outputs: Vec<Value>,
    pub witnesses: Vec<String>,
    pub prices: HashMap<u8, PriceConfig>,
}

impl TemplateGenerator {
    pub fn new(action: &str, params_opt: Option<Bytes>) -> TemplateGenerator {
        let witness = das_util::wrap_action_witness(action, params_opt);

        let mut prices = HashMap::new();
        prices.insert(1u8, gen_price_config(1, 12_000_000, 1_200_000));
        prices.insert(2u8, gen_price_config(2, 11_000_000, 1_100_000));
        prices.insert(3u8, gen_price_config(3, 10_000_000, 1_000_000));
        prices.insert(4u8, gen_price_config(4, 9_000_000, 900_000));
        prices.insert(5u8, gen_price_config(5, 8_000_000, 800_000));
        prices.insert(6u8, gen_price_config(6, 7_000_000, 700_000));
        prices.insert(7u8, gen_price_config(7, 6_000_000, 600_000));
        prices.insert(8u8, gen_price_config(8, 5_000_000, 500_000));

        TemplateGenerator {
            header_deps: Vec::new(),
            cell_deps: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            witnesses: vec![bytes_to_hex(witness)],
            prices,
        }
    }

    pub fn push_witness<T: Entity>(
        &mut self,
        data_type: DataType,
        output_opt: Option<(u32, u32, T)>,
        input_opt: Option<(u32, u32, T)>,
        dep_opt: Option<(u32, u32, T)>,
    ) {
        let witness = das_util::wrap_data_witness(data_type, output_opt, input_opt, dep_opt);
        self.witnesses.push(bytes_to_hex(witness));
    }

    pub fn push_witness_with_group(
        &mut self,
        data_type: DataType,
        group: Source,
        entity: (u32, u32, impl Entity),
    ) {
        let witness = match group {
            Source::Input => das_util::wrap_data_witness(data_type, None, Some(entity), None),
            Source::Output => das_util::wrap_data_witness(data_type, Some(entity), None, None),
            _ => das_util::wrap_data_witness(data_type, None, None, Some(entity)),
        };
        self.witnesses.push(bytes_to_hex(witness));
    }

    pub fn push_cell(
        &mut self,
        capacity: u64,
        lock_script: Value,
        type_script: Value,
        data: Option<Bytes>,
        source: Source,
    ) {
        let mut value;
        if let Some(tmp_data) = data {
            value = json!({
              "tmp_type": "full",
              "capacity": capacity,
              "lock": lock_script,
              "type": type_script,
              "tmp_data": bytes_to_hex(tmp_data.clone())
            });
        } else {
            value = json!({
              "tmp_type": "full",
              "capacity": capacity,
              "lock": lock_script,
              "type": type_script
            });
        }

        if source == Source::Input {
            value = json!({
                "previous_output": value,
                "since": "0x"
            });
        }

        match source {
            Source::HeaderDep => self.header_deps.push(value),
            Source::CellDep => self.cell_deps.push(value),
            Source::Input => self.inputs.push(value),
            Source::Output => self.outputs.push(value),
        }
    }

    pub fn push_contract_cell(&mut self, contract_filename: &str, deployed: bool) {
        let value;
        if deployed {
            value = json!({
                "tmp_type": "deployed_contract",
                "tmp_file_name": contract_filename
            });
        } else {
            value = json!({
                "tmp_type": "contract",
                "tmp_file_name": contract_filename
            });
        }

        self.cell_deps.push(value)
    }

    pub fn push_time_cell(&mut self, index: u8, timestamp: u64, capacity: u64, source: Source) {
        let mut cell_raw_data = Vec::new();
        cell_raw_data.extend(index.to_be_bytes().iter());
        cell_raw_data.extend((timestamp as u32).to_be_bytes().iter());
        let cell_data = Bytes::from(cell_raw_data);

        let lock_script = json!({
            "code_hash": "{{always_success}}"
        });
        let type_script = json!({
            "code_hash": "0x0100000000000000000000000000000000000000000000000000000000000000",
            "hash_type": "type"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    pub fn push_height_cell(&mut self, index: u8, height: u64, capacity: u64, source: Source) {
        let mut cell_raw_data = Vec::new();
        cell_raw_data.extend(index.to_be_bytes().iter());
        cell_raw_data.extend(height.to_be_bytes().iter());
        let cell_data = Bytes::from(cell_raw_data);

        let lock_script = json!({
            "code_hash": "{{always_success}}"
        });
        let type_script = json!({
            "code_hash": "0x0200000000000000000000000000000000000000000000000000000000000000",
            "hash_type": "type"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    pub fn push_quote_cell(&mut self, quote: u64, capacity: u64, source: Source) {
        let raw = quote.to_le_bytes();
        let cell_data = Bytes::from(&raw[..]);

        let lock_script = json!({
            "code_hash": "{{always_success}}",
            "args": QUOTE_LOCK_ARGS
        });
        let type_script = json!(null);

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    pub fn push_apply_register_cell(
        &mut self,
        lock_args: &str,
        account: &str,
        height: u64,
        capacity: u64,
        source: Source,
    ) {
        let hash_of_account = Hash::new_unchecked(
            blake2b_256(
                [
                    util::hex_to_bytes(lock_args).unwrap().as_ref(),
                    account.as_bytes(),
                ]
                .concat()
                .as_slice(),
            )
            .to_vec()
            .into(),
        );

        let raw = [
            hash_of_account.as_reader().raw_data(),
            &height.to_le_bytes(),
        ]
        .concat();
        let cell_data = Bytes::from(raw);

        let lock_script = json!({
            "code_hash": "{{always_success}}",
            "args": lock_args
        });
        let type_script = json!({
            "code_hash": "{{apply-register-cell-type}}"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    pub fn push_ref_cell(
        &mut self,
        lock_args: &str,
        account: &str,
        is_owner: bool,
        capacity: u64,
        source: Source,
    ) {
        let mut account_id = util::account_to_id(account.as_bytes());
        let lock_script = json!({
          "code_hash": "{{always_success}}",
          "args": lock_args
        });
        let type_script = json!({
          "code_hash": "{{ref-cell-type}}",
          "args": ""
        });

        if is_owner {
            account_id.push(0);
        } else {
            account_id.push(1);
        }
        let cell_data = Bytes::from(account_id);

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    fn gen_config_cell_main(&mut self) -> (Bytes, ConfigCellMain) {
        let entity = ConfigCellMain::new_builder()
            .account_expiration_grace_period(Uint32::from(2_592_000))
            .min_ttl(Uint32::from(300))
            .type_id_table(gen_type_id_table())
            .build();

        // Generate the cell structure of ConfigCell.
        let cell_data = Bytes::from(blake2b_256(entity.as_slice()).to_vec());

        (cell_data, entity)
    }

    fn gen_config_cell_register(&mut self) -> (Bytes, ConfigCellRegister) {
        let mut price_config_list_builder = PriceConfigList::new_builder();
        for (_, price) in self.prices.iter() {
            price_config_list_builder = price_config_list_builder.push(price.to_owned());
        }

        let profit_config = ProfitConfig::new_builder()
            .profit_rate_of_channel(Uint32::from(800))
            .profit_rate_of_inviter(Uint32::from(800))
            .profit_rate_of_das(Uint32::from(8000))
            .profit_rate_of_proposal_create(Uint32::from(400))
            .profit_rate_of_proposal_confirm(Uint32::from(0))
            .build();

        let discount_config = DiscountConfig::new_builder()
            .invited_discount(Uint32::from(500))
            .build();

        let entity = ConfigCellRegister::new_builder()
            .apply_min_waiting_block_number(Uint32::from(4))
            .apply_max_waiting_block_number(Uint32::from(5760))
            .account_max_length(Uint32::from(1000))
            .char_sets(gen_char_sets())
            .price_configs(price_config_list_builder.build())
            .proposal_min_confirm_interval(Uint8::from(4))
            .proposal_min_extend_interval(Uint8::from(2))
            .proposal_min_recycle_interval(Uint8::from(6))
            .proposal_max_account_affect(Uint32::from(50))
            .proposal_max_pre_account_contain(Uint32::from(50))
            .profit(profit_config)
            .discount(discount_config)
            .build();

        // Generate the cell structure of ConfigCell.
        let cell_data = Bytes::from(blake2b_256(entity.as_slice()).to_vec());

        (cell_data, entity)
    }

    fn gen_config_cell_market(&mut self) -> (Bytes, ConfigCellMarket) {
        let primary_market_config = MarketConfig::new_builder()
            .max_auction_waiting(Uint32::from(86400))
            .min_auction_raise_rate(Uint32::from(1000))
            .build();

        let secondary_market_config = MarketConfig::new_builder()
            .max_auction_time(Uint32::from(2_592_000))
            .max_auction_waiting(Uint32::from(86400))
            .max_selling_time(Uint32::from(2_592_000))
            .min_auction_raise_rate(Uint32::from(1000))
            .build();

        let entity = ConfigCellMarket::new_builder()
            .primary_market(primary_market_config)
            .secondary_market(secondary_market_config)
            .build();

        // Generate the cell structure of ConfigCell.
        let cell_data = Bytes::from(blake2b_256(entity.as_slice()).to_vec());

        (cell_data, entity)
    }

    fn gen_config_cell_bloom_filter(&mut self) -> (Bytes, Vec<u8>) {
        let mut bf = BloomFilter::new(BLOOM_FILTER_M, BLOOM_FILTER_K);
        bf.insert(b"google");
        bf.insert(b"apple");
        bf.insert(b"microsoft");
        bf.insert(b"qq");
        bf.insert(b"ali");
        bf.insert(b"baidu");
        bf.insert(b"das");
        let mut entity = bf.export_bit_u8();

        // Prepend length of bytes to bloom filter data, 4 bytes is the length of first u32.
        let mut length = (entity.len() as u32 + 4).to_le_bytes().to_vec();
        length.extend(entity);
        entity = length;

        // Generate the cell structure of ConfigCell.
        let cell_data = Bytes::from(blake2b_256(entity.as_slice()).to_vec());

        (cell_data, entity)
    }

    pub fn push_config_cell(
        &mut self,
        config_id: ConfigID,
        push_witness: bool,
        capacity: u64,
        source: Source,
    ) {
        let (cell_data, witness) = match config_id {
            ConfigID::ConfigCellMain => {
                let (cell_data, entity) = self.gen_config_cell_main();
                (
                    cell_data,
                    das_util::wrap_entity_witness(DataType::ConfigCellMain, entity),
                )
            }
            ConfigID::ConfigCellRegister => {
                let (cell_data, entity) = self.gen_config_cell_register();
                (
                    cell_data,
                    das_util::wrap_entity_witness(DataType::ConfigCellRegister, entity),
                )
            }
            ConfigID::ConfigCellRecord => {
                panic!("not implemented");
            }
            ConfigID::ConfigCellMarket => {
                let (cell_data, entity) = self.gen_config_cell_market();
                (
                    cell_data,
                    das_util::wrap_entity_witness(DataType::ConfigCellMarket, entity),
                )
            }
            ConfigID::ConfigCellBloomFilter => {
                let (cell_data, filter) = self.gen_config_cell_bloom_filter();
                (
                    cell_data,
                    das_util::wrap_raw_witness(DataType::ConfigCellBloomFilter, filter),
                )
            }
        };

        // Create config cell.
        let config_id_hex = hex_string(&(config_id as u32).to_le_bytes()).unwrap();
        let lock_script = json!({
          "code_hash": "{{always_success}}",
          "args": CONFIG_LOCK_ARGS
        });
        let type_script = json!({
          "code_hash": "{{config-cell-type}}",
          "args": config_id_hex,
        });
        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);

        if push_witness {
            // Create config cell witness.
            self.witnesses.push(bytes_to_hex(witness));
        }
    }

    pub fn gen_pre_account_cell_data(
        &mut self,
        account: &str,
        owner_lock_args: &str,
        refund_lock_args: &str,
        inviter_wallet: &str,
        channel_wallet: &str,
        quote: u64,
        invited_discount: u32,
        created_at: u64,
    ) -> (Bytes, PreAccountCellData) {
        let account_chars_raw = account
            .chars()
            .take(account.len() - 4)
            .map(|c| c.to_string())
            .collect::<Vec<String>>();
        let account_chars = gen_account_chars(account_chars_raw);
        let account_length = if account_chars.len() > 8 {
            8u8
        } else {
            account_chars.len() as u8
        };

        let price = self.prices.get(&account_length).unwrap();
        let entity = PreAccountCellData::new_builder()
            .account(account_chars.to_owned())
            .owner_lock(gen_always_success_lock(owner_lock_args))
            .refund_lock(gen_always_success_lock(refund_lock_args))
            .inviter_wallet(Bytes::from(account_to_id_bytes(inviter_wallet)))
            .channel_wallet(Bytes::from(account_to_id_bytes(channel_wallet)))
            .price(price.to_owned())
            .quote(Uint64::from(quote))
            .invited_discount(Uint32::from(invited_discount))
            .created_at(Timestamp::from(created_at))
            .build();

        let id = util::account_to_id(account.as_bytes());

        let hash = Hash::try_from(blake2b_256(entity.as_slice()).to_vec()).unwrap();
        let raw = [hash.as_reader().raw_data(), id.as_slice()].concat();
        let cell_data = Bytes::from(raw);

        (cell_data, entity)
    }

    pub fn push_pre_account_cell(
        &mut self,
        cell_data: Bytes,
        entity_opt: Option<(u32, u32, impl Entity)>,
        capacity: u64,
        source: Source,
    ) {
        let lock_script = json!({
          "code_hash": "{{always_success}}"
        });
        let type_script = json!({
          "code_hash": "{{pre-account-cell-type}}"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);

        if let Some(entity) = entity_opt {
            self.push_witness_with_group(DataType::PreAccountCellData, source, entity);
        }
    }

    pub fn gen_account_cell_data(
        &mut self,
        account: &str,
        owner_lock_args: &str,
        manager_lock_args: &str,
        next: bytes::Bytes,
        registered_at: u64,
        expired_at: u64,
        records_opt: Option<Records>,
    ) -> (Bytes, AccountCellData) {
        let account_chars_raw = account
            .chars()
            .take(account.len() - 4)
            .map(|c| c.to_string())
            .collect::<Vec<String>>();
        let account_chars = gen_account_chars(account_chars_raw);
        let id = util::account_to_id(account.as_bytes());

        let records = match records_opt {
            Some(records) => records,
            None => Records::default(),
        };

        let entity = AccountCellData::new_builder()
            .id(AccountId::try_from(id.clone()).unwrap())
            .account(account_chars.to_owned())
            .owner_lock(gen_always_success_lock(owner_lock_args))
            .manager_lock(gen_always_success_lock(manager_lock_args))
            .registered_at(Uint64::from(registered_at))
            .status(Uint8::from(0))
            .records(records)
            .build();

        let hash = Hash::try_from(blake2b_256(entity.as_slice()).to_vec()).unwrap();
        let raw = [
            hash.as_reader().raw_data(),
            id.as_slice(),
            &next[..],
            &expired_at.to_le_bytes()[..],
            account.as_bytes(),
        ]
        .concat();
        let cell_data = Bytes::from(raw);

        (cell_data, entity)
    }

    pub fn gen_root_account_cell_data(&mut self) -> (Bytes, AccountCellData) {
        let id: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let next: Vec<u8> = vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 255];
        let account = AccountChars::default();
        let expired_at = u64::MAX.to_le_bytes();

        let owner_lock = Script::default();
        let manager_lock = Script::default();

        let entity = AccountCellData::new_builder()
            .id(AccountId::try_from(id.clone()).unwrap())
            .account(account)
            .owner_lock(owner_lock)
            .manager_lock(manager_lock)
            .registered_at(Uint64::from(0))
            .status(Uint8::from(0))
            .build();

        let hash = Hash::try_from(blake2b_256(entity.as_slice()).to_vec()).unwrap();
        let raw = [
            hash.as_reader().raw_data(),
            id.as_slice(),
            next.as_slice(),
            &expired_at[..],
            &[0],
        ]
        .concat();
        let cell_data = Bytes::from(raw);

        (cell_data, entity)
    }

    pub fn push_account_cell(
        &mut self,
        cell_data: Bytes,
        entity_opt: Option<(u32, u32, AccountCellData)>,
        capacity: u64,
        source: Source,
    ) {
        let lock_script = json!({
          "code_hash": "{{always_success}}"
        });
        let type_script = json!({
          "code_hash": "{{account-cell-type}}"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);

        if let Some(entity) = entity_opt {
            self.push_witness_with_group(DataType::AccountCellData, source, entity);
        }
    }

    pub fn gen_proposal_cell_data(
        &mut self,
        proposer_lock_args: &str,
        created_at_height: u64,
        slices: &Vec<Vec<(&str, ProposalSliceItemType, &str)>>,
    ) -> (Bytes, ProposalCellData) {
        let entity = ProposalCellData::new_builder()
            .proposer_lock(gen_always_success_lock(proposer_lock_args))
            .created_at_height(Uint64::from(created_at_height))
            .slices(gen_slices(slices))
            .build();
        // println!("entity = {:?}", entity);

        let hash = blake2b_256(entity.as_slice());
        // println!("hash = {:?}", hash);
        let cell_data = Bytes::from(&hash[..]);

        (cell_data, entity)
    }

    pub fn push_proposal_cell(
        &mut self,
        cell_data: Bytes,
        entity_opt: Option<(u32, u32, impl Entity)>,
        capacity: u64,
        source: Source,
    ) {
        let lock_script = json!({
          "code_hash": "{{always_success}}"
        });
        let type_script = json!({
          "code_hash": "{{proposal-cell-type}}"
        });

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);

        if let Some(entity) = entity_opt {
            self.push_witness_with_group(DataType::ProposalCellData, source, entity);
        }
    }

    pub fn push_wallet_cell(&mut self, account: &str, capacity: u64, source: Source) {
        let account_id = Bytes::from(account_to_id_bytes(account));
        let lock_script = json!({
          "code_hash": "{{always_success}}"
        });
        let type_script = json!({
          "code_hash": "{{wallet-cell-type}}"
        });

        let cell_data = account_id;

        self.push_cell(capacity, lock_script, type_script, Some(cell_data), source);
    }

    pub fn push_signall_cell(&mut self, lock_args: &str, capacity: u64, source: Source) {
        let lock_script = json!({
          "code_hash": "{{always_success}}",
          "args": lock_args
        });

        self.push_cell(capacity, lock_script, json!(null), None, source);
    }

    pub fn gen_header() {}

    pub fn as_json(&self) -> serde_json::Value {
        json!({
            "cell_deps": self.cell_deps,
            "inputs": self.inputs,
            "outputs": self.outputs,
            "witnesses": self.witnesses,
        })
    }

    pub fn pretty_print(&self) {
        let data = self.as_json();
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
}

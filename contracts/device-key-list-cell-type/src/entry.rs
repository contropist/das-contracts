use alloc::boxed::Box;
use alloc::vec::Vec;

use ckb_std::ckb_constants::Source;
use das_core::constants::ScriptType;
use das_core::error::ScriptError;
use das_core::witness_parser::WitnessesParser;
use das_core::{code_to_error, debug, util, verifiers};
use das_types::constants::DataType;
use das_types::packed::{DeviceKey, DeviceKeyList};
use das_types::prelude::Entity;

use crate::error::ErrorCode;

pub fn main() -> Result<(), Box<dyn ScriptError>> {
    debug!("====== Running sub-account-cell-type ======");
    let mut parser = WitnessesParser::new()?;
    let action = match parser.parse_action_with_params()? {
        Some((action, _)) => action.to_vec(),
        None => return Err(code_to_error!(das_core::error::ErrorCode::ActionNotSupported)),
    };

    debug!(
        "Route to {:?} action ...",
        alloc::string::String::from_utf8(action.clone())
            .map_err(|_| das_core::error::ErrorCode::ActionNotSupported)?
    );

    parser.parse_cell()?;
    let this_script = ckb_std::high_level::load_script()?;

    match action.as_slice() {
        b"create_device_key_list" => {
            let (input_cells, output_cells) =
                util::find_cells_by_script_in_inputs_and_outputs(ScriptType::Type, this_script.as_reader())?;

            verifiers::common::verify_cell_number_and_position(
                "device-key-list",
                &input_cells,
                &[],
                &output_cells,
                &[0],
            )?;

            let (_, _, bytes) = parser.verify_and_get(DataType::DeviceKeyList, output_cells[0], Source::Output)?;
            let key_list = DeviceKeyList::from_compatible_slice(bytes.as_slice())
                .map_err(|_e| code_to_error!(ErrorCode::KeyListParseError))?;

            das_core::assert!(
                key_list.len() == 1,
                ErrorCode::WitnessArgsInvalid,
                "There should be excatly 1 device_key when create"
            );

            verify_key_list_lock_arg(output_cells[0], key_list, Source::Output)?;
        }
        b"update_device_key_list" => {
            let (input_cells, output_cells) =
                util::find_cells_by_script_in_inputs_and_outputs(ScriptType::Type, this_script.as_reader())?;
            verifiers::common::verify_cell_number_and_position(
                "device-key-list",
                &input_cells,
                &[0],
                &output_cells,
                &[0],
            )?;

            das_core::assert!(
                ckb_std::high_level::load_cell_lock(input_cells[0], Source::Input)?
                    .args()
                    .as_slice()
                    == ckb_std::high_level::load_cell_lock(output_cells[0], Source::Output)?
                        .args()
                        .as_slice(),
                ErrorCode::InvalidLockArg,
                "Output lock arg should be the same as the one of the input"
            );

            let (_, _, key_list_in_input) =
                parser.verify_and_get(DataType::DeviceKeyList, input_cells[0], Source::Input)?;
            let (_, _, key_list_in_output) =
                parser.verify_and_get(DataType::DeviceKeyList, output_cells[0], Source::Output)?;
            let key_list_in_input = DeviceKeyList::from_compatible_slice(key_list_in_input.as_slice())
                .map_err(|_e| code_to_error!(ErrorCode::KeyListParseError))?;
            let key_list_in_output = DeviceKeyList::from_compatible_slice(key_list_in_output.as_slice())
                .map_err(|_e| code_to_error!(ErrorCode::KeyListParseError))?;

            das_core::assert!(
                key_list_in_output.item_count() > 0 && key_list_in_output.item_count() < 11,
                ErrorCode::UpdateParamsInvalid,
                "The key list length should be from 1 to 10"
            );

            let len_diff: i32 = key_list_in_input.item_count() as i32 - key_list_in_output.item_count() as i32;
            das_core::assert!(
                len_diff == 1 || len_diff == -1,
                ErrorCode::KeyListNumberIncorrect,
                "There should be exactly 1 device key difference when update"
            );

            match len_diff {
                1 => {
                    debug!("update_device_key_list: add key");
                    // Should only append to the tail
                    let mut input_iter = key_list_in_input.into_iter();
                    let mut output_iter = key_list_in_output.into_iter();
                    loop {
                        match (input_iter.next(), output_iter.next()) {
                            (Some(a), Some(b)) if a.as_slice() == b.as_slice() => continue,
                            (Some(_), Some(_)) => Err(code_to_error!(ErrorCode::UpdateParamsInvalid))?,
                            (None, Some(_)) => break,
                            _ => unreachable!(),
                        }
                    }
                }
                -1 => {
                    debug!("update_device_key_list: remove key");
                    let keys_in_input: alloc::collections::BTreeSet<DeviceKeyWrapped> =
                        key_list_in_input.into_iter().map(|key| DeviceKeyWrapped(key)).collect();
                    let keys_in_output: alloc::collections::BTreeSet<DeviceKeyWrapped> = key_list_in_output
                        .into_iter()
                        .map(|key| DeviceKeyWrapped(key))
                        .collect();
                    das_core::assert!(
                        keys_in_input.is_superset(&keys_in_output),
                        ErrorCode::UpdateParamsInvalid,
                        "Output keys should be superset of input"
                    );
                    let removed_device_key: Vec<DeviceKeyWrapped> =
                        keys_in_input.difference(&keys_in_output).cloned().collect();
                    das_core::assert!(
                        removed_device_key.len() == 1,
                        ErrorCode::UpdateParamsInvalid,
                        "Output key should be exactly 1 less than input"
                    );
                }
                _ => unreachable!(),
            };
        }
        b"destroy_device_key_list" => {
            let (input_cells, output_cells) =
                util::find_cells_by_script_in_inputs_and_outputs(ScriptType::Type, this_script.as_reader())?;
            verifiers::common::verify_cell_number_and_position(
                "device-key-list",
                &input_cells,
                &[0],
                &output_cells,
                &[],
            )?;
        }
        _ => unimplemented!(),
    }

    Ok(())
}

#[derive(Clone)]
struct DeviceKeyWrapped(DeviceKey);
impl Eq for DeviceKeyWrapped {}
impl PartialEq for DeviceKeyWrapped {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice() == other.0.as_slice()
    }
}

impl PartialOrd for DeviceKeyWrapped {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.as_slice().partial_cmp(&other.0.as_slice())
    }
}

impl Ord for DeviceKeyWrapped {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_slice().cmp(&other.0.as_slice())
    }
}


// TODO: refactor the logic into common verifiers.
fn verify_key_list_lock_arg(index: usize, key_list: DeviceKeyList, source: Source) -> Result<(), Box<dyn ScriptError>> {
    let device_key = key_list.get(0).unwrap();
    let lock = ckb_std::high_level::load_cell_lock(index, source)?;
    let lock_arg = lock.args().raw_data();

    if lock_arg.len() != 44 {
        return Err(code_to_error!(ErrorCode::LockArgLengthIncorrect));
    }

    // First byte is main_alg_id
    das_core::assert!(
        lock_arg.slice(0..1) == device_key.main_alg_id().nth0().as_bytes(),
        ErrorCode::InvalidLockArg,
        "First byte of lock arg should be main_alg_id"
    );

    // Second byte is sub_alg_id
    das_core::assert!(
        lock_arg.slice(1..2) == device_key.sub_alg_id().nth0().as_bytes(),
        ErrorCode::InvalidLockArg,
        "Second byte of lock arg should be sub_alg_id"
    );

    // Next 10 bytes are pubkey hashed 5 times
    das_core::assert!(
        lock_arg.slice(2..12) == device_key.pubkey().raw_data(),
        ErrorCode::InvalidLockArg,
        "Byte 2..12 should be pubkey'"
    );

    // Next 10 bytes are cid hashed 5 times
    das_core::assert!(
        lock_arg.slice(12..22) == device_key.cid().raw_data(),
        ErrorCode::InvalidLockArg,
        "Byte 12..22 should be cid'"
    );

    // Owner and manager are the same
    das_core::assert!(
        lock_arg.slice(0..22) == lock_arg.slice(22..44),
        ErrorCode::InvalidLockArg,
        "Byte 0..22 should be the same with Byte 22..44"
    );

    Ok(())
}

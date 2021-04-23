use alloc::borrow::ToOwned;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::bytes,
    ckb_types::packed as ckb_packed,
    high_level::{load_cell_capacity, load_cell_lock, load_cell_type, load_script},
};
use core::convert::TryFrom;
use core::result::Result;
use das_core::{
    assert,
    constants::*,
    data_parser::{account_cell, ref_cell},
    debug,
    error::Error,
    util,
    witness_parser::WitnessesParser,
};
use das_sorted_list::DasSortedList;
use das_types::{constants::*, packed::*, prelude::*};
use das_wallet::Wallet;

pub fn main() -> Result<(), Error> {
    debug!("====== Running proposal-cell-type ======");

    debug!("Find out ProposalCell ...");

    // Find out PreAccountCells in current transaction.
    let this_type_script = load_script().map_err(|e| Error::from(e))?;
    let input_cells =
        util::find_cells_by_script(ScriptType::Type, &this_type_script, Source::Input)?;
    let output_cells =
        util::find_cells_by_script(ScriptType::Type, &this_type_script, Source::Output)?;
    let dep_cells =
        util::find_cells_by_script(ScriptType::Type, &this_type_script, Source::CellDep)?;

    let action_data = util::load_das_action()?;
    let action = action_data.as_reader().action().raw_data();
    if action == b"propose" {
        debug!("Route to propose action ...");

        let mut parser = util::load_das_witnesses(None)?;
        parser.parse_all_data()?;
        parser.parse_only_config(&[ConfigID::ConfigCellMain])?;
        let config_main = parser.configs().main()?;

        if !(dep_cells.len() == 0 && input_cells.len() == 0 && output_cells.len() == 1) {
            return Err(Error::ProposalFoundInvalidTransaction);
        }

        // Read outputs_data and witness of the ProposalCell.
        let index = &output_cells[0];
        let (_, _, entity) = parser.verify_and_get(index.to_owned(), Source::Output)?;
        let proposal_cell_data = ProposalCellData::from_slice(entity.as_reader().raw_data())
            .map_err(|_| Error::WitnessEntityDecodingError)?;
        let proposal_cell_data_reader = proposal_cell_data.as_reader();

        let required_cells_count = verify_slices(proposal_cell_data_reader.slices())?;
        let dep_related_cells = find_proposal_related_cells(config_main, Source::CellDep)?;

        #[cfg(not(feature = "mainnet"))]
        inspect_slices(proposal_cell_data_reader.slices())?;
        #[cfg(not(feature = "mainnet"))]
        inspect_related_cells(
            &parser,
            config_main,
            dep_related_cells.clone(),
            Source::CellDep,
            None,
        )?;

        if required_cells_count != dep_related_cells.len() {
            return Err(Error::ProposalSliceRelatedCellMissing);
        }

        verify_slices_relevant_cells(
            config_main,
            proposal_cell_data_reader.slices(),
            dep_related_cells,
            None,
        )?;
    } else if action == b"extend_proposal" {
        debug!("Route to extend_proposal action ...");

        let mut parser = util::load_das_witnesses(None)?;
        parser.parse_all_data()?;
        parser.parse_only_config(&[ConfigID::ConfigCellMain])?;
        let config_main = parser.configs().main()?;

        if !(dep_cells.len() == 1 && input_cells.len() == 0 && output_cells.len() == 1) {
            return Err(Error::ProposalFoundInvalidTransaction);
        }

        // Read outputs_data and witness of previous ProposalCell.
        let index = &dep_cells[0];
        let (_, _, entity) = parser.verify_and_get(index.to_owned(), Source::CellDep)?;
        let prev_proposal_cell_data = ProposalCellData::from_slice(entity.as_reader().raw_data())
            .map_err(|_| Error::WitnessEntityDecodingError)?;
        let prev_proposal_cell_data_reader = prev_proposal_cell_data.as_reader();

        // Read outputs_data and witness of the ProposalCell.
        let index = &output_cells[0];
        let (_, _, entity) = parser.verify_and_get(index.to_owned(), Source::Output)?;
        let proposal_cell_data = ProposalCellData::from_slice(entity.as_reader().raw_data())
            .map_err(|_| Error::WitnessEntityDecodingError)?;
        let proposal_cell_data_reader = proposal_cell_data.as_reader();

        let required_cells_count = verify_slices(proposal_cell_data_reader.slices())?;
        let dep_related_cells = find_proposal_related_cells(config_main, Source::CellDep)?;

        #[cfg(not(feature = "mainnet"))]
        inspect_slices(proposal_cell_data_reader.slices())?;
        #[cfg(not(feature = "mainnet"))]
        inspect_related_cells(
            &parser,
            config_main,
            dep_related_cells.clone(),
            Source::CellDep,
            None,
        )?;

        if required_cells_count != dep_related_cells.len() {
            return Err(Error::ProposalSliceRelatedCellMissing);
        }

        verify_slices_relevant_cells(
            config_main,
            proposal_cell_data_reader.slices(),
            dep_related_cells,
            Some(prev_proposal_cell_data_reader.slices()),
        )?;
    } else if action == b"confirm_proposal" {
        debug!("Route to confirm_proposal action ...");

        let timestamp = util::load_timestamp()?;
        // let height = util::load_height()?;

        let mut parser = util::load_das_witnesses(None)?;
        parser.parse_all_data()?;
        parser.parse_only_config(&[ConfigID::ConfigCellMain, ConfigID::ConfigCellRegister])?;
        let config_main = parser.configs().main()?;
        let config_register = parser.configs().register()?;

        if !(dep_cells.len() == 0 && input_cells.len() == 1 && output_cells.len() == 0) {
            return Err(Error::ProposalFoundInvalidTransaction);
        }

        // Read outputs_data and witness of ProposalCell.
        let index = &input_cells[0];
        let (_, _, entity) = parser.verify_and_get(index.to_owned(), Source::Input)?;
        let proposal_cell_data = ProposalCellData::from_slice(entity.as_reader().raw_data())
            .map_err(|_| Error::WitnessEntityDecodingError)?;
        let proposal_cell_data_reader = proposal_cell_data.as_reader();

        debug!("Check all AccountCells are updated or created base on proposal.");

        let input_related_cells = find_proposal_related_cells(config_main, Source::Input)?;
        let output_account_cells = find_output_account_cells(config_main)?;

        #[cfg(not(feature = "mainnet"))]
        inspect_slices(proposal_cell_data_reader.slices())?;
        #[cfg(not(feature = "mainnet"))]
        inspect_related_cells(
            &parser,
            config_main,
            input_related_cells.clone(),
            Source::Input,
            Some(output_account_cells.clone()),
        )?;

        verify_proposal_execution_result(
            &parser,
            config_main,
            config_register,
            timestamp,
            proposal_cell_data_reader,
            input_related_cells,
            output_account_cells,
        )?;

        debug!("Check that all revenues are correctly allocated to each roles in DAS.");
    } else if action == b"recycle_proposal" {
        debug!("Route to recycle_proposal action ...");

        let mut parser = util::load_das_witnesses(None)?;
        parser.parse_all_data()?;
        parser.parse_only_config(&[ConfigID::ConfigCellRegister])?;
        let config_register = parser.configs().register()?;

        if !(dep_cells.len() == 0 && input_cells.len() == 1 && output_cells.len() == 0) {
            return Err(Error::ProposalFoundInvalidTransaction);
        }

        debug!("Check if ProposalCell can be recycled.");

        let index = &input_cells[0];
        let (_, _, entity) = parser.verify_and_get(index.to_owned(), Source::Input)?;
        let proposal_cell_data = ProposalCellData::from_slice(entity.as_reader().raw_data())
            .map_err(|_| Error::WitnessEntityDecodingError)?;
        let proposal_cell_data_reader = proposal_cell_data.as_reader();

        let height = util::load_height()?;
        let proposal_min_recycle_interval =
            u8::from(config_register.proposal_min_recycle_interval()) as u64;
        let created_at_height = u64::from(proposal_cell_data_reader.created_at_height());
        if height - created_at_height < proposal_min_recycle_interval {
            return Err(Error::ProposalRecycleNeedWaitLonger);
        }

        debug!("Check if refund lock and amount is correct.");

        let refund_lock = proposal_cell_data_reader.proposer_lock().to_entity();
        let refund_cells =
            util::find_cells_by_script(ScriptType::Lock, &refund_lock.into(), Source::Output)?;
        if refund_cells.len() != 1 {
            return Err(Error::ProposalRecycleCanNotFoundRefundCell);
        }
        let proposal_capacity =
            load_cell_capacity(index.to_owned(), Source::Input).map_err(|e| Error::from(e))?;
        let refund_capacity =
            load_cell_capacity(refund_cells[0], Source::Output).map_err(|e| Error::from(e))?;
        if proposal_capacity > refund_capacity {
            return Err(Error::ProposalRecycleRefundAmountError);
        }
    } else {
        return Err(Error::ActionNotSupported);
    }

    Ok(())
}

#[cfg(not(feature = "mainnet"))]
fn inspect_slices(slices_reader: SliceListReader) -> Result<(), Error> {
    debug!("Inspect Slices [");
    for (sl_index, sl_reader) in slices_reader.iter().enumerate() {
        debug!("  Slice[{}] [", sl_index);
        for (index, item) in sl_reader.iter().enumerate() {
            let type_ = item.item_type().raw_data()[0];
            let item_type = match type_ {
                0 => "exist",
                1 => "proposed",
                _ => "new",
            };

            debug!(
                "    Item[{}] {{ account_id: {:?}, item_type: {}, next: {:?} }}",
                index,
                item.account_id(),
                item_type,
                item.next()
            );
        }
        debug!("  ]");
    }
    debug!("]");

    Ok(())
}

// #[cfg(not(feature = "mainnet"))]
fn inspect_related_cells(
    parser: &WitnessesParser,
    config_main: ConfigCellMainReader,
    related_cells: Vec<usize>,
    related_cells_source: Source,
    output_account_cells: Option<Vec<usize>>,
) -> Result<(), Error> {
    use das_core::inspect;

    debug!("Inspect {:?}:", related_cells_source);
    for i in related_cells {
        let script = load_cell_type(i, related_cells_source)
            .map_err(|e| Error::from(e))?
            .unwrap();
        let code_hash = Hash::from(script.code_hash());
        let (_, _, entity) = parser.verify_and_get(i, related_cells_source)?;
        let data = util::load_cell_data(i, related_cells_source)?;

        debug!("  Input[{}].cell.type: {}", i, script);

        if util::is_reader_eq(
            config_main.type_id_table().account_cell(),
            code_hash.as_reader(),
        ) {
            inspect::account_cell(Source::Input, i, &data, entity.to_owned());
        } else if util::is_reader_eq(
            config_main.type_id_table().pre_account_cell(),
            code_hash.as_reader(),
        ) {
            inspect::pre_account_cell(Source::Input, i, &data, entity.to_owned());
        }
    }

    if let Some(output_account_cells) = output_account_cells {
        for i in output_account_cells {
            let script = load_cell_type(i, Source::Output)
                .map_err(|e| Error::from(e))?
                .unwrap();
            let code_hash = Hash::from(script.code_hash());
            let (_, _, entity) = parser.verify_and_get(i, Source::Output)?;
            let data = util::load_cell_data(i, Source::Output)?;

            debug!("  Output[{}].cell.type: {}", i, script);

            if util::is_reader_eq(
                config_main.type_id_table().account_cell(),
                code_hash.as_reader(),
            ) {
                inspect::account_cell(Source::Output, i, &data, entity.to_owned());
            }
        }
    }

    Ok(())
}

fn verify_slices(slices_reader: SliceListReader) -> Result<usize, Error> {
    debug!("Check the data structure of proposal slices.");

    let mut required_cells_count: usize = 0;
    let mut exists_account_ids: Vec<bytes::Bytes> = Vec::new();

    // debug!("slices_reader = {}", slices_reader);

    for (sl_index, sl_reader) in slices_reader.iter().enumerate() {
        debug!("Check Slice[{}] ...", sl_index);
        let mut account_id_list = Vec::new();

        assert!(
            sl_reader.len() > 1,
            Error::ProposalSliceMustContainMoreThanOneElement,
            "Slice[{}] must contain more than one element, but {} found.",
            sl_index,
            sl_reader.len()
        );

        // The "next" of last item is refer to an existing account, so we put it into the vector.
        let last_item = sl_reader.get(sl_reader.len() - 1).unwrap();
        exists_account_ids.push(bytes::Bytes::from(last_item.next().raw_data()));

        for (index, item) in sl_reader.iter().enumerate() {
            debug!("  Check if Item[{}] refer to correct next.", index);

            // Check the uniqueness of current account.
            let account_id_bytes = bytes::Bytes::from(item.account_id().raw_data());
            for account_id in exists_account_ids.iter() {
                assert!(
                    account_id.ne(account_id_bytes.as_ref()),
                    Error::ProposalSliceItemMustBeUniqueAccount,
                    "  Item[{}] is an exists account.",
                    index
                );
            }

            // Check the continuity of the items in the slice.
            if let Some(next) = sl_reader.get(index + 1) {
                assert!(
                    util::is_reader_eq(item.next(), next.account_id()),
                    Error::ProposalSliceIsDiscontinuity,
                    "  Item[{}].next should be {}, but it is {} now.",
                    index,
                    next.account_id(),
                    item.next()
                );
            }

            // Store exists account IDs for uniqueness verification.
            exists_account_ids.push(account_id_bytes.clone());
            // Store account IDs for order verification.
            account_id_list.push(account_id_bytes);
            required_cells_count += 1;
        }

        // Check the order of items in the slice.
        let sorted_account_id_list = DasSortedList::new(account_id_list.clone());
        assert!(
            sorted_account_id_list.cmp_order_with(account_id_list),
            Error::ProposalSliceIsNotSorted,
            "The order of items in Slice[{}] is incorrect.",
            sl_index
        );
    }

    Ok(required_cells_count)
}

fn find_proposal_related_cells(
    config: ConfigCellMainReader,
    source: Source,
) -> Result<Vec<usize>, Error> {
    // Find related cells' indexes in cell_deps or inputs.
    let account_cell_type_id = config.type_id_table().account_cell();
    let account_cells =
        util::find_cells_by_type_id(ScriptType::Type, account_cell_type_id, source)?;
    let pre_account_cell_type_id = config.type_id_table().pre_account_cell();
    let pre_account_cells =
        util::find_cells_by_type_id(ScriptType::Type, pre_account_cell_type_id, source)?;

    if pre_account_cells.len() <= 0 {
        return Err(Error::ProposalFoundInvalidTransaction);
    }

    // Merge cells' indexes in sorted order.
    let mut sorted = Vec::new();
    if account_cells.len() > 0 {
        let mut i = 0;
        let mut j = 0;
        let remain;
        let remain_idx;
        loop {
            if account_cells[i] < pre_account_cells[j] {
                sorted.push(account_cells[i]);
                i += 1;
                if i == account_cells.len() {
                    remain = pre_account_cells;
                    remain_idx = j;
                    break;
                }
            } else {
                sorted.push(pre_account_cells[j]);
                j += 1;
                if j == pre_account_cells.len() {
                    remain = account_cells;
                    remain_idx = i;
                    break;
                }
            }
        }

        for i in remain_idx..remain.len() {
            sorted.push(remain[i]);
        }
    } else {
        sorted = pre_account_cells;
        sorted.sort();
    }

    debug!(
        "Inputs cells(AccountCell/PreAccountCell) sorted index list: {:?}",
        sorted
    );

    Ok(sorted)
}

fn find_output_account_cells(config: ConfigCellMainReader) -> Result<Vec<usize>, Error> {
    // Find updated cells' indexes in outputs.
    let account_cell_type_id = config.type_id_table().account_cell();
    let mut account_cells =
        util::find_cells_by_type_id(ScriptType::Type, account_cell_type_id, Source::Output)?;
    account_cells.sort();

    if account_cells.len() <= 0 {
        return Err(Error::ProposalFoundInvalidTransaction);
    }

    debug!(
        "Outputs cells(AccountCell) sorted index list: {:?}",
        account_cells
    );

    Ok(account_cells)
}

fn verify_slices_relevant_cells(
    config: ConfigCellMainReader,
    slices_reader: SliceListReader,
    relevant_cells: Vec<usize>,
    prev_slices_reader_opt: Option<SliceListReader>,
) -> Result<(), Error> {
    debug!("Check the proposal slices relevant cells are real exist and in correct status.");

    let mut i = 0;
    for (sl_index, sl_reader) in slices_reader.iter().enumerate() {
        debug!("Check slice {} ...", sl_index);
        let mut next_of_first_cell = AccountId::default();
        for (item_index, item) in sl_reader.iter().enumerate() {
            let item_account_id = item.account_id();
            let item_type = u8::from(item.item_type());

            let cell_index = relevant_cells[i];

            // Check if the relevant cells has the same type as in the proposal.
            let expected_type_id = if item_type == ProposalSliceItemType::Exist as u8 {
                config.type_id_table().account_cell()
            } else {
                config.type_id_table().pre_account_cell()
            };
            verify_cell_type_id(item_index, cell_index, Source::CellDep, &expected_type_id)?;

            let cell_data = util::load_cell_data(cell_index, Source::CellDep)?;
            // Check if the relevant cells have the same account ID as in the proposal.
            verify_cell_account_id(
                item_index,
                &cell_data,
                cell_index,
                Source::CellDep,
                item_account_id,
            )?;

            // ⚠️ The first item is very very important, its "next" must be correct so that
            // AccountCells can form a linked list.
            if item_index == 0 {
                // If this is the first proposal in proposal chain, all slice must start with an AccountCell.
                if prev_slices_reader_opt.is_none() {
                    assert!(
                        item_type == ProposalSliceItemType::Exist as u8,
                        Error::ProposalSliceMustStartWithAccountCell,
                        "  In the first proposal of a proposal chain, all slice should start with an AccountCell."
                    );

                    // The correct "next" of first proposal is come from the cell's outputs_data.
                    next_of_first_cell = AccountId::try_from(account_cell::get_next(&cell_data))
                        .map_err(|_| Error::InvalidCellData)?;

                // If this is the extended proposal in proposal chain, slice may starting with an
                // AccountCell/PreAccountCell included in previous proposal, or it may starting with
                // an AccountCell not included in previous proposal.
                } else {
                    assert!(
                        item_type == ProposalSliceItemType::Exist as u8 || item_type == ProposalSliceItemType::Proposed as u8,
                        Error::ProposalSliceMustStartWithAccountCell,
                        "  In the extended proposal of a proposal chain, slices should start with an AccountCell or a PreAccountCell which included in previous proposal."
                    );

                    let prev_slices_reader = prev_slices_reader_opt.as_ref().unwrap();
                    next_of_first_cell =
                        match find_item_contains_account_id(prev_slices_reader, &item_account_id) {
                            // If the item is included in previous proposal, then we need to get its latest "next" from the proposal.
                            Ok(prev_item) => prev_item.next(),
                            // If the item is not included in previous proposal, then we get its latest "next" from the cell's outputs_data.
                            Err(_) => AccountId::try_from(account_cell::get_next(&cell_data))
                                .map_err(|_| Error::InvalidCellData)?,
                        };
                }
            }

            i += 1;
        }

        // Check if the first item's "next" has pass to the last item correctly.
        let item = sl_reader.get(sl_reader.len() - 1).unwrap();
        let next_of_last_item = item.next();

        debug!(
            "  Compare the last next of slice: {} != {} => {}",
            next_of_first_cell,
            next_of_last_item,
            !util::is_reader_eq(next_of_first_cell.as_reader(), next_of_last_item)
        );
        if !util::is_reader_eq(next_of_first_cell.as_reader(), next_of_last_item) {
            return Err(Error::ProposalSliceNotEndCorrectly);
        }
    }

    Ok(())
}

fn find_item_contains_account_id(
    prev_slices_reader: &SliceListReader,
    account_id: &AccountIdReader,
) -> Result<ProposalItem, Error> {
    for slice in prev_slices_reader.iter() {
        for item in slice.iter() {
            if util::is_reader_eq(item.account_id(), *account_id) {
                return Ok(item.to_entity());
            }
        }
    }

    debug!("Can not find previous item: {}", account_id);
    Err(Error::PrevProposalItemNotFound)
}

fn verify_proposal_execution_result(
    parser: &WitnessesParser,
    config_main: ConfigCellMainReader,
    config_register: ConfigCellRegisterReader,
    timestamp: u64,
    proposal_cell_data_reader: ProposalCellDataReader,
    input_related_cells: Vec<usize>,
    output_account_cells: Vec<usize>,
) -> Result<(), Error> {
    debug!("Check that all AccountCells/PreAccountCells have been converted according to the proposal.");

    let slices_reader = proposal_cell_data_reader.slices();
    let account_cell_type_id = config_main.type_id_table().account_cell();
    let pre_account_cell_type_id = config_main.type_id_table().pre_account_cell();

    let mut wallet = Wallet::new();
    let inviter_profit_rate = u32::from(config_register.profit().profit_rate_of_inviter()) as u64;
    let channel_profit_rate = u32::from(config_register.profit().profit_rate_of_channel()) as u64;
    let proposal_create_profit_rate =
        u32::from(config_register.profit().profit_rate_of_proposal_create()) as u64;
    let proposal_confirm_profit_rate =
        u32::from(config_register.profit().profit_rate_of_proposal_confirm()) as u64;

    let mut output_ref_cells = load_ref_cells(config_main)?;

    let mut new_account_ids = Vec::new();

    let mut i = 0;
    for (sl_index, sl_reader) in slices_reader.iter().enumerate() {
        debug!("Check Slice[{}] ...", sl_index);
        for (item_index, item) in sl_reader.iter().enumerate() {
            let item_account_id = item.account_id();
            let item_type = u8::from(item.item_type());
            let item_next = item.next();

            let (input_cell_data, input_cell_entity) =
                util::load_cell_data_and_entity(&parser, input_related_cells[i], Source::Input)?;
            let (output_cell_data, output_cell_entity) =
                util::load_cell_data_and_entity(&parser, output_account_cells[i], Source::Output)?;

            if item_type == ProposalSliceItemType::Exist as u8
                || item_type == ProposalSliceItemType::Proposed as u8
            {
                debug!(
                    "  Item[{}] Check that the existing inputs[{}].AccountCell and outputs[{}].AccountCell is updated correctly.",
                    item_index, input_related_cells[i], output_account_cells[i]
                );

                // All cells' type is must be account-cell-type
                verify_cell_type_id(
                    item_index,
                    input_related_cells[i],
                    Source::Input,
                    &account_cell_type_id,
                )?;
                verify_cell_type_id(
                    item_index,
                    output_account_cells[i],
                    Source::Output,
                    &account_cell_type_id,
                )?;

                // All cells' account_id in data must be the same as the account_id in proposal.
                verify_cell_account_id(
                    item_index,
                    &input_cell_data,
                    input_related_cells[i],
                    Source::Input,
                    item_account_id,
                )?;
                verify_cell_account_id(
                    item_index,
                    &output_cell_data,
                    output_account_cells[i],
                    Source::Output,
                    item_account_id,
                )?;

                // For the existing AccountCell, only the next field in data can be modified.
                is_id_same(item_index, &output_cell_data, &input_cell_data)?;
                is_account_same(item_index, &output_cell_data, &input_cell_data)?;
                is_expired_at_same(item_index, &output_cell_data, &input_cell_data)?;
                is_next_correct(item_index, &output_cell_data, item_next)?;

                let input_account_witness =
                    AccountCellData::new_unchecked(input_cell_entity.as_reader().raw_data().into());
                let input_account_witness_reader = input_account_witness.as_reader();
                let output_account_witness = AccountCellData::new_unchecked(
                    output_cell_entity.as_reader().raw_data().into(),
                );
                let output_cell_witness_reader = output_account_witness.as_reader();

                assert!(
                    util::is_reader_eq(input_account_witness_reader, output_cell_witness_reader),
                    Error::ProposalWitnessCanNotBeModified,
                    "The witness of exist AccountCell should not be modified."
                );
            } else {
                debug!(
                    "  Item[{}] Check that the inputs[{}].PreAccountCell and outputs[{}].AccountCell is converted correctly.",
                    item_index, input_related_cells[i], output_account_cells[i]
                );

                // All cells' type is must be pre-account-cell-type/account-cell-type
                verify_cell_type_id(
                    item_index,
                    input_related_cells[i],
                    Source::Input,
                    &pre_account_cell_type_id,
                )?;
                verify_cell_type_id(
                    item_index,
                    output_account_cells[i],
                    Source::Output,
                    &account_cell_type_id,
                )?;

                // All cells' account_id in data must be the same as the account_id in proposal.
                // TODO here is a PreAccountCell
                verify_cell_account_id(
                    item_index,
                    &input_cell_data,
                    input_related_cells[i],
                    Source::Input,
                    item_account_id,
                )?;
                verify_cell_account_id(
                    item_index,
                    &output_cell_data,
                    output_account_cells[i],
                    Source::Output,
                    item_account_id,
                )?;

                // Store account IDs of all new accounts for later RefCell verification.
                new_account_ids.push(item_account_id.raw_data().to_vec());

                let input_cell_witness = PreAccountCellData::new_unchecked(
                    input_cell_entity.as_reader().raw_data().into(),
                );
                let input_cell_witness_reader = input_cell_witness.as_reader();
                let new_cell_witness = AccountCellData::new_unchecked(
                    output_cell_entity.as_reader().raw_data().into(),
                );
                let output_cell_witness_reader = new_cell_witness.as_reader();

                let account_length = account_cell::get_account(&output_cell_data).len() as u64;
                let total_capacity = load_cell_capacity(input_related_cells[i], Source::Input)
                    .map_err(|e| Error::from(e))?;
                let storage_capacity = util::calc_account_storage_capacity(account_length);
                // Allocate the profits carried by PreAccountCell to the wallets for later verification.
                let profit = total_capacity - storage_capacity;

                util::verify_account_length_and_years(
                    account_length as usize,
                    timestamp,
                    Some(item_index),
                )?;

                // Check all fields in the data of new AccountCell.
                is_id_correct(item_index, &output_cell_data, &input_cell_data)?;
                is_account_correct(item_index, &output_cell_data)?;
                is_next_correct(item_index, &output_cell_data, item_next)?;
                is_expired_at_correct(
                    item_index,
                    profit,
                    timestamp,
                    &output_cell_data,
                    input_cell_witness_reader,
                )?;

                // Check all fields in the witness of new AccountCell.
                verify_witness_id(item_index, &output_cell_data, output_cell_witness_reader)?;
                verify_witness_account(item_index, &output_cell_data, output_cell_witness_reader)?;
                verify_witness_locks(
                    item_index,
                    output_cell_witness_reader,
                    input_cell_witness_reader,
                )?;
                verify_witness_status(item_index, output_cell_witness_reader)?;
                verify_witness_records(item_index, output_cell_witness_reader)?;

                verify_ref_cells(
                    item_index,
                    output_cell_witness_reader,
                    &mut output_ref_cells,
                )?;

                // Only when inviter_wallet's length is equal to account ID it will be count in profit.
                let mut inviter_profit = 0;
                if input_cell_witness_reader.inviter_wallet().len() == ACCOUNT_ID_LENGTH {
                    let wallet_id = input_cell_witness_reader.inviter_wallet().raw_data();
                    inviter_profit = profit * inviter_profit_rate / RATE_BASE;
                    debug!(
                        "  Item[{}] Wallet[0x{}]: {}(inviter_profit) = {}(profit) * {}(inviter_profit_rate) / {}(RATE_BASE)",
                        item_index, util::hex_string(wallet_id), inviter_profit, profit, inviter_profit_rate, RATE_BASE
                    );
                    wallet.add_balance(wallet_id, inviter_profit);
                };

                let mut channel_profit = 0;
                if input_cell_witness_reader.channel_wallet().len() == ACCOUNT_ID_LENGTH {
                    let wallet_id = input_cell_witness_reader.channel_wallet().raw_data();
                    channel_profit = profit * channel_profit_rate / RATE_BASE;
                    debug!(
                        "  Item[{}] Wallet[0x{}]: {}(channel_profit) = {}(profit) * {}(channel_profit_rate) / {}(RATE_BASE)",
                        item_index, util::hex_string(wallet_id), channel_profit, profit, channel_profit_rate, RATE_BASE
                    );
                    wallet.add_balance(wallet_id, channel_profit);
                };

                let proposal_create_profit = profit * proposal_create_profit_rate / RATE_BASE;
                wallet.add_balance(&PROPOSAL_CREATOR_WALLET_ID, proposal_create_profit);

                let proposal_confirm_profit = profit * proposal_confirm_profit_rate / RATE_BASE;
                wallet.add_balance(&PROPOSAL_CONFIRMOR_WALLET_ID, proposal_confirm_profit);

                let das_profit = profit
                    - inviter_profit
                    - channel_profit
                    - proposal_create_profit
                    - proposal_confirm_profit;
                wallet.add_balance(&ROOT_WALLET_ID, das_profit);

                debug!(
                    "  Item[{}] Wallet[0x{}]: {}(das_profit) = {}(profit) - {}(inviter_profit) - {}(channel_profit) - {}(proposal_create_profit) - {}(proposal_confirm_profit)",
                    item_index, util::hex_string(&ROOT_WALLET_ID), das_profit, profit, inviter_profit, channel_profit, proposal_create_profit, proposal_confirm_profit
                );
            }

            i += 1;
        }
    }

    for item in output_ref_cells {
        if item.is_some() {
            debug!("Redundant RefCells is not allowed in this transaction.");
            return Err(Error::ProposalFoundInvalidTransaction);
        }
    }

    debug!("Check if the balance of all WalletCells have increased correctly.");

    let wallet_cell_type_id = config_main.type_id_table().wallet_cell();
    let old_wallet_cells =
        util::find_cells_by_type_id(ScriptType::Type, wallet_cell_type_id, Source::Input)?;
    let new_wallet_cells =
        util::find_cells_by_type_id(ScriptType::Type, wallet_cell_type_id, Source::Output)?;

    assert!(
        old_wallet_cells.len() == new_wallet_cells.len(),
        Error::ProposalFoundInvalidTransaction,
        "The number of WalletCells in inputs should equal to outputs. (inputs: {}, outputs{})",
        old_wallet_cells.len(),
        new_wallet_cells.len()
    );

    // The DAS wallet do not count.
    assert!(
        wallet.len() - 3 == new_wallet_cells.len(),
        Error::ProposalFoundInvalidTransaction,
        "The number of WalletCells in outputs should equal to the number of wallets involved by PreAccountCell. (involved: {}, outputs: {})",
        wallet.len() - 3,
        new_wallet_cells.len()
    );

    for (i, old_wallet_index) in old_wallet_cells.into_iter().enumerate() {
        let new_wallet_index = new_wallet_cells.get(i).unwrap().to_owned();

        let old_wallet_id = util::load_cell_data(old_wallet_index, Source::Input)?;
        let new_wallet_id = util::load_cell_data(new_wallet_index, Source::Output)?;

        assert!(
            old_wallet_id == new_wallet_id,
            Error::ProposalConfirmWalletMissMatch,
            "The WalletCells should have the same order in both inputs and outputs. (inputs[{}]: 0x{}, outputs[{}]: 0x{})",
            old_wallet_index,
            util::hex_string(old_wallet_id.as_slice()),
            new_wallet_index,
            util::hex_string(new_wallet_id.as_slice())
        );

        let old_balance =
            load_cell_capacity(old_wallet_index, Source::Input).map_err(|e| Error::from(e))?;
        let new_balance =
            load_cell_capacity(new_wallet_index, Source::Output).map_err(|e| Error::from(e))?;
        let current_profit = new_balance - old_balance;

        debug!(
            "Check if WalletCell[0x{}] has updated balance correctly.",
            util::hex_string(new_wallet_id.as_slice())
        );

        // Balance in wallet instance do not contains cell occupied capacities, so it is pure profit.
        let result = wallet
            .cmp_balance(new_wallet_id.as_slice(), current_profit)
            .map_err(|_| Error::ProposalConfirmWalletMissMatch)?;
        if !result {
            debug!(
                "Wallet balance variation: {}(current_profit) = {}(0x{}) - {}(0x{})",
                current_profit,
                new_balance,
                util::hex_string(new_wallet_id.as_slice()),
                old_balance,
                util::hex_string(old_wallet_id.as_slice())
            );
            debug!(
                "Compare profit in WalletCell[0x{}] with expected: {}(current_profit) != {}(expected_profit) -> true",
                util::hex_string(new_wallet_id.as_slice()),
                current_profit,
                wallet.get_balance(old_wallet_id.as_slice()).unwrap()
            );
            return Err(Error::ProposalConfirmWalletBalanceError);
        }
    }

    debug!("Check if the profit of proposer has been transfered correctly.");

    let mut expected_proposer_profit = wallet.get_balance(&PROPOSAL_CREATOR_WALLET_ID).unwrap();
    let original_expected_proposer_profit = expected_proposer_profit;
    let proposer_lock: ckb_packed::Script =
        proposal_cell_data_reader.proposer_lock().to_entity().into();
    let proposer_wallet_cells =
        util::find_cells_by_script(ScriptType::Lock, &proposer_lock, Source::Output)?;

    if expected_proposer_profit < 6_100_000_000 {
        expected_proposer_profit += 6_100_000_000;
    }

    // The keeper who create proposal and confirm proposal maybe the same one, so more than one outputs is allowed.
    assert!(
        proposer_wallet_cells.len() >= 1,
        Error::ProposalConfirmWalletBalanceError,
        "There should be more than 1 output with proposer lock, but {} found.",
        proposer_wallet_cells.len()
    );

    let mut proposer_current_profit = 0;
    for index in proposer_wallet_cells {
        proposer_current_profit +=
            load_cell_capacity(index, Source::Output).map_err(|e| Error::from(e))?;
    }

    assert!(
        expected_proposer_profit <= proposer_current_profit,
        Error::ProposalConfirmWalletBalanceError,
        "Outputs should has greater than or equal to expected capacity. (expected: {}, current: {})",
        expected_proposer_profit,
        proposer_current_profit
    );

    debug!("Check if the profit of DAS has been transfered correctly.");

    let mut expected_profit = wallet.get_balance(&ROOT_WALLET_ID).unwrap();
    let das_wallet_lock = das_wallet_lock();
    let das_wallet_cells =
        util::find_cells_by_script(ScriptType::Lock, &das_wallet_lock, Source::Output)?;

    if original_expected_proposer_profit < 6_100_000_000 {
        expected_profit -= 6_100_000_000;
    }

    assert!(
        das_wallet_cells.len() == 1,
        Error::ProposalConfirmWalletBalanceError,
        "There should be 1 output with DAS wallet lock, but {} found.",
        das_wallet_cells.len()
    );

    let current_profit =
        load_cell_capacity(das_wallet_cells[0], Source::Output).map_err(|e| Error::from(e))?;

    assert!(
        expected_profit <= current_profit,
        Error::ProposalConfirmWalletBalanceError,
        "Outputs[{}] should has greater than or equal to expected capacity. (expected: {}, current: {})",
        das_wallet_cells[0],
        expected_profit,
        current_profit
    );

    Ok(())
}

fn verify_cell_type_id(
    item_index: usize,
    cell_index: usize,
    source: Source,
    expected_type_id: &HashReader,
) -> Result<(), Error> {
    let cell_type_id = load_cell_type(cell_index, source)
        .map_err(|e| Error::from(e))?
        .map(|script| Hash::from(script.code_hash()))
        .ok_or(Error::ProposalSliceRelatedCellNotFound)?;

    assert!(
        util::is_reader_eq(expected_type_id.to_owned(), cell_type_id.as_reader()),
        Error::ProposalCellTypeError,
        "  The type ID of Item[{}] should be {}. (related_cell: {:?}[{}])",
        item_index,
        expected_type_id,
        source,
        cell_index
    );

    Ok(())
}

fn verify_cell_account_id(
    item_index: usize,
    cell_data: &Vec<u8>,
    cell_index: usize,
    source: Source,
    expected_account_id: AccountIdReader,
) -> Result<(), Error> {
    let account_id = AccountId::try_from(account_cell::get_id(&cell_data))
        .map_err(|_| Error::InvalidCellData)?;

    assert!(
        util::is_reader_eq(account_id.as_reader(), expected_account_id),
        Error::ProposalCellAccountIdError,
        "  The account ID of Item[{}] should be {}. (related_cell: {:?}[{}])",
        item_index,
        expected_account_id,
        source,
        cell_index
    );

    Ok(())
}

fn is_bytes_eq(
    item_index: usize,
    field: &str,
    current_bytes: &[u8],
    expected_bytes: &[u8],
    error_code: Error,
) -> Result<(), Error> {
    if current_bytes != expected_bytes {
        debug!(
            "  Item[{}] Check outputs[].AccountCell.{}: 0x{} != 0x{} => true",
            item_index,
            field,
            util::hex_string(current_bytes),
            util::hex_string(expected_bytes)
        );
        return Err(error_code);
    }

    Ok(())
}

fn is_id_same(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    input_cell_data: &Vec<u8>,
) -> Result<(), Error> {
    is_bytes_eq(
        item_index,
        "id",
        account_cell::get_id(output_cell_data),
        account_cell::get_id(input_cell_data),
        Error::ProposalFieldCanNotBeModified,
    )
}

fn is_account_same(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    input_cell_data: &Vec<u8>,
) -> Result<(), Error> {
    is_bytes_eq(
        item_index,
        "account",
        account_cell::get_account(output_cell_data),
        account_cell::get_account(input_cell_data),
        Error::ProposalFieldCanNotBeModified,
    )
}

fn is_expired_at_same(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    input_cell_data: &Vec<u8>,
) -> Result<(), Error> {
    let input_expired_at = account_cell::get_expired_at(input_cell_data);
    let output_expired_at = account_cell::get_expired_at(output_cell_data);

    if input_expired_at != output_expired_at {
        debug!(
            "  Item[{}] Check outputs[].AccountCell.expired_at: {:x?} != {:x?} => true",
            item_index, input_expired_at, output_expired_at
        );
        return Err(Error::ProposalFieldCanNotBeModified);
    }

    Ok(())
}

fn is_id_correct(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    input_cell_data: &Vec<u8>,
) -> Result<(), Error> {
    is_bytes_eq(
        item_index,
        "id",
        account_cell::get_id(output_cell_data),
        account_cell::get_id(input_cell_data),
        Error::ProposalConfirmIdError,
    )
}

fn is_next_correct(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    proposed_next: AccountIdReader,
) -> Result<(), Error> {
    let expected_next = proposed_next.raw_data();

    is_bytes_eq(
        item_index,
        "next",
        account_cell::get_next(output_cell_data),
        expected_next,
        Error::ProposalConfirmNextError,
    )
}

fn is_expired_at_correct(
    item_index: usize,
    profit: u64,
    current_timestamp: u64,
    output_cell_data: &Vec<u8>,
    pre_account_cell_witness: PreAccountCellDataReader,
) -> Result<(), Error> {
    let price = u64::from(pre_account_cell_witness.price().new());
    let quote = u64::from(pre_account_cell_witness.quote());
    let discount = u32::from(pre_account_cell_witness.invited_discount());
    let duration = util::calc_duration_from_paid(profit, price, quote, discount);
    let expired_at = account_cell::get_expired_at(output_cell_data);
    let calculated_expired_at = current_timestamp + duration;

    debug!(
        "  Item[{}] Check if AccountCell.expired_at correct: cell.expired_at({}) <-> calculated_expired_at({})",
        item_index,
        expired_at,
        calculated_expired_at
    );

    assert!(
        current_timestamp + duration == expired_at,
        Error::ProposalConfirmExpiredAtError,
        "  Item[{}] The AccountCell.expired_at should be {}, but {} found.",
        item_index,
        calculated_expired_at,
        expired_at
    );

    Ok(())
}

fn is_account_correct(item_index: usize, output_cell_data: &Vec<u8>) -> Result<(), Error> {
    let expected_account_id = account_cell::get_id(output_cell_data);
    let account = account_cell::get_account(output_cell_data);

    let hash = util::blake2b_256(account);
    let account_id = hash.get(..ACCOUNT_ID_LENGTH).unwrap();

    is_bytes_eq(
        item_index,
        "account",
        account_id,
        expected_account_id,
        Error::ProposalConfirmAccountError,
    )
}

fn verify_witness_id(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    output_cell_witness_reader: AccountCellDataReader,
) -> Result<(), Error> {
    let account_id = output_cell_witness_reader.id().raw_data();
    let expected_account_id = account_cell::get_id(output_cell_data);

    is_bytes_eq(
        item_index,
        "witness.id",
        account_id,
        expected_account_id,
        Error::ProposalConfirmWitnessIDError,
    )
}

fn verify_witness_account(
    item_index: usize,
    output_cell_data: &Vec<u8>,
    output_cell_witness_reader: AccountCellDataReader,
) -> Result<(), Error> {
    let mut account = output_cell_witness_reader.account().as_readable();
    account.append(&mut ACCOUNT_SUFFIX.as_bytes().to_vec());
    let expected_account = account_cell::get_account(output_cell_data);

    is_bytes_eq(
        item_index,
        "witness.account",
        account.as_slice(),
        expected_account,
        Error::ProposalConfirmWitnessAccountError,
    )
}

fn verify_witness_locks(
    item_index: usize,
    output_cell_witness_reader: AccountCellDataReader,
    input_cell_witness_reader: PreAccountCellDataReader,
) -> Result<(), Error> {
    let owner_lock = output_cell_witness_reader.owner_lock();
    let manager_lock = output_cell_witness_reader.manager_lock();
    let expected_lock = input_cell_witness_reader.owner_lock();

    if !util::is_reader_eq(owner_lock, expected_lock) {
        debug!(
            "  Item[{}] Check outputs[].AccountCell.owner: {:x?} != {:x?} => {}",
            item_index,
            owner_lock,
            expected_lock,
            !util::is_reader_eq(owner_lock, expected_lock)
        );
        return Err(Error::ProposalConfirmWitnessOwnerError);
    }

    if !util::is_reader_eq(manager_lock, expected_lock) {
        debug!(
            "  Item[{}] Check outputs[].AccountCell.owner: {:x?} != {:x?} => {}",
            item_index,
            manager_lock,
            expected_lock,
            !util::is_reader_eq(manager_lock, expected_lock)
        );
        return Err(Error::ProposalConfirmWitnessManagerError);
    }

    Ok(())
}

fn verify_witness_status(
    item_index: usize,
    output_cell_witness_reader: AccountCellDataReader,
) -> Result<(), Error> {
    let status = u8::from(output_cell_witness_reader.status());

    if status != AccountStatus::Normal as u8 {
        debug!(
            "  Item[{}] Check if outputs[].AccountCell.status is normal. (Result: {}, expected: 0)",
            item_index, status
        );
        return Err(Error::ProposalConfirmWitnessManagerError);
    }

    Ok(())
}

fn verify_witness_records(
    item_index: usize,
    output_cell_witness_reader: AccountCellDataReader,
) -> Result<(), Error> {
    let records = output_cell_witness_reader.records();

    if !records.is_empty() {
        debug!(
            "  Item[{}] Check if outputs[].AccountCell.records is empty. (Result: {}, expected: true)",
            item_index,
            records.is_empty()
        );
        return Err(Error::ProposalConfirmWitnessRecordsError);
    }

    Ok(())
}

struct RefCellItem {
    item_index: usize,
    id: Vec<u8>,
    is_owner: bool,
    lock: Script,
}

fn load_ref_cells(config_main: ConfigCellMainReader) -> Result<Vec<Option<RefCellItem>>, Error> {
    let ref_cell_type_id = config_main.type_id_table().ref_cell();
    let input_ref_cells =
        util::find_cells_by_type_id(ScriptType::Type, ref_cell_type_id, Source::Input)?;
    let output_ref_cells =
        util::find_cells_by_type_id(ScriptType::Type, ref_cell_type_id, Source::Output)?;

    if !(input_ref_cells.len() == 0 && output_ref_cells.len() != 0) {
        debug!("The number of RefCells should be 0 in inputs and not 0 in outputs.");
        return Err(Error::ProposalFoundInvalidTransaction);
    }

    let mut ret = Vec::new();
    for i in output_ref_cells {
        let data = util::load_cell_data(i, Source::Output)?;
        ret.push(Some(RefCellItem {
            item_index: i,
            id: ref_cell::get_id(&data).to_vec(),
            is_owner: ref_cell::get_is_owner(&data),
            lock: Script::from(load_cell_lock(i, Source::Output).map_err(|err| Error::from(err))?),
        }))
    }

    Ok(ret)
}

fn verify_ref_cells(
    item_index: usize,
    output_cell_witness_reader: AccountCellDataReader,
    output_ref_cells: &mut Vec<Option<RefCellItem>>,
) -> Result<(), Error> {
    debug!(
        "  Item[{}] Check if AccountCell's RefCells are created correctly.",
        item_index
    );

    let account_id = output_cell_witness_reader.id().raw_data();
    let owner_lock = output_cell_witness_reader.owner_lock();
    let manager_lock = output_cell_witness_reader.manager_lock();

    let mut index_to_purge = Vec::new();
    let mut ref_cells_found = Vec::new();

    // Find RefCells of the account, check if their lock script is matched with AccountCell, and if they are duplicated.
    for (i, item) in output_ref_cells.into_iter().enumerate() {
        if item.is_none() {
            continue;
        }

        let item = item.as_mut().unwrap();
        if account_id == item.id {
            if item.is_owner {
                if ref_cells_found.contains(&"owner") {
                    debug!(
                        "  AccountCell[{}]'s OwnerCell[{}] is duplicated.",
                        item_index, item.item_index
                    );
                    return Err(Error::ProposalConfirmRefCellDuplicated);
                }
                if !util::is_reader_eq(owner_lock, item.lock.as_reader()) {
                    debug!(
                        "  The lock script of AccountCell[{}] and OwnerCell[{}] is not matched.",
                        item_index, item.item_index
                    );
                    return Err(Error::ProposalConfirmRefCellMissMatch);
                }
                ref_cells_found.push("owner");
            } else {
                if ref_cells_found.contains(&"manager") {
                    debug!(
                        "  AccountCell[{}]'s ManagerCell[{}] is duplicated.",
                        item_index, item.item_index
                    );
                    return Err(Error::ProposalConfirmRefCellDuplicated);
                }
                if !util::is_reader_eq(manager_lock, item.lock.as_reader()) {
                    debug!(
                        "  The lock script of AccountCell[{}] and ManagerCell[{}] is not matched.",
                        item_index, item.item_index
                    );
                    return Err(Error::ProposalConfirmRefCellMissMatch);
                }
                ref_cells_found.push("manager");
            }

            index_to_purge.push(i);
        }
    }

    // Check if the OwnerCell or ManagerCell is missing for the account.
    if ref_cells_found.len() != 2 {
        debug!(
            "  The number of RefCells of AccountCell[{}] must equal to 2 .",
            item_index
        );
        return Err(Error::ProposalConfirmRefCellMissing);
    }

    // Purge used items in output_ref_cells for later verification.
    for i in index_to_purge {
        output_ref_cells[i] = None;
    }

    Ok(())
}

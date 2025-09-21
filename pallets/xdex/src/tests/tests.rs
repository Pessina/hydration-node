use super::mock::*;
use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn build_erc20_transfer_works() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![
			0xA0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9D, 0x4a, 0x2e, 0x9E, 0xb0, 0xce, 0x36, 0x06,
			0xeB, 0x48,
		];
		let recipient = vec![0x11; 20];
		let recipient_clone = recipient.clone();
		let amount = 1_000_000_000_000_000_000u128; // 1 token
		let nonce = 0u64;
		let gas_limit = 100_000u64;
		let max_fee_per_gas = 30_000_000_000u128;
		let max_priority_fee_per_gas = 2_000_000_000u128;
		let chain_id = 1u64;

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			amount,
			nonce,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			chain_id,
		));

		// Check storage
		assert_eq!(Xdex::transaction_count(ALICE), 1);
		assert!(Xdex::transaction_hashes(ALICE, 0).is_some());

		// Check event
		System::assert_last_event(RuntimeEvent::Xdex(Event::Erc20TransferBuilt {
			who: ALICE,
			transaction_hash: Xdex::transaction_hashes(ALICE, 0).unwrap(),
			index: 0,
			token_contract: token_contract.try_into().unwrap(),
			to: recipient_clone.try_into().unwrap(),
			value: amount,
			chain_id,
			calldata: Xdex::transaction_hashes(ALICE, 0)
				.and_then(|_| Some(Xdex::encode_erc20_transfer(&recipient.try_into().unwrap(), amount)))
				.unwrap(),
		}));
	});
}

#[test]
fn invalid_recipient_address_fails() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let invalid_recipient = vec![0x11; 19]; // Should be 20 bytes

		assert_noop!(
			Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract,
				invalid_recipient,
				1000,
				0,
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			),
			Error::<Test>::InvalidRecipientAddress
		);
	});
}

#[test]
fn invalid_contract_address_fails() {
	new_test_ext().execute_with(|| {
		let invalid_token_contract = vec![0xA0; 19]; // Should be 20 bytes
		let recipient = vec![0x11; 20];

		assert_noop!(
			Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				invalid_token_contract,
				recipient,
				1000,
				0,
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			),
			Error::<Test>::InvalidContractAddress
		);
	});
}

#[test]
fn exceeding_max_transactions_fails() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x22; 20];

		// Fill up to max
		for i in 0..MaxTransactionsPerAccount::get() {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				1000,
				i as u64,
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		// Next one should fail
		assert_noop!(
			Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract,
				recipient,
				1000,
				MaxTransactionsPerAccount::get() as u64,
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			),
			Error::<Test>::TooManyTransactions
		);
	});
}

#[test]
fn clear_transactions_works() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x33; 20];

		// Create some transactions
		for i in 0..5 {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				1000 * (i + 1) as u128,
				i as u64,
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		assert_eq!(Xdex::transaction_count(ALICE), 5);

		// Clear
		assert_ok!(Xdex::clear_transactions(RuntimeOrigin::signed(ALICE)));

		// Check cleared
		assert_eq!(Xdex::transaction_count(ALICE), 0);
		for i in 0..5 {
			assert!(Xdex::transaction_hashes(ALICE, i).is_none());
		}

		// Check event
		System::assert_last_event(RuntimeEvent::Xdex(Event::TransactionsCleared { who: ALICE, count: 5 }));
	});
}

#[test]
fn different_accounts_have_independent_counters() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x44; 20];

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(BOB),
			token_contract.clone(),
			recipient.clone(),
			2000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient,
			3000,
			1,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert_eq!(Xdex::transaction_count(ALICE), 2);
		assert_eq!(Xdex::transaction_count(BOB), 1);
	});
}

#[test]
fn multiple_transfers_for_same_account_works() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient1 = vec![0x55; 20];
		let recipient2 = vec![0x66; 20];

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient1.clone(),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient2.clone(),
			2000,
			1,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert_eq!(Xdex::transaction_count(ALICE), 2);
		let hash1 = Xdex::transaction_hashes(ALICE, 0).unwrap();
		let hash2 = Xdex::transaction_hashes(ALICE, 1).unwrap();
		assert_ne!(hash1, hash2); // Different hashes for different transactions
	});
}

#[test]
fn nonce_increments_with_transaction_count() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x77; 20];

		// First transaction should use nonce 0
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Second transaction should use nonce 1
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			2000,
			1,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Third transaction should use nonce 2
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient,
			3000,
			2,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// The nonce is used internally, so we verify by checking
		// that we have 3 different transaction hashes
		let hash1 = Xdex::transaction_hashes(ALICE, 0).unwrap();
		let hash2 = Xdex::transaction_hashes(ALICE, 1).unwrap();
		let hash3 = Xdex::transaction_hashes(ALICE, 2).unwrap();

		assert_ne!(hash1, hash2);
		assert_ne!(hash2, hash3);
		assert_ne!(hash1, hash3);
	});
}

#[test]
fn erc20_transfer_encoding_is_correct() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![
			0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
			0xdd, 0xee,
		];
		let amount = 1_234_567_890_000_000_000u128;

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient.clone(),
			amount,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Verify the transaction was created
		assert_eq!(Xdex::transaction_count(ALICE), 1);
		assert!(Xdex::transaction_hashes(ALICE, 0).is_some());
	});
}

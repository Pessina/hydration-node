use super::mock::*;
use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn integration_test_full_transaction_lifecycle() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x88; 20];

		// Alice creates multiple transactions
		for i in 0..3 {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				1000 * (i + 1) as u128,
				i as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		// Bob creates transactions independently
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(BOB),
			token_contract.clone(),
			recipient.clone(),
			5000,
			0, // nonce
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Verify counts
		assert_eq!(Xdex::transaction_count(ALICE), 3);
		assert_eq!(Xdex::transaction_count(BOB), 1);

		// Alice clears her transactions
		assert_ok!(Xdex::clear_transactions(RuntimeOrigin::signed(ALICE)));
		assert_eq!(Xdex::transaction_count(ALICE), 0);

		// Bob's transactions remain
		assert_eq!(Xdex::transaction_count(BOB), 1);
		assert!(Xdex::transaction_hashes(BOB, 0).is_some());
	});
}

#[test]
fn integration_test_with_build_evm_tx_pallet() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x99; 20];
		let recipient_clone = recipient.clone();
		let amount = 1_500_000_000_000_000_000u128;

		// Build ERC20 transfer through xdex
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			amount,
			0, // nonce
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Verify the transaction was built and stored
		assert_eq!(Xdex::transaction_count(ALICE), 1);
		let tx_hash = Xdex::transaction_hashes(ALICE, 0).unwrap();

		// Verify the event was emitted
		System::assert_last_event(RuntimeEvent::Xdex(Event::Erc20TransferBuilt {
			who: ALICE,
			transaction_hash: tx_hash,
			index: 0,
			token_contract: token_contract.try_into().unwrap(),
			to: recipient_clone.try_into().unwrap(),
			value: amount,
			chain_id: 1,
			calldata: Xdex::encode_erc20_transfer(&recipient.try_into().unwrap(), amount),
		}));

		// Note: build_evm_tx pallet no longer emits events after the refactor
	});
}

#[test]
fn integration_test_gas_parameters() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0xaa; 20];

		// Create transaction with specific gas parameters
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient.clone(),
			2_000_000_000_000_000_000u128,
			0,              // nonce
			150_000,        // higher gas limit
			50_000_000_000, // higher max fee
			5_000_000_000,  // higher priority fee
			1
		));

		// Transaction should be created with specified gas values
		assert_eq!(Xdex::transaction_count(ALICE), 1);
		assert!(Xdex::transaction_hashes(ALICE, 0).is_some());
	});
}

#[test]
fn integration_test_maximum_transaction_limit() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0xbb; 20];

		// Fill up to maximum
		for i in 0..MaxTransactionsPerAccount::get() {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				100 * (i + 1) as u128,
				i as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		// Verify we're at max
		assert_eq!(Xdex::transaction_count(ALICE), MaxTransactionsPerAccount::get());

		// Next one should fail
		assert_noop!(
			Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				999999,
				MaxTransactionsPerAccount::get() as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			),
			Error::<Test>::TooManyTransactions
		);

		// Clear and try again
		assert_ok!(Xdex::clear_transactions(RuntimeOrigin::signed(ALICE)));
		assert_eq!(Xdex::transaction_count(ALICE), 0);

		// Now we can add transactions again
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient,
			111111,
			0, // nonce resets after clear
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));
		assert_eq!(Xdex::transaction_count(ALICE), 1);
	});
}

#[test]
fn integration_test_different_recipients_same_sender() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipients = vec![vec![0x11; 20], vec![0x22; 20], vec![0x33; 20]];

		// Send to different recipients
		for (i, recipient) in recipients.iter().enumerate() {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				token_contract.clone(),
				recipient.clone(),
				1000 * (i + 1) as u128,
				i as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		// Verify all transactions are stored with different hashes
		assert_eq!(Xdex::transaction_count(ALICE), 3);

		let hash1 = Xdex::transaction_hashes(ALICE, 0).unwrap();
		let hash2 = Xdex::transaction_hashes(ALICE, 1).unwrap();
		let hash3 = Xdex::transaction_hashes(ALICE, 2).unwrap();

		assert_ne!(hash1, hash2);
		assert_ne!(hash2, hash3);
		assert_ne!(hash1, hash3);
	});
}

#[test]
fn integration_test_different_chains() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0xcc; 20];

		// Create transactions for different chains
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1 // Ethereum mainnet
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract.clone(),
			recipient.clone(),
			2000,
			1,
			100_000,
			20_000_000_000,
			1_000_000_000,
			56 // BSC mainnet
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			token_contract,
			recipient,
			3000,
			2,
			100_000,
			10_000_000_000,
			500_000_000,
			137 // Polygon mainnet
		));

		// All transactions should be stored
		assert_eq!(Xdex::transaction_count(ALICE), 3);

		// Different chain IDs should produce different hashes
		let hash1 = Xdex::transaction_hashes(ALICE, 0).unwrap();
		let hash2 = Xdex::transaction_hashes(ALICE, 1).unwrap();
		let hash3 = Xdex::transaction_hashes(ALICE, 2).unwrap();

		assert_ne!(hash1, hash2);
		assert_ne!(hash2, hash3);
		assert_ne!(hash1, hash3);
	});
}

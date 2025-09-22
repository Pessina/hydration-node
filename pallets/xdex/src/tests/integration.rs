use super::mock::*;
use crate::Event;
use frame_support::assert_ok;
use sp_core::H160;

#[test]
fn integration_test_full_transaction_lifecycle() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x88; 20];

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
	});
}

#[test]
fn integration_test_with_build_evm_tx_pallet() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0x99; 20];
		let _recipient_clone = recipient.clone();
		let amount = 1_500_000_000_000_000_000u128;

		// Build ERC20 transfer through xdex
		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			amount,
			0, // nonce
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Verify the event was emitted with RLP
		let token_arr: [u8; 20] = token_contract.clone().try_into().unwrap();
		let to = H160::from(token_arr);
		let expected_rlp = pallet_build_evm_tx::Pallet::<Test>::build_evm_tx(
			RuntimeOrigin::signed(ALICE),
			Some(to),
			0,
			Xdex::encode_erc20_transfer(&recipient.clone().try_into().unwrap(), amount),
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			Vec::new(),
			1,
		)
		.expect("should build rlp");
		System::assert_last_event(RuntimeEvent::Xdex(Event::Erc20TransferBuilt {
			who: ALICE,
			rlp: expected_rlp,
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
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			2_000_000_000_000_000_000u128,
			0,              // nonce
			150_000,        // higher gas limit
			50_000_000_000, // higher max fee
			5_000_000_000,  // higher priority fee
			1
		));

		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
	});
}

#[test]
fn integration_test_maximum_transaction_limit() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![0xA0; 20];
		let recipient = vec![0xbb; 20];

		// Fill up to maximum
		for i in 0..3 {
			assert_ok!(Xdex::build_erc20_transfer(
				RuntimeOrigin::signed(ALICE),
				H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
				H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
				100 * (i + 1) as u128,
				i as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
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
				H160::from(token_contract.clone().try_into().unwrap()),
				H160::from(recipient.clone().try_into().unwrap()),
				1000 * (i + 1) as u128,
				i as u64, // nonce
				100_000,
				30_000_000_000,
				2_000_000_000,
				1
			));
		}

		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
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
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			1000,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1 // Ethereum mainnet
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			2000,
			1,
			100_000,
			20_000_000_000,
			1_000_000_000,
			56 // BSC mainnet
		));

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient).unwrap()),
			3000,
			2,
			100_000,
			10_000_000_000,
			500_000_000,
			137 // Polygon mainnet
		));

		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
	});
}

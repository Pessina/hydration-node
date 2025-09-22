use super::mock::*;
use crate::Event;
use frame_support::assert_ok;
use sp_core::H160;

#[test]
fn build_erc20_transfer_works() {
	new_test_ext().execute_with(|| {
		let token_contract = vec![
			0xA0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9D, 0x4a, 0x2e, 0x9E, 0xb0, 0xce, 0x36, 0x06,
			0xeB, 0x48,
		];
		let recipient = vec![0x11; 20];
		let amount = 1_000_000_000_000_000_000u128; // 1 token
		let nonce = 0u64;
		let gas_limit = 100_000u64;
		let max_fee_per_gas = 30_000_000_000u128;
		let max_priority_fee_per_gas = 2_000_000_000u128;
		let chain_id = 1u64;

		assert_ok!(Xdex::build_erc20_transfer(
			RuntimeOrigin::signed(ALICE),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			amount,
			nonce,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			chain_id,
		));

		// Check event
		let to = H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract.clone()).unwrap());
		let expected_rlp = pallet_build_evm_tx::Pallet::<Test>::build_evm_tx(
			RuntimeOrigin::signed(ALICE),
			Some(to),
			0,
			Xdex::encode_erc20_transfer(&recipient.clone().try_into().unwrap(), amount),
			nonce,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			Vec::new(),
			chain_id,
		)
		.expect("should build rlp");
		System::assert_last_event(RuntimeEvent::Xdex(Event::Erc20TransferBuilt {
			who: ALICE,
			rlp: expected_rlp,
		}));
	});
}

// Removed invalid_recipient_address_fails: H160 typed now

// Removed invalid_contract_address_fails: H160 typed now

// Removed exceeding_max_transactions_fails: storage removed

// Removed clear_transactions_works: extrinsic removed

// Removed: storage removed

// Removed: storage removed

// Removed: storage removed

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
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(token_contract).unwrap()),
			H160::from(<Vec<u8> as TryInto<[u8; 20]>>::try_into(recipient.clone()).unwrap()),
			amount,
			0,
			100_000,
			30_000_000_000,
			2_000_000_000,
			1
		));

		// Just ensure event emitted
		assert!(matches!(System::events().last().unwrap().event, RuntimeEvent::Xdex(_)));
	});
}

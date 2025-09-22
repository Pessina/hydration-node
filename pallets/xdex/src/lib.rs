#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::weights::Weight;
use frame_system::pallet_prelude::*;
use sp_core::H160;
// use sp_runtime::traits::Hash;
use sp_std::vec::Vec;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_std::vec; // bring vec! macro into scope for no_std

	// ERC20 transfer function selector: transfer(address,uint256)

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_build_evm_tx::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	// No storage required

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// EIP-1559 unsigned transaction bytes built
		Erc20TransferBuilt { who: T::AccountId, rlp: Vec<u8> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to build EVM transaction
		EvmTransactionBuildFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Build an ERC20 transfer transaction and emit unsigned EIP-1559 RLP
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn build_erc20_transfer(
			origin: OriginFor<T>,
			token_contract: H160,
			recipient: H160,
			amount: u128,
			nonce: u64,
			gas_limit: u64,
			max_fee_per_gas: u128,
			max_priority_fee_per_gas: u128,
			chain_id: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let recipient_array: [u8; 20] = recipient.0;

			// Build ERC20 transfer ABI-encoded data (encoded fully on-chain)
			let data = Self::encode_erc20_transfer(&recipient_array, amount);

			// Build the EVM transaction using build_evm_tx pallet
			// The 'to' address is the token contract, not the recipient
			let to_h160 = token_contract;
			let rlp_data = pallet_build_evm_tx::Pallet::<T>::build_evm_tx(
				origin,
				Some(to_h160),
				0, // No ETH value for ERC20 transfer
				data,
				nonce,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				Vec::new(),
				chain_id,
			)
			.map_err(|_| Error::<T>::EvmTransactionBuildFailed)?;

			// Emit event with only who and rlp
			Self::deposit_event(Event::Erc20TransferBuilt { who, rlp: rlp_data });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Encode ERC20 transfer function call using ethabi
		/// transfer(address,uint256)
		pub(crate) fn encode_erc20_transfer(recipient: &[u8; 20], amount: u128) -> Vec<u8> {
			use ethabi::{Function, Param, ParamType, StateMutability, Token};
			use sp_core::{H160 as Address, U256};

			let function = Function {
				name: "transfer".into(),
				inputs: vec![
					Param {
						name: "to".into(),
						kind: ParamType::Address,
						internal_type: None,
					},
					Param {
						name: "amount".into(),
						kind: ParamType::Uint(256),
						internal_type: None,
					},
				],
				outputs: vec![Param {
					name: "".into(),
					kind: ParamType::Bool,
					internal_type: None,
				}],
				constant: None,
				state_mutability: StateMutability::NonPayable,
			};

			let to = Address::from(*recipient);
			let value = U256::from(amount);
			let tokens = vec![Token::Address(to), Token::Uint(value)];
			function.encode_input(&tokens).unwrap_or_default()
		}
	}
}

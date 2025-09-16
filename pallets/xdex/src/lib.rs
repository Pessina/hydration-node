#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::weights::Weight;
use frame_system::pallet_prelude::*;
use sp_core::H256;
use sp_runtime::traits::Hash;
use sp_std::vec::Vec;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// ERC20 transfer function selector: transfer(address,uint256)
	const ERC20_TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_build_evm_tx::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Maximum number of transaction hashes to store per account
		#[pallet::constant]
		type MaxTransactionsPerAccount: Get<u32>;

		/// The ERC20 token contract address (mocked for demo)
		#[pallet::constant]
		type TokenContractAddress: Get<[u8; 20]>;

		/// Default chain ID for EVM transactions
		#[pallet::constant]
		type DefaultChainId: Get<u64>;
	}

	/// Number of transactions per account
	#[pallet::storage]
	#[pallet::getter(fn transaction_count)]
	pub type TransactionCount<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Transaction hashes by account and index
	#[pallet::storage]
	#[pallet::getter(fn transaction_hashes)]
	pub type TransactionHashes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		u32,
		H256,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// ERC20 transfer transaction built and stored
		Erc20TransferBuilt {
			/// Account that initiated the transaction
			who: T::AccountId,
			/// Hash of the RLP-encoded transaction
			transaction_hash: H256,
			/// Index of this transaction for the account
			index: u32,
			/// Token contract address
			token_contract: [u8; 20],
			/// Recipient address for the transfer
			to: [u8; 20],
			/// Amount to transfer
			value: u128,
			/// Chain ID for the transaction
			chain_id: u64,
		},
		/// All transactions cleared for an account
		TransactionsCleared {
			/// Account that cleared transactions
			who: T::AccountId,
			/// Number of transactions cleared
			count: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Maximum number of transactions per account reached
		TooManyTransactions,
		/// Failed to build EVM transaction
		EvmTransactionBuildFailed,
		/// Invalid recipient address
		InvalidRecipientAddress,
		/// Invalid contract address
		InvalidContractAddress,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Build an ERC20 transfer transaction for cross-chain DEX operations
		///
		/// This builds an EVM transaction for transferring tokens
		/// using the standard ERC20 transfer function with dynamic parameters
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn build_erc20_transfer(
			origin: OriginFor<T>,
			token_contract_address: Vec<u8>,
			recipient: Vec<u8>,
			amount: u128,
			nonce: u64,
			gas_limit: u64,
			max_fee_per_gas: u128,
			max_priority_fee_per_gas: u128,
			chain_id: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Validate token contract address is 20 bytes
			ensure!(token_contract_address.len() == 20, Error::<T>::InvalidContractAddress);
			let token_contract_array: [u8; 20] = token_contract_address
				.clone()
				.try_into()
				.map_err(|_| Error::<T>::InvalidContractAddress)?;

			// Validate recipient address is 20 bytes
			ensure!(recipient.len() == 20, Error::<T>::InvalidRecipientAddress);
			let recipient_array: [u8; 20] = recipient
				.try_into()
				.map_err(|_| Error::<T>::InvalidRecipientAddress)?;

			// Get current transaction count for this account
			let current_count = TransactionCount::<T>::get(&who);
			ensure!(current_count < T::MaxTransactionsPerAccount::get(), Error::<T>::TooManyTransactions);

			// Build ERC20 transfer ABI-encoded data
			let data = Self::encode_erc20_transfer(&recipient_array, amount);

			// Build the EVM transaction using build_evm_tx pallet
			// The 'to' address is the token contract, not the recipient
			let rlp_data = pallet_build_evm_tx::Pallet::<T>::build_evm_tx(
				Some(who.clone()), // Pass who to emit event
				Some(token_contract_address),
				0, // No ETH value for ERC20 transfer
				data,
				nonce,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				chain_id,
			)
			.map_err(|_| Error::<T>::EvmTransactionBuildFailed)?;

			// Hash the RLP-encoded transaction data using Keccak256
			let transaction_hash = sp_runtime::traits::BlakeTwo256::hash(&rlp_data);

			// Store the hash on-chain
			TransactionHashes::<T>::insert(&who, current_count, transaction_hash);

			// Increment the transaction count
			TransactionCount::<T>::mutate(&who, |count| *count = count.saturating_add(1));

			// Emit event
			Self::deposit_event(Event::Erc20TransferBuilt {
				who,
				transaction_hash,
				index: current_count,
				token_contract: token_contract_array,
				to: recipient_array,
				value: amount,
				chain_id,
			});

			Ok(())
		}

		/// Clear all stored transactions for an account (for testing/demo purposes)
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn clear_transactions(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Clear all transaction hashes for this account
			let count = TransactionCount::<T>::get(&who);
			for i in 0..count {
				TransactionHashes::<T>::remove(&who, i);
			}

			// Reset transaction count
			TransactionCount::<T>::insert(&who, 0);

			// Emit event
			Self::deposit_event(Event::TransactionsCleared {
				who,
				count,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Encode ERC20 transfer function call
		/// transfer(address,uint256)
		fn encode_erc20_transfer(recipient: &[u8; 20], amount: u128) -> Vec<u8> {
			let mut data = Vec::with_capacity(68);

			// Function selector for transfer(address,uint256)
			data.extend_from_slice(&ERC20_TRANSFER_SELECTOR);

			// Recipient address (padded to 32 bytes)
			data.extend_from_slice(&[0u8; 12]); // Padding
			data.extend_from_slice(recipient);

			// Amount (uint256, 32 bytes)
			let mut amount_bytes = [0u8; 32];
			amount_bytes[16..].copy_from_slice(&amount.to_be_bytes());
			data.extend_from_slice(&amount_bytes);

			data
		}
	}
}
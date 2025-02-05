// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// Runtime-generated enums
#![allow(clippy::large_enum_variant)]
// Runtime-generated DecodeLimit::decode_all_With_depth_limit
#![allow(clippy::unnecessary_mut_passed)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod exchange;

#[cfg(feature = "runtime-benchmarks")]
pub mod benches;
#[cfg(feature = "bridge-kovan")]
pub mod kovan;
#[cfg(feature = "bridge-rialto")]
pub mod rialto;

#[cfg(feature = "runtime-benchmarks")]
pub use benches as bridge;
#[cfg(all(feature = "bridge-kovan", not(feature = "runtime-benchmarks")))]
pub use kovan as bridge;
#[cfg(all(feature = "bridge-rialto", not(feature = "runtime-benchmarks")))]
pub use rialto as bridge;

use codec::{Decode, Encode};
use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, IdentifyAccount, IdentityLookup, NumberFor, OpaqueKeys, Saturating, Verify,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{Currency, ExistenceRequirement, Imbalance, KeyOwnerProofSystem, Randomness},
	weights::{IdentityFee, RuntimeDbWeight, Weight},
	StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_bridge_currency_exchange::Call as BridgeCurrencyExchangeCall;
pub use pallet_bridge_eth_poa::Call as BridgeEthPoACall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bridge-node"),
	impl_name: create_runtime_str!("bridge-node"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 2_000_000_000_000;
	pub const ExtrinsicBaseWeight: Weight = 10_000_000;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
	pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight {
		read: 60_000_000, // ~0.06 ms = ~60 µs
		write: 200_000_000, // ~0.2 ms = 200 µs
	};
}

impl frame_system::Trait for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = ();
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = DbWeight;
	/// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = ();
	/// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	/// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
	/// idependent of the logic of that extrinsics. (Roughly max block weight - average on
	/// initialize cost).
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type ModuleToIndex = ModuleToIndex;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_aura::Trait for Runtime {
	type AuthorityId = AuraId;
}

impl pallet_bridge_eth_poa::Trait for Runtime {
	type AuraConfiguration = bridge::BridgeAuraConfiguration;
	type FinalityVotesCachingInterval = bridge::FinalityVotesCachingInterval;
	type ValidatorsConfiguration = bridge::BridgeValidatorsConfiguration;
	type PruningStrategy = bridge::PruningStrategy;
	type OnHeadersSubmitted = ();
}

impl pallet_bridge_currency_exchange::Trait for Runtime {
	type OnTransactionSubmitted = ();
	type PeerBlockchain = exchange::EthBlockchain;
	type PeerMaybeLockFundsTransaction = exchange::EthTransaction;
	type RecipientsMap = sp_currency_exchange::IdentityRecipients<AccountId>;
	type Amount = Balance;
	type CurrencyConverter = sp_currency_exchange::IdentityCurrencyConverter<Balance>;
	type DepositInto = DepositInto;
}

pub struct DepositInto;

impl sp_currency_exchange::DepositInto for DepositInto {
	type Recipient = AccountId;
	type Amount = Balance;

	fn deposit_into(recipient: Self::Recipient, amount: Self::Amount) -> sp_currency_exchange::Result<()> {
		// let balances module make all checks for us (it won't allow depositing lower than existential
		// deposit, balance overflow, ...)
		let deposited = <pallet_balances::Module<Runtime> as Currency<AccountId>>::deposit_creating(&recipient, amount);

		// I'm dropping deposited here explicitly to illustrate the fact that it'll update `TotalIssuance`
		// on drop
		let deposited_amount = deposited.peek();
		drop(deposited);

		// we have 3 cases here:
		// - deposited == amount: success
		// - deposited == 0: deposit has failed and no changes to storage were made
		// - deposited != 0: (should never happen in practice) deposit has been partially completed
		match deposited_amount {
			_ if deposited_amount == amount => {
				frame_support::debug::trace!(
					target: "runtime",
					"Deposited {} to {:?}",
					amount,
					recipient,
				);

				Ok(())
			}
			_ if deposited_amount == 0 => {
				frame_support::debug::error!(
					target: "runtime",
					"Deposit of {} to {:?} has failed",
					amount,
					recipient,
				);

				Err(sp_currency_exchange::Error::DepositFailed)
			}
			_ => {
				frame_support::debug::error!(
					target: "runtime",
					"Deposit of {} to {:?} has partially competed. {} has been deposited",
					amount,
					recipient,
					deposited_amount,
				);

				// we can't return DepositFailed error here, because storage changes were made
				Err(sp_currency_exchange::Error::DepositPartiallyFailed)
			}
		}
	}
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = ();

	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;

	type HandleEquivocation = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

parameter_types! {
	pub const TransactionBaseFee: Balance = 0;
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = pallet_balances::Module<Runtime>;
	type OnTransactionPayment = ();
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const Period: BlockNumber = 4;
	pub const Offset: BlockNumber = 0;
}

impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = ShiftSessionManager;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = ();
}

pub struct ShiftSessionManager;

impl ShiftSessionManager {
	/// Select validators for session.
	fn select_validators(
		session_index: sp_staking::SessionIndex,
		available_validators: &[AccountId],
	) -> Vec<AccountId> {
		let available_validators_count = available_validators.len();
		let count = sp_std::cmp::max(1, 2 * available_validators_count / 3);
		let offset = session_index as usize % available_validators_count;
		let end = offset + count;
		let session_validators = match end.overflowing_sub(available_validators_count) {
			(wrapped_end, false) if wrapped_end != 0 => available_validators[offset..]
				.iter()
				.chain(available_validators[..wrapped_end].iter())
				.cloned()
				.collect(),
			_ => available_validators[offset..end].to_vec(),
		};

		session_validators
	}
}

impl pallet_session::SessionManager<AccountId> for ShiftSessionManager {
	fn end_session(_: sp_staking::SessionIndex) {}
	fn start_session(_: sp_staking::SessionIndex) {}
	fn new_session(session_index: sp_staking::SessionIndex) -> Option<Vec<AccountId>> {
		// can't access genesis config here :/
		if session_index == 0 || session_index == 1 {
			return None;
		}

		// the idea that on first call (i.e. when session 1 ends) we're reading current
		// set of validators from session module (they are initial validators) and save
		// in our 'local storage'.
		// then for every session we select (deterministically) 2/3 of these initial
		// validators to serve validators of new session
		let available_validators = sp_io::storage::get(b":available_validators")
			.and_then(|validators| Decode::decode(&mut &validators[..]).ok())
			.unwrap_or_else(|| {
				let validators = <pallet_session::Module<Runtime>>::validators();
				sp_io::storage::set(b":available_validators", &validators.encode());
				validators
			});

		Some(Self::select_validators(session_index, &available_validators))
	}
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Aura: pallet_aura::{Module, Config<T>, Inherent(Timestamp)},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
		BridgeEthPoA: pallet_bridge_eth_poa::{Module, Call, Config, Storage, ValidateUnsigned},
		BridgeCurrencyExchange: pallet_bridge_currency_exchange::{Module, Call},
	}
);

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllModules>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl sp_bridge_eth_poa::EthereumHeadersApi<Block> for Runtime {
		fn best_block() -> (u64, sp_bridge_eth_poa::H256) {
			let best_block = BridgeEthPoA::best_block();
			(best_block.number, best_block.hash)
		}

		fn finalized_block() -> (u64, sp_bridge_eth_poa::H256) {
			let finalized_block = BridgeEthPoA::finalized_block();
			(finalized_block.number, finalized_block.hash)
		}

		fn is_import_requires_receipts(header: sp_bridge_eth_poa::Header) -> bool {
			BridgeEthPoA::is_import_requires_receipts(header)
		}

		fn is_known_block(hash: sp_bridge_eth_poa::H256) -> bool {
			BridgeEthPoA::is_known_block(hash)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			pallet: Vec<u8>,
			benchmark: Vec<u8>,
			lowest_range_values: Vec<u32>,
			highest_range_values: Vec<u32>,
			steps: Vec<u32>,
			repeat: u32,
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark};
			let mut batches = Vec::<BenchmarkBatch>::new();
			let whitelist: Vec<Vec<u8>> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec(),
				// Caller 0 Account
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da946c154ffd9992e395af90b5b13cc6f295c77033fce8a9045824a6690bbf99c6db269502f0a8d1d2a008542d5690a0749").to_vec(),
			];
			let params = (&pallet, &benchmark, &lowest_range_values, &highest_range_values, &steps, repeat, &whitelist);

			use pallet_bridge_currency_exchange::benchmarking::{
				Module as BridgeCurrencyExchangeBench,
				Trait as BridgeCurrencyExchangeTrait,
				ProofParams as BridgeCurrencyExchangeProofParams,
			};

			impl BridgeCurrencyExchangeTrait for Runtime {
				fn make_proof(
					proof_params: BridgeCurrencyExchangeProofParams<AccountId>,
				) -> crate::exchange::EthereumTransactionInclusionProof {
					use sp_currency_exchange::DepositInto;

					if proof_params.recipient_exists {
						<Runtime as pallet_bridge_currency_exchange::Trait>::DepositInto::deposit_into(
							proof_params.recipient.clone(),
							ExistentialDeposit::get(),
						).unwrap();
					}

					let transaction = crate::exchange::prepare_ethereum_transaction(
						&proof_params.recipient,
						|tx| {
							// our runtime only supports transactions where data is exactly 32 bytes long
							// (receiver key)
							// => we are ignoring `transaction_size_factor` here
							tx.value = (ExistentialDeposit::get() * 10).into();
						},
					);
					let transactions = sp_std::iter::repeat(transaction.clone())
						.take(1 + proof_params.proof_size_factor as usize)
						.collect::<Vec<_>>();
					let block_hash = crate::exchange::prepare_environment_for_claim::<Runtime>(&transactions);
					crate::exchange::EthereumTransactionInclusionProof {
						block: block_hash,
						index: 0,
						proof: transactions,
					}
				}
			}

			add_benchmark!(params, batches, b"bridge-eth-poa", BridgeEthPoA);
			add_benchmark!(params, batches, b"bridge-currency-exchange", BridgeCurrencyExchangeBench::<Runtime>);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_currency_exchange::DepositInto;

	#[test]
	fn shift_session_manager_works() {
		let acc1 = AccountId::from([1u8; 32]);
		let acc2 = AccountId::from([2u8; 32]);
		let acc3 = AccountId::from([3u8; 32]);
		let acc4 = AccountId::from([4u8; 32]);
		let acc5 = AccountId::from([5u8; 32]);
		let all_accs = vec![acc1.clone(), acc2.clone(), acc3.clone(), acc4.clone(), acc5.clone()];

		// at least 1 validator is selected
		assert_eq!(
			ShiftSessionManager::select_validators(0, &[acc1.clone()]),
			vec![acc1.clone()],
		);

		// at session#0, shift is also 0
		assert_eq!(
			ShiftSessionManager::select_validators(0, &all_accs),
			vec![acc1.clone(), acc2.clone(), acc3.clone()],
		);

		// at session#1, shift is also 1
		assert_eq!(
			ShiftSessionManager::select_validators(1, &all_accs),
			vec![acc2.clone(), acc3.clone(), acc4.clone()],
		);

		// at session#3, we're wrapping
		assert_eq!(
			ShiftSessionManager::select_validators(3, &all_accs),
			vec![acc4, acc5, acc1.clone()],
		);

		// at session#5, we're starting from the beginning again
		assert_eq!(
			ShiftSessionManager::select_validators(5, &all_accs),
			vec![acc1, acc2, acc3],
		);
	}

	fn run_deposit_into_test(test: impl Fn(AccountId) -> Balance) {
		let mut ext: sp_io::TestExternalities = SystemConfig::default().build_storage::<Runtime>().unwrap().into();
		ext.execute_with(|| {
			// initially issuance is zero
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::total_issuance(),
				0,
			);

			// create account
			let account: AccountId = [1u8; 32].into();
			let initial_amount = ExistentialDeposit::get();
			let deposited =
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::deposit_creating(&account, initial_amount);
			drop(deposited);
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::total_issuance(),
				initial_amount,
			);
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::free_balance(&account),
				initial_amount,
			);

			// run test
			let total_issuance_change = test(account);

			// check that total issuance has changed by `run_deposit_into_test`
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::total_issuance(),
				initial_amount + total_issuance_change,
			);
		});
	}

	#[test]
	fn deposit_into_existing_account_works() {
		run_deposit_into_test(|existing_account| {
			let initial_amount =
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::free_balance(&existing_account);
			let additional_amount = 10_000;
			<Runtime as pallet_bridge_currency_exchange::Trait>::DepositInto::deposit_into(
				existing_account.clone(),
				additional_amount,
			)
			.unwrap();
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::free_balance(&existing_account),
				initial_amount + additional_amount,
			);
			additional_amount
		});
	}

	#[test]
	fn deposit_into_new_account_works() {
		run_deposit_into_test(|_| {
			let initial_amount = 0;
			let additional_amount = ExistentialDeposit::get() + 10_000;
			let new_account: AccountId = [42u8; 32].into();
			<Runtime as pallet_bridge_currency_exchange::Trait>::DepositInto::deposit_into(
				new_account.clone(),
				additional_amount,
			)
			.unwrap();
			assert_eq!(
				<pallet_balances::Module<Runtime> as Currency<AccountId>>::free_balance(&new_account),
				initial_amount + additional_amount,
			);
			additional_amount
		});
	}
}

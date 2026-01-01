#![cfg_attr(not(feature = "std"), no_std)]

//! Pallet for Swarm Media Orchestration
//!
//! This pallet provides runtime storage and extrinsics for media production:
//! - Storage for media production state
//! - Extrinsics for media operations
//! - Runtime API for RPC access
//!
//! Types are defined inline to be SCALE-codec compatible (no external crate dependency for storage).

pub use pallet::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// ============================================================================
// Runtime-friendly types (SCALE-codec compatible)
// ============================================================================

/// Contributor role within the media system
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum ContributorRole {
	Founder,
	Educator,
	Narrator,
	Presenter,
	CommunityHost,
	GuestExpert,
	Producer,
}

impl Default for ContributorRole {
	fn default() -> Self {
		ContributorRole::Presenter
	}
}

/// Contributor status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum ContributorStatus {
	Active,
	Paused,
	Revoked,
	Retired,
}

impl Default for ContributorStatus {
	fn default() -> Self {
		ContributorStatus::Active
	}
}

/// Maximum length for string fields in storage
pub type MaxStringLength = ConstU32<256>;

/// Runtime-friendly contributor representation
/// Uses bounded vectors for SCALE compatibility
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(S))]
pub struct RuntimeContributor<S: Get<u32>> {
	/// Unique identifier
	pub id: BoundedVec<u8, S>,
	/// Display name
	pub name: BoundedVec<u8, S>,
	/// Role in production
	pub role: ContributorRole,
	/// Current status
	pub status: ContributorStatus,
	/// Email (bounded)
	pub email: BoundedVec<u8, S>,
	/// Optional wallet address (hex encoded)
	pub wallet_address: Option<BoundedVec<u8, S>>,
	/// Whether currently active
	pub is_active: bool,
}

/// Job status enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum JobStatus {
	Queued,
	Processing,
	Completed,
	Failed,
	Cancelled,
}

impl Default for JobStatus {
	fn default() -> Self {
		JobStatus::Queued
	}
}

/// Job priority level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum JobPriority {
	Low,
	Normal,
	High,
	Urgent,
}

impl Default for JobPriority {
	fn default() -> Self {
		JobPriority::Normal
	}
}

/// Runtime-friendly job status record
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(S))]
pub struct RuntimeJobStatusRecord<S: Get<u32>> {
	/// Job identifier
	pub job_id: BoundedVec<u8, S>,
	/// Current status
	pub status: JobStatus,
	/// Priority level
	pub priority: JobPriority,
	/// Asset type being created
	pub asset_type: BoundedVec<u8, S>,
	/// Target platform
	pub target: BoundedVec<u8, S>,
	/// Progress percentage (0-100)
	pub progress_percentage: u8,
	/// Block number when created
	pub created_at_block: u32,
	/// Block number of last update
	pub last_update_block: u32,
	/// Optional error message
	pub error_message: Option<BoundedVec<u8, S>>,
}

/// Runtime-friendly media production status
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Default)]
pub struct RuntimeMediaProductionStatus {
	/// Total recordings scheduled
	pub recordings_scheduled: u32,
	/// Completed recordings
	pub recordings_completed: u32,
	/// On-time percentage (basis points, 0-10000 = 0-100%)
	pub on_time_percentage_bps: u16,
	/// Total assets created
	pub total_assets_created: u32,
	/// Assets ready for publishing
	pub assets_ready: u32,
	/// Assets published
	pub assets_published: u32,
	/// Active contributor count
	pub active_contributors: u32,
	/// Total production hours (scaled by 100)
	pub total_production_hours_scaled: u32,
	/// Last recording block
	pub last_recording_block: Option<u32>,
	/// Next scheduled recording block
	pub next_recording_block: Option<u32>,
}

// ============================================================================
// Type aliases for bounded vectors
// ============================================================================

/// Contributor type alias
pub type Contributor = RuntimeContributor<MaxStringLength>;

/// Job status record type alias
pub type JobStatusRecord = RuntimeJobStatusRecord<MaxStringLength>;

/// Media production status type alias
pub type MediaProductionStatus = RuntimeMediaProductionStatus;

// ============================================================================
// Runtime API declaration
// ============================================================================

sp_api::decl_runtime_apis! {
	/// Runtime API for accessing Swarm Media data
	pub trait SwarmMediaRuntimeApi<AccountId> where
		AccountId: codec::Codec,
	{
		/// Get current media production status
		fn get_media_status() -> MediaProductionStatus;
		/// Get contributor by account ID
		fn get_contributor(account: AccountId) -> Option<Contributor>;
		/// Get job status by job ID
		fn get_job(job_id: Vec<u8>) -> Option<JobStatusRecord>;
		/// List all jobs
		fn list_jobs() -> Vec<(Vec<u8>, JobStatusRecord)>;
	}
}

// ============================================================================
// Pallet definition
// ============================================================================

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	/// Storage for media production status
	#[pallet::storage]
	#[pallet::getter(fn media_status)]
	pub type MediaStatusStorage<T: Config> = StorageValue<_, MediaProductionStatus, ValueQuery>;

	/// Storage for active contributors
	#[pallet::storage]
	#[pallet::getter(fn contributors)]
	pub type Contributors<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Contributor, OptionQuery>;

	/// Storage for job queue
	#[pallet::storage]
	#[pallet::getter(fn jobs)]
	pub type Jobs<T: Config> = StorageMap<_, Blake2_128Concat, BoundedVec<u8, MaxStringLength>, JobStatusRecord, OptionQuery>;

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Media production status updated
		MediaStatusUpdated {
			recordings_scheduled: u32,
			recordings_completed: u32,
			on_time_percentage_bps: u16,
		},
		/// New contributor registered
		ContributorRegistered {
			account: T::AccountId,
			name: Vec<u8>,
			role: ContributorRole,
		},
		/// Media repurposing job submitted
		RepurposingJobSubmitted {
			job_id: Vec<u8>,
			source_id: Vec<u8>,
			target: Vec<u8>,
		},
		/// Job status updated
		JobStatusUpdated {
			job_id: Vec<u8>,
			status: JobStatus,
			progress_percentage: u8,
		},
	}

	/// Errors emitted by this pallet
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid job ID
		InvalidJobId,
		/// Job not found
		JobNotFound,
		/// Contributor not found
		ContributorNotFound,
		/// Invalid parameters
		InvalidParameters,
		/// Unauthorized access
		Unauthorized,
		/// String too long
		StringTooLong,
	}

	/// Pallet implementation
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update media production status
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn update_media_status(
			origin: OriginFor<T>,
			recordings_scheduled: u32,
			recordings_completed: u32,
			on_time_percentage_bps: u16,
			total_assets_created: u32,
			assets_ready: u32,
			assets_published: u32,
			active_contributors: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			let current_block: u32 = frame_system::Pallet::<T>::block_number()
				.try_into()
				.unwrap_or(0);

			let status = MediaProductionStatus {
				recordings_scheduled,
				recordings_completed,
				on_time_percentage_bps,
				total_assets_created,
				assets_ready,
				assets_published,
				active_contributors,
				total_production_hours_scaled: 0,
				last_recording_block: Some(current_block),
				next_recording_block: None,
			};

			<MediaStatusStorage<T>>::put(status);

			Self::deposit_event(Event::MediaStatusUpdated {
				recordings_scheduled,
				recordings_completed,
				on_time_percentage_bps,
			});

			Ok(())
		}

		/// Register a new contributor
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn register_contributor(
			origin: OriginFor<T>,
			account: T::AccountId,
			name: Vec<u8>,
			role: ContributorRole,
			email: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let id: BoundedVec<u8, MaxStringLength> = account.encode()
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let bounded_name: BoundedVec<u8, MaxStringLength> = name.clone()
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let bounded_email: BoundedVec<u8, MaxStringLength> = email
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let contributor = Contributor {
				id,
				name: bounded_name.clone(),
				role,
				status: ContributorStatus::Active,
				email: bounded_email,
				wallet_address: None,
				is_active: true,
			};

			<Contributors<T>>::insert(&account, contributor);

			Self::deposit_event(Event::ContributorRegistered {
				account,
				name,
				role,
			});

			Ok(())
		}

		/// Submit a repurposing job
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(50_000, 0))]
		pub fn submit_repurposing_job(
			origin: OriginFor<T>,
			source_id: Vec<u8>,
			asset_type: Vec<u8>,
			target: Vec<u8>,
			priority: JobPriority,
			title: Vec<u8>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Create job ID from hash
			let job_id_raw = sp_io::hashing::blake2_256(&[&source_id[..], &title[..]].concat()).to_vec();
			let job_id: BoundedVec<u8, MaxStringLength> = job_id_raw.clone()
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let bounded_asset_type: BoundedVec<u8, MaxStringLength> = asset_type
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let bounded_target: BoundedVec<u8, MaxStringLength> = target.clone()
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let current_block: u32 = frame_system::Pallet::<T>::block_number()
				.try_into()
				.unwrap_or(0);

			let job_record = JobStatusRecord {
				job_id: job_id.clone(),
				status: JobStatus::Queued,
				priority,
				asset_type: bounded_asset_type,
				target: bounded_target,
				progress_percentage: 0,
				created_at_block: current_block,
				last_update_block: current_block,
				error_message: None,
			};

			<Jobs<T>>::insert(&job_id, job_record);

			Self::deposit_event(Event::RepurposingJobSubmitted {
				job_id: job_id_raw,
				source_id,
				target,
			});

			Ok(())
		}

		/// Update job status
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn update_job_status(
			origin: OriginFor<T>,
			job_id: Vec<u8>,
			status: JobStatus,
			progress_percentage: u8,
		) -> DispatchResult {
			ensure_root(origin)?;

			let bounded_job_id: BoundedVec<u8, MaxStringLength> = job_id.clone()
				.try_into()
				.map_err(|_| Error::<T>::StringTooLong)?;

			let current_block: u32 = frame_system::Pallet::<T>::block_number()
				.try_into()
				.unwrap_or(0);

			<Jobs<T>>::try_mutate(&bounded_job_id, |job_opt| {
				let job = job_opt.as_mut().ok_or(Error::<T>::JobNotFound)?;

				job.status = status;
				job.progress_percentage = progress_percentage;
				job.last_update_block = current_block;

				Self::deposit_event(Event::JobStatusUpdated {
					job_id,
					status,
					progress_percentage,
				});

				Ok(())
			})
		}
	}

	/// Runtime API implementation
	impl<T: Config> Pallet<T> {
		/// Get media status via runtime API
		pub fn get_media_status() -> MediaProductionStatus {
			<MediaStatusStorage<T>>::get()
		}

		/// Get contributor info
		pub fn get_contributor(account: &T::AccountId) -> Option<Contributor> {
			<Contributors<T>>::get(account)
		}

		/// Get job status
		pub fn get_job(job_id: &Vec<u8>) -> Option<JobStatusRecord> {
			let bounded_job_id: BoundedVec<u8, MaxStringLength> = job_id.clone()
				.try_into()
				.ok()?;
			<Jobs<T>>::get(&bounded_job_id)
		}

		/// List all jobs
		pub fn list_jobs() -> Vec<(BoundedVec<u8, MaxStringLength>, JobStatusRecord)> {
			<Jobs<T>>::iter().collect()
		}
	}
}
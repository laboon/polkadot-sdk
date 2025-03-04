// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Preimage pallet benchmarking.

use super::*;
use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use sp_std::{prelude::*, vec};

use crate::Pallet as Preimage;

fn funded_account<T: Config>() -> T::AccountId {
	let caller: T::AccountId = whitelisted_caller();
	T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
	caller
}

fn preimage_and_hash<T: Config>() -> (Vec<u8>, T::Hash) {
	sized_preimage_and_hash::<T>(MAX_SIZE)
}

fn sized_preimage_and_hash<T: Config>(size: u32) -> (Vec<u8>, T::Hash) {
	let mut preimage = vec![];
	preimage.resize(size as usize, 0);
	let hash = <T as frame_system::Config>::Hashing::hash(&preimage[..]);
	(preimage, hash)
}

benchmarks! {
	// Expensive note - will reserve.
	note_preimage {
		let s in 0 .. MAX_SIZE;
		let caller = funded_account::<T>();
		let (preimage, hash) = sized_preimage_and_hash::<T>(s);
	}: _(RawOrigin::Signed(caller), preimage)
	verify {
		assert!(Preimage::<T>::have_preimage(&hash));
	}
	// Cheap note - will not reserve since it was requested.
	note_requested_preimage {
		let s in 0 .. MAX_SIZE;
		let caller = funded_account::<T>();
		let (preimage, hash) = sized_preimage_and_hash::<T>(s);
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
	}: note_preimage(RawOrigin::Signed(caller), preimage)
	verify {
		assert!(Preimage::<T>::have_preimage(&hash));
	}
	// Cheap note - will not reserve since it's the manager.
	note_no_deposit_preimage {
		let s in 0 .. MAX_SIZE;
		let (preimage, hash) = sized_preimage_and_hash::<T>(s);
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
	}: note_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		preimage
	) verify {
		assert!(Preimage::<T>::have_preimage(&hash));
	}

	// Expensive unnote - will unreserve.
	unnote_preimage {
		let caller = funded_account::<T>();
		let (preimage, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::note_preimage(RawOrigin::Signed(caller.clone()).into(), preimage));
	}: _(RawOrigin::Signed(caller), hash)
	verify {
		assert!(!Preimage::<T>::have_preimage(&hash));
	}
	// Cheap unnote - will not unreserve since there's no deposit held.
	unnote_no_deposit_preimage {
		let (preimage, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::note_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			preimage,
		));
	}: unnote_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		assert!(!Preimage::<T>::have_preimage(&hash));
	}

	// Expensive request - will unreserve the noter's deposit.
	request_preimage {
		let (preimage, hash) = preimage_and_hash::<T>();
		let noter = funded_account::<T>();
		assert_ok!(Preimage::<T>::note_preimage(RawOrigin::Signed(noter.clone()).into(), preimage));
	}: _<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		let ticket = TicketOf::<T>::new(&noter, Footprint { count: 1, size: MAX_SIZE as u64 }).unwrap();
		let s = RequestStatus::Requested { maybe_ticket: Some((noter, ticket)), count: 1, maybe_len: Some(MAX_SIZE) };
		assert_eq!(RequestStatusFor::<T>::get(&hash), Some(s));
	}
	// Cheap request - would unreserve the deposit but none was held.
	request_no_deposit_preimage {
		let (preimage, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::note_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			preimage,
		));
	}: request_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		let s = RequestStatus::Requested { maybe_ticket: None, count: 2, maybe_len: Some(MAX_SIZE) };
		assert_eq!(RequestStatusFor::<T>::get(&hash), Some(s));
	}
	// Cheap request - the preimage is not yet noted, so deposit to unreserve.
	request_unnoted_preimage {
		let (_, hash) = preimage_and_hash::<T>();
	}: request_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		let s = RequestStatus::Requested { maybe_ticket: None, count: 1, maybe_len: None };
		assert_eq!(RequestStatusFor::<T>::get(&hash), Some(s));
	}
	// Cheap request - the preimage is already requested, so just a counter bump.
	request_requested_preimage {
		let (_, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
	}: request_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		let s = RequestStatus::Requested { maybe_ticket: None, count: 2, maybe_len: None };
		assert_eq!(RequestStatusFor::<T>::get(&hash), Some(s));
	}

	// Expensive unrequest - last reference and it's noted, so will destroy the preimage.
	unrequest_preimage {
		let (preimage, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
		assert_ok!(Preimage::<T>::note_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			preimage,
		));
	}: _<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		assert_eq!(RequestStatusFor::<T>::get(&hash), None);
	}
	// Cheap unrequest - last reference, but it's not noted.
	unrequest_unnoted_preimage {
		let (_, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
	}: unrequest_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		assert_eq!(RequestStatusFor::<T>::get(&hash), None);
	}
	// Cheap unrequest - not the last reference.
	unrequest_multi_referenced_preimage {
		let (_, hash) = preimage_and_hash::<T>();
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
		assert_ok!(Preimage::<T>::request_preimage(
			T::ManagerOrigin::try_successful_origin()
				.expect("ManagerOrigin has no successful origin required for the benchmark"),
			hash,
		));
	}: unrequest_preimage<T::RuntimeOrigin>(
		T::ManagerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
		hash
	) verify {
		let s = RequestStatus::Requested { maybe_ticket: None, count: 1, maybe_len: None };
		assert_eq!(RequestStatusFor::<T>::get(&hash), Some(s));
	}

	ensure_updated {
		let n in 0..MAX_HASH_UPGRADE_BULK_COUNT;

		let caller = funded_account::<T>();
		let hashes = (0..n).map(|i| insert_old_unrequested::<T>(i)).collect::<Vec<_>>();
	}: _(RawOrigin::Signed(caller), hashes)
	verify {
		assert_eq!(RequestStatusFor::<T>::iter_keys().count(), n as usize);
		#[allow(deprecated)]
		let c = StatusFor::<T>::iter_keys().count();
		assert_eq!(c, 0);
	}

	impl_benchmark_test_suite!(Preimage, crate::mock::new_test_ext(), crate::mock::Test);
}

fn insert_old_unrequested<T: Config>(s: u32) -> <T as frame_system::Config>::Hash {
	let acc = account("old", s, 0);
	T::Currency::make_free_balance_be(&acc, BalanceOf::<T>::max_value() / 2u32.into());

	// The preimage size does not matter here as it is not touched.
	let preimage = s.to_le_bytes();
	let hash = <T as frame_system::Config>::Hashing::hash(&preimage[..]);

	#[allow(deprecated)]
	StatusFor::<T>::insert(
		&hash,
		OldRequestStatus::Unrequested { deposit: (acc, 123u32.into()), len: preimage.len() as u32 },
	);
	hash
}

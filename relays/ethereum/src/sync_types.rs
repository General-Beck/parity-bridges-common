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

use std::{ops::Deref, sync::Arc};

/// Ethereum header Id.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct HeaderId<Hash, Number>(pub Number, pub Hash);

/// Ethereum header synchronization status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaderStatus {
	/// Header is unknown.
	Unknown,
	/// Header is in MaybeOrphan queue.
	MaybeOrphan,
	/// Header is in Orphan queue.
	Orphan,
	/// Header is in MaybeExtra queue.
	MaybeExtra,
	/// Header is in Extra queue.
	Extra,
	/// Header is in Ready queue.
	Ready,
	/// Header is in Incomplete queue.
	Incomplete,
	/// Header has been recently submitted to the target node.
	Submitted,
	/// Header is known to the target node.
	Synced,
}

/// Error type that can signal connection errors.
pub trait MaybeConnectionError {
	/// Returns true if error (maybe) represents connection error.
	fn is_connection_error(&self) -> bool;
}

/// Headers synchronization pipeline.
pub trait HeadersSyncPipeline: Clone + Copy {
	/// Name of the headers source.
	const SOURCE_NAME: &'static str;
	/// Name of the headers target.
	const TARGET_NAME: &'static str;

	/// Headers we're syncing are identified by this hash.
	type Hash: Eq + Clone + Copy + std::fmt::Debug + std::fmt::Display + std::hash::Hash;
	/// Headers we're syncing are identified by this number.
	type Number: From<u32>
		+ Ord
		+ Clone
		+ Copy
		+ std::fmt::Debug
		+ std::fmt::Display
		+ std::hash::Hash
		+ std::ops::Add<Output = Self::Number>
		+ std::ops::Sub<Output = Self::Number>
		+ num_traits::Saturating
		+ num_traits::Zero
		+ num_traits::One
		+ Into<u64>;
	/// Type of header that we're syncing.
	type Header: Clone + std::fmt::Debug + PartialEq + SourceHeader<Self::Hash, Self::Number>;
	/// Type of extra data for the header that we're receiving from the source node:
	/// 1) extra data is required for some headers;
	/// 2) target node may answer if it'll require extra data before header is submitted;
	/// 3) extra data available since the header creation time;
	/// 4) header and extra data are submitted in single transaction.
	///
	/// Example: Ethereum transactions receipts.
	type Extra: Clone + PartialEq + std::fmt::Debug;
	/// Type of data required to 'complete' header that we're receiving from the source node:
	/// 1) completion data is required for some headers;
	/// 2) target node can't answer if it'll require completion data before header is accepted;
	/// 3) completion data may be generated after header generation;
	/// 4) header and completion data are submitted in separate transactions.
	///
	/// Example: Substrate GRANDPA justifications.
	type Completion: Clone + std::fmt::Debug;

	/// Function used to estimate size of target-encoded header.
	fn estimate_size(source: &QueuedHeader<Self>) -> usize;
}

/// A HeaderId for `HeaderSyncPipeline`.
pub type HeaderIdOf<P> = HeaderId<<P as HeadersSyncPipeline>::Hash, <P as HeadersSyncPipeline>::Number>;

/// Header that we're receiving from source node.
pub trait SourceHeader<Hash, Number> {
	/// Returns ID of header.
	fn id(&self) -> HeaderId<Hash, Number>;
	/// Returns ID of parent header.
	fn parent_id(&self) -> HeaderId<Hash, Number>;
}

/// Header how it's stored in the synchronization queue.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct QueuedHeader<P: HeadersSyncPipeline>(Arc<QueuedHeaderData<P>>);

impl<P: HeadersSyncPipeline> QueuedHeader<P> {
	/// Creates new queued header.
	pub fn new(header: P::Header) -> Self {
		QueuedHeader(Arc::new(QueuedHeaderData { header, extra: None }))
	}

	/// Set associated extra data.
	pub fn set_extra(self, extra: P::Extra) -> Self {
		QueuedHeader(Arc::new(QueuedHeaderData {
			header: Arc::try_unwrap(self.0)
				.map(|data| data.header)
				.unwrap_or_else(|data| data.header.clone()),
			extra: Some(extra),
		}))
	}
}

impl<P: HeadersSyncPipeline> Deref for QueuedHeader<P> {
	type Target = QueuedHeaderData<P>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Header how it's stored in the synchronization queue.
#[derive(Clone, Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub struct QueuedHeaderData<P: HeadersSyncPipeline> {
	header: P::Header,
	extra: Option<P::Extra>,
}

impl<P: HeadersSyncPipeline> QueuedHeader<P> {
	/// Returns ID of header.
	pub fn id(&self) -> HeaderId<P::Hash, P::Number> {
		self.header.id()
	}

	/// Returns ID of parent header.
	pub fn parent_id(&self) -> HeaderId<P::Hash, P::Number> {
		self.header.parent_id()
	}

	/// Returns reference to header.
	pub fn header(&self) -> &P::Header {
		&self.header
	}

	/// Returns reference to associated extra data.
	pub fn extra(&self) -> &Option<P::Extra> {
		&self.extra
	}
}

/// Headers submission result.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SubmittedHeaders<Id, Error> {
	/// IDs of headers that have been submitted to target node.
	pub submitted: Vec<Id>,
	/// IDs of incomplete headers. These headers were submitted (so this id is also in `submitted` vec),
	/// but all descendants are not.
	pub incomplete: Vec<Id>,
	/// IDs of ignored headers that we have decided not to submit (they're either rejected by
	/// target node immediately, or they're descendants of incomplete headers).
	pub rejected: Vec<Id>,
	/// Fatal target node error, if it has occured during submission.
	pub fatal_error: Option<Error>,
}

impl<Id, Error> Default for SubmittedHeaders<Id, Error> {
	fn default() -> Self {
		SubmittedHeaders {
			submitted: Vec::new(),
			incomplete: Vec::new(),
			rejected: Vec::new(),
			fatal_error: None,
		}
	}
}

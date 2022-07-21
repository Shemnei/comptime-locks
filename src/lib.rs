use std::marker::PhantomData;

use capability::*;
use kind::*;
use topic::*;

mod topic {
	pub trait Topic {}

	pub struct Chunks;
	impl Topic for Chunks {}

	pub struct Index;
	impl Topic for Index {}
}

mod kind {
	pub trait Kind {}

	pub struct None;
	impl Kind for None {}

	pub trait AnyKind {}

	pub struct Shared;
	impl Kind for Shared {}
	impl AnyKind for Shared {}

	pub struct Exclusive;
	impl Kind for Exclusive {}
	impl AnyKind for Exclusive {}
}

mod capability {
	pub trait ReadChunk {
		fn read(&self);
	}

	pub trait WriteChunk {
		fn write(&self);
	}

	pub trait DeleteChunk {
		fn delete(&self);
	}
}

#[derive(Default)]
pub struct LockState<C, I> {
	chunks: PhantomData<C>,
	index: PhantomData<I>,
}

#[derive(Default)]
pub struct Transaction<C, I> {
	locks: LockState<C, I>,
}

pub trait Lock<T: Topic, K: Kind> {
	type Output;

	fn locka(self) -> Self::Output;
}

impl<K: Kind, C, I> Lock<Chunks, K> for Transaction<C, I> {
	type Output = Transaction<K, I>;

	fn locka(self) -> Self::Output {
		Transaction {
			locks: LockState {
				chunks: Default::default(),
				index: self.locks.index,
			},
		}
	}
}

impl<K: Kind, C, I> Lock<Index, K> for Transaction<C, I> {
	type Output = Transaction<C, K>;

	fn locka(self) -> Self::Output {
		Transaction {
			locks: LockState {
				index: Default::default(),
				chunks: self.locks.chunks,
			},
		}
	}
}

impl<C, I> Transaction<C, I> {
	pub fn lock<T: Topic, K: Kind>(self) -> <Self as Lock<T, K>>::Output
	where
		Self: Lock<T, K>,
	{
		Lock::<T, K>::locka(self)
	}
}

impl<C: AnyKind, I> ReadChunk for Transaction<C, I> {
	fn read(&self) {}
}

impl<C: AnyKind, I> WriteChunk for Transaction<C, I> {
	fn write(&self) {}
}

impl<I> DeleteChunk for Transaction<Exclusive, I> {
	fn delete(&self) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn full_example() {
		fn delete_chunk<T: DeleteChunk>(txn: T) {
			txn.delete();
		}

		let txn = Transaction {
			locks: LockState {
				chunks: PhantomData::<Shared>::default(),
				index: PhantomData::<Shared>::default(),
			},
		};

		// Does not work as the chunks topic is not locked as exlusive
		// delete_chunk(txn);

		let txn = txn.lock::<Chunks, Exclusive>();

		delete_chunk(txn);
	}
}

use std::marker::PhantomData;

use capability::*;
use kind::*;
use topic::*;

mod topic {
	pub trait Topic: std::fmt::Debug {}

	#[derive(Debug)]
	pub struct Chunks;
	impl Topic for Chunks {}

	#[derive(Debug)]
	pub struct Index;
	impl Topic for Index {}
}

mod kind {
	pub trait Kind: std::fmt::Debug {}

	#[derive(Debug)]
	pub struct None;
	impl Kind for None {}

	pub trait AnyKind: std::fmt::Debug {}

	#[derive(Debug)]
	pub struct Shared;
	impl Kind for Shared {}
	impl AnyKind for Shared {}

	#[derive(Debug)]
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

impl<C, I> LockState<C, I> {
	// Private copy
	fn _copy(&self) -> Self {
		LockState { chunks: self.chunks, index: self.index }
	}
}

#[derive(Default)]
pub struct Transaction<C, I> {
	locks: LockState<C, I>,
}

pub trait Lock<T: Topic, K: Kind>: Sized {
	type Output;
	type Error;

	fn aquire_lock(self) -> Result<Self::Output, (Self, Self::Error)>;
}

impl<K: Kind, C, I> Lock<Chunks, K> for Transaction<C, I> {
	type Error = String;
	type Output = Transaction<K, I>;

	fn aquire_lock(self) -> Result<Self::Output, (Self, Self::Error)> {
		Ok(Transaction {
			locks: LockState {
				chunks: Default::default(),
				index: self.locks.index,
			},
		})
	}
}

impl<K: Kind + 'static, C, I: 'static> Lock<Index, K> for Transaction<C, I> {
	type Error = String;
	type Output = Transaction<C, K>;

	fn aquire_lock(self) -> Result<Self::Output, (Self, Self::Error)> {
		if std::any::TypeId::of::<I>() == std::any::TypeId::of::<K>() {
			Err((self, String::from("Lock already aquired for index")))
		} else {
			Ok(Transaction {
				locks: LockState {
					index: Default::default(),
					chunks: self.locks.chunks,
				},
			})
		}
	}
}

impl<C, I> Transaction<C, I> {
	pub fn lock<T: Topic, K: Kind>(
		self,
	) -> Result<
		<Self as Lock<T, K>>::Output,
		(Self, <Self as Lock<T, K>>::Error),
	>
	where
		Self: Lock<T, K>,
	{
		Lock::<T, K>::aquire_lock(self)
	}

	// This implementation is janky and should probably not be used in a "real"
	// codebase
	pub fn with_lock<T: Topic, K: Kind>(
		&mut self,
		f: impl Fn(&mut <Self as Lock<T, K>>::Output),
	) -> Result<(), (Self, <Self as Lock<T, K>>::Error)>
	where
		Self: Lock<T, K>,
	{
		let locks = self.locks._copy();
		let mut this = Lock::<T, K>::aquire_lock(Transaction { locks })?;
		f(&mut this);
		Ok(())
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
	fn full_example_ok() {
		fn rw_chunk<T: ReadChunk + WriteChunk>(txn: &T) {
			txn.read();
			txn.write();
		}

		fn delete_chunk<T: DeleteChunk>(txn: &T) {
			txn.delete();
		}

		let txn = Transaction {
			locks: LockState {
				chunks: PhantomData::<Shared>::default(),
				index: PhantomData::<Shared>::default(),
			},
		};

		rw_chunk(&txn);

		// Does not work as the chunks topic is not locked as exlusive
		// delete_chunk(txn);

		if let Ok(txn) = txn.lock::<Chunks, Exclusive>() {
			delete_chunk(&txn);
		} else {
			panic!("Failed to aquire lock on index");
		}
	}

	#[test]
	fn with_lock() {
		trait DeleteIndex {
			fn delete_index(&self);
		}

		impl<C> DeleteIndex for Transaction<C, Exclusive> {
			fn delete_index(&self) {}
		}

		fn delete_chunk<T: DeleteChunk>(txn: &T) {
			txn.delete();
		}

		fn delete_index<T: DeleteIndex>(txn: &T) {
			txn.delete_index();
		}

		let mut txn = Transaction {
			locks: LockState {
				chunks: PhantomData::<Shared>::default(),
				index: PhantomData::<Shared>::default(),
			},
		};

		let res = txn.with_lock::<Chunks, Exclusive>(|txn| {
			let res = txn.with_lock::<Index, Exclusive>(|txn| {
				delete_chunk(txn);
				delete_index(txn);
			});

			// Does not work outside of fn context
			//delete_index(txn);

			assert!(res.is_ok());

			delete_chunk(txn);
		});

		// Does not work outside of fn context
		//delete_chunk(txn);

		assert!(res.is_ok());
	}

	#[test]
	fn kind_checking() {
		let txn = Transaction {
			locks: LockState {
				chunks: PhantomData::<Shared>::default(),
				index: PhantomData::<Shared>::default(),
			},
		};

		assert!(txn.lock::<Index, Shared>().is_err());

		let txn = Transaction {
			locks: LockState {
				chunks: PhantomData::<Shared>::default(),
				index: PhantomData::<Shared>::default(),
			},
		};

		assert!(txn.lock::<Index, Exclusive>().is_ok());
	}
}

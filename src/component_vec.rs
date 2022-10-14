use std::{mem, ptr::{NonNull, self}, marker::PhantomData, any::TypeId, alloc::{Layout, self}};

struct RawVec {
	ptr: NonNull<u8>,
	cap: usize,
	size: usize,
	align: usize,
	initialized: bool,
	_marker: PhantomData<u8>,
}
unsafe impl Send for RawVec {}
unsafe impl Sync for RawVec {}
impl RawVec {
	fn new_as<T:'static>() -> Self {
		Self {
			ptr: NonNull::dangling(),
			cap: 0,
			size: mem::size_of::<T>(),
			align: mem::align_of::<T>(),
			initialized: true,
			_marker: PhantomData,
		}
	}
	fn imitate(&self) -> Self {
		Self {
			ptr: NonNull::dangling(),
			cap: 0,
			size: self.size,
			align: self.align,
			initialized: self.initialized,
			_marker: PhantomData,
		}
	}
	fn initialize<T:'static>(&mut self) {
		self.size = mem::size_of::<T>();
		self.align = mem::align_of::<T>();
		self.initialized = true;
	}
	fn layout(&self) -> Layout {
		assert!(self.initialized, "Access violation: uninitialized");
		Layout::from_size_align(self.size * self.cap, self.align).unwrap()
	}
	fn grow(&mut self) {
		assert!(self.initialized, "Access violation: uninitialized");
		assert!(self.size != 0, "capacity overflow");

		let new_cap = if self.cap == 0 { 1 } else { 2 * self.cap };
		let new_layout = Layout::from_size_align(self.size * new_cap, self.align).unwrap();
		assert!(new_layout.size() <= isize::MAX as usize, "Allocation too large");
		let new_ptr = if self.cap == 0 {
			unsafe { alloc::alloc(new_layout) }
		} else {
			unsafe { alloc::realloc(self.ptr.as_ptr(), self.layout(), self.size * new_cap) }
		};

		self.ptr = match NonNull::new(new_ptr) {
			Some(p) => p,
			None => alloc::handle_alloc_error(new_layout),
		};
		self.cap = new_cap;
	}
}
impl Drop for RawVec {
	fn drop(&mut self) {
		if self.cap != 0 {
			let layout = Layout::from_size_align(self.size * self.cap, self.align).unwrap();
			unsafe { alloc::dealloc(self.ptr.as_ptr(), layout); }
		}
	}
}


pub struct ComponentVec {
	buf: RawVec,
	len: usize,
	type_id: Option<TypeId>,
}
impl ComponentVec {
	fn ptr_as<T:'static>(&self) -> *mut T {
		self.buf.ptr.as_ptr().cast::<T>()
	}
	fn ptr(&self) -> *mut u8 {
		self.buf.ptr.as_ptr()
	}
	fn cap(&self) -> usize {
		self.buf.cap
	}
	pub fn len(&self) -> usize {
		self.len
	}
	pub fn type_id(&self) -> TypeId {
		self.type_id.unwrap()
	}
	fn is_type_or_set<T:'static>(&mut self) -> bool {
		if let Some(tid) = self.type_id {
			tid == TypeId::of::<T>()
		} else {
			self.type_id = Some(TypeId::of::<T>());
			true
		}
	}
	pub fn is_type<T:'static>(&self) -> bool {
		if let Some(tid) = self.type_id {
			tid == TypeId::of::<T>()
		} else {
			false
		}
	}
	pub fn new_as<T:'static>() -> Self {
		Self {
			buf: RawVec::new_as::<T>(),
			len: 0,
			type_id: Some(TypeId::of::<T>()),
		}
	}
	pub fn from<T:'static, const N:usize>(data: [T;N]) -> Self {
		let mut cv = ComponentVec::new_as::<T>();
		for el in data { cv.push::<T>(el); }
		cv
	}
	pub fn imitate(&self) -> Self {
		Self {
			buf: self.buf.imitate(),
			len: 0,
			type_id: self.type_id,
		}
	}
	pub fn push<T:'static>(&mut self, elem: T) {
		assert!(self.is_type_or_set::<T>(), "Invalid type");
		if !self.buf.initialized { self.buf.initialize::<T>(); }
		if self.len == self.cap() { self.buf.grow(); }

		unsafe { ptr::write(self.ptr_as::<T>().add(self.len), elem) }
		self.len += 1;
	}
	pub fn pop<T:'static>(&mut self) -> Option<T> {
		assert!(self.is_type::<T>(), "Invalid type");
		if self.len == 0 {
			None
		} else {
			self.len -= 1;
			unsafe { Some(ptr::read(self.ptr_as::<T>().add(self.len))) }
		}
	}
	pub fn insert<T:'static>(&mut self, index: usize, elem: T) {
		assert!(self.is_type_or_set::<T>(), "Invalid type");
		assert!(index < self.len, "index out of bounds");
		if !self.buf.initialized { self.buf.initialize::<T>(); }
		if self.cap() == self.len { self.buf.grow(); }

		unsafe {
			ptr::copy(
				self.ptr_as::<T>().add(index),
				self.ptr_as::<T>().add(index + 1),
				self.len - index,
			);
			ptr::write(self.ptr_as::<T>().add(index), elem);
			self.len += 1;
		}
	}
	pub fn remove<T:'static>(&mut self, index: usize) -> T {
		assert!(self.is_type::<T>(), "Invalid type");
		assert!(index < self.len, "index out of bounds");
		unsafe {
			self.len -= 1;
			let result = ptr::read(self.ptr_as::<T>().add(index));
			ptr::copy(
				self.ptr_as::<T>().add(index + 1),
				self.ptr_as::<T>().add(index),
				self.len - index,
			);
			result
		}
	}
	pub fn swap_remove<T:'static>(&mut self, index: usize) -> T {
		assert!(self.is_type::<T>(), "Invalid type");
		assert!(index < self.len, "index out of bounds");
		unsafe {
			self.len -= 1;
			if self.len > 0 {
				ptr::swap(
					self.ptr_as::<T>().add(index),
					self.ptr_as::<T>().add(self.len)
				);
			}
			ptr::read(self.ptr_as::<T>().add(self.len))
		}
	}
	pub fn swap_forget(&mut self, index: usize) {
		assert!(index < self.len, "index out of bounds");
		unsafe {
			self.len -= 1;
			if self.len > 0 {
				ptr::copy(
					self.ptr().add(self.len * self.buf.size),
					self.ptr().add(index * self.buf.size),
					self.buf.size
				);
			}
		}
	}
	pub fn as_slice<T:'static>(&self) -> &[T] {
		assert!(self.is_type::<T>(), "Invalid type");
		unsafe { std::slice::from_raw_parts(self.ptr_as::<T>(), self.len) }
	}
	pub fn as_mut_slice<T:'static>(&mut self) -> &mut [T] {
		assert!(self.is_type::<T>(), "Invalid type");
		unsafe { std::slice::from_raw_parts_mut(self.ptr_as::<T>(), self.len) }
	}
	pub fn drain<T:'static>(&mut self) -> Drain<T> {
		assert!(self.is_type::<T>(), "Invalid type");
		unsafe {
			let iter = RawValIter::new(self.as_slice::<T>());
			self.len = 0;
			Drain {
				iter,
				vec: PhantomData,
			}
		}
	}
	pub fn into_iter<T:'static>(self) -> IntoIter<T> {
		assert!(self.is_type::<T>(), "Invalid type");
		unsafe {
			let iter = RawValIter::new(self.as_slice::<T>());
			let buf = ptr::read(&self.buf);
			mem::forget(self);

			IntoIter {
				iter,
				_buf: buf
			}
		}
	}
	pub fn swap_to_tail(&mut self, index: usize) {
		assert!(index < self.len, "index out of bounds");
		
		if index != self.len - 1 {
			for i in 0..self.buf.size {
				unsafe {ptr::swap(
					self.ptr().add(index * self.buf.size).add(i),
					self.ptr().add(self.len * self.buf.size).add(i),
				)};
			}
		}
	}
	pub fn adopt_tail(&mut self, src: &mut Self) {
		assert!(self.type_id == src.type_id, "Incompatible types");
		if src.len > 0 {
			if self.len == self.cap() { self.buf.grow(); }
			unsafe {
				src.len -= 1;
				ptr::copy_nonoverlapping(
					src.ptr().add(src.len * src.buf.size), 
					self.ptr().add(self.len * self.buf.size), 
					src.buf.size
				);
				self.len += 1;
			}
		}
	}
}


struct RawValIter<T> {
	start: *const T,
	end: *const T,
}
impl<T> RawValIter<T> {
	unsafe fn new(slice: &[T]) -> Self {
		Self {
			start: slice.as_ptr(),
			end: if mem::size_of::<T>() == 0 {
				((slice.as_ptr() as usize) + slice.len()) as *const _
			} else if slice.len() == 0 {
				slice.as_ptr()
			} else {
				slice.as_ptr().add(slice.len())
			},
		}
	}
}
impl<T> Iterator for RawValIter<T> {
	type Item = T;
	fn next(&mut self) -> Option<T> {
		if self.start == self.end {
			None
		} else {
			unsafe {
				if mem::size_of::<T>() == 0 {
					self.start = (self.start as usize + 1) as *const _;
					Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
				} else {
					let old_ptr = self.start;
					self.start = self.start.offset(1);
					Some(ptr::read(old_ptr))
				}
			}
		}
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		let elem_size = mem::size_of::<T>();
		let len = (self.end as usize - self.start as usize)
				/ if elem_size == 0 { 1 } else { elem_size };
		(len, Some(len))
	}
}
impl<T> DoubleEndedIterator for RawValIter<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		if self.start == self.end {
			None
		} else {
			unsafe {
				if mem::size_of::<T>() == 0 {
					self.end = (self.end as usize - 1) as *const _;
					Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
				} else {
					self.end = self.end.offset(-1);
					Some(ptr::read(self.end))
				}
			}
		}
	}
}


pub struct IntoIter<T> {
	_buf: RawVec,
	iter: RawValIter<T>
}
impl<T> Iterator for IntoIter<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}
}
impl<T> DoubleEndedIterator for IntoIter<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}
impl<T> Drop for IntoIter<T> {
	fn drop(&mut self) {
		for _ in &mut *self {}
	}
}


pub struct Drain<'a, T: 'a> {
	vec: PhantomData<&'a mut ComponentVec>,
	iter: RawValIter<T>,
}
impl<'a, T> Iterator for Drain<'a, T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}
}
impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back()
	}
}
impl<'a, T> Drop for Drain<'a, T> {
	fn drop(&mut self) {
		for _ in &mut *self {}
	}
}
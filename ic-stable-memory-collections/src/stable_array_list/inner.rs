use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use std::convert::TryInto;
use std::ops::{Add, Deref, DerefMut};
use std::{clone, ptr};

use crate::types::{StableArrayListError, STABLE_ARRAY_LIST_MARKER};
use ic_stable_memory_allocator::mem_block::{MemBlock, MemBlockSide};
use ic_stable_memory_allocator::mem_context::MemContext;
use ic_stable_memory_allocator::stable_memory_allocator::StableMemoryAllocator;
use ic_stable_memory_allocator::types::{SMAError, EMPTY_PTR};
use std::marker::PhantomData;
use std::mem::size_of;

//Tombstone
//без переноса и лишних копирований
// --в структурах лежат не данные а указатели--
// данные-&[u8], pointers - u64?

pub struct StableArrayListInner<T: MemContext + Clone> {
    ptr: u64,
    marker: PhantomData<T>,
    pub len: u64,
    pub cap: u64,
}

impl<T: MemContext + Clone> StableArrayListInner<T> {
    //todo: 1) создать обычный массив

    pub fn new(
        length: u64,
        allocator: &mut StableMemoryAllocator<T>,
        context: &mut T,
    ) -> Result<Self, StableArrayListError> {
        let mut mem_block = allocator
            .allocate(1 + size_of::<u64>() as u64 * (length + 1), context)
            .map_err(StableArrayListError::SMAError)?;

        mem_block
            .write_bytes(0, &STABLE_ARRAY_LIST_MARKER, context)
            .unwrap();

        mem_block
            .write_u64(1 + size_of::<u64>() as u64 * length, EMPTY_PTR, context)
            .unwrap();

        Ok(Self {
            ptr: mem_block.ptr,
            marker: PhantomData,
            len: 0,
            cap: length,
        })
    }

    pub fn set(
        &mut self,
        index: u64,
        value: u64,
        allocator: &mut StableMemoryAllocator<T>,
        context: &mut T,
    ) {
        assert!(index <= self.len);

        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block.write_u64(1 + index, value, context).unwrap();
    }

    pub fn insert(
        &mut self,
        value: u64,
        index: u64,
        allocator: &mut StableMemoryAllocator<T>,
        context: &mut T,
    ) {
        //todo: check grow
        assert!(index <= self.len);

        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        unsafe {
            ptr::copy(
                mem_block.ptr.add(1+index) as *const u64,
                mem_block.ptr.add(1+ index + 1) as *mut u64,
                (self.len - index).try_into().unwrap(),
            );
            mem_block
                .write_u64(1 + size_of::<u64>() as u64 * index, value, context)
                .unwrap();
        }
        self.len += 1;
    }

    pub fn push(&mut self, value: u64, allocator: &mut StableMemoryAllocator<T>, context: &mut T) {
        if self.len == self.cap {
            let new_mem_block = allocator.allocate(self.len * 2, context).unwrap();
            self.ptr = new_mem_block.ptr;
            self.cap += self.len * 2;
        }
        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block
            .write_u64(1 + self.len * size_of::<u64>() as u64, value, context)
            .unwrap();
        self.len += 1;
    }
}

impl<T: MemContext + Clone> Deref for StableArrayListInner<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const T, self.len as usize) }
    }
}

impl<T: MemContext + Clone> DerefMut for StableArrayListInner<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut T, self.len as usize) }
    }
}

#[cfg(test)]
mod tests {
    use crate::stable_array_list::inner::StableArrayListInner;
    use ic_stable_memory_allocator::mem_context::TestMemContext;
    use ic_stable_memory_allocator::stable_memory_allocator::StableMemoryAllocator;

    #[test]
    fn test1() {
        let mut context = TestMemContext::default();
        let mut allocator = StableMemoryAllocator::init(0, &mut context).ok().unwrap();
        let mut array_list = StableArrayListInner::new(8, &mut allocator, &mut context)
            .ok()
            .unwrap();
        // array_list.push(4, &mut allocator, &mut context);
        // array_list.push(7, &mut allocator, &mut context);
        // array_list.push(9, &mut allocator, &mut context);

        
        array_list.push(2, &mut allocator, &mut context);
        array_list.push(4, &mut allocator, &mut context);
        array_list.push(6, &mut allocator, &mut context);
        array_list.push(8, &mut allocator, &mut context);
        array_list.push(10, &mut allocator, &mut context);

        array_list.set(0, 3, &mut allocator, &mut context);
        array_list.set(1, 5, &mut allocator, &mut context);

        array_list.insert(99, 4, &mut allocator, &mut context)
        //let mut v1 = Vec::new_in(allocator);
    }
}

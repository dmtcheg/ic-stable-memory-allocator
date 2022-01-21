use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Add, Deref, DerefMut, Index};
use std::rc::Rc;
use std::slice::Iter;
use std::{clone, ptr};

use crate::types::{StableArrayListError, STABLE_ARRAY_LIST_MARKER};
use ic_stable_memory_allocator::mem_block::{MemBlock, MemBlockSide};
use ic_stable_memory_allocator::mem_context::MemContext;
use ic_stable_memory_allocator::stable_memory_allocator::StableMemoryAllocator;
use ic_stable_memory_allocator::types::{SMAError, EMPTY_PTR};

use log::{debug, info};

#[derive(Clone, Copy)]
pub struct StableArrayListInner<T: MemContext + Clone> {
    //buf: RawVec<u64>,
    ptr: u64,
    marker: PhantomData<T>,
    len: u64,
    cap: u64,
    context: T,
}

impl<T: MemContext + Clone> StableArrayListInner<T> {
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
            context: context.clone(),
        })
    }

    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn cap(&self) -> u64 {
        self.cap
    }

    // hashmap also has 'get()' but not []
    pub fn get(&mut self, index: u64, context: &T) -> u64 {
        assert!(index <= self.len);
        let mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block.read_u64(index, context).unwrap()
    }

    pub fn replace(&mut self, value: u64, index: u64, context: &mut T) {
        assert!(index <= self.len);
        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block
            .write_u64(1 + size_of::<u64>() as u64 * index, value, context)
            .unwrap();
    }

    pub fn insert(
        &mut self,
        value: u64,
        index: u64,
        allocator: &mut StableMemoryAllocator<T>,
        context: &mut T,
    ) {
        assert!(index < self.len);

        if self.len == self.cap {
            let block = allocator
                .reallocate(
                    self.ptr,
                    self.cap * 2, // or self.cap+1
                    context,
                )
                .unwrap();
            self.ptr = block.ptr;
            self.cap *= 2;
        }

        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        unsafe {
      
            for i in self.len..index {
                let v = mem_block
                    .read_u64(1 + size_of::<u64>() as u64 * (i - 1), &context)
                    .unwrap();
                mem_block
                    .write_u64(1 + size_of::<u64>() as u64 * i, v, context)
                    .unwrap();
            }

            mem_block
                .write_u64(1 + size_of::<u64>() as u64 * index, value, context)
                .unwrap();
        }
        self.len += 1;
        println!(
            "inserted at [{}] {}",
            index,
            mem_block
                .read_u64(1 + size_of::<u64>() as u64 * index, context)
                .unwrap()
        );
    }

    pub fn push(&mut self, value: u64, allocator: &mut StableMemoryAllocator<T>, context: &mut T) {
        if self.len == self.cap {
            let new_mem_block = allocator
                .reallocate(
                    self.ptr,
                    self.cap * 2,
                    context,
                )
                .unwrap();
            self.ptr = new_mem_block.ptr;
            self.cap *= 2;
        }
        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block
            .write_u64(1 + self.len * size_of::<u64>() as u64, value, context)
            .unwrap();
        self.len += 1;
    }

    pub fn remove() {
        //todo:
    }
}
/*
impl<T:MemContext+Clone> Index<u64> for StableArrayListInner<T> {
    type Output = u64;
    fn index(&self, index:u64) -> &u64{
        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, &self.context).unwrap();
        &mem_block.read_u64(index, &self.context).unwrap()
    }
}


impl<T: MemContext + Clone> Deref for StableArrayListInner<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.ptr as *const T, self.len as usize)
            .iter()
            .map(|context| {
                let mut buf =[0u8;size_of::<u64>()];
                context.read(0, &mut buf);
                u64::from_le_bytes(buf)
            })
            .rev().collect()
     }
    }
}
*/

#[cfg(test)]
mod tests {
    use crate::stable_array_list::inner::StableArrayListInner;
    use ic_stable_memory_allocator::mem_context::TestMemContext;
    use ic_stable_memory_allocator::stable_memory_allocator::StableMemoryAllocator;

    #[test]
    fn test1() {
        let mut context = TestMemContext::default();
        let mut allocator = StableMemoryAllocator::init(0, &mut context).ok().unwrap();
        let mut array_list = StableArrayListInner::new(4, &mut allocator, &mut context)
            .ok()
            .unwrap();

        // array_list.push(4, &mut allocator, &mut context);
        // array_list.push(7, &mut allocator, &mut context);
        // array_list.push(9, &mut allocator, &mut context);

        array_list.push(2, &mut allocator, &mut context);
        array_list.push(4, &mut allocator, &mut context);
        array_list.push(6, &mut allocator, &mut context);
        array_list.push(8, &mut allocator, &mut context);
        //array_list.push(10, &mut allocator, &mut context);

        array_list.push(13, &mut allocator, &mut context);
        array_list.push(1001, &mut allocator, &mut context);
        
        for i in 0..array_list.len() {
            let x = array_list.get(i, &context);
            println!("[{}] {}", i, x);
        }
    }
}

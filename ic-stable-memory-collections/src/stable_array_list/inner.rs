use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use std::convert::TryInto;
use std::ops::{Add, Deref, DerefMut, Index};
use std::slice::Iter;
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
    //buf: RawVec<u64>,
    ptr: u64,
    marker: PhantomData<T>,
    len: u64,
    cap: u64,
}

impl<T: MemContext + Clone> StableArrayListInner<T> {
    //todo: 1) создать обычный массив
    // 2) u64 or usize?

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

    pub fn len(self)->u64{
        self.len
    }
    pub fn cap(self)->u64{
        self.cap
    }

    // hashmap also has get but not []
    pub fn get(&mut self, index: u64, context: &mut T) -> u64 {
        assert!(index <= self.len);

        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block.read_u64(1 + index, context).unwrap()
    }

    pub fn insert(
        &mut self,
        value: u64,
        index: u64,
        allocator: &mut StableMemoryAllocator<T>,
        context: &mut T,
    ) {
        assert!(index <= self.len);

        //todo: check grow
        //todo: try realloc
        if self.len == self.cap {
            allocator
                .reallocate(
                    self.ptr,     // todo: check. что с ptr?
                    self.cap * 2, // or self.cap+1
                    context,
                )
                .unwrap();
        }

        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        unsafe {
            // ptr::copy(
            //     mem_block.ptr.add(1+index) as *const u64,
            //     mem_block.ptr.add(1+ index + 1) as *mut u64,
            //     (self.len - index).try_into().unwrap(),
            // );

            for i in self.len..index {
                let v = mem_block
                    .read_u64(1 + size_of::<u64>() as u64 * (i - 1), &context)
                    .unwrap(); //todo: correct offset
                mem_block
                    .write_u64(1 + size_of::<u64>() as u64 * i, v, context)
                    .unwrap();
            }

            mem_block
                .write_u64(1 + size_of::<u64>() as u64 * index, value, context)
                .unwrap();
        }
        self.len += 1;
    }

    pub fn push(&mut self, value: u64, allocator: &mut StableMemoryAllocator<T>, context: &mut T) {
        if self.len == self.cap {
            //or self.cap+1
            let new_mem_block = allocator.allocate(self.cap * 2, context).unwrap();
            self.ptr = new_mem_block.ptr;
        }
        let mut mem_block = MemBlock::read_at(self.ptr, MemBlockSide::Start, context).unwrap();
        mem_block
            .write_u64(1 + self.len * size_of::<u64>() as u64, value, context)
            .unwrap();
        self.len += 1;
    }
}

// impl<T:MemContext+Clone> Index<u64> for StableArrayListInner<T> {
//     type Output = u64;
//     fn index(&self, index:u64) -> &u64{
//         let block = MemBlock::read_at(self.ptr,MemBlockSide::Start,& self.context).unwrap();
//         &block.read_u64(index,& self.context).unwrap()
//     }
// }

// impl<T: MemContext + Clone> Deref for StableArrayListInner<T> {
//     type Target = [T];
//     fn deref(&self) -> &[T] {
//         unsafe {
//             std::slice::from_raw_parts(self.ptr as *const T, self.len as usize)
//             .iter()
//             .map(|context| {
//                 let mut buf =[0u8;size_of::<u64>()];
//                 context.read(0, &mut buf);
//                 u64::from_le_bytes(buf)
//             })
//             .rev().collect()
//      }
//     }
// }

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
        array_list.push(10, &mut allocator, &mut context);

        println!("{}", array_list.get(0, &mut context).to_be());
        println!("{}", array_list.get(1, &mut context).to_be());
        println!("{}", array_list.get(2, &mut context).to_be());
        println!("{}", array_list.get(3, &mut context).to_be());
        println!("{}", array_list.get(4, &mut context).to_be());

        array_list.insert(99, 3, &mut allocator, &mut context);

        assert_eq!(array_list.get(4,  &mut context), 8);
        assert_eq!(array_list.get(3,  &mut context), 99);
        //let mut v1 = Vec::new_in(allocator);
    }
}

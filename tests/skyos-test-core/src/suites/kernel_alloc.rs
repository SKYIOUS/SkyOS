use crate::{Test, assert_result, assert_eq_result};

/// Minimal buddy allocator implementation matching kernel logic.
/// Simplified for host-side testing — validates the algorithm.
struct BuddyAllocator {
    /// Free lists per order (0 = 4K, 1 = 8K, ... MAX_ORDER-1)
    free_lists: Vec<Vec<usize>>,
    max_order: usize,
    total_pages: usize,
    base: usize,
}

impl BuddyAllocator {
    fn new(total_pages: usize, max_order: usize) -> Self {
        let mut free_lists = vec![Vec::new(); max_order];
        let order = (total_pages as f64).log2().ceil() as usize;
        let order = order.min(max_order - 1);
        free_lists[order].push(0);
        BuddyAllocator { free_lists, max_order, total_pages, base: 0 }
    }

    fn allocate(&mut self, order: usize) -> Option<usize> {
        if order >= self.max_order { return None; }
        for o in order..self.max_order {
            if let Some(block) = self.free_lists[o].pop() {
                // Split remaining blocks down to requested order
                for split_o in (order..o).rev() {
                    let buddy = block + (1 << split_o);
                    self.free_lists[split_o].push(buddy);
                }
                return Some(block);
            }
        }
        None
    }

    fn free(&mut self, block: usize, order: usize) {
        if order >= self.max_order { return; }
        let mut block = block;
        let mut order = order;
        // Try to merge with buddy
        while order + 1 < self.max_order {
            let buddy = block ^ (1 << order);
            let idx = self.free_lists[order].iter().position(|&b| b == buddy);
            if let Some(pos) = idx {
                self.free_lists[order].remove(pos);
                block = block.min(buddy);
                order += 1;
            } else {
                break;
            }
        }
        self.free_lists[order].push(block);
    }

    fn is_free(&self, block: usize, order: usize) -> bool {
        if order >= self.max_order { return false; }
        self.free_lists[order].contains(&block)
    }
}

pub fn tests() -> Vec<Test> {
    vec![
        Test {
            name: "buddy_alloc_single_page",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(1024, 11);
                let block = alloc.allocate(0);
                assert_result!(block.is_some(), "allocate page");
                assert_eq_result!(block.unwrap(), 0);
                Ok(())
            }),
        },
        Test {
            name: "buddy_alloc_and_free",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(1024, 11);
                let b1 = alloc.allocate(0).unwrap();
                let b2 = alloc.allocate(0).unwrap();
                alloc.free(b1, 0);
                assert_result!(alloc.is_free(b1, 0), "page should be free after free");
                alloc.free(b2, 0);
                // After freeing both, they should merge into order-1 buddy
                let merged = b1.min(b2);
                assert_result!(alloc.is_free(merged, 1), "buddies should merge");
                Ok(())
            }),
        },
        Test {
            name: "buddy_alloc_large_block",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(1024, 11);
                let block = alloc.allocate(5); // 128 pages
                assert_result!(block.is_some(), "allocate 128-page block");
                assert_eq_result!(block.unwrap(), 0);
                Ok(())
            }),
        },
        Test {
            name: "buddy_alloc_exhaustion",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(64, 7);
                for _ in 0..64 {
                    assert_result!(alloc.allocate(0).is_some(), "should allocate");
                }
                // 65th should fail
                assert_result!(alloc.allocate(0).is_none(), "should be out of memory");
                Ok(())
            }),
        },
        Test {
            name: "buddy_alloc_fragmentation",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(64, 7);
                let mut blocks = Vec::new();
                for i in 0..64 {
                    if i % 2 == 0 {
                        blocks.push(alloc.allocate(0).unwrap());
                    } else {
                        alloc.allocate(0).unwrap();
                    }
                }
                // Free the even blocks, creating free pages
                for b in blocks { alloc.free(b, 0); }
                // Should still have enough contiguous space for at least some allocations
                assert_result!(alloc.allocate(2).is_some(), "allocate 4-page block after fragmentation");
                Ok(())
            }),
        },
        Test {
            name: "buddy_merge_chain",
            category: "kernel::alloc",
            run: Box::new(|| {
                let mut alloc = BuddyAllocator::new(64, 7);
                // Allocate all, then free in specific order to test merging
                let mut blocks: Vec<_> = (0..64).map(|_| alloc.allocate(0).unwrap()).collect();
                // Free in reverse order
                for b in blocks.into_iter().rev() {
                    alloc.free(b, 0);
                }
                // After freeing all, the top-level block should be free
                assert_result!(alloc.is_free(0, 6), "full merge back to single block");
                Ok(())
            }),
        },
    ]
}

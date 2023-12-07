# `Block`
Sui Blocks are not natural to the Sui ecosystem, as is discussed elsewhere. However, Sui-like behavior can be achieved with blocks. The types herein represent a few different types of Sui blocks.

## [Planned] `VerifiedExecutableBlock::get_max_parallel_groups`
`VerifiedExecutableBlock::get_max_parallel_groups` is a function that returns groups within a `VerifiedExecutableBlock` that can be executed in parallel. This is useful for parallelizing the execution of blocks.

Within the context of Sui transactions, we can compute this value by looking at the sets of objects for each transaction block `SenderSignedData`. If the sets of objects are disjoint, then the blocks can be executed in parallel. If the sets of shared objects are not disjoint, then the blocks cannot be executed in parallel.

You can obtain the sets of shared objects for each transaction block `SenderSignedData` by accessing the input objects on the [`TransactionData`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-core/src/authority.rs#L963C78-L963C78); you should not need any additional data. 

You can implement the algorithm for computing this value using brute force, for an $O(n^2 * ||S||)$ complexity where $||S||$ is the size of the set of objects.

This should look something like this:
```python
function find_parallel_groups(sets):
    parallel_groups = []  // List to hold groups of parallel sets
    for current_set in sets:
        added_to_group = False
        for group in parallel_groups:
            if current_set is disjoint with every set in group:
                add current_set to group
                added_to_group = True
                break
        if not added_to_group:
            create new group with current_set
            add this new group to parallel_groups
    return parallel_groups
```

You may also be able to implement a more efficient algorithm using a trie structure. But, I'm not sure...

> I believe you can do better with a trie algorithm. But, I haven't had time to verify. It could look something like this:
> 1. Insert a null node as the root in the trie.
> 2. Sort all the elements in the shared object sets.
> 3. For each set, insert the elements in the set into the trie.
> 4. Insert the transaction hash as if it were the last element in the set.
> 5. Run each separate subtree from the root in parallel, by doing an in-order traversal to obtain the transaction hash then executing the corresponding transaction. 
> The ordering of the elements in the sets ensures that intersecting sets will have the same path/subtree in the trie from the root.
> The complexity should be $O(n * ||S|| *log(||S||))$ I believe, which would be better in almost all cases because $||S||$ is likely to be smaller than $n$. However, I haven't had time to verify this.

A more optimistic alternative is to use something similar to Block-STM with reruns on collisions. However, this can be entertained at a later date.
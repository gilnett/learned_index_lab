# Recipe 4: Columnar Block Indexing & Zone Map Pruning

In analytical engines (such as ClickHouse, DuckDB, and Apache Parquet), table data is horizontally partitioned into compressed row groups or data pages (typically 64 KB to 100,000 rows each).

Rather than indexing every individual record, `PgmIndex` can serve as a ultra-compact **sparse block index** over row-group boundaries to prune non-matching blocks without decompression.

---

## Architecture: Sparse Boundary Indexing

```mermaid
flowchart LR
    Query["Target Key: 15,400"] --> PGM["PGM Sparse Index (Block Min Keys)"]
    PGM --> Predict["Lower Bound Lookup: Block #3"]
    Predict --> Disk["Physical Block #3 (Decompress & Scan)"]
    Predict -.->|"Pruned (Skip)"| Block1["Block #1 [0, 4999]"]
    Predict -.->|"Pruned (Skip)"| Block2["Block #2 [5000, 9999]"]
```

---

## Implementation: `ColumnarBlockIndex`

```rust
use learned_index_lab::PgmIndex;

#[derive(Debug, Clone)]
pub struct BlockDescriptor {
    pub block_id: u32,
    pub min_key: u64,
    pub max_key: u64,
    pub byte_offset: u64,
    pub compressed_size: u32,
}

pub struct ColumnarBlockIndex {
    /// PGM index over the min_key of each storage block
    sparse_index: PgmIndex,
    /// Physical block descriptors
    blocks: Vec<BlockDescriptor>,
}

impl ColumnarBlockIndex {
    pub fn new(blocks: Vec<BlockDescriptor>) -> Self {
        let boundary_keys: Vec<u64> = blocks.iter().map(|b| b.min_key).collect();

        // Small epsilon (e.g. 8) ensures block boundary resolution in 1-2 cache lines
        let sparse_index = PgmIndex::build(boundary_keys, 8);

        Self {
            sparse_index,
            blocks,
        }
    }

    /// Identifies the candidate storage blocks that may contain the target key
    pub fn locate_candidate_blocks(&self, target_key: u64) -> &[BlockDescriptor] {
        let lb = self.sparse_index.lower_bound(target_key);
        let candidate_idx = if lb == 0 { 0 } else { lb - 1 };

        let start = candidate_idx;
        let end = (candidate_idx + 2).min(self.blocks.len());

        &self.blocks[start..end]
    }
}
```

---

## Production Impact

- **Footprint**: For 100,000 blocks covering 1 billion rows, the sparse boundary index requires less than **32 KB of RAM**, residing permanently in L1/L2 cache.
- **I/O Savings**: Queries eliminate up to 99.9% of decompression and storage reads by targeting only candidate blocks identified by the PGM model.

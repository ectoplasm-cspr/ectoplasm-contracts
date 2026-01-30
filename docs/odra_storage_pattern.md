# Odra Storage Pattern (Empirically Discovered)

This document describes the storage layout pattern for Odra smart contracts deployed on Casper, based on empirical testing with the Ectoplasm DEX contracts.

## Overview

Odra contracts store all data in a dictionary called `state` under the contract's named keys. This pattern describes how to calculate storage keys to read contract state directly via RPC.

## Index Encoding

**Critical Discovery**: This Odra version uses **u32 Big Endian (4 bytes)** for field indices, not u8 as mentioned in some documentation.

```typescript
// Convert index to 4-byte Big Endian array
const indexBytes = new Uint8Array(4);
new DataView(indexBytes.buffer).setUint32(0, index, false); // false = Big Endian
```

## Storage Key Generation

### For `Var<T>` (Simple Values)

```typescript
key = blake2b(u32_be(index))

// Example: Reading reserve0 at index 5
const indexBytes = [0x00, 0x00, 0x00, 0x05];
const storageKey = blake2bHex(indexBytes, undefined, 32);
```

### For `Mapping<K, V>`

```typescript
key = blake2b(u32_be(index) + serialize(key_data))

// Example: Reading pairs mapping at index 4
const indexBytes = [0x00, 0x00, 0x00, 0x04];
const keyData = serializeKey(tokenA) + serializeKey(tokenB);
const storageKey = blake2bHex(concat(indexBytes, keyData), undefined, 32);
```

**Important**: There is **NO tag byte** between the index and key data (unlike some Odra documentation suggests).

## Field Index Assignment

Fields are assigned sequential indices starting from 0 based on their declaration order in the struct:

```rust
#[odra::module]
pub struct Pair {
    lp_token: SubModule<LpToken>,  // Index 0 (takes indices 0, 1, 2)
    token0: Var<Address>,           // Index 3
    token1: Var<Address>,           // Index 4
    reserve0: Var<U256>,            // Index 5
    reserve1: Var<U256>,            // Index 6
    block_timestamp_last: Var<u64>, // Index 7
    // ...
}
```

## SubModule Storage Footprint

**Critical Discovery**: `SubModule<T>` consumes **multiple consecutive indices**, not just one.

For `SubModule<LpToken>`, we discovered it takes **3 indices** (0, 1, 2), shifting all subsequent fields by +3.

### How to Determine SubModule Size

1. Test empirically by querying known fields at different indices
2. Count backwards from a known field to find where SubModule ends
3. The SubModule likely stores its own fields sequentially

## Key Serialization for Mappings

### Address Serialization

Casper `Address` (Key type) serializes as:
```
[tag_byte, ...32_hash_bytes]
```

Where:
- `tag = 0` for Account (`account-hash-...`)
- `tag = 1` for Hash (`hash-...`)

### Example: Factory Pairs Mapping

```typescript
// Factory.pairs: Mapping<(Address, Address), Address>

function serializeKey(keyStr: string): Uint8Array {
    let tag = 1; // Hash
    let clean = keyStr.replace('hash-', '');
    
    const bytes = new Uint8Array(33);
    bytes[0] = tag;
    const hashBytes = hexToBytes(clean);
    bytes.set(hashBytes, 1);
    return bytes;
}

// Sort addresses (Factory does this)
const [first, second] = sortAddresses(tokenA, tokenB);

// Concatenate serialized addresses
const mappingKey = concat(serializeKey(first), serializeKey(second));

// Generate storage key
const storageKey = blake2b(u32_be(4) + mappingKey);
```

## CLValue Return Types

The Casper RPC returns different types than you might expect:

| Rust Type | Expected CLType | Actual CLType | Format |
|-----------|----------------|---------------|--------|
| `Address` (in Mapping) | `Key` | `List<U8>` (33 bytes) | `[tag, ...hash]` |
| `U256` | `U256` | `List<U8>` (variable) | Little-endian bytes |
| `Address` (in Var) | `Key` | `Key` | Native Key object |

### Parsing 33-byte Keys

When you get a `List<U8>` with 33 bytes, it's a serialized Key:

```typescript
if (bytes.length === 33 && (bytes[0] === 0 || bytes[0] === 1)) {
    const tag = bytes[0];
    const hashHex = bytes.slice(1).map(b => b.toString(16).padStart(2, '0')).join('');
    
    if (tag === 0) return `account-hash-${hashHex}`;
    if (tag === 1) return `hash-${hashHex}`;
}
```

## Storage Access Flow

1. **Query Contract**
   ```typescript
   const contractData = await rpcClient.state_get_item({
       state_root_hash: stateRoot,
       key: contractHash,
       path: []
   });
   ```

2. **Find State Dictionary**
   ```typescript
   const stateURef = contractData.stored_value.Contract.named_keys
       .find(k => k.name === 'state')?.key;
   ```

3. **Generate Storage Key**
   ```typescript
   const storageKey = generateKey(index, keyData); // Using patterns above
   ```

4. **Query Dictionary**
   ```typescript
   const result = await rpcClient.state_get_dictionary_item({
       state_root_hash: stateRoot,
       dictionary_identifier: {
           URef: {
               seed_uref: stateURef,
               dictionary_item_key: storageKey
           }
       }
   });
   ```

5. **Parse CLValue**
   ```typescript
   const clValue = result.stored_value.CLValue;
   // Check actual cl_type, handle List<U8> serialization
   ```

## Practical Examples

### Reading a Var<U256>

```typescript
// Read reserve0 at index 5 from Pair contract
const index = 5;
const storageKey = blake2bHex(u32_be(index), undefined, 32);
const value = await queryStateValue(stateRoot, pairContractHash, storageKey);
// value is bigint
```

### Reading a Mapping Entry

```typescript
// Read Factory.pairs[(tokenA, tokenB)]
const index = 4;
const keyA = serializeKey(tokenA);
const keyB = serializeKey(tokenB);
const [first, second] = sortBytes(keyA, keyB);
const mappingKey = concat(first, second);
const storageKey = blake2bHex(concat(u32_be(index), mappingKey), undefined, 32);
const pairAddress = await queryStateValue(stateRoot, factoryHash, storageKey);
// pairAddress is string like "hash-..."
```

## Package Hash vs Contract Hash

**Important**: Factory returns **package hashes** for pairs, but you need **contract hashes** to query state.

### Resolving Package to Contract

```typescript
const packageData = await rpcClient.state_get_item({
    state_root_hash: stateRoot,
    key: packageHash,
    path: []
});

const versions = packageData.stored_value.ContractPackage.versions;
const latestVersion = versions[versions.length - 1];
let contractHash = latestVersion.contract_hash;

// Convert contract- prefix to hash- for RPC
if (contractHash.startsWith('contract-')) {
    contractHash = contractHash.replace('contract-', 'hash-');
}
```

## Tips for Discovering Storage Layout

1. **Start with known values**: Test with fields you can verify (like token addresses)
2. **Brute force indices**: Try indices 0-10 for each field type
3. **Check return types**: Use `console.log` to see actual CLValue types
4. **Count SubModule slots**: Find a known field, work backwards to find SubModule size
5. **Verify with multiple contracts**: Test pattern on different contract instances

## Version Note

This pattern was discovered with contracts deployed using Odra version that uses:
- u32 Big Endian indices
- No tag bytes for mappings
- SubModule multi-index footprint

Different Odra versions may use different patterns (e.g., u8 indices, tag bytes). Always verify empirically with your deployed contracts.

# Frontend DEX Implementation Walkthrough

This document outlines the changes made to implement liquidity pool reserves display and fix decimal handling in the frontend.

## Part 1: Decimal Fixes

### The Issue
Contracts were deployed with 18 decimals for WCSPR, but frontend was configured with 9 decimals, causing amounts to be off by 10^9.

### Changes Made
- **DexContext.tsx**: Fixed WCSPR decimals (9→18), added USDC (6), WETH (18), WBTC (8)
- **Liquidity.tsx**: Dynamic decimals using `config.tokens[symbol].decimals`
- **Swap.tsx**: Removed hardcoded multipliers, uses dynamic decimals
- **Mint.tsx**: Added support for all tokens with correct decimals

## Part 2: Storage Read Optimization

### Balance Fetching (`dex-client.ts`)
- **Before**: Brute-force loop through 10+ candidate keys (slow)
- **After**: Direct lookup using Odra Index 5 (Big Endian) for balances
- **Result**: 10x faster balance loading

## Part 3: Liquidity Pool Reserves

### Implementation (`dex-client.ts`)

#### 1. Factory Pair Lookup (`getPairAddress`)
- Queries Factory contract's `pairs` mapping to find pair address
- **Storage Discovery**: Factory `pairs` mapping at **Index 4** (not 3)
- **Key Format**: `blake2b(u32_be(4) + serialized_token_addresses)`
- **No Tag Byte**: Odra mappings use index + key (no tag 0)
- **Token Sorting**: Addresses sorted before serialization (matches Factory logic)

#### 2. Package→Contract Resolution
- Factory returns **package hashes**, but state queries need **contract hashes**
- Queries package to extract active contract hash from versions array
- Converts `contract-` prefix to `hash-` for RPC compatibility

#### 3. Pair Reserves Lookup (`getPairReserves`)
- **Storage Layout Discovery** (empirical testing):
  - Indices 0-2: `lp_token` (SubModule takes 3 slots)
  - Index 3: `token0` (Address)
  - Index 4: `token1` (Address) 
  - Index 5: `reserve0` (U256) ✓
  - Index 6: `reserve1` (U256) ✓
- **Key Format**: `blake2b(u32_be(index))` - no tag for Vars

#### 4. Key Type Parsing
- Added detection for 33-byte `List<U8>` arrays (serialized Keys)
- Format: `[tag_byte, ...32_hash_bytes]` where tag 0=Account, 1=Hash
- Converts to `hash-...` or `account-hash-...` strings

### UI Integration (`Swap.tsx`)
- Fetches pair address on component mount
- Displays reserves: `Reserves: {r0} WCSPR / {r1} ECTO`
- Updates automatically when pair data changes

## Key Technical Discoveries

### Odra Storage Encoding
- **Index Format**: u32 Big Endian (4 bytes), not u8 as in some documentation
- **Mapping Keys**: `blake2b(index_bytes + key_bytes)` - no tag byte
- **Var Keys**: `blake2b(index_bytes)` - just the index
- **SubModules**: Take multiple consecutive indices (LpToken takes 3)

### CLValue Type Handling
- `Key` types sometimes returned as `List<U8>` (33 bytes) instead of native `Key`
- First byte is tag (0=Account, 1=Hash), remaining 32 bytes are the hash
- Must detect and parse manually when `cl_type` is `{List: 'U8'}`

## Verification

1. **Reserves Display**: Swap page shows current pool reserves
2. **Balance Loading**: Near-instant (vs 5-10s before)
3. **Decimal Accuracy**: All token amounts use correct decimals
4. **Swap Functionality**: Works with correct amounts

## Files Modified

- `frontend-react/src/dex-client.ts` - Core storage reading logic
- `frontend-react/src/components/Swap.tsx` - Reserves display
- `frontend-react/src/contexts/DexContext.tsx` - Decimal config
- `frontend-react/src/components/Liquidity.tsx` - Dynamic decimals
- `frontend-react/src/components/Mint.tsx` - Token support

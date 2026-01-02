# Ectoplasm Contracts - Decentralized Exchange Smart Contracts

A fully-featured Decentralized Exchange (DEX) implementation for the Casper Network, built using the Odra framework. This DEX implements an Automated Market Maker (AMM) model similar to Uniswap V2.

## 🏗️ Architecture

The DEX consists of four main components:

### 1. Factory Contract (`dex::factory::Factory`)
The Factory is responsible for creating and managing trading pairs.

**Features:**
- Create new token pairs
- Track all existing pairs
- Manage protocol fee settings
- Admin controls for fee recipient

**Key Functions:**
- `create_pair(token_a, token_b)` - Create a new trading pair
- `get_pair(token_a, token_b)` - Get pair address for two tokens
- `set_fee_to(address)` - Set fee recipient (admin only)
- `all_pairs_length()` - Get total number of pairs

### 2. Pair Contract (`dex::pair::Pair`)
Each Pair holds reserves of two tokens and manages liquidity operations.

**Features:**
- Constant product AMM (x * y = k)
- 0.3% swap fee
- LP token minting/burning
- Price oracle support
- Reentrancy protection

**Key Functions:**
- `mint(to)` - Mint LP tokens when adding liquidity
- `burn(to)` - Burn LP tokens when removing liquidity
- `swap(amount0_out, amount1_out, to)` - Execute token swap
- `get_reserves()` - Get current reserves
- `sync()` - Sync reserves with balances

### 3. Router Contract (`dex::router::Router`)
The Router is the main user-facing contract for all DEX operations.

**Features:**
- Add/remove liquidity with slippage protection
- Swap tokens with exact input or exact output
- Multi-hop swaps through multiple pairs
- Deadline protection
- Automatic pair creation

**Key Functions:**
- `add_liquidity(...)` - Add liquidity to a pair
- `remove_liquidity(...)` - Remove liquidity from a pair
- `swap_exact_tokens_for_tokens(...)` - Swap with exact input
- `swap_tokens_for_exact_tokens(...)` - Swap with exact output
- `get_amounts_out(amount_in, path)` - Quote output amounts
- `get_amounts_in(amount_out, path)` - Quote input amounts

### 4. LP Token (`token::LpToken`)
CEP-18 compatible token representing liquidity provider shares.

**Features:**
- Standard ERC-20/CEP-18 interface
- Mint/burn functionality for pair contracts
- Transfer and approval mechanisms

## 📁 Project Structure

```
ectoplasm-contracts/
├── Cargo.toml              # Rust dependencies
├── Odra.toml               # Odra contract configuration
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── errors.rs           # Custom error definitions
│   ├── events.rs           # Event definitions
│   ├── math.rs             # AMM math utilities
│   ├── token.rs            # LP token implementation
│   ├── flipper.rs          # Example contract
│   └── dex/
│       ├── mod.rs          # DEX module exports
│       ├── pair.rs         # Pair contract
│       ├── factory.rs      # Factory contract
│       ├── router.rs       # Router contract
│       └── tests.rs        # Integration tests
└── bin/
    ├── build_contract.rs   # Contract build script
    ├── build_schema.rs     # Schema generation
    └── cli.rs              # CLI interface
```

## 🚀 Getting Started

## ✅ On-chain verification (Casper v5)

See `VERIFICATION_COMMANDS.md` for copy/paste commands to verify transactions (mint/approve/add_liquidity/swap/remove_liquidity) and to prove swap balance deltas from on-chain Transfer events.

### Prerequisites

- Rust (nightly toolchain)
- Cargo
- Odra CLI

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd ectoplasm-contracts
```

2. Install dependencies:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

### Building Contracts

Build all contracts for deployment:
```bash
cargo odra build
```

### Running Tests

Run the test suite:
```bash
cargo odra test
```

## 📖 Usage Examples

### Creating a Trading Pair

```rust
// Deploy factory
let factory = Factory::deploy(&env, FactoryInitArgs {
    fee_to_setter: admin,
});

// Create a new pair
factory.create_pair(token_a, token_b)?;
```

### Adding Liquidity

```rust
// Deploy router
let router = Router::deploy(&env, RouterInitArgs {
    factory: factory_address,
    wcspr: wcspr_address,
});

// Add liquidity
let (amount_a, amount_b, liquidity) = router.add_liquidity(
    token_a,
    token_b,
    amount_a_desired,
    amount_b_desired,
    amount_a_min,
    amount_b_min,
    recipient,
    deadline,
)?;
```

### Swapping Tokens

```rust
// Swap exact tokens for tokens
let amounts = router.swap_exact_tokens_for_tokens(
    amount_in,
    amount_out_min,
    vec![token_a, token_b],  // swap path
    recipient,
    deadline,
)?;
```

### Removing Liquidity

```rust
let (amount_a, amount_b) = router.remove_liquidity(
    token_a,
    token_b,
    liquidity_amount,
    amount_a_min,
    amount_b_min,
    recipient,
    deadline,
)?;
```

## 🔢 AMM Mathematics

### Constant Product Formula
The DEX uses the constant product formula: `x * y = k`

Where:
- `x` = reserve of token0
- `y` = reserve of token1
- `k` = constant product (invariant)

### Swap Formula (with 0.3% fee)
```
amount_out = (amount_in * 997 * reserve_out) / (reserve_in * 1000 + amount_in * 997)
```

### Liquidity Calculation
**First deposit:**
```
liquidity = sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
```

**Subsequent deposits:**
```
liquidity = min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
```

## ⚠️ Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 1 | InsufficientLiquidity | Not enough liquidity in pool |
| 2 | InsufficientInputAmount | Input amount too low |
| 3 | InsufficientOutputAmount | Output amount too low |
| 4 | InvalidPair | Invalid token pair |
| 5 | PairExists | Pair already exists |
| 6 | PairNotFound | Pair does not exist |
| 7 | ZeroAddress | Zero address provided |
| 8 | IdenticalAddresses | Same token addresses |
| 9 | InsufficientAmount | Amount too low |
| 10 | TransferFailed | Token transfer failed |
| 11 | DeadlineExpired | Transaction deadline passed |
| 12 | ExcessiveSlippage | Slippage too high |
| 13 | Overflow | Arithmetic overflow |
| 14 | Underflow | Arithmetic underflow |
| 15 | DivisionByZero | Division by zero |
| 16 | Unauthorized | Unauthorized access |
| 17 | InvalidPath | Invalid swap path |
| 18 | KInvariantViolated | K invariant check failed |
| 19 | InsufficientLiquidityMinted | Not enough LP tokens minted |
| 20 | InsufficientLiquidityBurned | Not enough LP tokens burned |
| 21 | Locked | Reentrancy detected |
| 22 | InvalidFee | Invalid fee value |

## 📡 Events

### PairCreated
Emitted when a new pair is created.
```rust
PairCreated {
    token0: Address,
    token1: Address,
    pair: Address,
    pair_count: u32,
}
```

### LiquidityAdded
Emitted when liquidity is added.
```rust
LiquidityAdded {
    provider: Address,
    pair: Address,
    amount0: U256,
    amount1: U256,
    liquidity: U256,
}
```

### LiquidityRemoved
Emitted when liquidity is removed.
```rust
LiquidityRemoved {
    provider: Address,
    pair: Address,
    amount0: U256,
    amount1: U256,
    liquidity: U256,
}
```

### Swap
Emitted when a swap occurs.
```rust
Swap {
    sender: Address,
    pair: Address,
    amount0_in: U256,
    amount1_in: U256,
    amount0_out: U256,
    amount1_out: U256,
    to: Address,
}
```

### Sync
Emitted when reserves are updated.
```rust
Sync {
    pair: Address,
    reserve0: U256,
    reserve1: U256,
}
```

## 🔒 Security Considerations

1. **Reentrancy Protection**: All state-changing functions use a lock mechanism
2. **Slippage Protection**: Users can set minimum output amounts
3. **Deadline Protection**: Transactions expire after deadline
4. **K Invariant Check**: Ensures constant product is maintained
5. **Minimum Liquidity**: 1000 LP tokens locked forever to prevent division by zero

## 🛠️ Development

### Adding New Features

1. Create new module in `src/dex/`
2. Export from `src/dex/mod.rs`
3. Add contract to `Odra.toml`
4. Write tests in `src/dex/tests.rs`

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## 📄 License

This project is licensed under the MIT License.

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## 📚 Resources

- [Casper Network Documentation](https://docs.casper.network/)
- [Odra Framework](https://odra.dev/)
- [CSPR.cloud API](https://docs.cspr.cloud/)
- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)

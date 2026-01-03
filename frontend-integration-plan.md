# Frontend Integration Plan

## Goal
Create a minimal frontend to demonstrate DEX interaction (Swap, Add Liquidity) using the Casper Wallet for signing.

## Steps
1.  **Setup**: Initialize Vite project (`frontend-example`).
2.  **Adapt Client**: Modify `DexClient` to separate **Deploy Creation** from **Signing/Sending**.
    *   Current: `approveToken(...)` -> creates deploy -> signs with key -> sends.
    *   New: `makeApproveDeploy(...)` -> returns `Deploy` object.
    *   Frontend: Call `makeApproveDeploy`, then use `CasperWallet` to sign, then send.
3.  **UI Implementation**:
    *   Connect Wallet button.
    *   Balance display.
    *   Swap Form (Amount In -> Amount Out).
4.  **Integration**:
    *   Use contract hashes from `deploy-new.out.env`.
    *   Test against local NCTL node (requires Casper Wallet connected to local node).

## Dependencies
- `casper-js-sdk` v5
- `casper-wallet-api` (optional, or just distinct window interaction)

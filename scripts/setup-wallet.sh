#!/bin/bash
# Wallet Setup Script for Ectoplasm Protocol Deployment

set -e

echo "🔐 Ectoplasm Protocol - Wallet Setup"
echo "===================================="
echo ""

# Check if casper-client is installed
if ! command -v casper-client &> /dev/null; then
    echo "❌ casper-client not found!"
    echo "📦 Installing casper-client..."
    cargo install casper-client
fi

echo "✅ casper-client found: $(casper-client --version)"
echo ""

# Create keys directory
KEYS_DIR="$HOME/casper-keys"
if [ -d "$KEYS_DIR" ]; then
    echo "⚠️  Keys directory already exists at $KEYS_DIR"
    read -p "Do you want to create a new wallet? This will backup existing keys. (y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        BACKUP_DIR="$KEYS_DIR.backup.$(date +%s)"
        echo "📦 Backing up existing keys to $BACKUP_DIR"
        mv "$KEYS_DIR" "$BACKUP_DIR"
    else
        echo "✅ Using existing wallet at $KEYS_DIR"
        cat "$KEYS_DIR/public_key_hex"
        echo ""
        echo "👆 This is your account hash. Use it to request testnet funds."
        exit 0
    fi
fi

# Generate new keys
echo "🔑 Generating new key pair..."
mkdir -p "$KEYS_DIR"
casper-client keygen "$KEYS_DIR"

echo ""
echo "✅ Wallet created successfully!"
echo ""
echo "📁 Keys saved to: $KEYS_DIR"
echo "   - secret_key.pem (⚠️  KEEP THIS SECURE!)"
echo "   - public_key.pem"
echo "   - public_key_hex"
echo ""

# Display account hash
echo "🆔 Your Account Hash:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cat "$KEYS_DIR/public_key_hex"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Instructions for getting testnet funds
echo "💰 Next Steps - Get Testnet Funds:"
echo ""
echo "Option 1: Using Casper Wallet (Recommended)"
echo "  1. Install Casper Wallet: https://www.casperwallet.io/"
echo "  2. Import your keys (secret_key.pem)"
echo "  3. Visit: https://testnet.cspr.live/tools/faucet"
echo "  4. Click 'Request tokens'"
echo ""
echo "Option 2: Using CLI (if available)"
echo "  1. Visit: https://testnet.cspr.live/tools/faucet"
echo "  2. Enter your account hash (shown above)"
echo "  3. Request tokens"
echo ""
echo "⚠️  Note: You can only request testnet funds ONCE per account"
echo ""

# Check balance function
echo "📊 To check your balance after funding:"
echo "   ./scripts/check-balance.sh"
echo ""

# Save account info
cat > "$KEYS_DIR/account_info.txt" << EOF
Ectoplasm Protocol Deployment Account
=====================================

Created: $(date)

Account Hash:
$(cat "$KEYS_DIR/public_key_hex")

Public Key (hex):
$(cat "$KEYS_DIR/public_key.pem")

Network: Casper Testnet (casper-test)
Node: http://65.21.235.122:7777

Testnet Faucet: https://testnet.cspr.live/tools/faucet
Block Explorer: https://testnet.cspr.live/

⚠️  SECURITY WARNING:
- NEVER share your secret_key.pem file
- Keep backups in a secure location
- This is for TESTNET ONLY - do not use for mainnet
EOF

echo "📝 Account info saved to: $KEYS_DIR/account_info.txt"
echo ""
echo "✨ Wallet setup complete!"

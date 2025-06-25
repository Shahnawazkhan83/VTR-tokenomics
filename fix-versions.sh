#!/bin/bash

echo "🔧 Fixing Solana version conflicts..."

# Navigate to project root
cd vtr-token 2>/dev/null || echo "Already in project directory"

# Clean everything
echo "🧹 Cleaning previous builds..."
anchor clean 2>/dev/null || true
rm -rf target/ node_modules/ package-lock.json Cargo.lock

# Create compatible root Cargo.toml
echo "📝 Creating root Cargo.toml with resolver = 2..."
cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "programs/*"
]
resolver = "2"

[workspace.dependencies]
anchor-lang = "0.30.1"
anchor-spl = "0.30.1"
solana-program = "~1.18.0"

[profile.release]
overflow-checks = true
lto = "fat"
codegen-units = 1

[profile.release.build-override]
opt-level = 3
incremental = false
codegen-units = 1
EOF

# Update program Cargo.toml
echo "📝 Updating program Cargo.toml..."
cat > programs/vtr-token/Cargo.toml << 'EOF'
[package]
name = "vtr-token"
version = "0.1.0"
description = "VTR Token - Solana SPL Token with advanced tokenomics"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "vtr_token"

[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []

[dependencies]
anchor-lang = { workspace = true }
anchor-spl = { workspace = true }
EOF

# Update package.json
echo "📝 Updating package.json..."
cat > package.json << 'EOF'
{
  "license": "ISC",
  "scripts": {
    "lint:fix": "prettier */*.js \"*/**/*{.js,.ts}\" -w",
    "lint": "prettier */*.js \"*/**/*{.js,.ts}\" --check"
  },
  "dependencies": {
    "@coral-xyz/anchor": "^0.30.1"
  },
  "devDependencies": {
    "@types/bn.js": "^5.1.0",
    "@types/chai": "^4.3.0",
    "@types/mocha": "^9.0.0",
    "@solana/spl-token": "^0.4.13",
    "@solana/web3.js": "^1.95.0",
    "chai": "^4.3.4",
    "mocha": "^9.0.3",
    "prettier": "^2.6.2",
    "ts-mocha": "^10.0.0",
    "typescript": "^5.7.3"
  }
}
EOF

# Update Anchor.toml
echo "📝 Updating Anchor.toml..."
cat > Anchor.toml << 'EOF'
[features]
seeds = false
skip-lint = false

[programs.localnet]
vtr_token = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
EOF

# Install dependencies
echo "📦 Installing compatible dependencies..."
npm install

# Check Anchor version
echo "🔍 Checking Anchor CLI version..."
anchor --version

echo ""
echo "⚠️  IMPORTANT: Make sure your Anchor CLI version matches:"
echo "   Expected: anchor-cli 0.30.1"
echo "   Current:  $(anchor --version)"
echo ""
echo "If versions don't match, update Anchor CLI:"
echo "   cargo install --git https://github.com/coral-xyz/anchor avm --locked --force"
echo "   avm install 0.30.1"
echo "   avm use 0.30.1"
echo ""

# Try building
echo "🔨 Attempting to build..."
if anchor build; then
    echo "✅ Build successful!"
    echo ""
    echo "🚀 Next steps:"
    echo "1. Start validator: solana-test-validator --reset"
    echo "2. Deploy: anchor deploy"
    echo "3. Test: anchor test"
else
    echo "❌ Build failed. Checking Anchor version compatibility..."
    echo ""
    echo "Try these steps:"
    echo "1. Check Anchor CLI version: anchor --version"
    echo "2. If not 0.30.1, install it:"
    echo "   cargo install --git https://github.com/coral-xyz/anchor avm --locked --force"
    echo "   avm install 0.30.1"
    echo "   avm use 0.30.1"
    echo "3. Try building again: anchor build"
fi
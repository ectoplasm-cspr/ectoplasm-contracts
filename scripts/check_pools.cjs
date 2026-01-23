#!/usr/bin/env node

/**
 * Query which liquidity pools exist on the DEX
 */

const { RpcClient, HttpHandler } = require('casper-js-sdk');
const { blake2bHex } = require('blakejs');
require('dotenv').config();

const nodeUrl = process.env.NODE_ADDRESS || 'http://localhost:11101/rpc';
const factoryHash = process.env.FACTORY_CONTRACT_HASH;

const tokens = {
    WCSPR: process.env.WCSPR_PACKAGE_HASH,
    ECTO: process.env.ECTO_PACKAGE_HASH,
    USDC: process.env.USDC_PACKAGE_HASH,
    WETH: process.env.WETH_PACKAGE_HASH,
    WBTC: process.env.WBTC_PACKAGE_HASH,
};

// All possible pairs
const pairs = [
    ['WCSPR', 'ECTO'],
    ['WCSPR', 'USDC'],
    ['WCSPR', 'WETH'],
    ['WCSPR', 'WBTC'],
    ['ECTO', 'USDC'],
    ['ECTO', 'WETH'],
    ['ECTO', 'WBTC'],
    ['USDC', 'WETH'],
    ['USDC', 'WBTC'],
    ['WETH', 'WBTC'],
];

function serializeKey(keyStr) {
    let tag = 1; // Hash
    let clean = keyStr.replace('hash-', '');

    const bytes = new Uint8Array(33);
    bytes[0] = tag;
    const hashBytes = Buffer.from(clean, 'hex');
    bytes.set(hashBytes, 1);
    return bytes;
}

function generateOdraMappingKey(index, keyBytes) {
    const indexBytes = new Uint8Array(4);
    new DataView(indexBytes.buffer).setUint32(0, index, false); // Big Endian

    const combined = new Uint8Array(indexBytes.length + keyBytes.length);
    combined.set(indexBytes);
    combined.set(keyBytes, indexBytes.length);

    return blake2bHex(combined, undefined, 32);
}

async function checkPairExists(tokenA, tokenB) {
    const rpcClient = new RpcClient(new HttpHandler(nodeUrl));

    try {
        // Get state root
        const stateRootResponse = await rpcClient.getStateRootHash();
        const stateRoot = stateRootResponse.state_root_hash || stateRootResponse;

        // Get Factory contract
        const factoryData = await rpcClient.getBlockState(stateRoot, factoryHash, []);
        const stateURef = factoryData.stored_value.Contract.named_keys
            .find(k => k.name === 'state')?.key;

        if (!stateURef) return null;

        // Sort tokens
        const keyA = serializeKey(tokens[tokenA]);
        const keyB = serializeKey(tokens[tokenB]);
        const [first, second] = keyA < keyB ? [keyA, keyB] : [keyB, keyA];

        // Generate mapping key
        const combined = new Uint8Array(first.length + second.length);
        combined.set(first);
        combined.set(second, first.length);

        const storageKey = generateOdraMappingKey(4, combined);

        // Query pair address
        const result = await rpcClient.getDictionaryItemByURef(
            stateRoot,
            stateURef,
            storageKey
        );

        if (result?.stored_value?.CLValue) {
            const bytes = result.stored_value.CLValue.bytes;
            if (bytes && bytes.length === 33) {
                const hashHex = Buffer.from(bytes.slice(1)).toString('hex');
                return `hash-${hashHex}`;
            }
        }

        return null;
    } catch (e) {
        return null;
    }
}

async function main() {
    console.log('🔍 Checking which liquidity pools exist...\n');

    const existingPairs = [];

    for (const [tokenA, tokenB] of pairs) {
        process.stdout.write(`Checking ${tokenA}-${tokenB}... `);
        const pairAddress = await checkPairExists(tokenA, tokenB);

        if (pairAddress) {
            console.log(`✅ EXISTS (${pairAddress.slice(0, 15)}...)`);
            existingPairs.push({ tokenA, tokenB, address: pairAddress });
        } else {
            console.log('❌ Not found');
        }
    }

    console.log(`\n📊 Summary: ${existingPairs.length} pools found\n`);

    if (existingPairs.length > 0) {
        console.log('Existing pools:');
        existingPairs.forEach(p => {
            console.log(`  - ${p.tokenA}-${p.tokenB}: ${p.address}`);
        });
    } else {
        console.log('⚠️  No liquidity pools found. You need to create pairs first!');
    }
}

main().catch(console.error);

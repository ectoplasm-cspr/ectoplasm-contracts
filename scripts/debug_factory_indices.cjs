#!/usr/bin/env node

/**
 * Debug script to find correct indices for all_pairs_length and all_pairs
 */

const { RpcClient, HttpHandler } = require('casper-js-sdk');
const { blake2bHex } = require('blakejs');
require('dotenv').config();

const nodeUrl = process.env.NODE_ADDRESS;
const factoryHash = process.env.FACTORY_CONTRACT_HASH;

const rpcClient = new RpcClient(new HttpHandler(nodeUrl));

function generateOdraVarKey(index) {
    const indexBytes = new Uint8Array(4);
    new DataView(indexBytes.buffer).setUint32(0, index, false); // Big Endian
    return blake2bHex(indexBytes, undefined, 32);
}

async function testIndex(index, description) {
    try {
        const stateRoot = (await rpcClient.getStateRootHashLatest()).stateRootHash;
        const key = generateOdraVarKey(index);

        const contractData = await rpcClient.getBlockState(stateRoot, factoryHash, []);
        const stateURef = contractData.stored_value.Contract.named_keys
            .find(k => k.name === 'state')?.key;

        const result = await rpcClient.getDictionaryItemByURef(stateRoot, stateURef, key);

        if (result?.stored_value?.CLValue) {
            const parsed = result.stored_value.CLValue.parsed;
            console.log(`✅ Index ${index} (${description}): ${JSON.stringify(parsed)}`);
            return parsed;
        } else {
            console.log(`❌ Index ${index} (${description}): Not found`);
            return null;
        }
    } catch (e) {
        console.log(`❌ Index ${index} (${description}): Error - ${e.message}`);
        return null;
    }
}

async function main() {
    console.log('🔍 Testing Factory storage indices...\n');
    console.log(`Factory: ${factoryHash}\n`);

    // Test indices 0-10
    await testIndex(0, 'fee_to');
    await testIndex(1, 'fee_to_setter');
    await testIndex(2, 'pair_factory');
    await testIndex(3, 'pairs (mapping - skip)');
    await testIndex(4, 'all_pairs (mapping - skip)');
    await testIndex(5, 'all_pairs_length');
    await testIndex(6, 'unknown');
    await testIndex(7, 'unknown');

    console.log('\n📝 Based on Factory struct:');
    console.log('  fee_to: Var<Option<Address>>');
    console.log('  fee_to_setter: Var<Address>');
    console.log('  pair_factory: Var<Address>');
    console.log('  pairs: Mapping<(Address, Address), Address>');
    console.log('  all_pairs: Mapping<u32, Address>');
    console.log('  all_pairs_length: Var<u32>');
}

main().catch(console.error);

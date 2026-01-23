
const fs = require('fs');
const { CasperClient, Contracts, Keys } = require('casper-js-sdk');

const NODE_URL = 'http://127.0.0.1:11101/rpc';
const client = new CasperClient(NODE_URL);

async function main() {
    // Read the contract hash from the env file or hardcoded
    // From previous view_file: FACTORY_CONTRACT_HASH=hash-464e54c4e050fb995ac7bb3a9a4eef08f0b9010daf490ceb062ab5f7a8149263
    const factoryHash = 'hash-464e54c4e050fb995ac7bb3a9a4eef08f0b9010daf490ceb062ab5f7a8149263';

    console.log(`Querying Factory Contract: ${factoryHash}`);

    const stateRoot = await client.nodeClient.getStateRootHash();
    const result = await client.nodeClient.getBlockState(
        stateRoot,
        factoryHash,
        []
    );

    console.log("Contract Data:");
    // console.log(JSON.stringify(result, null, 2));

    if (result.Contract) {
        console.log("Named Keys:");
        result.Contract.named_keys.forEach(k => {
            console.log(`- ${k.name}: ${k.key}`);
        });
    } else {
        console.log("Result is not a Contract:", result);
    }
}

main().catch(console.error);

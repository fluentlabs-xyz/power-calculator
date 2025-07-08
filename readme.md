# Power Calculator

This Rust smart contract demonstrates Rust contract verification using Block Scout and creating reproducible builds with Docker.

## Building the Contract

To build the contract, run:

```sh
cargo build --release
```

If the required image is not present in the Client Base Build, it will be automatically downloaded, and artifacts will be placed in the `out` directory.

## Deploying the Contract

Deploy the contract using `gblend`. Example:

```sh
gblend deploy \
  --private-key YOUR_PRIVATE_KEY \
  out/PowerCalculator.wasm/lib.wasm \
  --rpc https://rpc.dev.gblend.xyz \
  --chain-id 20993
```

## Contract Verification

To submit the contract for verification, run:

```sh
curl -X POST https://blockscout.dev.gblend.xyz/api/v2/smart-contracts/CONTRACT_ADDRESS/verification/via/fluent \
  -H "Content-Type: application/json" \
  -d @verification-request.json \
  -v
```

> **Note:** Remember to update the `CONTRACT_ADDRESS` and the `git commit` hash in `verification-request.json`.

import { ApiPromise, WsProvider } from '@polkadot/api';
import Keyring from '@polkadot/keyring';
import { hexToU8a, isHex } from '@polkadot/util';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import { CONFIG } from './config.js';

function hexAddressToBytes20(hex: string): Uint8Array {
  if (!isHex(hex)) {
    throw new Error(`Invalid hex: ${hex}`);
  }
  const bytes = hexToU8a(hex);
  if (bytes.length === 20) return bytes;
  if (bytes.length === 32) {
    return bytes.slice(12);
  }
  throw new Error(`Address must be 20 bytes, got ${bytes.length}`);
}

async function main() {
  await cryptoWaitReady();

  const provider = new WsProvider(CONFIG.chopsticks);
  const api = await ApiPromise.create({ provider: provider as unknown as any });

  if (!api.tx.xdex || !api.tx.xdex.buildErc20Transfer) {
    throw new Error('xdex.buildErc20Transfer extrinsic not found in metadata');
  }

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri(CONFIG.substrateAccount);

  const token = hexAddressToBytes20(CONFIG.tokenAddress);
  const recipient = hexAddressToBytes20(CONFIG.recipient);
  const amount = CONFIG.amount.toString(); // u128 as string

  const nonce = 0; // u64
  const gasLimit = 100000; // u64
  const maxFeePerGas = String(30_000_000_000n); // u128
  const maxPriorityFeePerGas = String(2_000_000_000n); // u128
  const chainId = CONFIG.chainId; // u64

  console.log('Calling xdex.buildErc20Transfer ...');
  const unsub = await api.tx.xdex
    .buildErc20Transfer(
      Array.from(token),
      Array.from(recipient),
      amount,
      nonce,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
    )
    .signAndSend(alice, ({ status, events }) => {
      if (status.isInBlock) {
        console.log(`Included in block ${status.asInBlock.toString()}`);
      }
      if (status.isFinalized) {
        console.log(`Finalized in block ${status.asFinalized.toString()}`);
        for (const { event } of events) {
          const { section, method } = event;
          if (section === 'xdex' && method === 'Erc20TransferBuilt') {
            console.log('Erc20TransferBuilt:', event.toHuman());
          }
        }
        unsub();
      }
    });

  // After finalized, query storage for confirmation
  await api.rpc.chain.subscribeFinalizedHeads(async () => {
    const who = alice.address;
    if (
      api.query.xdex &&
      api.query.xdex.transactionCount &&
      api.query.xdex.transactionHashes
    ) {
      const count: any = await api.query.xdex.transactionCount(who);
      console.log('TransactionCount:', count.toString());
      if (count.toNumber() > 0) {
        const lastIdx = count.toNumber() - 1;
        const hashOpt: any = await api.query.xdex.transactionHashes(
          who,
          lastIdx,
        );
        console.log(
          'Last transaction hash (Blake2-256 of RLP):',
          hashOpt.toString(),
        );
        process.exit(0);
      }
    } else {
      console.warn('xdex storage not available on this chain');
      process.exit(0);
    }
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

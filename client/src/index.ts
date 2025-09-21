import { ApiPromise, WsProvider, type SubmittableResult } from '@polkadot/api';
import Keyring from '@polkadot/keyring';
import { hexToU8a, isHex, u8aToHex } from '@polkadot/util';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import { CONFIG } from './config.js';
import { createPublicClient, createWalletClient, http, getAddress } from 'viem';
import type { Hex } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { sepolia } from 'viem/chains';

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
  const xdex = api.tx.xdex!;

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri(CONFIG.substrateAccount);

  // viem setup (Sepolia via Infura)
  const account = privateKeyToAccount(CONFIG.evmPrivateKey as Hex);
  const publicClient = createPublicClient({
    chain: sepolia,
    transport: http(CONFIG.sepoliaRpc),
  });
  const walletClient = createWalletClient({
    account,
    chain: sepolia,
    transport: http(CONFIG.sepoliaRpc),
  });

  // Prepare params
  const token = hexAddressToBytes20(CONFIG.tokenAddress);
  const recipient = hexAddressToBytes20(CONFIG.recipient);
  const amountU128String = CONFIG.amount.toString(); // for pallet (u128)
  const chainId = CONFIG.chainId; // u64

  // Resolve live EVM params (no calldata encoding in FE)
  const evmNonce = await publicClient.getTransactionCount({
    address: account.address,
    blockTag: 'pending',
  });
  const fees = await publicClient.estimateFeesPerGas();
  const maxFeePerGasBig = fees.maxFeePerGas ?? 30_000_000_000n;
  const maxPriorityFeePerGasBig = fees.maxPriorityFeePerGas ?? 2_000_000_000n;

  // Map to pallet types
  const nonce = Number(evmNonce);
  const gasLimit = 100000; // conservative default; not estimated on FE
  const maxFeePerGas = String(maxFeePerGasBig);
  const maxPriorityFeePerGas = String(maxPriorityFeePerGasBig);

  console.log('Calling xdex.buildErc20Transfer ...');
  let builtCalldata: Hex | null = null;
  await new Promise<void>(async (resolve) => {
    const build = xdex.buildErc20Transfer;
    if (!build) throw new Error('buildErc20Transfer not available');
    const unsub = await build(
      Array.from(token),
      Array.from(recipient),
      amountU128String,
      nonce,
      gasLimit,
      maxFeePerGas,
      maxPriorityFeePerGas,
      chainId,
    ).signAndSend(alice, ({ status, events }: SubmittableResult) => {
      if (status.isInBlock) {
        console.log(`Included in block ${status.asInBlock.toString()}`);
      }
      for (const { event } of events) {
        const { section, method } = event;
        if (section === 'xdex' && method === 'Erc20TransferBuilt') {
          const dataVec: any[] = (event as any).data as any[];
          const last = dataVec[dataVec.length - 1];
          let hex: string | null = null;
          if (last && typeof last.toHex === 'function') hex = last.toHex();
          else if (last && typeof last.toU8a === 'function')
            hex = u8aToHex(last.toU8a());
          else if (Array.isArray(last))
            hex = u8aToHex(Uint8Array.from(last as number[]));
          else if (typeof last === 'string' && isHex(last)) hex = last;
          if (hex && isHex(hex)) {
            builtCalldata = hex as Hex;
          }
          console.log('Erc20TransferBuilt (calldata extracted)');
        }
      }
      if (status.isFinalized) {
        console.log(`Finalized in block ${status.asFinalized.toString()}`);
        unsub();
        resolve();
      }
    });
  });

  // After finalized, broadcast via viem using calldata from pallet
  console.log('Broadcasting EVM tx to Sepolia ...');
  if (!builtCalldata) throw new Error('Missing calldata from pallet event');
  const estimatedGas = await publicClient.estimateGas({
    account: account.address,
    to: getAddress(CONFIG.tokenAddress),
    data: builtCalldata,
    value: 0n,
  });
  const txHash = await walletClient.sendTransaction({
    to: getAddress(CONFIG.tokenAddress),
    data: builtCalldata,
    value: 0n,
    nonce: evmNonce,
    gas: estimatedGas,
    maxFeePerGas: maxFeePerGasBig,
    maxPriorityFeePerGas: maxPriorityFeePerGasBig,
    chain: sepolia,
    account,
  });
  console.log('Sepolia tx hash:', txHash);

  const receipt = await publicClient.waitForTransactionReceipt({
    hash: txHash,
  });
  console.log('Sepolia tx receipt status:', receipt.status);

  // Cleanly disconnect and exit to avoid hanging the terminal
  try {
    await api.disconnect();
  } catch {}
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

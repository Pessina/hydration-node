import { ApiPromise, WsProvider, type SubmittableResult } from '@polkadot/api';
import Keyring from '@polkadot/keyring';
import { u8aToHex } from '@polkadot/util';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import { CONFIG } from './config.js';
import {
  createPublicClient,
  createWalletClient,
  http,
  hexToBytes,
  keccak256,
  getAddress,
  encodeFunctionData,
  serializeTransaction,
} from 'viem';
import type { Hex } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { sepolia } from 'viem/chains';
import * as noble from '@noble/secp256k1';

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

  const token = hexToBytes(getAddress(CONFIG.tokenAddress));
  const recipient = hexToBytes(getAddress(CONFIG.recipient));
  const amountU128String = CONFIG.amount.toString();

  const evmNonce = await publicClient.getTransactionCount({
    address: account.address,
    blockTag: 'pending',
  });
  const nonce = Number(evmNonce);

  const erc20Abi = [
    {
      type: 'function',
      name: 'transfer',
      stateMutability: 'nonpayable',
      inputs: [
        { name: 'to', type: 'address' },
        { name: 'amount', type: 'uint256' },
      ],
      outputs: [{ type: 'bool' }],
    },
  ] as const;
  const calldataLocal = encodeFunctionData({
    abi: erc20Abi,
    functionName: 'transfer',
    args: [getAddress(CONFIG.recipient), BigInt(CONFIG.amount)],
  }) as Hex;
  const [fees, estimatedGas] = await Promise.all([
    publicClient.estimateFeesPerGas(),
    publicClient.estimateGas({
      account: account.address,
      to: getAddress(CONFIG.tokenAddress),
      data: calldataLocal,
      value: 0n,
    }),
  ]);
  const gasLimit = Number(estimatedGas);
  const maxFeePerGas = (fees.maxFeePerGas ?? 30_000_000_000n).toString();
  const maxPriorityFeePerGas = (
    fees.maxPriorityFeePerGas ?? 2_000_000_000n
  ).toString();

  let builtRlp: Hex | null = null;
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
      CONFIG.chainId,
    ).signAndSend(alice, ({ status, events }: SubmittableResult) => {
      for (const { event } of events) {
        const { section, method } = event;
        if (section === 'xdex' && method === 'Erc20TransferBuilt') {
          const dataVec: any[] = (event as any).data as any[];
          const rlpRaw = dataVec[dataVec.length - 1];
          builtRlp = (rlpRaw as any).toHex() as Hex;
        }
      }
      if (status.isFinalized) {
        unsub();
        resolve();
      }
    });
  });

  if (!builtRlp) throw new Error('Missing RLP from pallet event');

  const viemTx = {
    type: 'eip1559' as const,
    chainId: CONFIG.chainId,
    nonce,
    maxFeePerGas: fees.maxFeePerGas ?? 30_000_000_000n,
    maxPriorityFeePerGas: fees.maxPriorityFeePerGas ?? 2_000_000_000n,
    gas: estimatedGas,
    to: getAddress(CONFIG.tokenAddress),
    value: 0n,
    data: calldataLocal,
    accessList: [],
  };
  const viemUnsignedRlp = serializeTransaction(viemTx);
  if (viemUnsignedRlp.toLowerCase() !== (builtRlp as string).toLowerCase()) {
    throw new Error('Unsigned RLP mismatch between viem and xDex');
  }

  const msgHash = keccak256(builtRlp);
  const msgHashBytes = hexToBytes(msgHash);
  const privBytes = hexToBytes(CONFIG.evmPrivateKey as Hex);
  const [signature, recid] = (await noble.sign(msgHashBytes, privBytes, {
    recovered: true,
    der: false,
  })) as unknown as [Uint8Array, number];
  const rHex = u8aToHex(signature.slice(0, 32)) as Hex;
  const sHex = u8aToHex(signature.slice(32, 64)) as Hex;

  const signedRaw = serializeTransaction(viemTx, {
    yParity: recid as 0 | 1,
    r: rHex,
    s: sHex,
  });
  const txHash = await walletClient.sendRawTransaction({
    serializedTransaction: signedRaw,
  });
  console.log('Sepolia tx hash:', txHash);

  const receipt = await publicClient.waitForTransactionReceipt({
    hash: txHash,
  });
  console.log('Sepolia tx receipt status:', receipt.status);

  try {
    await api.disconnect();
  } catch {}
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

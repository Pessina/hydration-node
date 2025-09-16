import 'dotenv/config';
import { z } from 'zod';

const EnvSchema = z.object({
  INFURA_KEY: z.string().min(1),
  EVM_PRIVATE_KEY: z.string().regex(/^0x[0-9a-fA-F]{64}$/),
});

const env = EnvSchema.parse({
  INFURA_KEY: process.env.INFURA_KEY,
  EVM_PRIVATE_KEY: process.env.EVM_PRIVATE_KEY,
});

export const CONFIG = {
  // Chopsticks WebSocket
  chopsticks: 'ws://localhost:8000',

  // Sepolia RPC
  sepoliaRpc: `https://sepolia.infura.io/v3/${env.INFURA_KEY}`,

  // Fixed ERC20 transfer parameters
  tokenAddress: '0xbe72E441BF55620febc26715db68d3494213D8Cb', // LINK on Sepolia
  recipient: '0x4174678c78fEaFd778c1ff319D5D326701449b25',
  amount: BigInt('56'),

  // EVM private key for signing the transaction
  evmPrivateKey: env.EVM_PRIVATE_KEY,

  // Substrate account for Chopsticks (Alice - pre-funded)
  substrateAccount: '//Alice',

  // Sepolia chain ID
  chainId: 11155111,
} as const;

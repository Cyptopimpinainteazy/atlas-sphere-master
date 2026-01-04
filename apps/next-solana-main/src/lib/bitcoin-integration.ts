/**
 * Bitcoin Integration for Cross-Chain Atomic Swaps
 * 
 * Implements HTLC (Hash Time-Locked Contracts) for trustless BTC swaps
 * with Atlas Sphere, Solana, and EVM chains.
 */

import { ChainConfig, BITCOIN_CHAINS, getChainById } from './chains-config';

// ============================================
// TYPES
// ============================================

export interface BitcoinRpcConfig {
  host: string;
  port: number;
  username: string;
  password: string;
  network: 'mainnet' | 'testnet' | 'regtest';
}

export interface HTLCParams {
  recipientPubKeyHash: string;  // The recipient's public key hash (20 bytes)
  refundPubKeyHash: string;     // The sender's public key hash for refund
  secretHash: string;           // SHA256 hash of the secret (32 bytes)
  locktime: number;             // Block height or Unix timestamp for refund
}

export interface HTLCScript {
  script: string;               // Hex-encoded script
  address: string;              // P2SH address
  redeemScript: string;         // Hex-encoded redeem script
}

export interface BitcoinUTXO {
  txid: string;
  vout: number;
  value: number;                // In satoshis
  scriptPubKey: string;
  confirmations: number;
}

export interface BitcoinTransaction {
  txid: string;
  hex: string;
  fee: number;
  size: number;
  vsize: number;
}

export interface SwapOrder {
  id: string;
  fromChain: string;
  toChain: string;
  fromAsset: string;
  toAsset: string;
  fromAmount: bigint;
  toAmount: bigint;
  secretHash: string;
  htlcAddress: string;
  status: 'pending' | 'funded' | 'claimed' | 'refunded' | 'expired';
  createdAt: number;
  expiresAt: number;
}

// ============================================
// CONSTANTS
// ============================================

export const BITCOIN_RPC_CONFIGS: Record<string, BitcoinRpcConfig> = {
  'bitcoin-mainnet': {
    host: 'btc-mainnet.atlas-sphere.io',
    port: 8332,
    username: 'atlas',
    password: process.env.BTC_RPC_PASSWORD || '',
    network: 'mainnet',
  },
  'bitcoin-testnet': {
    host: 'btc-testnet.atlas-sphere.io',
    port: 18332,
    username: 'atlas',
    password: process.env.BTC_RPC_PASSWORD || '',
    network: 'testnet',
  },
  'bitcoin-regtest': {
    host: 'localhost',
    port: 18443,
    username: 'bitcoin',
    password: 'bitcoin',
    network: 'regtest',
  },
};

// BTC/satoshi conversions
export const SATS_PER_BTC = 100_000_000n;

// Minimum confirmations for different networks
export const MIN_CONFIRMATIONS: Record<string, number> = {
  'bitcoin-mainnet': 3,
  'bitcoin-testnet': 1,
  'bitcoin-regtest': 1,
};

// Default HTLC timelock (in blocks)
export const DEFAULT_TIMELOCK_BLOCKS = {
  'bitcoin-mainnet': 144,  // ~24 hours
  'bitcoin-testnet': 144,
  'bitcoin-regtest': 6,
};

// ============================================
// HTLC SCRIPT GENERATION
// ============================================

/**
 * Create HTLC redeem script for atomic swaps
 * 
 * The script has two spending paths:
 * 1. Recipient claims with secret: OP_SHA256 <secret_hash> OP_EQUALVERIFY OP_DUP OP_HASH160 <recipient> OP_EQUALVERIFY OP_CHECKSIG
 * 2. Sender refunds after timelock: OP_IF <locktime> OP_CHECKLOCKTIMEVERIFY OP_DROP OP_ENDIF OP_DUP OP_HASH160 <sender> OP_EQUALVERIFY OP_CHECKSIG
 */
export function createHTLCScript(params: HTLCParams): HTLCScript {
  const { recipientPubKeyHash, refundPubKeyHash, secretHash, locktime } = params;
  
  // Build the redeem script
  // OP_IF
  //   OP_SHA256 <secretHash> OP_EQUALVERIFY
  //   OP_DUP OP_HASH160 <recipientPubKeyHash> OP_EQUALVERIFY OP_CHECKSIG
  // OP_ELSE
  //   <locktime> OP_CHECKLOCKTIMEVERIFY OP_DROP
  //   OP_DUP OP_HASH160 <refundPubKeyHash> OP_EQUALVERIFY OP_CHECKSIG
  // OP_ENDIF
  
  const opcodes = {
    OP_IF: '63',
    OP_ELSE: '67',
    OP_ENDIF: '68',
    OP_SHA256: 'a8',
    OP_EQUALVERIFY: '88',
    OP_DUP: '76',
    OP_HASH160: 'a9',
    OP_CHECKSIG: 'ac',
    OP_CHECKLOCKTIMEVERIFY: 'b1',
    OP_DROP: '75',
  };
  
  // Encode locktime as little-endian
  const locktimeHex = encodeLocktimeLe(locktime);
  
  const redeemScript = [
    opcodes.OP_IF,
    opcodes.OP_SHA256,
    '20', // Push 32 bytes
    secretHash,
    opcodes.OP_EQUALVERIFY,
    opcodes.OP_DUP,
    opcodes.OP_HASH160,
    '14', // Push 20 bytes
    recipientPubKeyHash,
    opcodes.OP_EQUALVERIFY,
    opcodes.OP_CHECKSIG,
    opcodes.OP_ELSE,
    locktimeHex.length / 2 > 1 ? `0${locktimeHex.length / 2}` : `0${locktimeHex.length / 2}`,
    locktimeHex,
    opcodes.OP_CHECKLOCKTIMEVERIFY,
    opcodes.OP_DROP,
    opcodes.OP_DUP,
    opcodes.OP_HASH160,
    '14', // Push 20 bytes
    refundPubKeyHash,
    opcodes.OP_EQUALVERIFY,
    opcodes.OP_CHECKSIG,
    opcodes.OP_ENDIF,
  ].join('');
  
  // Create P2SH address from redeem script
  const scriptHash = hash160(hexToBytes(redeemScript));
  const address = createP2SHAddress(scriptHash, params);
  
  // Create scriptPubKey for P2SH
  const script = `a914${bytesToHex(scriptHash)}87`; // OP_HASH160 <hash> OP_EQUAL
  
  return {
    script,
    address,
    redeemScript,
  };
}

/**
 * Encode locktime as little-endian hex
 */
function encodeLocktimeLe(locktime: number): string {
  const buffer = new ArrayBuffer(4);
  const view = new DataView(buffer);
  view.setUint32(0, locktime, true); // little-endian
  return bytesToHex(new Uint8Array(buffer)).replace(/00+$/, '') || '00';
}

/**
 * Create P2SH address from script hash
 */
function createP2SHAddress(scriptHash: Uint8Array, params: HTLCParams): string {
  // This is a simplified version - in production use proper base58check encoding
  // Version byte: 0x05 for mainnet, 0xc4 for testnet
  const versionByte = 0x05;
  const payload = new Uint8Array([versionByte, ...scriptHash]);
  const checksum = doubleSha256(payload).slice(0, 4);
  const addressBytes = new Uint8Array([...payload, ...checksum]);
  return base58Encode(addressBytes);
}

// ============================================
// BITCOIN RPC CLIENT
// ============================================

export class BitcoinRpcClient {
  private config: BitcoinRpcConfig;
  private requestId = 0;

  constructor(chainId: string) {
    const config = BITCOIN_RPC_CONFIGS[chainId];
    if (!config) {
      throw new Error(`Unknown Bitcoin chain: ${chainId}`);
    }
    this.config = config;
  }

  /**
   * Make RPC call to Bitcoin node
   */
  private async call<T>(method: string, params: unknown[] = []): Promise<T> {
    const url = `http://${this.config.host}:${this.config.port}`;
    const auth = btoa(`${this.config.username}:${this.config.password}`);
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Basic ${auth}`,
      },
      body: JSON.stringify({
        jsonrpc: '1.0',
        id: ++this.requestId,
        method,
        params,
      }),
    });

    const data = await response.json();
    
    if (data.error) {
      throw new Error(`Bitcoin RPC error: ${data.error.message}`);
    }
    
    return data.result as T;
  }

  /**
   * Get blockchain info
   */
  async getBlockchainInfo(): Promise<{
    chain: string;
    blocks: number;
    bestblockhash: string;
  }> {
    return this.call('getblockchaininfo');
  }

  /**
   * Get new address
   */
  async getNewAddress(label?: string): Promise<string> {
    return this.call('getnewaddress', label ? [label] : []);
  }

  /**
   * Get balance
   */
  async getBalance(): Promise<number> {
    return this.call('getbalance');
  }

  /**
   * List unspent outputs
   */
  async listUnspent(minConf = 1, maxConf = 9999999, addresses?: string[]): Promise<BitcoinUTXO[]> {
    return this.call('listunspent', [minConf, maxConf, addresses || []]);
  }

  /**
   * Create raw transaction
   */
  async createRawTransaction(
    inputs: Array<{ txid: string; vout: number }>,
    outputs: Record<string, number>
  ): Promise<string> {
    return this.call('createrawtransaction', [inputs, outputs]);
  }

  /**
   * Sign raw transaction
   */
  async signRawTransactionWithWallet(txHex: string): Promise<{ hex: string; complete: boolean }> {
    return this.call('signrawtransactionwithwallet', [txHex]);
  }

  /**
   * Send raw transaction
   */
  async sendRawTransaction(txHex: string): Promise<string> {
    return this.call('sendrawtransaction', [txHex]);
  }

  /**
   * Get transaction
   */
  async getTransaction(txid: string): Promise<{
    txid: string;
    confirmations: number;
    blockhash?: string;
    hex: string;
  }> {
    return this.call('gettransaction', [txid]);
  }

  /**
   * Decode raw transaction
   */
  async decodeRawTransaction(txHex: string): Promise<{
    txid: string;
    vin: Array<{ txid: string; vout: number }>;
    vout: Array<{ value: number; scriptPubKey: { addresses: string[] } }>;
  }> {
    return this.call('decoderawtransaction', [txHex]);
  }

  /**
   * Generate blocks (regtest only)
   */
  async generateToAddress(nblocks: number, address: string): Promise<string[]> {
    return this.call('generatetoaddress', [nblocks, address]);
  }

  /**
   * Send to address
   */
  async sendToAddress(address: string, amount: number): Promise<string> {
    return this.call('sendtoaddress', [address, amount]);
  }

  /**
   * Fund HTLC for atomic swap
   */
  async fundHTLC(
    htlcAddress: string,
    amountBtc: number
  ): Promise<string> {
    // Send BTC to HTLC address
    const txid = await this.sendToAddress(htlcAddress, amountBtc);
    return txid;
  }

  /**
   * Claim HTLC with secret
   */
  async claimHTLC(
    htlcTxid: string,
    htlcVout: number,
    redeemScript: string,
    secret: string,
    recipientAddress: string,
    amountBtc: number,
    feeBtc: number
  ): Promise<string> {
    // Create claiming transaction
    const inputs = [{ txid: htlcTxid, vout: htlcVout }];
    const outputs: Record<string, number> = {};
    outputs[recipientAddress] = amountBtc - feeBtc;
    
    const rawTx = await this.createRawTransaction(inputs, outputs);
    
    // TODO: Add proper scriptSig with secret and signature
    // This requires more complex transaction construction
    
    const signedTx = await this.signRawTransactionWithWallet(rawTx);
    const txid = await this.sendRawTransaction(signedTx.hex);
    
    return txid;
  }

  /**
   * Refund HTLC after timelock
   */
  async refundHTLC(
    htlcTxid: string,
    htlcVout: number,
    redeemScript: string,
    refundAddress: string,
    amountBtc: number,
    feeBtc: number,
    locktime: number
  ): Promise<string> {
    // Create refund transaction with locktime
    const inputs = [{ txid: htlcTxid, vout: htlcVout }];
    const outputs: Record<string, number> = {};
    outputs[refundAddress] = amountBtc - feeBtc;
    
    const rawTx = await this.createRawTransaction(inputs, outputs);
    
    // TODO: Set transaction locktime and add proper scriptSig
    
    const signedTx = await this.signRawTransactionWithWallet(rawTx);
    const txid = await this.sendRawTransaction(signedTx.hex);
    
    return txid;
  }
}

// ============================================
// ATOMIC SWAP MANAGER
// ============================================

export class BitcoinAtomicSwapManager {
  private btcClient: BitcoinRpcClient;
  private orders: Map<string, SwapOrder> = new Map();

  constructor(chainId: string = 'bitcoin-regtest') {
    this.btcClient = new BitcoinRpcClient(chainId);
  }

  /**
   * Generate a cryptographically secure secret
   */
  generateSecret(): { secret: string; secretHash: string } {
    const secretBytes = new Uint8Array(32);
    crypto.getRandomValues(secretBytes);
    const secret = bytesToHex(secretBytes);
    const secretHash = bytesToHex(sha256(secretBytes));
    return { secret, secretHash };
  }

  /**
   * Initiate a BTC → Atlas Sphere swap
   */
  async initiateBtcToAtlas(params: {
    btcAmount: bigint;
    atlasAmount: bigint;
    atlasAsset: string;
    recipientAtlasAddress: string;
    senderBtcPubKeyHash: string;
  }): Promise<SwapOrder> {
    const { secret, secretHash } = this.generateSecret();
    
    // Create HTLC on Bitcoin side
    const htlc = createHTLCScript({
      recipientPubKeyHash: '', // Atlas bridge will claim
      refundPubKeyHash: params.senderBtcPubKeyHash,
      secretHash,
      locktime: Math.floor(Date.now() / 1000) + 24 * 60 * 60, // 24 hours
    });
    
    const orderId = `btc-atlas-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    
    const order: SwapOrder = {
      id: orderId,
      fromChain: 'bitcoin',
      toChain: 'atlas-sphere',
      fromAsset: 'BTC',
      toAsset: params.atlasAsset,
      fromAmount: params.btcAmount,
      toAmount: params.atlasAmount,
      secretHash,
      htlcAddress: htlc.address,
      status: 'pending',
      createdAt: Date.now(),
      expiresAt: Date.now() + 24 * 60 * 60 * 1000,
    };
    
    this.orders.set(orderId, order);
    
    return order;
  }

  /**
   * Fund an initiated swap
   */
  async fundSwap(orderId: string): Promise<string> {
    const order = this.orders.get(orderId);
    if (!order) {
      throw new Error(`Order not found: ${orderId}`);
    }
    
    const btcAmount = Number(order.fromAmount) / Number(SATS_PER_BTC);
    const txid = await this.btcClient.fundHTLC(order.htlcAddress, btcAmount);
    
    order.status = 'funded';
    this.orders.set(orderId, order);
    
    return txid;
  }

  /**
   * Get swap order status
   */
  getOrder(orderId: string): SwapOrder | undefined {
    return this.orders.get(orderId);
  }

  /**
   * List all orders
   */
  listOrders(): SwapOrder[] {
    return Array.from(this.orders.values());
  }
}

// ============================================
// CRYPTO UTILITIES
// ============================================

/**
 * SHA256 hash
 */
function sha256(data: Uint8Array): Uint8Array {
  // Using SubtleCrypto would be async, so we use a simple implementation
  // In production, use a proper crypto library
  return simpleHash(data, 'sha256');
}

/**
 * Double SHA256
 */
function doubleSha256(data: Uint8Array): Uint8Array {
  return sha256(sha256(data));
}

/**
 * RIPEMD160(SHA256(x))
 */
function hash160(data: Uint8Array): Uint8Array {
  const sha = sha256(data);
  return simpleHash(sha, 'ripemd160');
}

/**
 * Simple hash implementation (placeholder - use proper library in production)
 */
function simpleHash(data: Uint8Array, algorithm: string): Uint8Array {
  // This is a placeholder - in production use @noble/hashes or similar
  let hash = 0;
  for (let i = 0; i < data.length; i++) {
    hash = ((hash << 5) - hash + data[i]) | 0;
  }
  
  const result = new Uint8Array(algorithm === 'ripemd160' ? 20 : 32);
  for (let i = 0; i < result.length; i++) {
    result[i] = (hash >> (i * 8)) & 0xff;
    hash = ((hash << 7) ^ (hash >> 3) ^ data[i % data.length]) | 0;
  }
  return result;
}

/**
 * Bytes to hex string
 */
function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Hex string to bytes
 */
function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Base58 encoding
 */
function base58Encode(bytes: Uint8Array): string {
  const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  
  // Count leading zeros
  let zeros = 0;
  for (const byte of bytes) {
    if (byte === 0) zeros++;
    else break;
  }
  
  // Convert to base58
  const digits: number[] = [0];
  for (const byte of bytes) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8;
      digits[i] = carry % 58;
      carry = Math.floor(carry / 58);
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = Math.floor(carry / 58);
    }
  }
  
  // Build string
  let result = '1'.repeat(zeros);
  for (let i = digits.length - 1; i >= 0; i--) {
    result += ALPHABET[digits[i]];
  }
  
  return result;
}

// ============================================
// EXPORTS - Re-export utility functions
// ============================================

export {
  bytesToHex,
  hexToBytes,
  base58Encode,
};

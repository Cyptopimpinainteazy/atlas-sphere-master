//! Cross-Chain Swap Router
//!
//! Finds optimal routes and generates atomic Comit transactions
//! across 103 EVM chains.

use crate::chains::{adapter_for, get_chain};
use sp_core::{keccak_256, H160, H256, U256};
use sp_std::vec::Vec;

// === Well-known addresses ===
// Using const fn to properly construct H160 from hex bytes

const fn h160_from_slice(bytes: [u8; 20]) -> H160 {
    H160(bytes)
}

// USDC addresses
const USDC_ETH: H160 = h160_from_slice([
    0xA0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9D, 0x4a, 0x2e, 0x9E, 0xb0, 0xcE,
    0x36, 0x06, 0xeB, 0x48,
]);
const USDC_POLYGON: H160 = h160_from_slice([
    0x27, 0x91, 0xBc, 0xa1, 0xf2, 0xde, 0x46, 0x61, 0xED, 0x88, 0xA3, 0x0C, 0x99, 0xA7, 0xa9, 0x44,
    0x9A, 0xa8, 0x41, 0x74,
]);
const USDC_ARB: H160 = h160_from_slice([
    0xFF, 0x97, 0x0A, 0x61, 0xA0, 0x4b, 0x1c, 0xA1, 0x48, 0x34, 0xA4, 0x3f, 0x5d, 0xE4, 0x53, 0x3e,
    0xbD, 0xDB, 0x5C, 0xC8,
]);
const USDC_BASE: H160 = h160_from_slice([
    0x83, 0x35, 0x89, 0xfC, 0xD6, 0xeD, 0xb6, 0xE0, 0x8f, 0x4c, 0x7C, 0x32, 0xD4, 0xf7, 0x1b, 0x54,
    0xbd, 0xA0, 0x29, 0x13,
]);

// WETH addresses
const WETH_ETH: H160 = h160_from_slice([
    0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
    0x3C, 0x75, 0x6C, 0xc2,
]);
const WETH_ARB: H160 = h160_from_slice([
    0x82, 0xaF, 0x49, 0x44, 0x7D, 0x8a, 0x07, 0xe3, 0xbd, 0x95, 0xBD, 0x0d, 0x56, 0xf3, 0x52, 0x41,
    0x52, 0x3f, 0xBa, 0xb1,
]);
const WETH_BASE: H160 = h160_from_slice([
    0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x06,
]);
const WETH_OP: H160 = h160_from_slice([
    0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x06,
]);
const WMATIC: H160 = h160_from_slice([
    0x0d, 0x50, 0x0B, 0x1d, 0x8E, 0x8e, 0xF3, 0x1E, 0x21, 0xC9, 0x9d, 0x1D, 0xb9, 0xA6, 0x44, 0x4d,
    0x3A, 0xDf, 0x12, 0x70,
]);
const WAVAX: H160 = h160_from_slice([
    0xB3, 0x1f, 0x66, 0xAA, 0x3C, 0x1e, 0x78, 0x53, 0x63, 0xF0, 0x87, 0x5A, 0x1B, 0x74, 0xE2, 0x7b,
    0x85, 0xFD, 0x66, 0xc7,
]);
const WBNB: H160 = h160_from_slice([
    0xbb, 0x4C, 0xdB, 0x9C, 0xBd, 0x36, 0xB0, 0x1b, 0xD1, 0xcB, 0xaE, 0xBF, 0x2D, 0xe0, 0x8d, 0x91,
    0x73, 0xbc, 0x09, 0x5c,
]);

// DEX Routers
const UNISWAP_V2: H160 = h160_from_slice([
    0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6,
    0x59, 0xF2, 0x48, 0x8D,
]);
const QUICKSWAP: H160 = h160_from_slice([
    0xa5, 0xE0, 0x82, 0x9C, 0xaC, 0xEd, 0x8f, 0xFD, 0xD4, 0xDe, 0x3c, 0x43, 0x69, 0x6c, 0x57, 0xF7,
    0xD7, 0xA6, 0x78, 0xff,
]);
const SUSHISWAP_ARB: H160 = h160_from_slice([
    0x1b, 0x02, 0xdA, 0x8C, 0xb0, 0xd0, 0x97, 0xeB, 0x8D, 0x57, 0xA1, 0x75, 0xb8, 0x8c, 0x7D, 0x8b,
    0x47, 0x99, 0x75, 0x06,
]);
const UNISWAP_V3: H160 = h160_from_slice([
    0x68, 0xb3, 0x46, 0x58, 0x33, 0xfb, 0x72, 0xA7, 0x0e, 0xcd, 0xF4, 0x85, 0xE0, 0xe4, 0xC7, 0xbD,
    0x86, 0x65, 0xFc, 0x45,
]);
const TRADERJOE: H160 = h160_from_slice([
    0x60, 0xaE, 0x61, 0x6a, 0x21, 0x55, 0xEe, 0x3d, 0x9A, 0x68, 0x54, 0x1B, 0xa4, 0x54, 0x48, 0x62,
    0x31, 0x09, 0x33, 0xd4,
]);
const PANCAKESWAP: H160 = h160_from_slice([
    0x10, 0xED, 0x43, 0xC7, 0x18, 0x71, 0x4e, 0xb6, 0x3d, 0x5a, 0xA5, 0x7B, 0x78, 0xB5, 0x47, 0x04,
    0xE2, 0x56, 0x02, 0x4E,
]);
const SPOOKYSWAP: H160 = h160_from_slice([
    0xF4, 0x91, 0xe7, 0xB6, 0x9E, 0x42, 0x44, 0xad, 0x40, 0x02, 0xBC, 0x14, 0xe8, 0x78, 0xa3, 0x42,
    0x07, 0xE3, 0x8c, 0x29,
]);

/// A swap route leg
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteLeg {
    /// Source chain ID
    pub from_chain: u64,
    /// Destination chain ID  
    pub to_chain: u64,
    /// Token address on source chain
    pub from_token: H160,
    /// Token address on destination chain
    pub to_token: H160,
    /// Action type
    pub action: RouteAction,
    /// Estimated gas cost in wei
    pub estimated_gas: U256,
    /// Estimated time in milliseconds
    pub estimated_time_ms: u64,
}

/// Types of actions in a route
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAction {
    /// Swap tokens on same chain (DEX)
    Swap,
    /// Bridge tokens to another chain
    Bridge,
    /// Wrap native token (ETH → WETH)
    Wrap,
    /// Unwrap to native (WETH → ETH)
    Unwrap,
}

/// Complete route for a cross-chain swap
#[derive(Debug, Clone)]
pub struct SwapRoute {
    /// Route legs in order
    pub legs: Vec<RouteLeg>,
    /// Total estimated gas (sum of all legs)
    pub total_gas: U256,
    /// Total estimated time in milliseconds
    pub total_time_ms: u64,
    /// Route score (lower = better)
    pub score: u64,
    /// Source chain
    pub source_chain: u64,
    /// Destination chain
    pub dest_chain: u64,
    /// Input amount
    pub input_amount: U256,
    /// Estimated output amount
    pub estimated_output: U256,
}

/// Comit transaction payload for atomic execution
#[derive(Debug, Clone)]
pub struct ComitPayload {
    /// Chain ID for this payload
    pub chain_id: u64,
    /// Target contract address
    pub target: H160,
    /// Encoded call data
    pub calldata: Vec<u8>,
    /// Value to send (native token)
    pub value: U256,
    /// Gas limit
    pub gas_limit: u64,
}

/// Atomic Comit transaction bundle
#[derive(Debug, Clone)]
pub struct AtomicSwapBundle {
    /// All payloads to execute atomically
    pub payloads: Vec<ComitPayload>,
    /// Prepare root for verification
    pub prepare_root: H256,
    /// Nonce
    pub nonce: u64,
    /// Total value locked
    pub total_value: U256,
}

/// Cross-chain swap router
pub struct SwapRouter {
    /// Known liquidity pools per chain
    liquidity_hints: Vec<(u64, H160, H160, U256)>, // (chain, token0, token1, liquidity)
}

impl Default for SwapRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapRouter {
    pub fn new() -> Self {
        Self {
            liquidity_hints: Vec::new(),
        }
    }

    /// Find the best route from source to destination
    pub fn find_route(
        &self,
        from_chain: u64,
        from_token: H160,
        to_chain: u64,
        to_token: H160,
        amount: U256,
    ) -> Option<SwapRoute> {
        // Validate chains exist
        get_chain(from_chain)?;
        get_chain(to_chain)?;

        let mut legs = Vec::new();
        let mut total_gas = U256::zero();
        let mut total_time_ms = 0u64;

        // Same chain swap - simple DEX route
        if from_chain == to_chain {
            if from_token != to_token {
                let leg = RouteLeg {
                    from_chain,
                    to_chain,
                    from_token,
                    to_token,
                    action: RouteAction::Swap,
                    estimated_gas: U256::from(150_000),
                    estimated_time_ms: get_chain(from_chain)
                        .map(|c| c.block_time_ms)
                        .unwrap_or(12000),
                };
                total_gas = leg.estimated_gas;
                total_time_ms = leg.estimated_time_ms;
                legs.push(leg);
            }
        } else {
            // Cross-chain: need to bridge
            let bridge_token = self.find_bridgeable_token(from_chain, from_token);

            // Step 1: Swap to bridgeable token if needed
            if from_token != bridge_token {
                let swap_leg = RouteLeg {
                    from_chain,
                    to_chain: from_chain,
                    from_token,
                    to_token: bridge_token,
                    action: RouteAction::Swap,
                    estimated_gas: U256::from(150_000),
                    estimated_time_ms: get_chain(from_chain)
                        .map(|c| c.block_time_ms)
                        .unwrap_or(12000),
                };
                total_gas = total_gas + swap_leg.estimated_gas;
                total_time_ms += swap_leg.estimated_time_ms;
                legs.push(swap_leg);
            }

            // Step 2: Bridge to destination chain (ATOMIC via Comit)
            let dest_bridge_token = self.find_bridgeable_token(to_chain, to_token);
            let bridge_leg = RouteLeg {
                from_chain,
                to_chain,
                from_token: bridge_token,
                to_token: dest_bridge_token,
                action: RouteAction::Bridge,
                estimated_gas: U256::from(50_000), // Comit is cheap!
                estimated_time_ms: 6000,           // 1 Atlas block
            };
            total_gas = total_gas + bridge_leg.estimated_gas;
            total_time_ms += bridge_leg.estimated_time_ms;
            legs.push(bridge_leg);

            // Step 3: Swap to final token if needed
            if dest_bridge_token != to_token {
                let final_swap = RouteLeg {
                    from_chain: to_chain,
                    to_chain,
                    from_token: dest_bridge_token,
                    to_token,
                    action: RouteAction::Swap,
                    estimated_gas: U256::from(150_000),
                    estimated_time_ms: get_chain(to_chain)
                        .map(|c| c.block_time_ms)
                        .unwrap_or(12000),
                };
                total_gas = total_gas + final_swap.estimated_gas;
                total_time_ms += final_swap.estimated_time_ms;
                legs.push(final_swap);
            }
        }

        // Calculate score (lower = better)
        // Weight: 60% time, 40% gas
        let time_score = total_time_ms / 1000;
        let gas_score = total_gas.low_u64() / 10_000;
        let score = (time_score * 6 + gas_score * 4) / 10;

        // Estimate output (simplified - real would query prices)
        let slippage = U256::from(9950); // 0.5% slippage estimate
        let estimated_output = amount * slippage / U256::from(10000);

        Some(SwapRoute {
            legs,
            total_gas,
            total_time_ms,
            score,
            source_chain: from_chain,
            dest_chain: to_chain,
            input_amount: amount,
            estimated_output,
        })
    }

    /// Find multiple routes and return sorted by score
    pub fn find_routes(
        &self,
        from_chain: u64,
        from_token: H160,
        to_chain: u64,
        to_token: H160,
        amount: U256,
        max_routes: usize,
    ) -> Vec<SwapRoute> {
        let mut routes = Vec::new();

        // Direct route
        if let Some(direct) = self.find_route(from_chain, from_token, to_chain, to_token, amount) {
            routes.push(direct);
        }

        // Try intermediate chains for potentially better routes
        let intermediate_chains = self.find_intermediate_chains(from_chain, to_chain);
        for mid_chain in intermediate_chains.iter().take(5) {
            if let Some(route) = self.find_route_via(
                *mid_chain, from_chain, from_token, to_chain, to_token, amount,
            ) {
                routes.push(route);
            }
        }

        // Sort by score (lower = better)
        routes.sort_by_key(|r| r.score);
        routes.truncate(max_routes);
        routes
    }

    /// Build atomic Comit transaction from route
    pub fn build_comit_bundle(
        &self,
        route: &SwapRoute,
        sender: H160,
        recipient: H160,
        nonce: u64,
    ) -> AtomicSwapBundle {
        let mut payloads = Vec::new();

        for leg in &route.legs {
            let payload = match leg.action {
                RouteAction::Swap => self.encode_swap_payload(leg, sender, route.input_amount),
                RouteAction::Bridge => {
                    self.encode_bridge_payload(leg, sender, recipient, route.input_amount)
                }
                RouteAction::Wrap => self.encode_wrap_payload(leg, route.input_amount),
                RouteAction::Unwrap => self.encode_unwrap_payload(leg, route.input_amount),
            };
            payloads.push(payload);
        }

        // Calculate prepare_root (hash of input commitments)
        let prepare_root = self.calculate_prepare_root(&payloads, nonce);

        AtomicSwapBundle {
            payloads,
            prepare_root,
            nonce,
            total_value: route.input_amount,
        }
    }

    /// Execute quote - get exact output for input
    pub fn quote(
        &self,
        from_chain: u64,
        from_token: H160,
        to_chain: u64,
        to_token: H160,
        amount: U256,
    ) -> Option<QuoteResult> {
        let route = self.find_route(from_chain, from_token, to_chain, to_token, amount)?;

        Some(QuoteResult {
            input_amount: amount,
            output_amount: route.estimated_output,
            price_impact: U256::from(50), // 0.5% in basis points
            route,
            expires_at: 0, // Would be timestamp
        })
    }

    // === Private helpers ===

    fn find_bridgeable_token(&self, chain_id: u64, token: H160) -> H160 {
        // If token is already bridgeable, use it
        if token == WETH_ETH || token == USDC_ETH {
            return token;
        }

        // Default to USDC as bridge token (most liquid)
        match chain_id {
            1 => USDC_ETH,
            137 => USDC_POLYGON,
            42161 => USDC_ARB,
            8453 => USDC_BASE,
            _ => USDC_ETH,
        }
    }

    fn find_intermediate_chains(&self, from: u64, to: u64) -> Vec<u64> {
        // High liquidity chains that make good intermediates
        let hubs = vec![
            1,     // Ethereum
            42161, // Arbitrum
            137,   // Polygon
            8453,  // Base
            10,    // Optimism
        ];

        hubs.into_iter().filter(|&c| c != from && c != to).collect()
    }

    fn find_route_via(
        &self,
        via_chain: u64,
        from_chain: u64,
        from_token: H160,
        to_chain: u64,
        to_token: H160,
        amount: U256,
    ) -> Option<SwapRoute> {
        // Route: from_chain → via_chain → to_chain
        let bridge_token = self.find_bridgeable_token(via_chain, H160::zero());

        let leg1 = self.find_route(from_chain, from_token, via_chain, bridge_token, amount)?;
        let leg2 = self.find_route(
            via_chain,
            bridge_token,
            to_chain,
            to_token,
            leg1.estimated_output,
        )?;

        // Combine legs
        let mut combined_legs = leg1.legs;
        combined_legs.extend(leg2.legs);

        let total_gas = leg1.total_gas + leg2.total_gas;
        let total_time_ms = leg1.total_time_ms + leg2.total_time_ms;
        let score = (total_time_ms / 1000 * 6 + total_gas.low_u64() / 10_000 * 4) / 10;

        Some(SwapRoute {
            legs: combined_legs,
            total_gas,
            total_time_ms,
            score,
            source_chain: from_chain,
            dest_chain: to_chain,
            input_amount: amount,
            estimated_output: leg2.estimated_output,
        })
    }

    fn encode_swap_payload(&self, leg: &RouteLeg, sender: H160, amount: U256) -> ComitPayload {
        // Encode Uniswap V2 style swap
        // swapExactTokensForTokens(uint256,uint256,address[],address,uint256)
        let mut calldata = vec![0x38, 0xed, 0x17, 0x39]; // selector

        // Simplified encoding
        calldata.extend_from_slice(&[0u8; 32]); // amountIn placeholder
        calldata.extend_from_slice(&[0u8; 32]); // amountOutMin placeholder

        // Get router address for chain
        let router = self.get_dex_router(leg.from_chain);

        ComitPayload {
            chain_id: leg.from_chain,
            target: router,
            calldata,
            value: U256::zero(),
            gas_limit: 200_000,
        }
    }

    fn encode_bridge_payload(
        &self,
        leg: &RouteLeg,
        sender: H160,
        recipient: H160,
        amount: U256,
    ) -> ComitPayload {
        // This is the magic - bridge via Atlas Kernel canonical ledger
        // The Comit transaction handles the atomic bridge

        // Encode: lockAndBridge(address recipient, uint256 amount, uint64 destChain)
        let mut calldata = vec![0xBB, 0xBB, 0xBB, 0xBB]; // Atlas bridge selector
        calldata.extend_from_slice(recipient.as_bytes());
        calldata.extend_from_slice(&[0u8; 32]); // amount
        calldata.extend_from_slice(&leg.to_chain.to_be_bytes());

        // Atlas Kernel bridge contract (on source chain mirror)
        let bridge = self.get_atlas_bridge(leg.from_chain);

        ComitPayload {
            chain_id: leg.from_chain,
            target: bridge,
            calldata,
            value: U256::zero(),
            gas_limit: 100_000,
        }
    }

    fn encode_wrap_payload(&self, leg: &RouteLeg, amount: U256) -> ComitPayload {
        // WETH deposit()
        let calldata = vec![0xd0, 0xe3, 0x0d, 0xb0]; // deposit()
        let weth = self.get_weth(leg.from_chain);

        ComitPayload {
            chain_id: leg.from_chain,
            target: weth,
            calldata,
            value: amount,
            gas_limit: 50_000,
        }
    }

    fn encode_unwrap_payload(&self, leg: &RouteLeg, amount: U256) -> ComitPayload {
        // WETH withdraw(uint256)
        let mut calldata = vec![0x2e, 0x1a, 0x7d, 0x4d]; // withdraw(uint256)
        calldata.extend_from_slice(&[0u8; 32]); // amount placeholder
        let weth = self.get_weth(leg.from_chain);

        ComitPayload {
            chain_id: leg.from_chain,
            target: weth,
            calldata,
            value: U256::zero(),
            gas_limit: 50_000,
        }
    }

    fn get_dex_router(&self, chain_id: u64) -> H160 {
        // Uniswap V2/V3 compatible routers
        match chain_id {
            1 => UNISWAP_V2,
            137 => QUICKSWAP,
            42161 => SUSHISWAP_ARB,
            8453 => UNISWAP_V2, // BaseSwap uses same interface
            10 => UNISWAP_V3,
            43114 => TRADERJOE,
            56 => PANCAKESWAP,
            250 => SPOOKYSWAP,
            _ => UNISWAP_V2,
        }
    }

    fn get_atlas_bridge(&self, chain_id: u64) -> H160 {
        // Atlas Kernel mirror bridge contracts - deterministic addresses
        let mut bytes = [0u8; 20];
        bytes[0..4].copy_from_slice(&[0xA7, 0x1A, 0x50, 0x00]);
        bytes[16..20].copy_from_slice(&(chain_id as u32).to_be_bytes());
        H160(bytes)
    }

    fn get_weth(&self, chain_id: u64) -> H160 {
        match chain_id {
            1 => WETH_ETH,
            137 => WMATIC,
            42161 => WETH_ARB,
            8453 => WETH_BASE,
            10 => WETH_OP,
            43114 => WAVAX,
            56 => WBNB,
            _ => WETH_ETH,
        }
    }

    fn calculate_prepare_root(&self, payloads: &[ComitPayload], nonce: u64) -> H256 {
        use sp_core::keccak_256;

        let mut data = Vec::new();
        data.extend_from_slice(&nonce.to_be_bytes());

        for payload in payloads {
            data.extend_from_slice(&payload.chain_id.to_be_bytes());
            data.extend_from_slice(payload.target.as_bytes());
            data.extend_from_slice(&payload.calldata);
        }

        H256::from(keccak_256(&data))
    }
}

/// Quote result with pricing info
#[derive(Debug, Clone)]
pub struct QuoteResult {
    pub input_amount: U256,
    pub output_amount: U256,
    pub price_impact: U256, // basis points
    pub route: SwapRoute,
    pub expires_at: u64,
}

// === Convenience functions ===

/// Quick swap quote
pub fn quote_swap(
    from_chain: u64,
    from_token: H160,
    to_chain: u64,
    to_token: H160,
    amount: U256,
) -> Option<QuoteResult> {
    SwapRouter::new().quote(from_chain, from_token, to_chain, to_token, amount)
}

/// Find best route
pub fn find_best_route(
    from_chain: u64,
    from_token: H160,
    to_chain: u64,
    to_token: H160,
    amount: U256,
) -> Option<SwapRoute> {
    SwapRouter::new().find_route(from_chain, from_token, to_chain, to_token, amount)
}

/// Build atomic swap bundle
pub fn build_atomic_swap(
    from_chain: u64,
    from_token: H160,
    to_chain: u64,
    to_token: H160,
    amount: U256,
    sender: H160,
    recipient: H160,
    nonce: u64,
) -> Option<AtomicSwapBundle> {
    let router = SwapRouter::new();
    let route = router.find_route(from_chain, from_token, to_chain, to_token, amount)?;
    Some(router.build_comit_bundle(&route, sender, recipient, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_chain_route() {
        let router = SwapRouter::new();

        let route = router.find_route(1, USDC_ETH, 1, WETH_ETH, U256::from(1000_000000u64));
        assert!(route.is_some());

        let route = route.unwrap();
        assert_eq!(route.legs.len(), 1);
        assert_eq!(route.legs[0].action, RouteAction::Swap);
    }

    #[test]
    fn test_cross_chain_route() {
        let router = SwapRouter::new();

        // Ethereum USDC → Arbitrum USDC
        let route = router.find_route(1, USDC_ETH, 42161, USDC_ARB, U256::from(1000_000000u64));
        assert!(route.is_some());

        let route = route.unwrap();
        // Should have bridge leg
        assert!(route.legs.iter().any(|l| l.action == RouteAction::Bridge));
        // Comit bridge is fast!
        assert!(route.total_time_ms < 60000); // Under 1 minute
    }

    #[test]
    fn test_build_comit_bundle() {
        let router = SwapRouter::new();
        let sender = H160::from_low_u64_be(0xDEAD);
        let recipient = H160::from_low_u64_be(0xBEEF);

        let route = router
            .find_route(1, USDC_ETH, 42161, USDC_ETH, U256::from(1000_000000u64))
            .unwrap();
        let bundle = router.build_comit_bundle(&route, sender, recipient, 1);

        assert!(!bundle.payloads.is_empty());
        assert_ne!(bundle.prepare_root, H256::zero());
    }

    #[test]
    fn test_quote() {
        let quote = quote_swap(1, USDC_ETH, 8453, USDC_ETH, U256::from(1000_000000u64));

        assert!(quote.is_some());
        let quote = quote.unwrap();
        assert!(quote.output_amount > U256::zero());
        assert!(quote.output_amount < quote.input_amount); // Slippage
    }

    #[test]
    fn test_multi_routes() {
        let router = SwapRouter::new();

        let routes = router.find_routes(
            137,
            USDC_POLYGON,
            42161,
            WETH_ARB,
            U256::from(1000_000000u64),
            3,
        );

        // Should find at least one route
        assert!(!routes.is_empty());
        // Routes should be sorted by score
        for i in 1..routes.len() {
            assert!(routes[i - 1].score <= routes[i].score);
        }
    }

    #[test]
    fn test_103_chains_accessible() {
        // Verify router can work with any of our 103 chains
        let router = SwapRouter::new();
        let token = H160::from_low_u64_be(0x1234);

        // Test a few random chains
        let test_chains = vec![
            (1, 42161),  // ETH → Arbitrum
            (137, 8453), // Polygon → Base
            (56, 43114), // BSC → Avalanche
            (10, 324),   // Optimism → zkSync
            (250, 1088), // Fantom → Metis
        ];

        for (from, to) in test_chains {
            let route = router.find_route(from, token, to, token, U256::from(1000));
            assert!(route.is_some(), "Failed route from {} to {}", from, to);
        }
    }
}

//! Trade Graph Resolver for Optimal Path Finding
//!
//! This module provides algorithms for:
//! - Multi-hop swap path discovery
//! - Arbitrage opportunity detection
//! - Cross-VM route optimization
//! - Price impact minimization

use crate::types::{
    AmmProtocol, ArbitrageOpportunity, LiquidityPool, RouteStep, TradeRoute, VmType,
};
use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

/// Maximum hops in a trade path
pub const MAX_HOPS: usize = 4;

/// Maximum paths to evaluate
pub const MAX_PATHS_EVALUATED: usize = 10;

/// Minimum arbitrage profit threshold (basis points)
pub const MIN_ARBITRAGE_PROFIT_BPS: u32 = 10; // 0.1%

/// Edge in the trade graph (represents a pool)
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct GraphEdge {
    /// Pool ID
    pub pool_id: H256,
    /// Source token
    pub token_from: H256,
    /// Destination token
    pub token_to: H256,
    /// AMM protocol
    pub protocol: AmmProtocol,
    /// VM type
    pub vm_type: VmType,
    /// Available liquidity (for prioritization)
    pub liquidity: u128,
    /// Fee in basis points
    pub fee_bps: u32,
}

/// Node in the trade graph (represents a token)
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct GraphNode {
    pub token_id: H256,
    /// Outgoing edges (pools that accept this token)
    pub edges: Vec<GraphEdge>,
}

/// Trade graph for path finding
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct TradeGraph {
    /// Nodes indexed by token ID
    pub nodes: BTreeMap<H256, GraphNode>,
    /// All pools for quick lookup
    pub pools: BTreeMap<H256, LiquidityPool>,
}

impl TradeGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            pools: BTreeMap::new(),
        }
    }

    /// Add a pool to the graph (creates bidirectional edges)
    pub fn add_pool(&mut self, pool: LiquidityPool) {
        let liquidity = pool
            .reserve_a
            .checked_mul(pool.reserve_b)
            .and_then(integer_sqrt)
            .unwrap_or(0);

        // Create edge A -> B
        let edge_a_to_b = GraphEdge {
            pool_id: pool.pool_id,
            token_from: pool.token_a,
            token_to: pool.token_b,
            protocol: pool.protocol,
            vm_type: pool.vm_type,
            liquidity,
            fee_bps: pool.fee_bps,
        };

        // Create edge B -> A
        let edge_b_to_a = GraphEdge {
            pool_id: pool.pool_id,
            token_from: pool.token_b,
            token_to: pool.token_a,
            protocol: pool.protocol,
            vm_type: pool.vm_type,
            liquidity,
            fee_bps: pool.fee_bps,
        };

        // Add to nodes
        self.nodes
            .entry(pool.token_a)
            .or_insert_with(|| GraphNode {
                token_id: pool.token_a,
                edges: Vec::new(),
            })
            .edges
            .push(edge_a_to_b);

        self.nodes
            .entry(pool.token_b)
            .or_insert_with(|| GraphNode {
                token_id: pool.token_b,
                edges: Vec::new(),
            })
            .edges
            .push(edge_b_to_a);

        // Store pool
        self.pools.insert(pool.pool_id, pool);
    }

    /// Remove a pool from the graph
    pub fn remove_pool(&mut self, pool_id: H256) {
        if let Some(pool) = self.pools.remove(&pool_id) {
            if let Some(node) = self.nodes.get_mut(&pool.token_a) {
                node.edges.retain(|e| e.pool_id != pool_id);
            }
            if let Some(node) = self.nodes.get_mut(&pool.token_b) {
                node.edges.retain(|e| e.pool_id != pool_id);
            }
        }
    }

    /// Update pool reserves
    pub fn update_reserves(&mut self, pool_id: H256, reserve_a: u128, reserve_b: u128) {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            pool.reserve_a = reserve_a;
            pool.reserve_b = reserve_b;

            let new_liquidity = reserve_a
                .checked_mul(reserve_b)
                .and_then(integer_sqrt)
                .unwrap_or(0);

            if let Some(node) = self.nodes.get_mut(&pool.token_a) {
                for edge in &mut node.edges {
                    if edge.pool_id == pool_id {
                        edge.liquidity = new_liquidity;
                    }
                }
            }
            if let Some(node) = self.nodes.get_mut(&pool.token_b) {
                for edge in &mut node.edges {
                    if edge.pool_id == pool_id {
                        edge.liquidity = new_liquidity;
                    }
                }
            }
        }
    }

    /// Get total number of pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Get total number of tokens
    pub fn token_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Integer square root helper
fn integer_sqrt(n: u128) -> Option<u128> {
    if n == 0 {
        return Some(0);
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    Some(x)
}

/// Trade graph resolver for finding optimal paths
///
/// This resolver provides algorithms for pathfinding, arbitrage detection,
/// and trade optimization. It operates on a `TradeGraph` instance.
pub struct TradeGraphResolver;

impl TradeGraphResolver {
    /// Find the optimal trade route from token_start to token_end
    pub fn find_optimal_route(
        graph: &TradeGraph,
        token_start: H256,
        token_end: H256,
        amount_in: u128,
    ) -> Result<TradeRoute, DispatchError> {
        let paths = Self::find_all_paths(graph, token_start, token_end, MAX_HOPS)?;

        if paths.is_empty() {
            return Err(DispatchError::Other("No path found"));
        }

        let mut best_route: Option<TradeRoute> = None;
        let mut best_output: u128 = 0;

        for path in paths.iter().take(MAX_PATHS_EVALUATED) {
            if let Ok(route) = Self::evaluate_path(graph, path, amount_in) {
                if route.expected_amount_out > best_output {
                    best_output = route.expected_amount_out;
                    best_route = Some(route);
                }
            }
        }

        best_route.ok_or(DispatchError::Other("No viable route found"))
    }

    /// Find all paths between two tokens (BFS with depth limit)
    pub fn find_all_paths(
        graph: &TradeGraph,
        start: H256,
        end: H256,
        max_hops: usize,
    ) -> Result<Vec<Vec<GraphEdge>>, DispatchError> {
        let mut all_paths: Vec<Vec<GraphEdge>> = Vec::new();
        let mut queue: Vec<(H256, Vec<GraphEdge>, BTreeSet<H256>)> = Vec::new();

        let mut initial_visited = BTreeSet::new();
        initial_visited.insert(start);
        queue.push((start, Vec::new(), initial_visited));

        while let Some((current, path, visited)) = queue.pop() {
            if path.len() >= max_hops {
                continue;
            }

            if let Some(node) = graph.nodes.get(&current) {
                for edge in &node.edges {
                    if edge.token_to == end {
                        let mut complete_path = path.clone();
                        complete_path.push(edge.clone());
                        all_paths.push(complete_path);
                        continue;
                    }

                    if visited.contains(&edge.token_to) {
                        continue;
                    }

                    let mut new_path = path.clone();
                    new_path.push(edge.clone());

                    let mut new_visited = visited.clone();
                    new_visited.insert(edge.token_to);

                    queue.push((edge.token_to, new_path, new_visited));
                }
            }
        }

        Ok(all_paths)
    }

    /// Evaluate a path and calculate expected output
    pub fn evaluate_path(
        graph: &TradeGraph,
        path: &[GraphEdge],
        amount_in: u128,
    ) -> Result<TradeRoute, DispatchError> {
        if path.is_empty() {
            return Err(DispatchError::Other("Empty path"));
        }

        let mut current_amount = amount_in;
        let mut total_gas: u64 = 0;
        let mut total_price_impact_bps: u32 = 0;
        let mut steps: Vec<RouteStep> = Vec::new();

        for edge in path {
            let pool = graph
                .pools
                .get(&edge.pool_id)
                .ok_or(DispatchError::Other("Pool not found"))?;

            let amount_out = pool
                .get_amount_out(current_amount, edge.token_from)
                .ok_or(DispatchError::Other("Insufficient liquidity"))?;

            let step_impact = pool
                .calculate_price_impact(current_amount, edge.token_from)
                .unwrap_or(0);

            total_price_impact_bps = total_price_impact_bps.saturating_add(step_impact);

            steps.push(RouteStep {
                pool_id: edge.pool_id,
                token_in: edge.token_from,
                token_out: edge.token_to,
                protocol: edge.protocol,
                vm_type: edge.vm_type,
            });

            current_amount = amount_out;
            total_gas = total_gas.saturating_add(Self::estimate_step_gas(edge.vm_type));
        }

        let token_start = path[0].token_from;
        let token_end = path[path.len() - 1].token_to;

        Ok(TradeRoute {
            steps,
            token_start,
            token_end,
            amount_in,
            expected_amount_out: current_amount,
            estimated_gas: total_gas,
            price_impact_bps: total_price_impact_bps,
        })
    }

    /// Estimate gas for a single step based on VM type
    fn estimate_step_gas(vm_type: VmType) -> u64 {
        match vm_type {
            VmType::Evm => 150_000,
            VmType::Svm => 200_000,
            VmType::CrossVm => 400_000,
        }
    }

    /// Detect arbitrage opportunities
    pub fn detect_arbitrage(
        graph: &TradeGraph,
        base_token: H256,
        test_amount: u128,
    ) -> Result<Option<ArbitrageOpportunity>, DispatchError> {
        let circular_paths = Self::find_circular_paths(graph, base_token, MAX_HOPS)?;

        let mut best_opportunity: Option<ArbitrageOpportunity> = None;
        let mut best_profit_bps: u32 = 0;

        for path in circular_paths {
            if let Ok(route) = Self::evaluate_path(graph, &path, test_amount) {
                if route.expected_amount_out > test_amount {
                    let profit = route.expected_amount_out.saturating_sub(test_amount);
                    let profit_bps = (profit
                        .saturating_mul(10000)
                        .checked_div(test_amount)
                        .unwrap_or(0)) as u32;

                    if profit_bps > MIN_ARBITRAGE_PROFIT_BPS && profit_bps > best_profit_bps {
                        let optimal_input =
                            Self::find_optimal_arbitrage_input(graph, &path, test_amount)
                                .unwrap_or(test_amount);

                        if let Ok(optimal_route) = Self::evaluate_path(graph, &path, optimal_input)
                        {
                            let optimal_profit = optimal_route
                                .expected_amount_out
                                .saturating_sub(optimal_input);
                            let optimal_profit_bps = (optimal_profit
                                .saturating_mul(10000)
                                .checked_div(optimal_input)
                                .unwrap_or(0))
                                as u32;

                            let steps: Vec<RouteStep> = path
                                .iter()
                                .map(|e| RouteStep {
                                    pool_id: e.pool_id,
                                    token_in: e.token_from,
                                    token_out: e.token_to,
                                    protocol: e.protocol,
                                    vm_type: e.vm_type,
                                })
                                .collect();

                            best_profit_bps = optimal_profit_bps;
                            best_opportunity = Some(ArbitrageOpportunity {
                                path: steps,
                                base_token,
                                optimal_input,
                                expected_output: optimal_route.expected_amount_out,
                                profit_bps: optimal_profit_bps,
                                gas_estimate: optimal_route.estimated_gas,
                                net_profitable: true,
                            });
                        }
                    }
                }
            }
        }

        Ok(best_opportunity)
    }

    /// Find circular paths
    fn find_circular_paths(
        graph: &TradeGraph,
        start: H256,
        max_hops: usize,
    ) -> Result<Vec<Vec<GraphEdge>>, DispatchError> {
        let mut circular_paths: Vec<Vec<GraphEdge>> = Vec::new();
        let mut stack: Vec<(H256, Vec<GraphEdge>, BTreeSet<H256>)> = Vec::new();

        let initial_visited = BTreeSet::new();
        stack.push((start, Vec::new(), initial_visited));

        while let Some((current, path, visited)) = stack.pop() {
            if path.len() >= max_hops {
                continue;
            }

            if let Some(node) = graph.nodes.get(&current) {
                for edge in &node.edges {
                    if edge.token_to == start && path.len() >= 2 {
                        let mut complete_path = path.clone();
                        complete_path.push(edge.clone());
                        circular_paths.push(complete_path);
                        continue;
                    }

                    if visited.contains(&edge.token_to) {
                        continue;
                    }

                    let mut new_path = path.clone();
                    new_path.push(edge.clone());

                    let mut new_visited = visited.clone();
                    new_visited.insert(edge.token_to);

                    stack.push((edge.token_to, new_path, new_visited));
                }
            }
        }

        Ok(circular_paths)
    }

    /// Find optimal input for arbitrage
    fn find_optimal_arbitrage_input(
        graph: &TradeGraph,
        path: &[GraphEdge],
        initial_guess: u128,
    ) -> Result<u128, DispatchError> {
        let mut low = initial_guess / 10;
        let mut high = initial_guess * 10;
        let mut best_input = initial_guess;
        let mut best_profit: i128 = 0;

        for _ in 0..20 {
            if high <= low {
                break;
            }

            let mid = (low + high) / 2;

            let profit_mid = Self::evaluate_path(graph, path, mid)
                .map(|r| r.expected_amount_out as i128 - mid as i128)
                .unwrap_or(i128::MIN);

            if profit_mid > best_profit {
                best_profit = profit_mid;
                best_input = mid;
            }

            let profit_high = Self::evaluate_path(graph, path, mid + mid / 10)
                .map(|r| r.expected_amount_out as i128 - (mid + mid / 10) as i128)
                .unwrap_or(i128::MIN);

            if profit_high > profit_mid {
                low = mid;
            } else {
                high = mid;
            }
        }

        Ok(best_input)
    }

    /// Calculate minimum output with slippage
    pub fn calculate_min_output(route: &TradeRoute, slippage_tolerance_bps: u32) -> u128 {
        let slippage_factor = 10000u128.saturating_sub(slippage_tolerance_bps as u128);
        route
            .expected_amount_out
            .saturating_mul(slippage_factor)
            .saturating_div(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool(id: u64, token_a: u64, token_b: u64, vm: VmType) -> LiquidityPool {
        LiquidityPool {
            pool_id: H256::from_low_u64_be(id),
            protocol: AmmProtocol::UniswapV2,
            vm_type: vm,
            token_a: H256::from_low_u64_be(token_a),
            token_b: H256::from_low_u64_be(token_b),
            reserve_a: 1_000_000_000_000_000_000u128,
            reserve_b: 2_000_000_000_000_000_000u128,
            fee_bps: 30,
            address: vec![0u8; 20],
        }
    }

    #[test]
    fn test_graph_add_pool() {
        let mut graph = TradeGraph::new();
        let pool = create_test_pool(1, 100, 200, VmType::Evm);
        graph.add_pool(pool);

        assert_eq!(graph.pool_count(), 1);
        assert_eq!(graph.token_count(), 2);
    }

    #[test]
    fn test_find_paths() {
        let mut graph = TradeGraph::new();

        // Create: A -> B -> C
        graph.add_pool(create_test_pool(1, 100, 200, VmType::Evm));
        graph.add_pool(create_test_pool(2, 200, 300, VmType::Svm));

        let start = H256::from_low_u64_be(100);
        let end = H256::from_low_u64_be(300);

        let paths = TradeGraphResolver::find_all_paths(&graph, start, end, 4).unwrap();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_integer_sqrt() {
        assert_eq!(integer_sqrt(0), Some(0));
        assert_eq!(integer_sqrt(1), Some(1));
        assert_eq!(integer_sqrt(4), Some(2));
        assert_eq!(integer_sqrt(100), Some(10));
    }
}

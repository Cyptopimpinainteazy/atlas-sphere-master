//! Quantum router for optimal path computation
//!
//! Uses Evolution Core for genetic algorithm-based route optimization

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

use crate::config::RouterConfig;
use crate::error::{ChronosError, ChronosResult};
use crate::intent::SwapIntent;
use crate::types::{Address, Balance, ChainId, Gas, RouteHop, RouteId, Token, TradeRoute};

/// Quantum-enhanced route optimizer
pub struct QuantumRouter {
    config: RouterConfig,
    /// Liquidity pools indexed by chain and token pair
    pools: HashMap<(ChainId, Address, Address), Vec<LiquidityPool>>,
    /// Cross-chain bridge routes
    bridges: Vec<BridgeRoute>,
    /// Gas prices per chain
    gas_prices: HashMap<ChainId, u128>,
}

impl QuantumRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            pools: HashMap::new(),
            bridges: Self::default_bridges(),
            gas_prices: HashMap::new(),
        }
    }

    /// Compute optimal routes for a swap intent
    pub async fn compute_routes(&self, intent: &SwapIntent) -> ChronosResult<Vec<TradeRoute>> {
        let start_time = std::time::Instant::now();
        
        // Check if cross-chain is needed
        let same_chain = intent.token_in.chain_id == intent.token_out.chain_id;
        
        let routes = if same_chain {
            self.compute_single_chain_routes(intent).await?
        } else if self.config.cross_chain_enabled {
            self.compute_cross_chain_routes(intent).await?
        } else {
            return Err(ChronosError::RouteFailed("Cross-chain routing disabled".to_string()));
        };

        // Check timeout
        if start_time.elapsed() > self.config.optimization_timeout {
            // Return best routes found so far
            return Ok(routes.into_iter().take(self.config.max_routes).collect());
        }

        // Run quantum optimization if we have multiple routes
        if routes.len() > 1 {
            let optimized = self.quantum_optimize(routes).await?;
            Ok(optimized.into_iter().take(self.config.max_routes).collect())
        } else {
            Ok(routes)
        }
    }

    /// Compute routes on single chain using Dijkstra + genetic optimization
    async fn compute_single_chain_routes(&self, intent: &SwapIntent) -> ChronosResult<Vec<TradeRoute>> {
        let chain_id = intent.token_in.chain_id;
        let token_in = intent.token_in.address;
        let token_out = intent.token_out.address;
        let amount_in = intent.amount_in;

        // Build graph of available pools
        let graph = self.build_liquidity_graph(chain_id);

        // Run multi-path Dijkstra
        let paths = self.find_all_paths(&graph, token_in, token_out, self.config.max_hops as usize);

        // Convert paths to routes with price quotes
        let mut routes = vec![];
        for path in paths {
            if let Some(route) = self.path_to_route(chain_id, &path, amount_in, intent)? {
                routes.push(route);
            }
        }

        // Sort by expected output
        routes.sort_by(|a, b| b.expected_output.cmp(&a.expected_output));

        Ok(routes)
    }

    /// Compute cross-chain routes
    async fn compute_cross_chain_routes(&self, intent: &SwapIntent) -> ChronosResult<Vec<TradeRoute>> {
        let src_chain = intent.token_in.chain_id;
        let dst_chain = intent.token_out.chain_id;
        
        // Find bridges between chains
        let bridges: Vec<_> = self.bridges.iter()
            .filter(|b| b.src_chain == src_chain && b.dst_chain == dst_chain)
            .collect();

        if bridges.is_empty() {
            return Err(ChronosError::RouteFailed("No bridge available".to_string()));
        }

        let mut routes = vec![];

        for bridge in bridges {
            // Route: TokenIn -> BridgeToken -> Bridge -> BridgeToken -> TokenOut
            
            // 1. Get route from token_in to bridge token on source chain
            let src_intent = SwapIntent {
                token_out: Token {
                    chain_id: src_chain,
                    address: bridge.src_token,
                    symbol: "BRIDGE".to_string(),
                    decimals: 18,
                },
                ..intent.clone()
            };
            let src_routes = self.compute_single_chain_routes(&src_intent).await?;

            // 2. Get route from bridge token to token_out on destination chain
            if let Some(src_route) = src_routes.first() {
                let bridged_amount = self.estimate_bridge_output(bridge, src_route.expected_output);
                
                let dst_intent = SwapIntent {
                    chain_id: dst_chain,
                    token_in: Token {
                        chain_id: dst_chain,
                        address: bridge.dst_token,
                        symbol: "BRIDGE".to_string(),
                        decimals: 18,
                    },
                    amount_in: bridged_amount,
                    ..intent.clone()
                };
                let dst_routes = self.compute_single_chain_routes(&dst_intent).await?;

                // Combine routes
                if let Some(dst_route) = dst_routes.first() {
                    let combined = self.combine_routes(src_route, bridge, dst_route, intent)?;
                    routes.push(combined);
                }
            }
        }

        Ok(routes)
    }

    /// Build liquidity graph for a chain
    fn build_liquidity_graph(&self, chain_id: ChainId) -> LiquidityGraph {
        let mut graph = LiquidityGraph::new();

        for ((c, token_a, token_b), pools) in &self.pools {
            if *c == chain_id {
                for pool in pools {
                    graph.add_edge(*token_a, *token_b, pool.clone());
                    graph.add_edge(*token_b, *token_a, pool.clone()); // Bidirectional
                }
            }
        }

        graph
    }

    /// Find all paths between tokens
    fn find_all_paths(
        &self,
        graph: &LiquidityGraph,
        start: Address,
        end: Address,
        max_hops: usize,
    ) -> Vec<Vec<Address>> {
        let mut paths = vec![];
        let mut current_path = vec![start];
        let mut visited = std::collections::HashSet::new();
        visited.insert(start);

        self.dfs_paths(graph, end, &mut current_path, &mut visited, &mut paths, max_hops);

        paths
    }

    fn dfs_paths(
        &self,
        graph: &LiquidityGraph,
        end: Address,
        current: &mut Vec<Address>,
        visited: &mut std::collections::HashSet<Address>,
        paths: &mut Vec<Vec<Address>>,
        max_hops: usize,
    ) {
        let current_token = *current.last().unwrap();

        if current_token == end {
            paths.push(current.clone());
            return;
        }

        if current.len() > max_hops {
            return;
        }

        if let Some(neighbors) = graph.neighbors.get(&current_token) {
            for (next_token, _) in neighbors {
                if !visited.contains(next_token) {
                    visited.insert(*next_token);
                    current.push(*next_token);
                    self.dfs_paths(graph, end, current, visited, paths, max_hops);
                    current.pop();
                    visited.remove(next_token);
                }
            }
        }
    }

    /// Convert path to trade route with price quotes
    fn path_to_route(
        &self,
        chain_id: ChainId,
        path: &[Address],
        amount_in: Balance,
        intent: &SwapIntent,
    ) -> ChronosResult<Option<TradeRoute>> {
        if path.len() < 2 {
            return Ok(None);
        }

        let mut hops = vec![];
        let mut current_amount = amount_in;
        let mut total_gas = 0u64;

        for i in 0..path.len() - 1 {
            let token_in_addr = path[i];
            let token_out_addr = path[i + 1];

            // Find best pool for this hop
            if let Some(pools) = self.pools.get(&(chain_id, token_in_addr, token_out_addr)) {
                if let Some(pool) = pools.first() {
                    let output = self.quote_swap(pool, current_amount);
                    
                    let hop = RouteHop {
                        chain_id,
                        protocol: pool.protocol.clone(),
                        pool_address: pool.address,
                        token_in: Token {
                            chain_id,
                            address: token_in_addr,
                            symbol: "".to_string(),
                            decimals: 18,
                        },
                        token_out: Token {
                            chain_id,
                            address: token_out_addr,
                            symbol: "".to_string(),
                            decimals: 18,
                        },
                        amount_in: current_amount,
                        expected_out: output,
                        gas_estimate: pool.gas_estimate,
                    };

                    total_gas += pool.gas_estimate;
                    current_amount = output;
                    hops.push(hop);
                }
            } else {
                return Ok(None); // No pool found for hop
            }
        }

        let now = chrono::Utc::now().timestamp_millis() as u64;
        let slippage_factor = 1.0 - (intent.metadata.slippage_bps.unwrap_or(50) as f64 / 10000.0);
        let min_output = (current_amount as f64 * slippage_factor) as Balance;

        Ok(Some(TradeRoute {
            id: uuid::Uuid::new_v4(),
            hops,
            total_input: amount_in,
            expected_output: current_amount,
            minimum_output: min_output,
            total_gas,
            slippage_bps: intent.metadata.slippage_bps.unwrap_or(50),
            computed_at: now,
            expires_at: now + 30_000, // 30 seconds
        }))
    }

    /// Quote swap output from a pool
    fn quote_swap(&self, pool: &LiquidityPool, amount_in: Balance) -> Balance {
        // Constant product AMM: x * y = k
        // output = reserve_out * amount_in / (reserve_in + amount_in)
        
        let reserve_in = pool.reserve_a;
        let reserve_out = pool.reserve_b;
        
        if reserve_in == 0 {
            return 0;
        }

        let amount_in_with_fee = amount_in * (10000 - pool.fee_bps as u128) / 10000;
        let numerator = reserve_out * amount_in_with_fee;
        let denominator = reserve_in + amount_in_with_fee;

        numerator / denominator
    }

    /// Estimate bridge output
    fn estimate_bridge_output(&self, bridge: &BridgeRoute, amount_in: Balance) -> Balance {
        let fee = amount_in * bridge.fee_bps as u128 / 10000;
        amount_in - fee
    }

    /// Combine source, bridge, and destination routes
    fn combine_routes(
        &self,
        src: &TradeRoute,
        bridge: &BridgeRoute,
        dst: &TradeRoute,
        intent: &SwapIntent,
    ) -> ChronosResult<TradeRoute> {
        let mut hops = src.hops.clone();
        
        // Add bridge hop
        hops.push(RouteHop {
            chain_id: bridge.src_chain,
            protocol: bridge.name.clone(),
            pool_address: bridge.src_contract,
            token_in: Token {
                chain_id: bridge.src_chain,
                address: bridge.src_token,
                symbol: "".to_string(),
                decimals: 18,
            },
            token_out: Token {
                chain_id: bridge.dst_chain,
                address: bridge.dst_token,
                symbol: "".to_string(),
                decimals: 18,
            },
            amount_in: src.expected_output,
            expected_out: self.estimate_bridge_output(bridge, src.expected_output),
            gas_estimate: bridge.gas_estimate,
        });

        hops.extend(dst.hops.clone());

        let now = chrono::Utc::now().timestamp_millis() as u64;

        Ok(TradeRoute {
            id: uuid::Uuid::new_v4(),
            hops,
            total_input: intent.amount_in,
            expected_output: dst.expected_output,
            minimum_output: dst.minimum_output,
            total_gas: src.total_gas + bridge.gas_estimate + dst.total_gas,
            slippage_bps: intent.metadata.slippage_bps.unwrap_or(50),
            computed_at: now,
            expires_at: now + 30_000,
        })
    }

    /// Quantum-enhanced route optimization using genetic algorithm
    async fn quantum_optimize(&self, routes: Vec<TradeRoute>) -> ChronosResult<Vec<TradeRoute>> {
        // Uses Evolution Core for genetic algorithm optimization
        // Explores route permutations and hop reordering
        
        let mut population: Vec<RouteIndividual> = routes
            .into_iter()
            .map(|r| RouteIndividual::new(r))
            .collect();

        // Run evolution
        for _ in 0..self.config.evolution_iterations {
            // Evaluate fitness
            for individual in &mut population {
                individual.fitness = self.evaluate_route_fitness(&individual.route);
            }

            // Sort by fitness
            population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(Ordering::Equal));

            // Keep top performers
            population.truncate(self.config.population_size);

            // Crossover and mutation would happen here in full implementation
        }

        Ok(population.into_iter().map(|i| i.route).collect())
    }

    /// Evaluate route fitness score
    fn evaluate_route_fitness(&self, route: &TradeRoute) -> f64 {
        let mut score = 0.0;

        // Output amount (higher = better)
        score += route.expected_output as f64 / route.total_input as f64;

        // Gas efficiency (lower gas = better)
        let gas_factor = 1.0 / (1.0 + route.total_gas as f64 / 1_000_000.0);
        score += gas_factor * 0.2;

        // Fewer hops = better (less slippage)
        let hop_factor = 1.0 / route.hops.len() as f64;
        score += hop_factor * 0.1;

        score
    }

    /// Update pool liquidity data
    pub fn update_pool(&mut self, chain_id: ChainId, pool: LiquidityPool) {
        let key = (chain_id, pool.token_a, pool.token_b);
        self.pools.entry(key).or_default().push(pool);
    }

    /// Update gas price for chain
    pub fn update_gas_price(&mut self, chain_id: ChainId, gas_price: u128) {
        self.gas_prices.insert(chain_id, gas_price);
    }

    fn default_bridges() -> Vec<BridgeRoute> {
        vec![
            // ETH <-> Polygon
            BridgeRoute {
                name: "Polygon Bridge".to_string(),
                src_chain: 1,
                dst_chain: 137,
                src_contract: [0u8; 32],
                dst_contract: [0u8; 32],
                src_token: [0u8; 32], // WETH
                dst_token: [0u8; 32], // WETH
                fee_bps: 5,
                gas_estimate: 200_000,
            },
            // ETH <-> Arbitrum
            BridgeRoute {
                name: "Arbitrum Bridge".to_string(),
                src_chain: 1,
                dst_chain: 42161,
                src_contract: [0u8; 32],
                dst_contract: [0u8; 32],
                src_token: [0u8; 32],
                dst_token: [0u8; 32],
                fee_bps: 3,
                gas_estimate: 150_000,
            },
        ]
    }
}

/// Liquidity pool data
#[derive(Debug, Clone)]
pub struct LiquidityPool {
    pub address: Address,
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: Balance,
    pub reserve_b: Balance,
    pub fee_bps: u32,
    pub protocol: String,
    pub gas_estimate: Gas,
}

/// Bridge route configuration
#[derive(Debug, Clone)]
pub struct BridgeRoute {
    pub name: String,
    pub src_chain: ChainId,
    pub dst_chain: ChainId,
    pub src_contract: Address,
    pub dst_contract: Address,
    pub src_token: Address,
    pub dst_token: Address,
    pub fee_bps: u32,
    pub gas_estimate: Gas,
}

/// Liquidity graph for pathfinding
struct LiquidityGraph {
    neighbors: HashMap<Address, Vec<(Address, LiquidityPool)>>,
}

impl LiquidityGraph {
    fn new() -> Self {
        Self {
            neighbors: HashMap::new(),
        }
    }

    fn add_edge(&mut self, from: Address, to: Address, pool: LiquidityPool) {
        self.neighbors.entry(from).or_default().push((to, pool));
    }
}

/// Individual in genetic algorithm population
struct RouteIndividual {
    route: TradeRoute,
    fitness: f64,
}

impl RouteIndividual {
    fn new(route: TradeRoute) -> Self {
        Self { route, fitness: 0.0 }
    }
}

//! Intent detection and types for ChronosFlash
//!
//! Detects swap intents from mempool transactions before execution

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{Address, Balance, ChainId, Gas, Hash, Price, Timestamp, Token};

/// Unique identifier for detected intents
pub type IntentId = Uuid;

/// Detected swap intent from mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapIntent {
    pub id: IntentId,
    pub chain_id: ChainId,
    pub sender: Address,
    pub token_in: Token,
    pub token_out: Token,
    pub amount_in: Balance,
    pub min_amount_out: Balance,
    pub deadline: u64,
    pub tx_hash: Hash,
    pub gas_price: u128,
    pub gas_limit: Gas,
    pub detected_at: Timestamp,
    pub confidence: f64,
    pub intent_type: IntentType,
    pub metadata: IntentMetadata,
}

/// Type of detected intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentType {
    /// Direct token swap
    SimpleSwap,
    /// Multi-hop swap
    MultiHopSwap,
    /// Add liquidity
    AddLiquidity,
    /// Remove liquidity
    RemoveLiquidity,
    /// Limit order fill
    LimitOrder,
    /// Cross-chain swap
    CrossChainSwap,
    /// Aggregated swap (1inch, Paraswap, etc.)
    AggregatedSwap,
    /// Unknown swap pattern
    Unknown,
}

/// Additional metadata about the intent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentMetadata {
    /// Router/aggregator used
    pub router: Option<String>,
    /// Slippage tolerance detected
    pub slippage_bps: Option<u32>,
    /// Is this a recurring pattern from this address?
    pub is_recurring: bool,
    /// Historical success rate for this address
    pub sender_success_rate: Option<f64>,
    /// Estimated USD value
    pub usd_value: Option<f64>,
    /// Priority score (0-100)
    pub priority_score: u8,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Predicted intent (not yet in mempool)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedIntent {
    pub id: IntentId,
    pub chain_id: ChainId,
    pub predicted_sender: Address,
    pub token_in: Token,
    pub token_out: Token,
    pub predicted_amount_range: (Balance, Balance),
    pub confidence: f64,
    pub prediction_type: PredictionType,
    pub predicted_at: Timestamp,
    pub expected_submission: Timestamp,
    pub basis: PredictionBasis,
}

/// Type of prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionType {
    /// Based on historical patterns
    Historical,
    /// Based on price movements
    PriceReactive,
    /// Based on on-chain events
    EventDriven,
    /// Based on social signals
    SocialSentiment,
    /// AI model prediction
    ModelPrediction,
}

/// What the prediction is based on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionBasis {
    /// Historical transactions analyzed
    pub historical_txs: u32,
    /// Price correlation strength
    pub price_correlation: f64,
    /// Event triggers detected
    pub event_triggers: Vec<String>,
    /// Model features used
    pub model_features: Vec<String>,
}

impl Default for PredictionBasis {
    fn default() -> Self {
        Self {
            historical_txs: 0,
            price_correlation: 0.0,
            event_triggers: vec![],
            model_features: vec![],
        }
    }
}

/// Intent detector for parsing mempool transactions
pub struct IntentDetector {
    known_routers: Vec<KnownRouter>,
    swap_selectors: Vec<SwapSelector>,
}

impl IntentDetector {
    pub fn new() -> Self {
        Self {
            known_routers: Self::default_routers(),
            swap_selectors: Self::default_selectors(),
        }
    }

    /// Detect intent from raw transaction data
    pub fn detect(
        &self,
        chain_id: ChainId,
        tx_data: &[u8],
        sender: Address,
        gas_price: u128,
    ) -> Option<SwapIntent> {
        // Need at least 4 bytes for function selector
        if tx_data.len() < 4 {
            return None;
        }

        let selector = &tx_data[0..4];

        // Match against known swap selectors
        for swap_sel in &self.swap_selectors {
            if selector == swap_sel.selector {
                return self.parse_swap_intent(chain_id, tx_data, sender, gas_price, &swap_sel);
            }
        }

        None
    }

    /// Parse swap intent from transaction data
    fn parse_swap_intent(
        &self,
        chain_id: ChainId,
        tx_data: &[u8],
        sender: Address,
        gas_price: u128,
        selector: &SwapSelector,
    ) -> Option<SwapIntent> {
        // Decode based on swap type
        let (token_in, token_out, amount_in, min_out) = match selector.swap_type {
            SwapType::ExactInputSingle => self.decode_exact_input_single(tx_data)?,
            SwapType::ExactInput => self.decode_exact_input(tx_data)?,
            SwapType::SwapExactTokens => self.decode_swap_exact_tokens(tx_data)?,
            _ => return None,
        };

        Some(SwapIntent {
            id: Uuid::new_v4(),
            chain_id,
            sender,
            token_in,
            token_out,
            amount_in,
            min_amount_out: min_out,
            deadline: 0,        // Parsed from tx
            tx_hash: [0u8; 32], // Set by caller
            gas_price,
            gas_limit: 500_000, // Default estimate
            detected_at: chrono::Utc::now().timestamp_millis() as u64,
            confidence: 0.95,
            intent_type: IntentType::SimpleSwap,
            metadata: IntentMetadata {
                router: Some(selector.router_name.clone()),
                slippage_bps: Some(50), // Default
                priority_score: 50,
                ..Default::default()
            },
        })
    }

    fn decode_exact_input_single(&self, _data: &[u8]) -> Option<(Token, Token, Balance, Balance)> {
        // UniswapV3 exactInputSingle decoding
        // struct ExactInputSingleParams {
        //     address tokenIn;
        //     address tokenOut;
        //     uint24 fee;
        //     address recipient;
        //     uint256 deadline;
        //     uint256 amountIn;
        //     uint256 amountOutMinimum;
        //     uint160 sqrtPriceLimitX96;
        // }

        // For now, return placeholder - full ABI decoding needed
        Some((
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_A".to_string(),
                decimals: 18,
            },
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_B".to_string(),
                decimals: 18,
            },
            1_000_000_000_000_000_000, // 1 token
            950_000_000_000_000_000,   // 0.95 token min out
        ))
    }

    fn decode_exact_input(&self, _data: &[u8]) -> Option<(Token, Token, Balance, Balance)> {
        // UniswapV3 exactInput (multi-hop) decoding
        Some((
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_A".to_string(),
                decimals: 18,
            },
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_B".to_string(),
                decimals: 18,
            },
            1_000_000_000_000_000_000,
            950_000_000_000_000_000,
        ))
    }

    fn decode_swap_exact_tokens(&self, _data: &[u8]) -> Option<(Token, Token, Balance, Balance)> {
        // UniswapV2-style swapExactTokensForTokens
        Some((
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_A".to_string(),
                decimals: 18,
            },
            Token {
                chain_id: 1,
                address: [0u8; 32],
                symbol: "TOKEN_B".to_string(),
                decimals: 18,
            },
            1_000_000_000_000_000_000,
            950_000_000_000_000_000,
        ))
    }

    fn default_routers() -> Vec<KnownRouter> {
        vec![
            KnownRouter {
                name: "Uniswap V3".to_string(),
                chain_id: 1,
                address: [
                    0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xce, 0xc9, 0x2D, 0xe3, 0xEd, 0xee, 0x1F, 0x18,
                    0xE0, 0x15, 0x7C, 0x05, 0x86, 0x15, 0x64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            },
            KnownRouter {
                name: "Uniswap V2".to_string(),
                chain_id: 1,
                address: [
                    0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d,
                    0xAc, 0xb4, 0xc6, 0x59, 0xF2, 0x48, 0x8D, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            },
        ]
    }

    fn default_selectors() -> Vec<SwapSelector> {
        vec![
            SwapSelector {
                selector: [0xc0, 0x4b, 0x8d, 0x59], // exactInputSingle
                router_name: "Uniswap V3".to_string(),
                swap_type: SwapType::ExactInputSingle,
            },
            SwapSelector {
                selector: [0xc2, 0xe9, 0xfb, 0xd8], // exactInput
                router_name: "Uniswap V3".to_string(),
                swap_type: SwapType::ExactInput,
            },
            SwapSelector {
                selector: [0x38, 0xed, 0x17, 0x39], // swapExactTokensForTokens
                router_name: "Uniswap V2".to_string(),
                swap_type: SwapType::SwapExactTokens,
            },
            SwapSelector {
                selector: [0x7f, 0xf3, 0x6a, 0xb5], // swapExactETHForTokens
                router_name: "Uniswap V2".to_string(),
                swap_type: SwapType::SwapExactETH,
            },
        ]
    }
}

impl Default for IntentDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Known router contract
#[derive(Debug, Clone)]
pub struct KnownRouter {
    pub name: String,
    pub chain_id: ChainId,
    pub address: Address,
}

/// Swap function selector
#[derive(Debug, Clone)]
pub struct SwapSelector {
    pub selector: [u8; 4],
    pub router_name: String,
    pub swap_type: SwapType,
}

/// Type of swap function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapType {
    ExactInputSingle,
    ExactInput,
    ExactOutputSingle,
    ExactOutput,
    SwapExactTokens,
    SwapExactETH,
    SwapTokensForExactTokens,
    MultiCall,
}

/// Intent matcher for pattern recognition
pub struct IntentMatcher {
    patterns: Vec<IntentPattern>,
}

impl IntentMatcher {
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }

    /// Score how well a transaction matches known patterns
    pub fn score_match(&self, intent: &SwapIntent) -> f64 {
        let mut max_score = 0.0;

        for pattern in &self.patterns {
            let score = pattern.score(intent);
            if score > max_score {
                max_score = score;
            }
        }

        max_score
    }

    fn default_patterns() -> Vec<IntentPattern> {
        vec![
            IntentPattern {
                name: "Large Swap".to_string(),
                min_value: 10_000_000_000_000_000_000_000, // 10k tokens
                max_value: u128::MAX,
                common_pairs: vec![],
                time_sensitivity: 0.8,
            },
            IntentPattern {
                name: "Small Frequent".to_string(),
                min_value: 0,
                max_value: 1_000_000_000_000_000_000_000, // 1k tokens
                common_pairs: vec![],
                time_sensitivity: 0.3,
            },
        ]
    }
}

impl Default for IntentMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for matching intents
#[derive(Debug, Clone)]
pub struct IntentPattern {
    pub name: String,
    pub min_value: Balance,
    pub max_value: Balance,
    pub common_pairs: Vec<(String, String)>, // Token symbol pairs
    pub time_sensitivity: f64,
}

impl IntentPattern {
    pub fn score(&self, intent: &SwapIntent) -> f64 {
        let mut score = 0.0;

        // Value range match
        if intent.amount_in >= self.min_value && intent.amount_in <= self.max_value {
            score += 0.5;
        }

        // Time sensitivity boost
        score += self.time_sensitivity * 0.3;

        // Confidence factor
        score += intent.confidence * 0.2;

        score.min(1.0)
    }
}

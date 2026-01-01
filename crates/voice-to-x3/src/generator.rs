//! X3 Code Generator - Natural Language to X3 Smart Contracts

use std::collections::HashMap;

use crate::error::{VoiceError, VoiceResult};
use crate::intent::{ContractType, Intent, ParamValue};
use crate::templates::Templates;

/// Generated X3 contract output
#[derive(Debug, Clone)]
pub struct GeneratedContract {
    /// Contract name
    pub name: String,
    /// Generated X3 source code
    pub code: String,
    /// Contract type
    pub contract_type: ContractType,
    /// Parameters used in generation
    pub params: HashMap<String, ParamValue>,
    /// Warnings/suggestions for the user
    pub warnings: Vec<String>,
}

/// Code generator that transforms intents into X3 code
pub struct CodeGenerator {
    /// Custom template overrides
    custom_templates: HashMap<ContractType, String>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            custom_templates: HashMap::new(),
        }
    }

    /// Generate X3 code from a parsed intent
    pub fn generate(&self, intent: &Intent) -> VoiceResult<GeneratedContract> {
        let mut warnings = Vec::new();

        // Check confidence
        if intent.confidence < 0.7 {
            warnings.push(format!(
                "Low confidence ({:.0}%) in intent detection. Review generated code carefully.",
                intent.confidence * 100.0
            ));
        }

        let code = match intent.contract_type {
            ContractType::Token => self.generate_token(intent, &mut warnings)?,
            ContractType::NFT => self.generate_nft(intent, &mut warnings)?,
            ContractType::DEX => self.generate_dex(intent, &mut warnings)?,
            ContractType::Vault => self.generate_vault(intent, &mut warnings)?,
            ContractType::Governance => self.generate_governance(intent, &mut warnings)?,
            ContractType::Bridge => self.generate_bridge(intent, &mut warnings)?,
            ContractType::Lending => self.generate_lending(intent, &mut warnings)?,
            ContractType::Oracle => self.generate_oracle(intent, &mut warnings)?,
            ContractType::MultiSig => self.generate_multisig(intent, &mut warnings)?,
            ContractType::Custom => {
                return Err(VoiceError::CodeGenError(
                    "Custom contracts not yet supported. Please specify a contract type."
                        .to_string(),
                ))
            }
        };

        Ok(GeneratedContract {
            name: intent.name.clone(),
            code,
            contract_type: intent.contract_type,
            params: intent.params.clone(),
            warnings,
        })
    }

    fn generate_token(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        let name = &intent.name;

        let symbol = intent
            .params
            .get("symbol")
            .and_then(|v| v.as_string())
            .map(String::from)
            .unwrap_or_else(|| {
                warnings.push("No symbol specified, using contract name as symbol".to_string());
                name.to_uppercase()
            });

        let decimals = intent
            .params
            .get("decimals")
            .and_then(|v| v.as_number())
            .unwrap_or(18) as u8;

        let total_supply = intent
            .params
            .get("total_supply")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No supply specified, defaulting to 1,000,000 tokens".to_string());
                1_000_000
            })
            * 10u128.pow(decimals as u32);

        let mintable = intent
            .params
            .get("mintable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let burnable = intent
            .params
            .get("burnable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Templates::token(
            name,
            &symbol,
            decimals,
            total_supply,
            mintable,
            burnable,
        ))
    }

    fn generate_nft(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        let name = &intent.name;
        let symbol = name.chars().take(4).collect::<String>().to_uppercase();

        let max_supply = intent
            .params
            .get("max_supply")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No max supply specified, defaulting to 10,000".to_string());
                10_000
            }) as u64;

        let royalty_percent = intent
            .params
            .get("royalty_percent")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No royalty specified, defaulting to 5%".to_string());
                5
            }) as u8;

        if royalty_percent > 25 {
            warnings.push(format!(
                "High royalty ({}%) may deter buyers",
                royalty_percent
            ));
        }

        Ok(Templates::nft(name, &symbol, max_supply, royalty_percent))
    }

    fn generate_dex(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        let name = &intent.name;

        let fee_percent = intent
            .params
            .get("fee_percent")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No fee specified, defaulting to 0.3%".to_string());
                30
            }) as u16;

        if fee_percent > 100 {
            warnings.push(format!(
                "High fee ({}%) may be uncompetitive",
                fee_percent as f64 / 100.0
            ));
        }

        Ok(Templates::dex(name, fee_percent))
    }

    fn generate_vault(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        let name = &intent.name;

        let lock_days = intent
            .params
            .get("lock_days")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No lock period specified, defaulting to 30 days".to_string());
                30
            }) as u32;

        let target_apy = intent
            .params
            .get("target_apy")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No APY specified, defaulting to 10%".to_string());
                1000 // basis points
            }) as u16;

        if target_apy > 10000 {
            warnings.push("Very high APY may not be sustainable".to_string());
        }

        Ok(Templates::vault(name, lock_days, target_apy))
    }

    fn generate_governance(
        &self,
        intent: &Intent,
        warnings: &mut Vec<String>,
    ) -> VoiceResult<String> {
        let name = &intent.name;

        let quorum_percent = intent
            .params
            .get("quorum_percent")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No quorum specified, defaulting to 10%".to_string());
                10
            }) as u8;

        let voting_period_days = intent
            .params
            .get("voting_period_days")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No voting period specified, defaulting to 7 days".to_string());
                7
            }) as u32;

        Ok(Templates::governance(
            name,
            quorum_percent,
            voting_period_days,
        ))
    }

    fn generate_bridge(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        warnings.push("Bridge contracts require careful security auditing".to_string());

        let name = &intent.name;

        Ok(format!(
            r#"// X3 Bridge Contract: {name}
// Auto-generated by Voice-to-X3
// WARNING: Bridge contracts are high-risk - professional audit required

contract {name} {{
    storage {{
        owner: address;
        validators: map<address, bool>;
        validator_count: u64;
        required_confirmations: u64;
        nonce: u256;
        processed: map<bytes32, bool>;
        pending_transfers: map<bytes32, PendingTransfer>;
    }}

    struct PendingTransfer {{
        from_chain: u32;
        to_chain: u32;
        token: address;
        sender: address;
        recipient: address;
        amount: u256;
        confirmations: u64;
        validators_confirmed: map<address, bool>;
    }}

    event TransferInitiated {{
        transfer_id: bytes32 indexed;
        token: address indexed;
        sender: address indexed;
        recipient: address;
        amount: u256;
        to_chain: u32;
    }}

    event TransferConfirmed {{
        transfer_id: bytes32 indexed;
        validator: address indexed;
    }}

    event TransferExecuted {{
        transfer_id: bytes32 indexed;
        recipient: address indexed;
        amount: u256;
    }}

    fn init(owner: address, required_confirmations: u64) {{
        self.owner = owner;
        self.required_confirmations = required_confirmations;
        self.nonce = 0;
    }}

    fn add_validator(validator: address) {{
        require(msg.sender == self.owner, "Only owner");
        require(!self.validators[validator], "Already validator");
        self.validators[validator] = true;
        self.validator_count += 1;
    }}

    fn remove_validator(validator: address) {{
        require(msg.sender == self.owner, "Only owner");
        require(self.validators[validator], "Not validator");
        self.validators[validator] = false;
        self.validator_count -= 1;
    }}

    fn initiate_transfer(token: address, recipient: address, amount: u256, to_chain: u32) -> bytes32 {{
        require(amount > 0, "Amount must be positive");
        
        self.nonce += 1;
        let transfer_id = keccak256(abi.encode(block.chainid, to_chain, token, msg.sender, recipient, amount, self.nonce));
        
        // Lock tokens in bridge
        ERC20(token).transfer_from(msg.sender, address(this), amount);
        
        emit TransferInitiated {{
            transfer_id: transfer_id,
            token: token,
            sender: msg.sender,
            recipient: recipient,
            amount: amount,
            to_chain: to_chain
        }};
        
        return transfer_id;
    }}

    fn confirm_transfer(transfer_id: bytes32, from_chain: u32, token: address, sender: address, recipient: address, amount: u256) {{
        require(self.validators[msg.sender], "Not validator");
        require(!self.processed[transfer_id], "Already processed");
        
        let pending = self.pending_transfers[transfer_id];
        if pending.amount == 0 {{
            self.pending_transfers[transfer_id] = PendingTransfer {{
                from_chain: from_chain,
                to_chain: block.chainid,
                token: token,
                sender: sender,
                recipient: recipient,
                amount: amount,
                confirmations: 0,
                validators_confirmed: map<address, bool>{{}}
            }};
        }}
        
        require(!self.pending_transfers[transfer_id].validators_confirmed[msg.sender], "Already confirmed");
        
        self.pending_transfers[transfer_id].validators_confirmed[msg.sender] = true;
        self.pending_transfers[transfer_id].confirmations += 1;
        
        emit TransferConfirmed {{ transfer_id: transfer_id, validator: msg.sender }};
        
        // Execute if enough confirmations
        if self.pending_transfers[transfer_id].confirmations >= self.required_confirmations {{
            self._execute_transfer(transfer_id);
        }}
    }}

    fn _execute_transfer(transfer_id: bytes32) {{
        let transfer = self.pending_transfers[transfer_id];
        self.processed[transfer_id] = true;
        
        // Release tokens to recipient
        ERC20(transfer.token).transfer(transfer.recipient, transfer.amount);
        
        emit TransferExecuted {{
            transfer_id: transfer_id,
            recipient: transfer.recipient,
            amount: transfer.amount
        }};
    }}
}}
"#
        ))
    }

    fn generate_lending(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        warnings.push("Lending protocols require careful risk management and auditing".to_string());

        let name = &intent.name;

        Ok(format!(
            r#"// X3 Lending Contract: {name}
// Auto-generated by Voice-to-X3

contract {name} {{
    storage {{
        owner: address;
        collateral_factor: u256 = 75; // 75% LTV
        liquidation_threshold: u256 = 80;
        liquidation_bonus: u256 = 5;
        base_rate: u256 = 2; // 2% base APR
        
        markets: map<address, Market>;
        user_deposits: map<address, map<address, u256>>; // user -> token -> amount
        user_borrows: map<address, map<address, u256>>; // user -> token -> amount
    }}

    struct Market {{
        is_active: bool;
        total_supply: u256;
        total_borrows: u256;
        last_update: u64;
        borrow_index: u256;
        supply_index: u256;
    }}

    event Deposit {{
        user: address indexed;
        token: address indexed;
        amount: u256;
    }}

    event Withdraw {{
        user: address indexed;
        token: address indexed;
        amount: u256;
    }}

    event Borrow {{
        user: address indexed;
        token: address indexed;
        amount: u256;
    }}

    event Repay {{
        user: address indexed;
        token: address indexed;
        amount: u256;
    }}

    event Liquidation {{
        liquidator: address indexed;
        borrower: address indexed;
        debt_token: address;
        collateral_token: address;
        debt_repaid: u256;
        collateral_seized: u256;
    }}

    fn init(owner: address) {{
        self.owner = owner;
    }}

    fn add_market(token: address) {{
        require(msg.sender == self.owner, "Only owner");
        self.markets[token] = Market {{
            is_active: true,
            total_supply: 0,
            total_borrows: 0,
            last_update: block.timestamp,
            borrow_index: 1e18,
            supply_index: 1e18
        }};
    }}

    fn deposit(token: address, amount: u256) {{
        require(self.markets[token].is_active, "Market not active");
        require(amount > 0, "Amount must be positive");
        
        ERC20(token).transfer_from(msg.sender, address(this), amount);
        
        self.user_deposits[msg.sender][token] += amount;
        self.markets[token].total_supply += amount;
        
        emit Deposit {{ user: msg.sender, token: token, amount: amount }};
    }}

    fn withdraw(token: address, amount: u256) {{
        require(self.user_deposits[msg.sender][token] >= amount, "Insufficient deposit");
        require(self._is_healthy(msg.sender), "Would become undercollateralized");
        
        self.user_deposits[msg.sender][token] -= amount;
        self.markets[token].total_supply -= amount;
        
        ERC20(token).transfer(msg.sender, amount);
        
        emit Withdraw {{ user: msg.sender, token: token, amount: amount }};
    }}

    fn borrow(token: address, amount: u256) {{
        require(self.markets[token].is_active, "Market not active");
        require(self.markets[token].total_supply - self.markets[token].total_borrows >= amount, "Insufficient liquidity");
        
        self.user_borrows[msg.sender][token] += amount;
        self.markets[token].total_borrows += amount;
        
        require(self._is_healthy(msg.sender), "Undercollateralized");
        
        ERC20(token).transfer(msg.sender, amount);
        
        emit Borrow {{ user: msg.sender, token: token, amount: amount }};
    }}

    fn repay(token: address, amount: u256) {{
        let debt = self.user_borrows[msg.sender][token];
        let repay_amount = min(amount, debt);
        
        ERC20(token).transfer_from(msg.sender, address(this), repay_amount);
        
        self.user_borrows[msg.sender][token] -= repay_amount;
        self.markets[token].total_borrows -= repay_amount;
        
        emit Repay {{ user: msg.sender, token: token, amount: repay_amount }};
    }}

    fn liquidate(borrower: address, debt_token: address, collateral_token: address, amount: u256) {{
        require(!self._is_healthy(borrower), "Position is healthy");
        
        let debt = self.user_borrows[borrower][debt_token];
        let max_liquidatable = debt / 2; // Can liquidate up to 50%
        let repay_amount = min(amount, max_liquidatable);
        
        // Calculate collateral to seize (with bonus)
        let collateral_amount = (repay_amount * (100 + self.liquidation_bonus)) / 100;
        
        require(self.user_deposits[borrower][collateral_token] >= collateral_amount, "Insufficient collateral");
        
        // Transfer debt from liquidator
        ERC20(debt_token).transfer_from(msg.sender, address(this), repay_amount);
        
        // Update balances
        self.user_borrows[borrower][debt_token] -= repay_amount;
        self.user_deposits[borrower][collateral_token] -= collateral_amount;
        self.markets[debt_token].total_borrows -= repay_amount;
        
        // Transfer collateral to liquidator
        ERC20(collateral_token).transfer(msg.sender, collateral_amount);
        
        emit Liquidation {{
            liquidator: msg.sender,
            borrower: borrower,
            debt_token: debt_token,
            collateral_token: collateral_token,
            debt_repaid: repay_amount,
            collateral_seized: collateral_amount
        }};
    }}

    fn _is_healthy(user: address) -> bool {{
        // Simplified: would need oracle for real implementation
        return true; // TODO: Implement with oracle prices
    }}

    fn min(a: u256, b: u256) -> u256 {{
        return if a < b {{ a }} else {{ b }};
    }}
}}
"#
        ))
    }

    fn generate_oracle(&self, intent: &Intent, warnings: &mut Vec<String>) -> VoiceResult<String> {
        warnings.push(
            "Oracle security is critical - consider using established solutions like Chainlink"
                .to_string(),
        );

        let name = &intent.name;

        Ok(format!(
            r#"// X3 Oracle Contract: {name}
// Auto-generated by Voice-to-X3

contract {name} {{
    storage {{
        owner: address;
        authorized_reporters: map<address, bool>;
        prices: map<bytes32, PriceData>;
        heartbeat: u64 = 3600; // 1 hour max staleness
    }}

    struct PriceData {{
        price: u256;
        decimals: u8;
        timestamp: u64;
        round_id: u256;
    }}

    event PriceUpdated {{
        asset_id: bytes32 indexed;
        price: u256;
        timestamp: u64;
        reporter: address indexed;
    }}

    event ReporterAdded {{
        reporter: address indexed;
    }}

    event ReporterRemoved {{
        reporter: address indexed;
    }}

    fn init(owner: address) {{
        self.owner = owner;
        self.authorized_reporters[owner] = true;
    }}

    fn add_reporter(reporter: address) {{
        require(msg.sender == self.owner, "Only owner");
        self.authorized_reporters[reporter] = true;
        emit ReporterAdded {{ reporter: reporter }};
    }}

    fn remove_reporter(reporter: address) {{
        require(msg.sender == self.owner, "Only owner");
        self.authorized_reporters[reporter] = false;
        emit ReporterRemoved {{ reporter: reporter }};
    }}

    fn update_price(asset_id: bytes32, price: u256, decimals: u8) {{
        require(self.authorized_reporters[msg.sender], "Not authorized");
        require(price > 0, "Invalid price");
        
        let current = self.prices[asset_id];
        
        self.prices[asset_id] = PriceData {{
            price: price,
            decimals: decimals,
            timestamp: block.timestamp,
            round_id: current.round_id + 1
        }};
        
        emit PriceUpdated {{
            asset_id: asset_id,
            price: price,
            timestamp: block.timestamp,
            reporter: msg.sender
        }};
    }}

    fn get_price(asset_id: bytes32) -> (u256, u8, u64) {{
        let data = self.prices[asset_id];
        require(data.timestamp > 0, "Price not available");
        require(block.timestamp - data.timestamp <= self.heartbeat, "Price stale");
        
        return (data.price, data.decimals, data.timestamp);
    }}

    fn get_latest_round(asset_id: bytes32) -> PriceData {{
        return self.prices[asset_id];
    }}

    fn is_price_fresh(asset_id: bytes32) -> bool {{
        let data = self.prices[asset_id];
        return data.timestamp > 0 && block.timestamp - data.timestamp <= self.heartbeat;
    }}

    fn set_heartbeat(new_heartbeat: u64) {{
        require(msg.sender == self.owner, "Only owner");
        self.heartbeat = new_heartbeat;
    }}

    // Utility: get asset_id from token addresses
    fn get_asset_id(base: address, quote: address) -> bytes32 {{
        return keccak256(abi.encode(base, quote));
    }}
}}
"#
        ))
    }

    fn generate_multisig(
        &self,
        intent: &Intent,
        warnings: &mut Vec<String>,
    ) -> VoiceResult<String> {
        let name = &intent.name;

        let required_signatures = intent
            .params
            .get("required_signatures")
            .and_then(|v| v.as_number())
            .unwrap_or_else(|| {
                warnings.push("No signature threshold specified, defaulting to 2".to_string());
                2
            }) as u32;

        Ok(format!(
            r#"// X3 MultiSig Wallet: {name}
// Auto-generated by Voice-to-X3

contract {name} {{
    storage {{
        owners: address[];
        is_owner: map<address, bool>;
        required: u32 = {required_signatures};
        transaction_count: u256;
        transactions: map<u256, Transaction>;
        confirmations: map<u256, map<address, bool>>;
    }}

    struct Transaction {{
        to: address;
        value: u256;
        data: bytes;
        executed: bool;
        confirmation_count: u32;
    }}

    event Deposit {{
        sender: address indexed;
        amount: u256;
    }}

    event SubmitTransaction {{
        tx_id: u256 indexed;
        owner: address indexed;
        to: address;
        value: u256;
    }}

    event ConfirmTransaction {{
        tx_id: u256 indexed;
        owner: address indexed;
    }}

    event RevokeConfirmation {{
        tx_id: u256 indexed;
        owner: address indexed;
    }}

    event ExecuteTransaction {{
        tx_id: u256 indexed;
    }}

    fn init(owners: address[]) {{
        require(owners.length >= self.required, "Not enough owners");
        
        for owner in owners {{
            require(owner != address(0), "Invalid owner");
            require(!self.is_owner[owner], "Duplicate owner");
            
            self.is_owner[owner] = true;
            self.owners.push(owner);
        }}
    }}

    // Receive ETH
    receive() {{
        emit Deposit {{ sender: msg.sender, amount: msg.value }};
    }}

    fn submit_transaction(to: address, value: u256, data: bytes) -> u256 {{
        require(self.is_owner[msg.sender], "Not owner");
        
        let tx_id = self.transaction_count;
        self.transaction_count += 1;
        
        self.transactions[tx_id] = Transaction {{
            to: to,
            value: value,
            data: data,
            executed: false,
            confirmation_count: 0
        }};
        
        emit SubmitTransaction {{ tx_id: tx_id, owner: msg.sender, to: to, value: value }};
        
        // Auto-confirm for submitter
        self._confirm(tx_id);
        
        return tx_id;
    }}

    fn confirm_transaction(tx_id: u256) {{
        require(self.is_owner[msg.sender], "Not owner");
        require(tx_id < self.transaction_count, "Invalid tx");
        require(!self.transactions[tx_id].executed, "Already executed");
        require(!self.confirmations[tx_id][msg.sender], "Already confirmed");
        
        self._confirm(tx_id);
    }}

    fn _confirm(tx_id: u256) {{
        self.confirmations[tx_id][msg.sender] = true;
        self.transactions[tx_id].confirmation_count += 1;
        
        emit ConfirmTransaction {{ tx_id: tx_id, owner: msg.sender }};
        
        // Auto-execute if threshold reached
        if self.transactions[tx_id].confirmation_count >= self.required {{
            self._execute(tx_id);
        }}
    }}

    fn revoke_confirmation(tx_id: u256) {{
        require(self.is_owner[msg.sender], "Not owner");
        require(tx_id < self.transaction_count, "Invalid tx");
        require(!self.transactions[tx_id].executed, "Already executed");
        require(self.confirmations[tx_id][msg.sender], "Not confirmed");
        
        self.confirmations[tx_id][msg.sender] = false;
        self.transactions[tx_id].confirmation_count -= 1;
        
        emit RevokeConfirmation {{ tx_id: tx_id, owner: msg.sender }};
    }}

    fn execute_transaction(tx_id: u256) {{
        require(self.is_owner[msg.sender], "Not owner");
        require(tx_id < self.transaction_count, "Invalid tx");
        require(!self.transactions[tx_id].executed, "Already executed");
        require(self.transactions[tx_id].confirmation_count >= self.required, "Not enough confirmations");
        
        self._execute(tx_id);
    }}

    fn _execute(tx_id: u256) {{
        let tx = self.transactions[tx_id];
        self.transactions[tx_id].executed = true;
        
        (bool success, ) = tx.to.call{{value: tx.value}}(tx.data);
        require(success, "Execution failed");
        
        emit ExecuteTransaction {{ tx_id: tx_id }};
    }}

    // View functions
    fn get_transaction(tx_id: u256) -> Transaction {{
        return self.transactions[tx_id];
    }}

    fn get_confirmation_count(tx_id: u256) -> u32 {{
        return self.transactions[tx_id].confirmation_count;
    }}

    fn is_confirmed(tx_id: u256, owner: address) -> bool {{
        return self.confirmations[tx_id][owner];
    }}

    fn get_owners() -> address[] {{
        return self.owners;
    }}

    fn get_pending_transactions() -> u256[] {{
        let mut pending: u256[] = [];
        for i in 0..self.transaction_count {{
            if !self.transactions[i].executed {{
                pending.push(i);
            }}
        }}
        return pending;
    }}
}}
"#
        ))
    }

    /// Register a custom template for a contract type
    pub fn register_template(&mut self, contract_type: ContractType, template: String) {
        self.custom_templates.insert(contract_type, template);
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::IntentParser;

    #[test]
    fn test_generate_token() {
        let parser = IntentParser::new();
        let generator = CodeGenerator::new();

        let intent = parser
            .parse("Create a mintable token called TestToken with 1 million supply and symbol TEST")
            .unwrap();
        let result = generator.generate(&intent);

        assert!(result.is_ok());
        let contract = result.unwrap();
        assert!(contract.code.contains("contract TestToken"));
        assert!(contract.code.contains("fn mint"));
    }

    #[test]
    fn test_generate_nft() {
        let parser = IntentParser::new();
        let generator = CodeGenerator::new();

        let intent = parser
            .parse("Make an NFT collection with 5000 items")
            .unwrap();
        let result = generator.generate(&intent);

        assert!(result.is_ok());
        let contract = result.unwrap();
        assert!(contract.code.contains("max_supply"));
    }
}

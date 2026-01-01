//! Hover information provider.

use crate::document::DocumentStore;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Url};
use std::sync::Arc;

/// Provides hover information.
pub struct HoverProvider {
    documents: Arc<DocumentStore>,
}

impl HoverProvider {
    pub fn new(documents: Arc<DocumentStore>) -> Self {
        Self { documents }
    }

    /// Get hover information at a position.
    pub async fn hover(&self, uri: &Url, position: Position) -> Option<Hover> {
        let doc = self.documents.get(uri)?;
        let word = doc.word_at(position)?;

        let content = self.get_hover_content(&word, uri)?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: None,
        })
    }

    fn get_hover_content(&self, word: &str, uri: &Url) -> Option<String> {
        // Check if it's a .comit file
        if uri.path().ends_with(".comit") || uri.path().ends_with(".atlas") {
            return self.comit_hover(word);
        }

        // Check if it's a Rust file
        if uri.path().ends_with(".rs") {
            return self.rust_hover(word);
        }

        // General Atlas terms
        self.atlas_hover(word)
    }

    fn comit_hover(&self, word: &str) -> Option<String> {
        match word {
            "comit" => Some(
                "## Comit Transaction\n\n\
                An atomic cross-VM transaction that executes on both EVM and SVM with ACID guarantees.\n\n\
                ### Structure\n\
                ```\n\
                comit \"name\" {\n\
                    evm { ... }  // EVM payload (optional)\n\
                    svm { ... }  // SVM payload (optional)\n\
                }\n\
                ```\n\n\
                At least one of `evm` or `svm` must be specified.\n\n\
                ### Execution\n\
                1. Both payloads are validated\n\
                2. EVM executes first\n\
                3. SVM executes second\n\
                4. If either fails, both are rolled back"
                    .to_string(),
            ),
            "evm" => Some(
                "## EVM Payload\n\n\
                Defines Ethereum Virtual Machine execution within a Comit.\n\n\
                ### Properties\n\
                | Property | Type | Description |\n\
                |----------|------|-------------|\n\
                | `contract` | address | Target contract (0x...) |\n\
                | `method` | string | Function name |\n\
                | `args` | array | Function arguments |\n\
                | `gas_limit` | number | Max gas to use |\n\
                | `value` | number | ETH to send (optional) |"
                    .to_string(),
            ),
            "svm" => Some(
                "## SVM Payload\n\n\
                Defines Solana Virtual Machine execution within a Comit.\n\n\
                ### Properties\n\
                | Property | Type | Description |\n\
                |----------|------|-------------|\n\
                | `program` | pubkey | Program ID |\n\
                | `instruction` | number | Instruction index |\n\
                | `accounts` | array | Account metas |\n\
                | `compute_units` | number | Compute budget |\n\
                | `data` | bytes | Instruction data (optional) |"
                    .to_string(),
            ),
            "contract" => Some(
                "## EVM Contract Address\n\n\
                The target smart contract address for EVM execution.\n\n\
                Format: `0x` followed by 40 hexadecimal characters\n\n\
                Example: `\"0x1234567890abcdef1234567890abcdef12345678\"`"
                    .to_string(),
            ),
            "gas_limit" => Some(
                "## Gas Limit\n\n\
                Maximum gas units allowed for EVM execution.\n\n\
                - Typical range: 21,000 - 10,000,000\n\
                - Block gas limit: ~30,000,000\n\n\
                Unused gas is refunded."
                    .to_string(),
            ),
            "compute_units" => Some(
                "## Compute Units\n\n\
                Budget for SVM instruction execution.\n\n\
                - Max per transaction: 1,400,000\n\
                - Default: 200,000\n\n\
                Higher values allow more complex operations."
                    .to_string(),
            ),
            "program" => Some(
                "## Solana Program ID\n\n\
                The target program's public key (base58 encoded).\n\n\
                Example: `\"11111111111111111111111111111111\"`"
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn rust_hover(&self, word: &str) -> Option<String> {
        match word {
            "pallet" => Some(
                "## FRAME Pallet\n\n\
                A modular component of a Substrate runtime.\n\n\
                ### Key Macros\n\
                - `#[pallet::config]` - Configuration trait\n\
                - `#[pallet::storage]` - Storage items\n\
                - `#[pallet::call]` - Dispatchables\n\
                - `#[pallet::event]` - Events\n\
                - `#[pallet::error]` - Errors"
                    .to_string(),
            ),
            "Config" => Some(
                "## Pallet Config Trait\n\n\
                Defines the pallet's configuration requirements.\n\n\
                ```rust\n\
                #[pallet::config]\n\
                pub trait Config: frame_system::Config {\n\
                    type RuntimeEvent: From<Event<Self>>;\n\
                    // ... other associated types\n\
                }\n\
                ```"
                .to_string(),
            ),
            "DispatchResult" => Some(
                "## DispatchResult\n\n\
                Return type for dispatchable functions.\n\n\
                ```rust\n\
                pub type DispatchResult = Result<(), DispatchError>;\n\
                ```\n\n\
                Return `Ok(())` on success or `Err(error)` on failure."
                    .to_string(),
            ),
            "ensure" => Some(
                "## ensure! Macro\n\n\
                Returns an error if a condition is false.\n\n\
                ```rust\n\
                ensure!(condition, Error::<T>::SomeError);\n\
                ```\n\n\
                Equivalent to:\n\
                ```rust\n\
                if !condition {\n\
                    return Err(Error::<T>::SomeError.into());\n\
                }\n\
                ```"
                .to_string(),
            ),
            "StorageValue" => Some(
                "## StorageValue\n\n\
                Single-value storage item.\n\n\
                ```rust\n\
                #[pallet::storage]\n\
                pub type MyValue<T> = StorageValue<_, u32, ValueQuery>;\n\
                ```\n\n\
                Access: `MyValue::<T>::get()`, `MyValue::<T>::put(value)`"
                    .to_string(),
            ),
            "StorageMap" => Some(
                "## StorageMap\n\n\
                Key-value storage map.\n\n\
                ```rust\n\
                #[pallet::storage]\n\
                pub type MyMap<T> = StorageMap<_, Blake2_128Concat, AccountId, Balance>;\n\
                ```\n\n\
                Access: `MyMap::<T>::get(key)`, `MyMap::<T>::insert(key, value)`"
                    .to_string(),
            ),
            "origin" => Some(
                "## Origin\n\n\
                The source of a dispatchable call.\n\n\
                Common origins:\n\
                - `Signed(account)` - From a specific account\n\
                - `Root` - Sudo/governance\n\
                - `None` - Unsigned transaction\n\n\
                Use `ensure_signed(origin)?` to get the signing account."
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn atlas_hover(&self, word: &str) -> Option<String> {
        match word {
            "AtlasKernel" | "atlas_kernel" => Some(
                "## Atlas Kernel Pallet\n\n\
                Core pallet orchestrating dual-VM (EVM + SVM) execution.\n\n\
                ### Key Functions\n\
                - `submit_comit` - Submit atomic cross-VM transaction\n\
                - `authorize_account` - Enable account for Comit submission\n\
                - `revoke_authorization` - Disable account authorization\n\n\
                ### Storage\n\
                - `CanonicalLedger` - Cross-VM canonical state\n\
                - `AuthorizedAccounts` - Accounts allowed to submit Comits"
                    .to_string(),
            ),
            "ComitPayload" => Some(
                "## ComitPayload\n\n\
                Complete Comit transaction payload.\n\n\
                ```rust\n\
                pub struct ComitPayload {\n\
                    pub evm_payload: Option<EvmPayload>,\n\
                    pub svm_payload: Option<SvmPayload>,\n\
                    pub nonce: u64,\n\
                    pub prepare_root: H256,\n\
                }\n\
                ```"
                .to_string(),
            ),
            "EvmPayload" => Some(
                "## EvmPayload\n\n\
                EVM execution payload for Comit transactions.\n\n\
                ```rust\n\
                pub struct EvmPayload {\n\
                    pub contract: H160,\n\
                    pub calldata: Vec<u8>,\n\
                    pub gas_limit: u64,\n\
                    pub value: U256,\n\
                }\n\
                ```"
                .to_string(),
            ),
            "SvmPayload" => Some(
                "## SvmPayload\n\n\
                SVM execution payload for Comit transactions.\n\n\
                ```rust\n\
                pub struct SvmPayload {\n\
                    pub program_id: [u8; 32],\n\
                    pub accounts: Vec<AccountMeta>,\n\
                    pub data: Vec<u8>,\n\
                    pub compute_units: u64,\n\
                }\n\
                ```"
                .to_string(),
            ),
            _ => None,
        }
    }
}

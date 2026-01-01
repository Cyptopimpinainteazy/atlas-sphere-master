//! Code generation templates.

use handlebars::Handlebars;
use serde::Serialize;

/// Template registry.
pub struct Templates {
    pub handlebars: Handlebars<'static>,
}

impl Default for Templates {
    fn default() -> Self {
        Self::new()
    }
}

impl Templates {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        // Register templates
        handlebars
            .register_template_string("solidity_contract", SOLIDITY_CONTRACT)
            .expect("Failed to register solidity template");
        handlebars
            .register_template_string("anchor_program", ANCHOR_PROGRAM)
            .expect("Failed to register anchor template");
        handlebars
            .register_template_string("test_file", TEST_FILE)
            .expect("Failed to register test template");

        Self { handlebars }
    }

    /// Render a template with data.
    pub fn render<T: Serialize>(&self, name: &str, data: &T) -> crate::error::Result<String> {
        self.handlebars
            .render(name, data)
            .map_err(|e| crate::error::CliError::Config(e.to_string()))
    }
}

/// Solidity contract template.
const SOLIDITY_CONTRACT: &str = r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title {{name}}
 * @dev {{description}}
 */
contract {{name}} {
    // State variables
    address public owner;
    
    // Events
    event Initialized(address indexed owner);
    
    // Errors
    error Unauthorized();
    
    // Modifiers
    modifier onlyOwner() {
        if (msg.sender != owner) revert Unauthorized();
        _;
    }
    
    constructor() {
        owner = msg.sender;
        emit Initialized(msg.sender);
    }
    
    // Add your functions here
}
"#;

/// Anchor program template.
const ANCHOR_PROGRAM: &str = r#"use anchor_lang::prelude::*;

declare_id!("{{program_id}}");

#[program]
pub mod {{name}} {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.authority = ctx.accounts.authority.key();
        state.initialized = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + State::INIT_SPACE
    )]
    pub state: Account<'info, State>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct State {
    pub authority: Pubkey,
    pub initialized: bool,
}
"#;

/// Test file template.
const TEST_FILE: &str = r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../{{contract_path}}";

contract {{name}}Test is Test {
    {{contract_name}} public instance;
    
    function setUp() public {
        instance = new {{contract_name}}();
    }
    
    function test_Initialize() public {
        assertEq(instance.owner(), address(this));
    }
}
"#;

#[derive(Serialize)]
pub struct ContractData {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct ProgramData {
    pub name: String,
    pub program_id: String,
}

#[derive(Serialize)]
pub struct TestData {
    pub name: String,
    pub contract_name: String,
    pub contract_path: String,
}

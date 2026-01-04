// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

/**
 * @title X3HtlcEvm
 * @dev Hash Time-Locked Contract for cross-chain atomic swaps between EVM and X3VM
 * 
 * Mirrors the X3VM HTLC opcodes:
 * - HTLC_CREATE: lockTokens(bytes32 secret_hash, uint256 timelock, address recipient)
 * - HTLC_CLAIM: claim(bytes32 preimage)
 * - HTLC_REFUND: refund(bytes32 htlc_id)
 * - HTLC_STATUS: getStatus(bytes32 htlc_id) -> HtlcStatus
 * 
 * # Architecture
 * 
 * ```
 * Alice (EVM)                         Bob (X3VM)
 * 1. Generate secret P
 * 2. Compute H = SHA256(P)
 * 3. lockTokens(H, timelock, bob)
 *    (ETH locked in contract)
 *                              4. claim HTLC with H
 *                              5. X3VM detects claim, reveals P
 * 6. claim(P) <─────────────── Bob reveals P
 *    (unlock ETH using P)
 * 7. Bob gets ETH
 *                              8. Alice gets X3 tokens
 * 
 * Timeout path (after timelock):
 * - If Bob never claims: Alice calls refund() to recover ETH
 * ```
 */

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
}

import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract X3HtlcEvm is ReentrancyGuard {
    // ========================================================================
    // Events
    // ========================================================================

    event HtlcCreated(
        bytes32 indexed htlcId,
        address indexed initiator,
        address indexed recipient,
        uint256 amount,
        bytes32 secretHash,
        uint256 timelock,
        address token
    );

    event HtlcClaimed(
        bytes32 indexed htlcId,
        address indexed claimer,
        bytes32 preimage,
        uint256 amount
    );

    event HtlcRefunded(
        bytes32 indexed htlcId,
        address indexed initiator,
        uint256 amount
    );

    event ProofEmitted(
        bytes32 indexed htlcId,
        uint32 fromChain,
        uint32 toChain,
        address indexed recipientOnTarget,
        uint256 amount,
        bytes32 preimage
    );

    // ========================================================================
    // Data Types
    // ========================================================================

    enum HtlcStatus {
        Locked,    // 0: Waiting for claim
        Claimed,   // 1: Successfully claimed with preimage
        Refunded,  // 2: Refunded after timelock expiry
        Expired    // 3: Timelock expired without claim/refund
    }

    struct HtlcState {
        address initiator;      // Account that created this HTLC
        address recipient;      // Account that can claim this HTLC
        bytes32 secretHash;     // SHA-256 hash of the preimage secret
        uint256 amount;         // Amount of tokens locked
        uint256 timelock;       // Unix timestamp when HTLC expires
        HtlcStatus status;      // Current status
        uint256 createdAt;      // Block timestamp when created
        address claimedBy;      // Account that claimed it (if claimed)
        uint256 claimedAt;      // Block timestamp when claimed (if claimed)
        address tokenAddress;   // ERC20 token (address(0) for ETH)
    }

    // ========================================================================
    // Storage
    // ========================================================================

    /// HTLC state by ID
    mapping(bytes32 => HtlcState) public htlcStates;

    /// HTLC counter for generating unique IDs
    uint256 public htlcCounter;

    /// Nonce for recipient to prevent replay attacks
    mapping(address => uint256) public nonces;

    // ========================================================================
    // Constants
    // ========================================================================

    /// Chain ID for event emission (set during initialization)
    uint32 public immutable chainId;

    /// Minimum timelock (1 hour)
    uint256 public constant MIN_TIMELOCK = 3600;

    /// Maximum timelock (7 days)
    uint256 public constant MAX_TIMELOCK = 7 days;

    // ========================================================================
    // Constructor
    // ========================================================================

    constructor(uint32 _chainId) {
        chainId = _chainId;
        htlcCounter = 0;
    }

    // ========================================================================
    // HTLC Creation (mirrors HTLC_CREATE)
    // ========================================================================

    /**
     * @dev Create an HTLC lock with hash-time constraints.
     *      Locks ETH or ERC20 tokens and requires preimage to claim.
     * 
     * @param _recipient Address that can claim this HTLC
     * @param _secretHash SHA-256 hash of preimage secret (use SHA256 not keccak256!)
     * @param _timelock Unix timestamp when HTLC expires (must be 1h-7d from now)
     * @param _tokenAddress Address of ERC20 token, or address(0) for ETH
     * @param _amount Amount to lock
     * 
     * @return htlcId Unique identifier for this HTLC
     */
    function lockTokens(
        address _recipient,
        bytes32 _secretHash,
        uint256 _timelock,
        address _tokenAddress,
        uint256 _amount
    ) external payable returns (bytes32) {
        require(_recipient != address(0), "Invalid recipient");
        require(_secretHash != bytes32(0), "Invalid secret hash");
        require(_amount > 0, "Invalid amount");

        // Validate timelock constraints
        uint256 currentTime = block.timestamp;
        uint256 lockDuration = _timelock - currentTime;
        require(lockDuration >= MIN_TIMELOCK, "Timelock too short (min 1 hour)");
        require(lockDuration <= MAX_TIMELOCK, "Timelock too long (max 7 days)");

        // Handle token transfers
        if (_tokenAddress == address(0)) {
            // ETH transfer
            require(msg.value == _amount, "ETH amount mismatch");
        } else {
            // ERC20 token transfer
            require(msg.value == 0, "Cannot send ETH with ERC20");
            IERC20(_tokenAddress).transferFrom(msg.sender, address(this), _amount);
        }

        // Generate HTLC ID (deterministic based on parameters)
        bytes32 htlcId = keccak256(
            abi.encodePacked(
                msg.sender,
                _recipient,
                _secretHash,
                _amount,
                _timelock,
                htlcCounter
            )
        );

        // Ensure no duplicate
        require(htlcStates[htlcId].initiator == address(0), "HTLC already exists");

        // Create HTLC state
        htlcStates[htlcId] = HtlcState({
            initiator: msg.sender,
            recipient: _recipient,
            secretHash: _secretHash,
            amount: _amount,
            timelock: _timelock,
            status: HtlcStatus.Locked,
            createdAt: currentTime,
            claimedBy: address(0),
            claimedAt: 0,
            tokenAddress: _tokenAddress
        });

        htlcCounter++;

        emit HtlcCreated(
            htlcId,
            msg.sender,
            _recipient,
            _amount,
            _secretHash,
            _timelock,
            _tokenAddress
        );

        return htlcId;
    }

    // ========================================================================
    // HTLC Claiming (mirrors HTLC_CLAIM)
    // ========================================================================

    /**
     * @dev Claim an HTLC by revealing the preimage.
     *      Only the recipient can claim, and must provide valid preimage.
     * 
     * @param _htlcId ID of the HTLC to claim
     * @param _preimage Raw preimage secret (not hashed)
     * 
     * @return success True if claim succeeded
     */
    function claim(bytes32 _htlcId, bytes32 _preimage) external returns (bool) {
        HtlcState storage htlc = htlcStates[_htlcId];

        // Verify HTLC exists and is claimable
        require(htlc.initiator != address(0), "HTLC not found");
        require(htlc.status == HtlcStatus.Locked, "HTLC not locked");
        require(msg.sender == htlc.recipient, "Only recipient can claim");

        // Verify preimage matches secret hash
        // CRITICAL: Use SHA-256 (Bitcoin-compatible) NOT keccak256!
        bytes32 computedHash = sha256(abi.encodePacked(_preimage));
        require(computedHash == htlc.secretHash, "Invalid preimage");

        // Update HTLC state
        htlc.status = HtlcStatus.Claimed;
        htlc.claimedBy = msg.sender;
        htlc.claimedAt = block.timestamp;

        // Transfer tokens to claimer
        if (htlc.tokenAddress == address(0)) {
            // Transfer ETH
            (bool success, ) = msg.sender.call{ value: htlc.amount }("");
            require(success, "ETH transfer failed");
        } else {
            // Transfer ERC20
            IERC20(htlc.tokenAddress).transfer(msg.sender, htlc.amount);
        }

        emit HtlcClaimed(_htlcId, msg.sender, _preimage, htlc.amount);

        // Emit proof for cross-chain relay
        // Target chain will receive this and verify using preimage
        emit ProofEmitted(
            _htlcId,
            chainId,
            1,  // Target chain ID (X3VM chain = 1)
            htlc.initiator,  // Recipient on target chain
            htlc.amount,
            _preimage
        );

        return true;
    }

    // ========================================================================
    // HTLC Refund (mirrors HTLC_REFUND)
    // ========================================================================

    /**
     * @dev Refund an HTLC after timelock expiry.
     *      Only the initiator can refund, and only after timelock expires.
     * 
     * @param _htlcId ID of the HTLC to refund
     * 
     * @return success True if refund succeeded
     */
    function refund(bytes32 _htlcId) external returns (bool) {
        HtlcState storage htlc = htlcStates[_htlcId];

        // Verify HTLC exists
        require(htlc.initiator != address(0), "HTLC not found");

        // Verify timelock has expired
        require(block.timestamp >= htlc.timelock, "Timelock not expired");

        // Verify HTLC is still claimable (not already claimed or refunded)
        require(htlc.status == HtlcStatus.Locked, "HTLC already claimed or refunded");

        // Verify caller is initiator
        require(msg.sender == htlc.initiator, "Only initiator can refund");

        // Update HTLC state
        htlc.status = HtlcStatus.Refunded;

        // Transfer tokens back to initiator
        if (htlc.tokenAddress == address(0)) {
            // Transfer ETH
            (bool success, ) = htlc.initiator.call{ value: htlc.amount }("");
            require(success, "ETH transfer failed");
        } else {
            // Transfer ERC20
            IERC20(htlc.tokenAddress).transfer(htlc.initiator, htlc.amount);
        }

        emit HtlcRefunded(_htlcId, htlc.initiator, htlc.amount);

        return true;
    }

    // ========================================================================
    // HTLC Status Query (mirrors HTLC_STATUS)
    // ========================================================================

    /**
     * @dev Query the status of an HTLC.
     * 
     * @param _htlcId ID of the HTLC to query
     * 
     * @return status Current HtlcStatus (0=locked, 1=claimed, 2=refunded, 3=expired)
     */
    function getStatus(bytes32 _htlcId) external view returns (uint8) {
        HtlcState storage htlc = htlcStates[_htlcId];
        
        if (htlc.initiator == address(0)) {
            revert("HTLC not found");
        }

        // Check if timelock has expired without action
        if (htlc.status == HtlcStatus.Locked && block.timestamp >= htlc.timelock) {
            return uint8(HtlcStatus.Expired);
        }

        return uint8(htlc.status);
    }

    /**
     * @dev Get full HTLC state for verification.
     * 
     * @param _htlcId ID of the HTLC to query
     */
    function getHtlc(bytes32 _htlcId) external view returns (HtlcState memory) {
        require(htlcStates[_htlcId].initiator != address(0), "HTLC not found");
        return htlcStates[_htlcId];
    }

    // ========================================================================
    // Proof Relay Helpers
    // ========================================================================

    /**
     * @dev Verify that a preimage matches a secret hash.
     *      Used by cross-chain relayers to validate proofs.
     * 
     * @param _preimage Preimage secret
     * @param _secretHash Expected hash
     * 
     * @return valid True if preimage is valid
     */
    function verifyPreimage(bytes32 _preimage, bytes32 _secretHash)
        external
        pure
        returns (bool)
    {
        return sha256(abi.encodePacked(_preimage)) == _secretHash;
    }

    /**
     * @dev Get HTLC details for relay verification.
     *      Allows off-chain relayers to verify state before relaying proof.
     * 
     * @param _htlcId ID of the HTLC
     */
    function getHtlcForRelay(bytes32 _htlcId)
        external
        view
        returns (
            address recipient,
            bytes32 secretHash,
            uint256 amount,
            HtlcStatus status
        )
    {
        HtlcState storage htlc = htlcStates[_htlcId];
        require(htlc.initiator != address(0), "HTLC not found");
        
        return (htlc.recipient, htlc.secretHash, htlc.amount, htlc.status);
    }

    // ========================================================================
    // Emergency Functions
    // ========================================================================

    /**
     * @dev Emergency withdrawal (only for contract owner - can be extended to governance).
     */
    function emergencyWithdraw(address _token) external {
        // In production, this should check ownership/governance
        // For now, just revert to demonstrate pattern
        revert("Emergency withdraw disabled");
    }
}

/// Validator Setup and Key Rotation for Atlas Sphere
///
/// Handles validator registration, session key derivation, and key rotation scheduling.

use sp_core::{sr25519, ed25519};
use std::sync::Arc;

/// Validator configuration
#[derive(Clone, Debug)]
pub struct ValidatorConfig {
	/// Validator account ID
	pub account_id: Vec<u8>,
	/// Aura public key for block production
	pub aura_key: sr25519::Public,
	/// GRANDPA public key for finality
	pub grandpa_key: ed25519::Public,
	/// Session keys
	pub session_keys: SessionKeys,
	/// Stake amount
	pub stake: u128,
	/// Registration block
	pub registered_at: u32,
}

/// Session keys for consensus participation
#[derive(Clone, Debug)]
pub struct SessionKeys {
	/// Aura session key
	pub aura: sr25519::Public,
	/// GRANDPA session key
	pub grandpa: ed25519::Public,
	/// Authority index in session
	pub authority_index: u32,
}

impl SessionKeys {
	/// Create new session keys
	pub fn new(aura: sr25519::Public, grandpa: ed25519::Public, authority_index: u32) -> Self {
		Self {
			aura,
			grandpa,
			authority_index,
		}
	}
}

/// Key rotation schedule
#[derive(Clone, Debug)]
pub struct KeyRotationSchedule {
	/// Next rotation block
	pub next_rotation_block: u32,
	/// Rotation period in blocks
	pub rotation_period: u32,
	/// Pending new keys
	pub pending_keys: Option<SessionKeys>,
	/// Last rotation block
	pub last_rotation_block: u32,
}

impl KeyRotationSchedule {
	/// Create new key rotation schedule
	pub fn new(rotation_period: u32) -> Self {
		Self {
			next_rotation_block: rotation_period,
			rotation_period,
			pending_keys: None,
			last_rotation_block: 0,
		}
	}

	/// Check if rotation is needed
	pub fn should_rotate(&self, current_block: u32) -> bool {
		current_block >= self.next_rotation_block
	}

	/// Schedule next rotation
	pub fn schedule_next_rotation(&mut self, current_block: u32) {
		self.next_rotation_block = current_block + self.rotation_period;
		self.last_rotation_block = current_block;
	}

	/// Set pending keys for next rotation
	pub fn set_pending_keys(&mut self, keys: SessionKeys) {
		self.pending_keys = Some(keys);
	}

	/// Consume pending keys
	pub fn consume_pending_keys(&mut self) -> Option<SessionKeys> {
		self.pending_keys.take()
	}
}

/// Validator registry
pub struct ValidatorRegistry {
	validators: Vec<ValidatorConfig>,
	key_rotation_schedules: std::collections::HashMap<Vec<u8>, KeyRotationSchedule>,
}

impl ValidatorRegistry {
	/// Create new validator registry
	pub fn new() -> Self {
		Self {
			validators: Vec::new(),
			key_rotation_schedules: std::collections::HashMap::new(),
		}
	}

	/// Register a new validator
	pub fn register_validator(
		&mut self,
		config: ValidatorConfig,
		rotation_period: u32,
	) -> Result<(), &'static str> {
		// Check if already registered
		if self.validators.iter().any(|v| v.account_id == config.account_id) {
			return Err("Validator already registered");
		}

		self.validators.push(config.clone());
		self.key_rotation_schedules.insert(
			config.account_id.clone(),
			KeyRotationSchedule::new(rotation_period),
		);

		Ok(())
	}

	/// Unregister a validator
	pub fn unregister_validator(&mut self, account_id: &[u8]) -> Result<(), &'static str> {
		if let Some(pos) = self.validators.iter().position(|v| v.account_id == account_id) {
			self.validators.remove(pos);
			self.key_rotation_schedules.remove(account_id);
			Ok(())
		} else {
			Err("Validator not found")
		}
	}

	/// Get validator config
	pub fn get_validator(&self, account_id: &[u8]) -> Option<&ValidatorConfig> {
		self.validators.iter().find(|v| v.account_id == account_id)
	}

	/// Get all validators
	pub fn validators(&self) -> &[ValidatorConfig] {
		&self.validators
	}

	/// Get key rotation schedule for validator
	pub fn get_rotation_schedule(&self, account_id: &[u8]) -> Option<&KeyRotationSchedule> {
		self.key_rotation_schedules.get(account_id)
	}

	/// Update key rotation schedule
	pub fn update_rotation_schedule(
		&mut self,
		account_id: &[u8],
		schedule: KeyRotationSchedule,
	) -> Result<(), &'static str> {
		if self.key_rotation_schedules.contains_key(account_id) {
			self.key_rotation_schedules.insert(account_id.to_vec(), schedule);
			Ok(())
		} else {
			Err("Validator not found")
		}
	}

	/// Rotate keys for validator
	pub fn rotate_keys(
		&mut self,
		account_id: &[u8],
		new_keys: SessionKeys,
	) -> Result<(), &'static str> {
		// Find validator and update keys
		if let Some(validator) = self.validators.iter_mut().find(|v| v.account_id == account_id) {
			validator.session_keys = new_keys.clone();

			// Update rotation schedule
			if let Some(schedule) = self.key_rotation_schedules.get_mut(account_id) {
				schedule.pending_keys = None;
			}

			Ok(())
		} else {
			Err("Validator not found")
		}
	}

	/// Check validators that need key rotation
	pub fn validators_needing_rotation(&self, current_block: u32) -> Vec<Vec<u8>> {
		self.key_rotation_schedules
			.iter()
			.filter_map(|(id, schedule)| {
				if schedule.should_rotate(current_block) {
					Some(id.clone())
				} else {
					None
				}
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_key_rotation_schedule() {
		let mut schedule = KeyRotationSchedule::new(100);
		assert!(!schedule.should_rotate(50));
		assert!(schedule.should_rotate(100));
		assert!(schedule.should_rotate(101));

		schedule.schedule_next_rotation(100);
		assert_eq!(schedule.next_rotation_block, 200);
		assert_eq!(schedule.last_rotation_block, 100);
	}

	#[test]
	fn test_validator_registry() {
		let mut registry = ValidatorRegistry::new();

		let config = ValidatorConfig {
			account_id: vec![1, 2, 3],
			aura_key: sr25519::Public::from_raw([0u8; 32]),
			grandpa_key: ed25519::Public::from_raw([0u8; 32]),
			session_keys: SessionKeys::new(
				sr25519::Public::from_raw([0u8; 32]),
				ed25519::Public::from_raw([0u8; 32]),
				0,
			),
			stake: 1000,
			registered_at: 0,
		};

		assert!(registry.register_validator(config, 100).is_ok());
		assert_eq!(registry.validators().len(), 1);
	}
}

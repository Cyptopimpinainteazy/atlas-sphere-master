use super::*;

use crate::mock::{new_test_ext, AtomicGuardian, RuntimeOrigin, System, Test};
use asga_receipts::{
    Attestation, AttestationScheme, BtcReceipt, DomainId, EvmReceipt, Phase, Receipt, ReceiptHeader,
    ReceiptPayload,
};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;
use sp_runtime::transaction_validity::TransactionSource;

fn make_attested_receipt(
    pair: &sr25519::Pair,
    intent_id: [u8; 32],
    domain_id: DomainId,
    phase: Phase,
) -> AttestedReceipt {
    let header = ReceiptHeader {
        intent_id,
        domain_id,
        phase,
        amount: 1,
        asset_id: [1u8; 32],
        timestamp: 1,
        signer: vec![9u8; 33],
    };

    let payload = match domain_id {
        DomainId::Evm => ReceiptPayload::Evm(EvmReceipt {
            tx_hash: [2u8; 32],
            block_number: 7,
            confirmations: 1,
            contract_address: [3u8; 20],
            calldata_hash: [4u8; 32],
        }),
        DomainId::Btc => ReceiptPayload::Btc(BtcReceipt {
            txid: [5u8; 32],
            vout: 0,
            script_hash: [6u8; 20],
            confirmations: 1,
            locktime: 0,
        }),
        DomainId::Svm => ReceiptPayload::Svm(asga_receipts::SvmReceipt {
            signature: [7u8; 64],
            slot: 1,
            program_id: [8u8; 32],
            escrow_pda: [9u8; 32],
        }),
        DomainId::X3 => ReceiptPayload::X3(asga_receipts::X3Receipt {
            block_hash: [10u8; 32],
            runtime_version: 1,
            arbiter_signature: [11u8; 64],
        }),
    };

    let receipt = Receipt { header, payload };
    let msg = receipt.encode();
    let sig = pair.sign(&msg);

    AttestedReceipt {
        receipt,
        attestation: Attestation {
            attester_pubkey: pair.public().0,
            scheme: AttestationScheme::Sr25519,
            signature: sig.0.to_vec(),
        },
    }
}

#[test]
fn unsigned_validation_rejects_unregistered_attester() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let pair = sr25519::Pair::from_seed(&[1u8; 32]);
        let intent_id = [42u8; 32];

        assert_ok!(AtomicGuardian::submit_intent(
            RuntimeOrigin::signed(1),
            intent_id,
            vec![DomainId::Evm],
            10
        ));

        let attested = make_attested_receipt(&pair, intent_id, DomainId::Evm, Phase::Lock);
        let call = crate::Call::<Test>::submit_attested_receipt_unsigned { attested };

        let validity = <crate::Pallet<Test> as frame_support::unsigned::ValidateUnsigned>::validate_unsigned(
            TransactionSource::External,
            &call,
        );

        assert!(validity.is_err());
    });
}

#[test]
fn advances_phases_when_all_required_domains_submit_receipts() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let pair = sr25519::Pair::from_seed(&[2u8; 32]);
        let attester = pair.public().0;
        let intent_id = [7u8; 32];

        assert_ok!(AtomicGuardian::register_attester(RuntimeOrigin::root(), attester));
        assert_ok!(AtomicGuardian::submit_intent(
            RuntimeOrigin::signed(1),
            intent_id,
            vec![DomainId::Evm, DomainId::Btc],
            10
        ));

        // LOCK phase
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Evm, Phase::Lock)
        ));
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Btc, Phase::Lock)
        ));
        assert_eq!(AtomicGuardian::progress(intent_id).phase, Phase::Exec);

        // EXEC phase
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Evm, Phase::Exec)
        ));
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Btc, Phase::Exec)
        ));
        assert_eq!(AtomicGuardian::progress(intent_id).phase, Phase::Final);

        // FINAL phase
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Evm, Phase::Final)
        ));
        assert_ok!(AtomicGuardian::submit_attested_receipt_unsigned(
            RuntimeOrigin::none(),
            make_attested_receipt(&pair, intent_id, DomainId::Btc, Phase::Final)
        ));

        let progress = AtomicGuardian::progress(intent_id);
        assert_eq!(progress.phase, Phase::Final);
        assert_eq!(progress.status, crate::SwapStatus::Completed);
    });
}

#[test]
fn rejects_bad_signature() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let pair = sr25519::Pair::from_seed(&[3u8; 32]);
        let attacker = sr25519::Pair::from_seed(&[9u8; 32]);
        let attester = pair.public().0;
        let intent_id = [1u8; 32];

        assert_ok!(AtomicGuardian::register_attester(RuntimeOrigin::root(), attester));
        assert_ok!(AtomicGuardian::submit_intent(
            RuntimeOrigin::signed(1),
            intent_id,
            vec![DomainId::Evm],
            10
        ));

        // Create a receipt but sign with the wrong key.
        let mut attested = make_attested_receipt(&pair, intent_id, DomainId::Evm, Phase::Lock);
        let msg = attested.receipt.encode();
        let bad_sig = attacker.sign(&msg);
        attested.attestation.signature = bad_sig.0.to_vec();

        assert_noop!(
            AtomicGuardian::submit_attested_receipt_unsigned(RuntimeOrigin::none(), attested),
            crate::Error::<Test>::BadAttestation
        );
    });
}

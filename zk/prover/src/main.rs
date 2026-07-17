//! Dario Dash prover CLI.
//!
//! Commands:
//!   export-constants <out_dir>   Write contract verification constants
//!                                (image id, control root, bn254 control id,
//!                                pvk blob, gamma_abc points).
//!   prove <run.json> <out.json>  Prove a recorded run and emit a proof
//!                                bundle ready for on-chain submission.
//!
//! All arkworks material is produced with ark 0.4 so byte formats match
//! Dusk's `verify_groth16_bn254` host function exactly.

mod snark;
mod vk;

use std::fs;

use anyhow::{anyhow, bail, Context, Result};
use ark_bn254::{Bn254, Fr, G1Projective};
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use base64::Engine;
use risc0_zkvm::sha::Digestible;
use risc0_zkvm::{
    default_prover, ExecutorEnv, Groth16ReceiptVerifierParameters, ProverOpts, ReceiptClaim,
};
use serde::{Deserialize, Serialize};

use dash_methods::{DASH_GUEST_ELF, DASH_GUEST_ID};

const ACCOUNT_LEN: usize = 96;

/// ark 0.4 errors don't implement std::error::Error; map via Debug.
fn ae<E: core::fmt::Debug>(e: E) -> anyhow::Error {
    anyhow!("{e:?}")
}


#[derive(Deserialize)]
struct RunFile {
    /// bs58-encoded Moonlight public account (96 compressed bytes).
    account: String,
    seed: u64,
    /// base64-encoded input trace, one byte per tick.
    trace_b64: String,
}

#[derive(Serialize)]
struct ProofBundle {
    account: String,
    seed: u64,
    score: u64,
    ticks: u32,
    /// ark-0.4 compressed Groth16 proof (128 bytes), hex.
    proof_hex: String,
    /// Guest journal bytes, hex (account || seed || score || ticks).
    journal_hex: String,
    /// True when produced under RISC0_DEV_MODE (not on-chain verifiable).
    dev_mode: bool,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("export-constants") => {
            let out = args.get(2).map(String::as_str).unwrap_or("../contract/assets");
            export_constants(out)
        }
        Some("prove") => {
            let run = args.get(2).context("usage: prove <run.json> <out.json>")?;
            let out = args.get(3).context("usage: prove <run.json> <out.json>")?;
            prove(run, out)
        }
        Some("export-snarkjs-vkey") => {
            let vkey = args.get(2).context("usage: export-snarkjs-vkey <vkey.json> <out_dir> <prefix>")?;
            let out = args.get(3).context("usage: export-snarkjs-vkey <vkey.json> <out_dir> <prefix>")?;
            let prefix = args.get(4).context("usage: export-snarkjs-vkey <vkey.json> <out_dir> <prefix>")?;
            snark::export_snarkjs_vkey(vkey, out, prefix)
        }
        Some("verify-ark-proof") => {
            let vkey = args.get(2).context("usage: verify-ark-proof <vkey.json> <proof_hex_file> <public.json>")?;
            let proof_hex = fs::read_to_string(
                args.get(3).context("usage: verify-ark-proof <vkey.json> <proof_hex_file> <public.json>")?,
            )?;
            let publics = args.get(4).context("usage: verify-ark-proof <vkey.json> <proof_hex_file> <public.json>")?;
            snark::verify_ark_proof(vkey, &proof_hex, publics)
        }
        _ => {
            eprintln!("usage: dash-prover <export-constants [out_dir] | prove <run.json> <out.json> | export-snarkjs-vkey ... | verify-ark-proof ...>");
            std::process::exit(2);
        }
    }
}

fn image_id_bytes() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, word) in DASH_GUEST_ID.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn export_constants(out_dir: &str) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    let params = Groth16ReceiptVerifierParameters::default();

    fs::write(format!("{out_dir}/image_id.bin"), image_id_bytes())?;
    fs::write(
        format!("{out_dir}/control_root.bin"),
        params.control_root.as_bytes(),
    )?;
    fs::write(
        format!("{out_dir}/bn254_control_id.bin"),
        params.bn254_control_id.as_bytes(),
    )?;

    let vk = vk::risc0_verifying_key()?;
    let pvk = ark_groth16::prepare_verifying_key(&vk);
    let mut pvk_bytes = Vec::new();
    pvk.serialize_uncompressed(&mut pvk_bytes).map_err(ae)?;
    fs::write(format!("{out_dir}/pvk.bin"), &pvk_bytes)?;

    let mut gamma_abc = Vec::new();
    for point in &vk.gamma_abc_g1 {
        point.serialize_uncompressed(&mut gamma_abc).map_err(ae)?;
    }
    fs::write(format!("{out_dir}/gamma_abc.bin"), &gamma_abc)?;

    println!("image id:          {}", hex::encode(image_id_bytes()));
    println!("control root:      {}", hex::encode(params.control_root.as_bytes()));
    println!("bn254 control id:  {}", hex::encode(params.bn254_control_id.as_bytes()));
    println!("pvk:               {} bytes", pvk_bytes.len());
    println!("wrote constants to {out_dir}");
    Ok(())
}

fn prove(run_path: &str, out_path: &str) -> Result<()> {
    let run: RunFile =
        serde_json::from_str(&fs::read_to_string(run_path).context("reading run file")?)?;

    let account: [u8; ACCOUNT_LEN] = bs58::decode(&run.account)
        .into_vec()
        .context("invalid bs58 account")?
        .try_into()
        .map_err(|_| anyhow!("account must decode to {ACCOUNT_LEN} bytes"))?;
    let trace = base64::engine::general_purpose::STANDARD
        .decode(&run.trace_b64)
        .context("invalid base64 trace")?;

    // Local pre-check so we fail fast on bad traces.
    let expected = dash_core::replay(run.seed, &trace);
    if !expected.over {
        bail!("trace does not end in game over; refusing to prove");
    }
    println!(
        "replayed locally: score {} over {} ticks",
        expected.score, expected.ticks
    );

    let mut input = Vec::with_capacity(ACCOUNT_LEN + 12 + trace.len());
    input.extend_from_slice(&account);
    input.extend_from_slice(&run.seed.to_le_bytes());
    input.extend_from_slice(&(trace.len() as u32).to_le_bytes());
    input.extend_from_slice(&trace);

    let env = ExecutorEnv::builder()
        .write_slice(&input)
        .build()
        .map_err(|e| anyhow!("building executor env: {e}"))?;

    println!("proving (Groth16 wrap; this can take a few minutes)...");
    let receipt = default_prover()
        .prove_with_opts(env, DASH_GUEST_ELF, &ProverOpts::groth16())
        .map_err(|e| anyhow!("proving failed: {e}"))?
        .receipt;

    receipt
        .verify(DASH_GUEST_ID)
        .map_err(|e| anyhow!("receipt self-verification failed: {e}"))?;

    let journal = receipt.journal.bytes.clone();
    if journal.len() != ACCOUNT_LEN + 8 + 8 + 4 {
        bail!("unexpected journal length {}", journal.len());
    }
    let score = u64::from_le_bytes(journal[ACCOUNT_LEN + 8..ACCOUNT_LEN + 16].try_into()?);
    let ticks = u32::from_le_bytes(journal[ACCOUNT_LEN + 16..ACCOUNT_LEN + 20].try_into()?);
    if score != expected.score || ticks != expected.ticks {
        bail!("guest result mismatch: local {expected:?} vs journal ({score}, {ticks})");
    }

    let (proof_hex, dev_mode) = match receipt.inner.groth16() {
        Ok(groth16) => {
            let proof = seal_to_ark_proof(&groth16.seal)?;
            let mut bytes = Vec::new();
            proof.serialize_compressed(&mut bytes).map_err(ae)?;

            // Mirror the exact on-chain verification before emitting.
            verify_like_contract(&proof, &journal)?;
            println!("proof verified against contract-equivalent pipeline ✔");
            (hex::encode(bytes), false)
        }
        Err(_) => {
            println!("WARNING: no Groth16 seal (dev mode?); bundle is NOT on-chain verifiable");
            (String::new(), true)
        }
    };

    let bundle = ProofBundle {
        account: run.account,
        seed: run.seed,
        score,
        ticks,
        proof_hex,
        journal_hex: hex::encode(&journal),
        dev_mode,
    };
    fs::write(out_path, serde_json::to_string_pretty(&bundle)?)?;
    println!("wrote proof bundle to {out_path} (score {score}, {ticks} ticks)");
    Ok(())
}

/// Converts a risc0 256-byte big-endian gnark seal into an ark 0.4 proof.
fn seal_to_ark_proof(seal: &[u8]) -> Result<Proof<Bn254>> {
    if seal.len() != 256 {
        bail!("seal must be 256 bytes, got {}", seal.len());
    }
    let rev = |chunk: &[u8]| -> Vec<u8> { chunk.iter().rev().cloned().collect() };

    // a: (x, y) big-endian -> ark little-endian uncompressed
    let a_bytes = [rev(&seal[0..32]), rev(&seal[32..64])].concat();
    // b: ((x_c1, x_c0), (y_c1, y_c0)) big-endian -> ark (x_c0, x_c1, y_c0, y_c1)
    let b_bytes = [
        rev(&seal[96..128]),
        rev(&seal[64..96]),
        rev(&seal[160..192]),
        rev(&seal[128..160]),
    ]
    .concat();
    let c_bytes = [rev(&seal[192..224]), rev(&seal[224..256])].concat();

    Ok(Proof {
        a: ark_bn254::G1Affine::deserialize_uncompressed(&*a_bytes).map_err(ae)?,
        b: ark_bn254::G2Affine::deserialize_uncompressed(&*b_bytes).map_err(ae)?,
        c: ark_bn254::G1Affine::deserialize_uncompressed(&*c_bytes).map_err(ae)?,
    })
}

/// Recomputes the claim digest and public inputs exactly like the contract
/// and verifies the proof with ark 0.4, i.e. what Dusk's host function runs.
fn verify_like_contract(proof: &Proof<Bn254>, journal: &[u8]) -> Result<()> {
    let params = Groth16ReceiptVerifierParameters::default();
    let claim_digest = ReceiptClaim::ok(DASH_GUEST_ID, journal.to_vec()).digest();

    let inputs = public_inputs(
        params.control_root.as_bytes().try_into()?,
        claim_digest.as_bytes().try_into()?,
        params.bn254_control_id.as_bytes().try_into()?,
    );

    let vk = vk::risc0_verifying_key()?;
    let pvk: PreparedVerifyingKey<Bn254> = ark_groth16::prepare_verifying_key(&vk);
    let prepared = Groth16::<Bn254>::prepare_inputs(&pvk, &inputs).map_err(ae)?;

    // Round-trip through the exact byte formats Dusk deserializes.
    let mut pvk_bytes = Vec::new();
    pvk.serialize_uncompressed(&mut pvk_bytes).map_err(ae)?;
    let mut proof_bytes = Vec::new();
    proof.serialize_compressed(&mut proof_bytes).map_err(ae)?;
    let mut inputs_bytes = Vec::new();
    prepared.into_affine().serialize_compressed(&mut inputs_bytes).map_err(ae)?;

    let pvk = PreparedVerifyingKey::<Bn254>::deserialize_uncompressed(&pvk_bytes[..]).map_err(ae)?;
    let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..]).map_err(ae)?;
    let prepared = G1Projective::deserialize_compressed(&inputs_bytes[..]).map_err(ae)?;

    match Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, &proof, &prepared).map_err(ae)? {
        true => Ok(()),
        false => Err(anyhow!("Groth16 verification failed")),
    }
}

/// The 5 Fr public inputs: split(control_root), split(claim_digest), bn254 id.
/// Digest halves are interpreted as little-endian 128-bit integers.
pub fn public_inputs(
    control_root: [u8; 32],
    claim_digest: [u8; 32],
    bn254_control_id: [u8; 32],
) -> [Fr; 5] {
    [
        Fr::from_le_bytes_mod_order(&control_root[0..16]),
        Fr::from_le_bytes_mod_order(&control_root[16..32]),
        Fr::from_le_bytes_mod_order(&claim_digest[0..16]),
        Fr::from_le_bytes_mod_order(&claim_digest[16..32]),
        Fr::from_le_bytes_mod_order(&bn254_control_id),
    ]
}

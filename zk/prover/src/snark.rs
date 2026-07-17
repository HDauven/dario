//! snarkjs (circom) Groth16 BN254 interop.
//!
//! Converts snarkjs `vkey.json` into the same ark-0.4 asset blobs the
//! contract embeds (`PreparedVerifyingKey` + `gamma_abc` points), and
//! verifies browser-produced 128-byte compressed proofs through the exact
//! pipeline Dusk's `verify_groth16_bn254` host function runs.

use anyhow::{anyhow, bail, Context, Result};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use num_bigint::BigUint;
use serde::Deserialize;
use std::fs;

fn ae<E: core::fmt::Debug>(e: E) -> anyhow::Error {
    anyhow!("{e:?}")
}

#[derive(Deserialize)]
struct SnarkVkey {
    protocol: String,
    curve: String,
    #[serde(rename = "nPublic")]
    n_public: usize,
    vk_alpha_1: Vec<String>,
    vk_beta_2: Vec<Vec<String>>,
    vk_gamma_2: Vec<Vec<String>>,
    vk_delta_2: Vec<Vec<String>>,
    #[serde(rename = "IC")]
    ic: Vec<Vec<String>>,
}

fn fq(dec: &str) -> Result<Fq> {
    let n: BigUint = dec.parse().map_err(|e| anyhow!("bad decimal: {e}"))?;
    Ok(Fq::from_le_bytes_mod_order(&n.to_bytes_le()))
}

/// snarkjs G1: [x, y, z] projective decimal strings with z in {0, 1}.
fn g1(coords: &[String]) -> Result<G1Affine> {
    if coords.len() != 3 {
        bail!("G1 point must have 3 coordinates");
    }
    if coords[2] == "0" {
        return Ok(G1Affine::identity());
    }
    let p = G1Affine::new_unchecked(fq(&coords[0])?, fq(&coords[1])?);
    if !p.is_on_curve() || !p.is_in_correct_subgroup_assuming_on_curve() {
        bail!("G1 point not on curve");
    }
    Ok(p)
}

/// snarkjs G2: [[x_c0, x_c1], [y_c0, y_c1], [z_c0, z_c1]].
fn g2(coords: &[Vec<String>]) -> Result<G2Affine> {
    if coords.len() != 3 || coords.iter().any(|c| c.len() != 2) {
        bail!("G2 point must have 3x2 coordinates");
    }
    if coords[2][0] == "0" && coords[2][1] == "0" {
        return Ok(G2Affine::identity());
    }
    let p = G2Affine::new_unchecked(
        Fq2::new(fq(&coords[0][0])?, fq(&coords[0][1])?),
        Fq2::new(fq(&coords[1][0])?, fq(&coords[1][1])?),
    );
    if !p.is_on_curve() || !p.is_in_correct_subgroup_assuming_on_curve() {
        bail!("G2 point not on curve");
    }
    Ok(p)
}

fn load_vkey(path: &str) -> Result<VerifyingKey<Bn254>> {
    let vkey: SnarkVkey =
        serde_json::from_str(&fs::read_to_string(path).context("reading vkey")?)?;
    if vkey.protocol != "groth16" || vkey.curve != "bn128" {
        bail!("expected groth16/bn128 vkey, got {}/{}", vkey.protocol, vkey.curve);
    }
    if vkey.ic.len() != vkey.n_public + 1 {
        bail!("IC length {} != nPublic + 1", vkey.ic.len());
    }
    Ok(VerifyingKey {
        alpha_g1: g1(&vkey.vk_alpha_1)?,
        beta_g2: g2(&vkey.vk_beta_2)?,
        gamma_g2: g2(&vkey.vk_gamma_2)?,
        delta_g2: g2(&vkey.vk_delta_2)?,
        gamma_abc_g1: vkey.ic.iter().map(|p| g1(p)).collect::<Result<_>>()?,
    })
}

/// Writes `<prefix>_pvk.bin` and `<prefix>_gamma_abc.bin` contract assets
/// from a snarkjs vkey.
pub fn export_snarkjs_vkey(vkey_path: &str, out_dir: &str, prefix: &str) -> Result<()> {
    fs::create_dir_all(out_dir)?;
    let vk = load_vkey(vkey_path)?;

    // The host verifies with *prepared* inputs, so the pvk's embedded
    // gamma_abc_g1 vector is never used. Strip it to keep the pvk small
    // enough for the VM's inter-contract argument buffer (64 KiB) — the
    // full point list is exported separately for input preparation.
    let mut stripped = vk.clone();
    stripped.gamma_abc_g1 = alloc_first_point(&vk)?;
    let pvk = ark_groth16::prepare_verifying_key(&stripped);

    let mut pvk_bytes = Vec::new();
    pvk.serialize_uncompressed(&mut pvk_bytes).map_err(ae)?;
    fs::write(format!("{out_dir}/{prefix}_pvk.bin"), &pvk_bytes)?;

    let mut gamma_abc = Vec::new();
    for point in &vk.gamma_abc_g1 {
        point.serialize_uncompressed(&mut gamma_abc).map_err(ae)?;
    }
    fs::write(format!("{out_dir}/{prefix}_gamma_abc.bin"), &gamma_abc)?;

    println!(
        "wrote {prefix}_pvk.bin ({} bytes) and {prefix}_gamma_abc.bin ({} G1 points)",
        pvk_bytes.len(),
        vk.gamma_abc_g1.len()
    );
    Ok(())
}

fn alloc_first_point(
    vk: &VerifyingKey<Bn254>,
) -> Result<Vec<<Bn254 as ark_ec::pairing::Pairing>::G1Affine>> {
    vk.gamma_abc_g1
        .first()
        .cloned()
        .map(|p| vec![p])
        .ok_or_else(|| anyhow!("vkey has no gamma_abc points"))
}

/// Verifies a browser-produced proof exactly like the contract will:
/// 128-byte ark-compressed proof + decimal public inputs, MSM over
/// gamma_abc, then `verify_proof_with_prepared_inputs`.
pub fn verify_ark_proof(vkey_path: &str, proof_hex: &str, public_path: &str) -> Result<()> {
    let vk = load_vkey(vkey_path)?;
    let pvk: PreparedVerifyingKey<Bn254> = ark_groth16::prepare_verifying_key(&vk);

    let proof_bytes = hex::decode(proof_hex.trim()).context("decoding proof hex")?;
    let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..]).map_err(ae)?;

    let publics: Vec<String> =
        serde_json::from_str(&fs::read_to_string(public_path).context("reading publics")?)?;
    let inputs: Vec<Fr> = publics
        .iter()
        .map(|d| {
            let n: BigUint = d.parse().map_err(|e| anyhow!("bad public input: {e}"))?;
            Ok(Fr::from_le_bytes_mod_order(&n.to_bytes_le()))
        })
        .collect::<Result<_>>()?;

    let prepared = Groth16::<Bn254>::prepare_inputs(&pvk, &inputs).map_err(ae)?;

    // Round-trip the exact byte formats the host function deserializes.
    let mut pvk_bytes = Vec::new();
    pvk.serialize_uncompressed(&mut pvk_bytes).map_err(ae)?;
    let mut inputs_bytes = Vec::new();
    use ark_ec::CurveGroup;
    prepared
        .into_affine()
        .serialize_compressed(&mut inputs_bytes)
        .map_err(ae)?;
    let pvk =
        PreparedVerifyingKey::<Bn254>::deserialize_uncompressed(&pvk_bytes[..]).map_err(ae)?;
    let prepared = G1Projective::deserialize_compressed(&inputs_bytes[..]).map_err(ae)?;

    match Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, &proof, &prepared)
        .map_err(ae)?
    {
        true => {
            println!("proof verified via contract-equivalent pipeline ✔");
            Ok(())
        }
        false => Err(anyhow!("Groth16 verification FAILED")),
    }
}

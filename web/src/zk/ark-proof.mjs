// Converts a snarkjs Groth16 proof (BN254, decimal strings) into the ark-0.4
// compressed byte format Dusk's `verify_groth16_bn254` host function expects:
//   proof = a (G1, 32B) || b (G2, 64B) || c (G1, 32B) = 128 bytes.
//
// ark 0.4 compressed encoding (short Weierstrass):
//   - G1: x as 32 little-endian bytes; flags in the most-significant bits of
//     the LAST byte: bit7 = "y is negative" (y > -y mod p), bit6 = infinity.
//   - G2: x.c0 || x.c1 as 2x32 little-endian bytes; same flags on the last
//     byte, with Fq2 ordered by (c1, c0) for the y comparison.

/** BN254 base field modulus. */
export const P = 21888242871839275222246405745257275088696311157297823662689037894645226208583n;

function fqToLeBytes(x) {
  const out = new Uint8Array(32);
  let v = BigInt(x);
  for (let i = 0; i < 32; i++) {
    out[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return out;
}

/** ark SWFlags::from_y_coordinate: negative iff y > -y (mod p). */
function fqIsNegative(y) {
  const yv = BigInt(y) % P;
  return yv > P - yv;
}

/** Fq2 comparison mirrors ark's QuadExtField Ord: c1 first, then c0. */
function fq2IsNegative(c0, c1) {
  const y1 = BigInt(c1) % P;
  const negY1 = (P - y1) % P;
  if (y1 !== negY1) return y1 > negY1;
  const y0 = BigInt(c0) % P;
  return y0 > (P - y0) % P;
}

function g1Compressed(point) {
  // snarkjs G1: [x, y, z] projective with z = 1 (or 0 for infinity).
  const [x, y, z] = point.map(BigInt);
  const bytes = fqToLeBytes(x % P);
  if (z === 0n) {
    bytes.fill(0);
    bytes[31] |= 1 << 6;
  } else if (fqIsNegative(y)) {
    bytes[31] |= 1 << 7;
  }
  return bytes;
}

function g2Compressed(point) {
  // snarkjs G2: [[x_c0, x_c1], [y_c0, y_c1], [z_c0, z_c1]].
  const [[x0, x1], [y0, y1], [z0, z1]] = point.map((c) => c.map(BigInt));
  const bytes = new Uint8Array(64);
  bytes.set(fqToLeBytes(x0 % P), 0);
  bytes.set(fqToLeBytes(x1 % P), 32);
  if (z0 === 0n && z1 === 0n) {
    bytes.fill(0);
    bytes[63] |= 1 << 6;
  } else if (fq2IsNegative(y0, y1)) {
    bytes[63] |= 1 << 7;
  }
  return bytes;
}

/** snarkjs proof JSON -> 128-byte ark-0.4 compressed proof. */
export function proofToArkBytes(proof) {
  const out = new Uint8Array(128);
  out.set(g1Compressed(proof.pi_a), 0);
  out.set(g2Compressed(proof.pi_b), 32);
  out.set(g1Compressed(proof.pi_c), 96);
  return out;
}

export function toHex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

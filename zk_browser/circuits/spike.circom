pragma circom 2.1.0;

// Pipeline spike: minimal circuit with the SAME public input layout the real
// dash_zk circuit will use:
//   [seed, score, ticks, acct[0..5]]  (9 public inputs, 10 IC points)
// acct limbs are bound by the verification equation alone; the circuit does
// not constrain them.
template Spike() {
    signal input seed;      // public
    signal input score;     // public
    signal input ticks;     // public
    signal input acct[6];   // public (16-byte LE limbs of the 96B account)
    signal input w;         // private witness

    // Dummy relation so the circuit is non-trivial: w^2 = seed, score = w + ticks.
    w * w === seed;
    score === w + ticks;

    // Touch acct so the compiler keeps the signals (public inputs are kept
    // anyway, but be explicit).
    signal accSum;
    accSum <== acct[0] + acct[1] + acct[2] + acct[3] + acct[4] + acct[5];
}

component main {public [seed, score, ticks, acct]} = Spike();

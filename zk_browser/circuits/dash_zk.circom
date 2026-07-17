pragma circom 2.1.0;

// Dario Dash — proof of gameplay.
//
// Proves that a claimed (score, ticks) is achievable against the published
// obstacle/item schedule (recomputed on-chain from the run seed) under the
// exact rules of the 30 Hz provable core in dash_zk/src/lib.rs.
//
// Public input order (defines the on-chain layout):
//   score, ticks, groundCount, batCount, itemCount,
//   gspawn[NG], gw[NG], gh[NG],
//   bspawn[NB], bbase[NB], bphase[NB],
//   ispawn[NI], ikind[NI], iy[NI],
//   acct[6]
// The contract guarantees all public schedule values are in range
// (spawn <= 3600, w/phase < 128, h/base < 512, ikind < 3, iy < 512,
//  counts <= caps, padding entries all-zero), so the circuit does not
// range-check publics.
//
// Fixed point: y-domain fp = px*256, x-domain fp100 = px*25600.
// All tick-valued witnesses are range-checked to 12 bits; padding tick 4000.

include "circomlib/circuits/comparators.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/multiplexer.circom";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// Signed a < b for x-domain (fp100) values, |v| < 2^39.
template SLtX() {
    signal input a;
    signal input b;
    signal output out;
    component lt = LessThan(40);
    lt.in[0] <== a + 549755813888;
    lt.in[1] <== b + 549755813888;
    out <== lt.out;
}

// Signed a < b for y-domain (fp) values, |v| < 2^23.
template SLtY() {
    signal input a;
    signal input b;
    signal output out;
    component lt = LessThan(24);
    lt.in[0] <== a + 8388608;
    lt.in[1] <== b + 8388608;
    out <== lt.out;
}

// Signed a <= b for y-domain values.
template SLeqY() {
    signal input a;
    signal input b;
    signal output out;
    component lt = SLtY();
    lt.a <== b;
    lt.b <== a;
    out <== 1 - lt.out;
}

// Cumulative scroll distance d100(t) in fp100 (dash_zk::d100).
// Caller must ensure 0 <= t < 4096.
template D100() {
    signal input t;
    signal output out;
    component le = LessEqThan(12);
    le.in[0] <== t;
    le.in[1] <== 1433;
    signal t2 <== t * t;
    // low phase: 281600*t + 128*t*(t+1) = 128*t^2 + 281728*t
    // high phase: 648600*t - 262880984
    signal dLow <== 128 * t2 + 281728 * t;
    signal dHigh <== 648600 * t - 262880984;
    out <== le.out * (dLow - dHigh) + dHigh;
}

// Jump displacement (fp, signed) n ticks after the press tick.
// n must be 1..63 (caller range-checks Num2Bits(6) and guards n >= 1).
// regular: 370n^2 - 7480n   (v0 = -7850)
// super:   370n^2 - 8846n   (v0 = -9216)
// cape:    parabola for n < 13, then glide tail 1280n - 51840.
template JumpDisp() {
    signal input n;
    signal input isSup;
    signal input isCap;
    signal output out;
    signal n2 <== n * n;
    signal p0 <== 370 * n2 - 7480 * n;
    signal p1 <== 370 * n2 - 8846 * n;
    component lt = LessThan(6);
    lt.in[0] <== n;
    lt.in[1] <== 13;
    signal tail <== 1280 * n - 51840;
    signal capeD <== lt.out * (p0 - tail) + tail;
    signal dSup <== isSup * (p1 - p0);
    signal dCap <== isCap * (capeD - p0);
    out <== p0 + dSup + dCap;
}

// Bat triangle-wave vertical offset (fp) for phase counter p = 36q + r.
// p must be 0 when unused (then q = r = 0 satisfies).
// q2 = r < 18 ? r : 36 - r;  off = (2*q2 - 18) * 280.
template TriOff() {
    signal input p;
    signal input q;
    signal input r;
    signal output off;
    p === 36 * q + r;
    component qb = Num2Bits(7);
    qb.in <== q;
    component rb = Num2Bits(6);
    rb.in <== r;
    component rlt36 = LessThan(6);
    rlt36.in[0] <== r;
    rlt36.in[1] <== 36;
    rlt36.out === 1;
    component rlt = LessThan(6);
    rlt.in[0] <== r;
    rlt.in[1] <== 18;
    signal q2 <== rlt.out * (2 * r - 36) + 36 - r;
    off <== 560 * q2 - 5040;
}

// Form at tick t = form_after of the last timeline entry with tick < t.
// Virtual index 0 = (tick 0, form Regular, pack 0); index k = entry k-1.
// When need = 0 the selector may be all-zero and every check is gated off.
template FormAt(NE) {
    signal input t;
    signal input need;
    signal input sel[NE + 1];
    signal input packV[NE + 1];     // etick + 4096*formAfter
    signal input tickNextV[NE + 1]; // tick of the following entry (sentinel 4001)
    signal output form;

    var s = 0;
    for (var i = 0; i <= NE; i++) {
        sel[i] * (sel[i] - 1) === 0;
        s += sel[i];
    }
    need === s;

    component dp = EscalarProduct(NE + 1);
    component dn = EscalarProduct(NE + 1);
    for (var i = 0; i <= NE; i++) {
        dp.in1[i] <== sel[i];
        dp.in2[i] <== packV[i];
        dn.in1[i] <== sel[i];
        dn.in2[i] <== tickNextV[i];
    }
    component ub = Num2Bits(15);
    ub.in <== dp.out;
    var tk = 0;
    for (var i = 0; i < 12; i++) { tk += ub.out[i] * (1 << i); }
    form <== ub.out[12] + 2 * ub.out[13] + 4 * ub.out[14];

    component lt = LessThan(13);
    lt.in[0] <== tk;
    lt.in[1] <== t;
    need * (1 - lt.out) === 0;
    component ge = LessEqThan(13);
    ge.in[0] <== t;
    ge.in[1] <== dn.out;
    need * (1 - ge.out) === 0;
}

// Last jump with tick <= t (and the jump after it starts strictly after t).
// Virtual index 0 = "grounded since start" (pack 0); index k = jump k-1.
// WN = 1 additionally unpacks the full parameters of the following jump
// (nextV holds full packs, sentinel 4001); WN = 0 expects nextV = ticks only.
template LastJumpAt(NJ, WN) {
    signal input t;
    signal input need;
    signal input sel[NJ + 1];
    signal input packV[NJ + 1];
    signal input nextV[NJ + 1];
    signal output jt;
    signal output land;
    signal output isSup;
    signal output isCap;
    signal output jt2;
    signal output land2;
    signal output isSup2;
    signal output isCap2;

    var s = 0;
    for (var i = 0; i <= NJ; i++) {
        sel[i] * (sel[i] - 1) === 0;
        s += sel[i];
    }
    need === s;

    component dp = EscalarProduct(NJ + 1);
    component dn = EscalarProduct(NJ + 1);
    for (var i = 0; i <= NJ; i++) {
        dp.in1[i] <== sel[i];
        dp.in2[i] <== packV[i];
        dn.in1[i] <== sel[i];
        dn.in2[i] <== nextV[i];
    }
    component ub = Num2Bits(21);
    ub.in <== dp.out;
    var tkv = 0;
    for (var i = 0; i < 12; i++) { tkv += ub.out[i] * (1 << i); }
    jt <== tkv;
    var lv = 0;
    for (var i = 12; i < 18; i++) { lv += ub.out[i] * (1 << (i - 12)); }
    land <== lv;
    isSup <== ub.out[18];
    isCap <== ub.out[19];

    if (WN == 1) {
        component un = Num2Bits(21);
        un.in <== dn.out;
        var t2v = 0;
        for (var i = 0; i < 12; i++) { t2v += un.out[i] * (1 << i); }
        jt2 <== t2v;
        var l2v = 0;
        for (var i = 12; i < 18; i++) { l2v += un.out[i] * (1 << (i - 12)); }
        land2 <== l2v;
        isSup2 <== un.out[18];
        isCap2 <== un.out[19];
    } else {
        jt2 <== dn.out;
        land2 <== 0;
        isSup2 <== 0;
        isCap2 <== 0;
    }

    component le = LessEqThan(13);
    le.in[0] <== jt;
    le.in[1] <== t;
    need * (1 - le.out) === 0;
    component gt = LessThan(13);
    gt.in[0] <== t;
    gt.in[1] <== jt2;
    need * (1 - gt.out) === 0;
}

// Player y (fp, top-left convention of dash_zk: 118784 = grounded) at tick t
// given the last-jump context. Gated: y = 118784 when need = 0 or grounded.
template PlayerY1() {
    signal input t;
    signal input need;
    signal input jt;
    signal input land;
    signal input isSup;
    signal input isCap;
    signal output y;
    component lt = LessThan(13);
    lt.in[0] <== t;
    lt.in[1] <== jt + land;
    signal air <== need * lt.out;
    signal n0 <== air * (t - jt);
    component z = IsZero();
    z.in <== n0;
    signal ng <== n0 + z.out;
    component nb = Num2Bits(6);
    nb.in <== ng;
    component jd = JumpDisp();
    jd.n <== ng;
    jd.isSup <== isSup;
    jd.isCap <== isCap;
    signal yd <== air * jd.out;
    y <== 118784 + yd;
}

// Player y at tick t when t may fall in either the last jump at some anchor
// tick or the jump immediately after it (used for the <= 8 ticks of a bat
// window). The two air phases are mutually exclusive by the grounded chain.
template PlayerY2() {
    signal input t;
    signal input need;
    signal input jt1;
    signal input land1;
    signal input isSup1;
    signal input isCap1;
    signal input jt2;
    signal input land2;
    signal input isSup2;
    signal input isCap2;
    signal output y;

    component lt1 = LessThan(13);
    lt1.in[0] <== t;
    lt1.in[1] <== jt1 + land1;
    signal a1 <== need * lt1.out;
    signal n10 <== a1 * (t - jt1);
    component z1 = IsZero();
    z1.in <== n10;
    signal n1g <== n10 + z1.out;
    component nb1 = Num2Bits(6);
    nb1.in <== n1g;
    component jd1 = JumpDisp();
    jd1.n <== n1g;
    jd1.isSup <== isSup1;
    jd1.isCap <== isCap1;

    component lt2 = LessThan(13);
    lt2.in[0] <== t;
    lt2.in[1] <== jt2;
    signal a2 <== need * (1 - lt2.out);
    signal n20 <== a2 * (t - jt2);
    component z2 = IsZero();
    z2.in <== n20;
    signal n2g <== n20 + z2.out;
    component nb2 = Num2Bits(6);
    nb2.in <== n2g;
    component jd2 = JumpDisp();
    jd2.n <== n2g;
    jd2.isSup <== isSup2;
    jd2.isCap <== isCap2;

    signal yd1 <== a1 * jd1.out;
    signal yd2 <== a2 * jd2.out;
    y <== 118784 + yd1 + yd2;
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

template DashZK(NG, NB, NI, NJ, NE, NK) {
    // ---- public ----
    signal input score;
    signal input ticks;
    signal input groundCount;
    signal input batCount;
    signal input itemCount;
    signal input gspawn[NG];
    signal input gw[NG];
    signal input gh[NG];
    signal input bspawn[NB];
    signal input bbase[NB];
    signal input bphase[NB];
    signal input ispawn[NI];
    signal input ikind[NI];
    signal input iy[NI];
    signal input acct[6];

    // ---- private witness ----
    signal input jtick[NJ];
    signal input jact[NJ];
    signal input jfsel[NJ][NE + 1];
    signal input etick[NE];
    signal input ekind[NE]; // 0 espresso, 1 chili, 2 cape, 3 damage, 4 touch/noop
    signal input eact[NE];
    signal input eisel[NE][NI];
    signal input eosel[NE][NG + NB];
    signal input ejsel[NE][NJ + 1];
    signal input etq[NE];
    signal input etr[NE];
    signal input kfire[NK];
    signal input khit[NK];
    signal input kact[NK];
    signal input kosel[NK][NG + NB];
    signal input kfsel[NK][NE + 1];
    signal input kjsel[NK][NJ + 1];
    signal input ktq[NK];
    signal input ktr[NK];
    signal input gw1[NG];
    signal input gw2[NG];
    signal input gs[NG][5]; // status onehot: alive, killed, damaged, touched, unreached
    signal input gevt[NG];
    signal input gcsel[NG][NJ];
    signal input bw1[NB];
    signal input bw2[NB];
    signal input bs[NB][5];
    signal input bevt[NB];
    signal input bjsel[NB][NJ + 1];
    signal input btq[NB][8];
    signal input btr[NB][8];
    signal input scoreQ;
    signal input scoreR;

    var NO = NG + NB;

    // ================= section 0: globals =================
    component tb = Num2Bits(12);
    tb.in <== ticks;
    component tle = LessEqThan(12);
    tle.in[0] <== ticks;
    tle.in[1] <== 3600;
    tle.out === 1;

    component dT = D100();
    dT.t <== ticks;

    // Bind acct limbs (public inputs are kept by Groth16 regardless; be explicit).
    signal acctSum <== acct[0] + acct[1] + acct[2] + acct[3] + acct[4] + acct[5];
    signal acctSq <== acctSum * acctSum;

    // ================= section 1: form-event timeline =================
    component etb[NE];
    component ekEq[NE][5];
    signal kk[NE][5];
    signal fs[NE + 1][5];
    signal p[NE][5][5];
    signal formVal[NE];
    signal epackV[NE + 1];
    signal tickNextV[NE + 1];
    signal ldt[NE + 1];
    signal ldtd[NE];
    signal tfAcc[NE + 1];
    signal tfe[NE];
    signal newFatal[NE];
    signal isPick[NE];
    signal isDmg[NE];
    signal isTouch[NE];
    signal isDT[NE];
    component ldtz[NE];
    component ldtGap[NE];
    signal okd1[NE];
    component tchLo[NE];
    component tchHi[NE];
    signal tch1[NE];
    signal tch2[NE];
    component ordGt[NE];
    component ordEq[NE];
    signal ordE1[NE];
    signal ordE2[NE];
    component etLe[NE];

    fs[0][0] <== 1;
    fs[0][1] <== 0;
    fs[0][2] <== 0;
    fs[0][3] <== 0;
    fs[0][4] <== 0;
    epackV[0] <== 0;
    ldt[0] <== 0;
    tfAcc[0] <== 0;
    tickNextV[NE] <== 4001;

    for (var e = 0; e < NE; e++) {
        eact[e] * (eact[e] - 1) === 0;
        if (e > 0) {
            eact[e] * (1 - eact[e - 1]) === 0;
        }
        etb[e] = Num2Bits(12);
        etb[e].in <== etick[e];
        (1 - eact[e]) * (etick[e] - 4000) === 0;
        (1 - eact[e]) * (ekind[e] - 4) === 0;

        var ks = 0;
        for (var k = 0; k < 5; k++) {
            ekEq[e][k] = IsEqual();
            ekEq[e][k].in[0] <== ekind[e];
            ekEq[e][k].in[1] <== k;
            kk[e][k] <== ekEq[e][k].out;
            ks += kk[e][k];
        }
        ks === 1;

        isPick[e] <== kk[e][0] + kk[e][1] + kk[e][2];
        isDmg[e] <== kk[e][3];
        isTouch[e] <== kk[e][4] * eact[e];
        isDT[e] <== isDmg[e] + isTouch[e];

        // FSM transition (dario_fsm table); kind 4 (touch/noop) keeps state.
        for (var s = 0; s < 5; s++) {
            for (var k = 0; k < 5; k++) {
                p[e][s][k] <== fs[e][s] * kk[e][k];
            }
        }
        fs[e + 1][0] <== p[e][0][4] + p[e][1][3] + p[e][2][3] + p[e][3][3];
        fs[e + 1][1] <== p[e][0][0] + p[e][1][0] + p[e][1][4];
        fs[e + 1][2] <== p[e][0][1] + p[e][1][1] + p[e][2][0] + p[e][2][1] + p[e][2][4] + p[e][3][1];
        fs[e + 1][3] <== p[e][0][2] + p[e][1][2] + p[e][2][2] + p[e][3][0] + p[e][3][2] + p[e][3][4];
        fs[e + 1][4] <== p[e][0][3] + p[e][4][0] + p[e][4][1] + p[e][4][2] + p[e][4][3] + p[e][4][4];

        formVal[e] <== fs[e + 1][1] + 2 * fs[e + 1][2] + 3 * fs[e + 1][3] + 4 * fs[e + 1][4];
        epackV[e + 1] <== etick[e] + 4096 * formVal[e];
        tickNextV[e] <== etick[e];

        newFatal[e] <== fs[e + 1][4] - fs[e][4];
        tfe[e] <== newFatal[e] * etick[e];
        tfAcc[e + 1] <== tfAcc[e] + tfe[e];
        if (e < NE - 1) {
            eact[e + 1] * fs[e + 1][4] === 0;
        }

        // last-damage-tick chain; only damage (kind 3) updates it.
        ldtd[e] <== isDmg[e] * (etick[e] - ldt[e]);
        ldt[e + 1] <== ldt[e] + ldtd[e];

        // damage only when not invulnerable: no prior damage, or gap > 38.
        ldtz[e] = IsZero();
        ldtz[e].in <== ldt[e];
        ldtGap[e] = LessThan(13);
        ldtGap[e].in[0] <== ldt[e] + 38;
        ldtGap[e].in[1] <== etick[e];
        okd1[e] <== ldtz[e].out * ldtGap[e].out;
        isDmg[e] * (1 - ldtz[e].out - ldtGap[e].out + okd1[e]) === 0;

        // touch only while invulnerable: prior damage, ldt < etick <= ldt + 38.
        tchLo[e] = LessThan(13);
        tchLo[e].in[0] <== ldt[e];
        tchLo[e].in[1] <== etick[e];
        tchHi[e] = LessEqThan(13);
        tchHi[e].in[0] <== etick[e];
        tchHi[e].in[1] <== ldt[e] + 38;
        tch1[e] <== (1 - ldtz[e].out) * tchLo[e].out;
        tch2[e] <== tch1[e] * tchHi[e].out;
        isTouch[e] * (1 - tch2[e]) === 0;

        // strict tick ordering; equal ticks only for pickup -> damage/touch.
        if (e > 0) {
            ordGt[e] = LessThan(13);
            ordGt[e].in[0] <== etick[e - 1];
            ordGt[e].in[1] <== etick[e];
            ordEq[e] = IsEqual();
            ordEq[e].in[0] <== etick[e - 1];
            ordEq[e].in[1] <== etick[e];
            ordE1[e] <== ordEq[e].out * isPick[e - 1];
            ordE2[e] <== ordE1[e] * (kk[e][3] + kk[e][4]);
            eact[e] * (1 - ordGt[e].out - ordE2[e]) === 0;
        }

        etLe[e] = LessEqThan(12);
        etLe[e].in[0] <== etick[e];
        etLe[e].in[1] <== ticks;
        eact[e] * (1 - etLe[e].out) === 0;
    }

    // Run-end constraints are applied after score calculation so score-capped
    // non-death runs can finish before the two-minute tick cap.
    signal died <== fs[NE][4];

    // ================= section 2: jumps =================
    component jtb[NJ];
    component jfa[NJ];
    component jse[NJ];
    component jce[NJ];
    signal jland[NJ];
    signal jpack[NJ];
    signal jpackV[NJ + 1];
    signal jnextTickV[NJ + 1];
    signal jnextPackV[NJ + 1];
    component jchain[NJ];

    jpackV[0] <== 0;
    jnextTickV[NJ] <== 4001;
    jnextPackV[NJ] <== 4001;

    for (var i = 0; i < NJ; i++) {
        jact[i] * (jact[i] - 1) === 0;
        if (i > 0) {
            jact[i] * (1 - jact[i - 1]) === 0;
        }
        jtb[i] = Num2Bits(12);
        jtb[i].in <== jtick[i];
        (1 - jact[i]) * (jtick[i] - 4000) === 0;

        jfa[i] = FormAt(NE);
        jfa[i].t <== jtick[i];
        jfa[i].need <== 1;
        for (var k = 0; k <= NE; k++) {
            jfa[i].sel[k] <== jfsel[i][k];
            jfa[i].packV[k] <== epackV[k];
            jfa[i].tickNextV[k] <== tickNextV[k];
        }
        jse[i] = IsEqual();
        jse[i].in[0] <== jfa[i].form;
        jse[i].in[1] <== 1;
        jce[i] = IsEqual();
        jce[i].in[0] <== jfa[i].form;
        jce[i].in[1] <== 3;
        jland[i] <== 21 + 3 * jse[i].out + 20 * jce[i].out;
        jpack[i] <== jtick[i] + 4096 * jland[i] + 262144 * jse[i].out + 524288 * jce[i].out + 1048576 * jact[i];
        jpackV[i + 1] <== jpack[i];
        jnextTickV[i] <== jtick[i];
        jnextPackV[i] <== jpack[i];

        // grounded chain: a jump may only start once the previous one landed.
        if (i > 0) {
            jchain[i] = LessEqThan(13);
            jchain[i].in[0] <== jtick[i - 1] + jland[i - 1];
            jchain[i].in[1] <== jtick[i];
            jact[i] * (1 - jchain[i].out) === 0;
        }
    }

    // ================= section 3: schedule packs =================
    component gcnt[NG];
    signal gact[NG];
    component bcnt[NB];
    signal bactv[NB];
    component icnt[NI];
    signal iactv[NI];
    signal opackAll[NO];
    signal statPackAll[NO];
    signal ipackAll[NI];
    component gevb[NG];
    component bevb[NB];

    for (var i = 0; i < NG; i++) {
        gcnt[i] = LessThan(8);
        gcnt[i].in[0] <== i;
        gcnt[i].in[1] <== groundCount;
        gact[i] <== gcnt[i].out;

        var ss = 0;
        for (var k = 0; k < 5; k++) {
            gs[i][k] * (gs[i][k] - 1) === 0;
            ss += gs[i][k];
        }
        ss === 1;
        (1 - gact[i]) * (1 - gs[i][4]) === 0;

        gevb[i] = Num2Bits(12);
        gevb[i].in <== gevt[i];

        opackAll[i] <== gspawn[i] + 4096 * gw[i] + 524288 * gh[i] + 536870912 * gact[i];
        statPackAll[i] <== gevt[i] + 4096 * gs[i][1] + 8192 * gs[i][2] + 16384 * gs[i][3];
    }
    for (var j = 0; j < NB; j++) {
        bcnt[j] = LessThan(5);
        bcnt[j].in[0] <== j;
        bcnt[j].in[1] <== batCount;
        bactv[j] <== bcnt[j].out;

        var ss = 0;
        for (var k = 0; k < 5; k++) {
            bs[j][k] * (bs[j][k] - 1) === 0;
            ss += bs[j][k];
        }
        ss === 1;
        (1 - bactv[j]) * (1 - bs[j][4]) === 0;

        bevb[j] = Num2Bits(12);
        bevb[j].in <== bevt[j];

        opackAll[NG + j] <== bspawn[j] + 4096 * bphase[j] + 524288 * bbase[j] + 268435456 + 536870912 * bactv[j];
        statPackAll[NG + j] <== bevt[j] + 4096 * bs[j][1] + 8192 * bs[j][2] + 16384 * bs[j][3];
    }
    for (var i = 0; i < NI; i++) {
        icnt[i] = LessThan(6);
        icnt[i].in[0] <== i;
        icnt[i].in[1] <== itemCount;
        iactv[i] <== icnt[i].out;
        ipackAll[i] <== ispawn[i] + 4096 * ikind[i] + 16384 * iy[i] + 8388608 * iactv[i];
    }

    // ================= section 4: timeline entry checks =================
    component dte[NE];
    component elj[NE];
    component epy[NE];
    component eidot[NE];
    component eiub[NE];
    signal eIsp[NE];
    signal eIkd[NE];
    signal eIyv[NE];
    component eidsm[NE];
    component eixl[NE];
    component eixr[NE];
    component eiyt[NE];
    component eiyb[NE];
    signal pvIdx[NE + 1];
    signal pvd[NE];
    component pvLt[NE];
    component eodot[NE];
    component eoub[NE];
    component esdot[NE];
    component esub[NE];
    signal eOsp[NE];
    signal eOf1[NE];
    signal eOf2[NE];
    signal eOcls[NE];
    signal eSevt[NE];
    component eodsm[NE];
    signal eRq[NE];
    component eoxl[NE];
    component eoxr[NE];
    signal ePb[NE];
    component etri[NE];
    component eyg[NE];
    component eyb1[NE];
    component eyb2[NE];
    signal eOvB[NE];
    signal eOvd[NE];

    pvIdx[0] <== 0;

    for (var e = 0; e < NE; e++) {
        dte[e] = D100();
        dte[e].t <== etick[e];

        elj[e] = LastJumpAt(NJ, 0);
        elj[e].t <== etick[e];
        elj[e].need <== eact[e];
        for (var k = 0; k <= NJ; k++) {
            elj[e].sel[k] <== ejsel[e][k];
            elj[e].packV[k] <== jpackV[k];
            elj[e].nextV[k] <== jnextTickV[k];
        }
        epy[e] = PlayerY1();
        epy[e].t <== etick[e];
        epy[e].need <== eact[e];
        epy[e].jt <== elj[e].jt;
        epy[e].land <== elj[e].land;
        epy[e].isSup <== elj[e].isSup;
        epy[e].isCap <== elj[e].isCap;

        // ---- pickup (kinds 0..2): consume item eisel[e] ----
        var isv = 0;
        for (var i = 0; i < NI; i++) {
            eisel[e][i] * (eisel[e][i] - 1) === 0;
            isv += eisel[e][i];
        }
        isPick[e] === isv;

        eidot[e] = EscalarProduct(NI);
        for (var i = 0; i < NI; i++) {
            eidot[e].in1[i] <== eisel[e][i];
            eidot[e].in2[i] <== ipackAll[i];
        }
        eiub[e] = Num2Bits(24);
        eiub[e].in <== eidot[e].out;
        var v = 0;
        for (var i = 0; i < 12; i++) { v += eiub[e].out[i] * (1 << i); }
        eIsp[e] <== v;
        eIkd[e] <== eiub[e].out[12] + 2 * eiub[e].out[13];
        v = 0;
        for (var i = 14; i < 23; i++) { v += eiub[e].out[i] * (1 << (i - 14)); }
        eIyv[e] <== v;
        eiub[e].out[23] === isPick[e];
        isPick[e] * (eIkd[e] - ekind[e]) === 0;

        eidsm[e] = D100();
        eidsm[e].t <== eIsp[e] - isPick[e];
        // item x at etick: ITEM_X0 + d100(spawn-1) - d100(t)
        eixl[e] = SLtX();
        eixl[e].a <== 25344000 + eidsm[e].out - dte[e].out;
        eixl[e].b <== 4300800;
        isPick[e] * (1 - eixl[e].out) === 0;
        eixr[e] = SLtX();
        eixr[e].a <== 3532800;
        eixr[e].b <== 25344000 + eidsm[e].out - dte[e].out + 870400;
        isPick[e] * (1 - eixr[e].out) === 0;

        // y overlap: ptop < iy + 34px && pbot > iy
        eiyt[e] = SLtY();
        eiyt[e].a <== epy[e].y - 17408;
        eiyt[e].b <== 256 * eIyv[e] + 8704;
        isPick[e] * (1 - eiyt[e].out) === 0;
        eiyb[e] = SLtY();
        eiyb[e].a <== 256 * eIyv[e];
        eiyb[e].b <== epy[e].y - 1024;
        isPick[e] * (1 - eiyb[e].out) === 0;

        // item indices strictly increasing (each item consumed at most once).
        var idxOf = 0;
        for (var i = 0; i < NI; i++) { idxOf += eisel[e][i] * i; }
        pvLt[e] = LessThan(7);
        pvLt[e].in[0] <== pvIdx[e];
        pvLt[e].in[1] <== idxOf + 1;
        isPick[e] * (1 - pvLt[e].out) === 0;
        pvd[e] <== isPick[e] * (idxOf + 1 - pvIdx[e]);
        pvIdx[e + 1] <== pvIdx[e] + pvd[e];

        // ---- damage / touch (kinds 3..4): collide with obstacle eosel[e] ----
        var osv = 0;
        for (var i = 0; i < NO; i++) {
            eosel[e][i] * (eosel[e][i] - 1) === 0;
            osv += eosel[e][i];
        }
        isDT[e] === osv;

        eodot[e] = EscalarProduct(NO);
        esdot[e] = EscalarProduct(NO);
        for (var i = 0; i < NO; i++) {
            eodot[e].in1[i] <== eosel[e][i];
            eodot[e].in2[i] <== opackAll[i];
            esdot[e].in1[i] <== eosel[e][i];
            esdot[e].in2[i] <== statPackAll[i];
        }
        eoub[e] = Num2Bits(30);
        eoub[e].in <== eodot[e].out;
        v = 0;
        for (var i = 0; i < 12; i++) { v += eoub[e].out[i] * (1 << i); }
        eOsp[e] <== v;
        v = 0;
        for (var i = 12; i < 19; i++) { v += eoub[e].out[i] * (1 << (i - 12)); }
        eOf1[e] <== v;
        v = 0;
        for (var i = 19; i < 28; i++) { v += eoub[e].out[i] * (1 << (i - 19)); }
        eOf2[e] <== v;
        eOcls[e] <== eoub[e].out[28];
        eoub[e].out[29] === isDT[e];

        esub[e] = Num2Bits(15);
        esub[e].in <== esdot[e].out;
        v = 0;
        for (var i = 0; i < 12; i++) { v += esub[e].out[i] * (1 << i); }
        eSevt[e] <== v;
        isDmg[e] * (1 - esub[e].out[13]) === 0;
        isTouch[e] * (1 - esub[e].out[14]) === 0;
        isDT[e] * (eSevt[e] - etick[e]) === 0;

        // x overlap at etick (obstacle hitbox depends on class).
        eodsm[e] = D100();
        eodsm[e].t <== eOsp[e] - isDT[e];
        // l = x + (1-cls)*102400 ; r = x + cls*1024000 + (1-cls)*(w*25600 - 102400)
        eRq[e] <== eOcls[e] * (1024000 - 25600 * eOf1[e] + 102400);
        eoxl[e] = SLtX();
        eoxl[e].a <== 25600000 + eodsm[e].out - dte[e].out + 102400 - 102400 * eOcls[e];
        eoxl[e].b <== 4300800;
        isDT[e] * (1 - eoxl[e].out) === 0;
        eoxr[e] = SLtX();
        eoxr[e].a <== 3532800;
        eoxr[e].b <== 25600000 + eodsm[e].out - dte[e].out + 25600 * eOf1[e] - 102400 + eRq[e];
        isDT[e] * (1 - eoxr[e].out) === 0;

        // y overlap: ground => pbot > top; bat => box vs triangle-wave y.
        ePb[e] <== eOcls[e] * (eOf1[e] + etick[e] - eOsp[e]);
        etri[e] = TriOff();
        etri[e].p <== ePb[e];
        etri[e].q <== etq[e];
        etri[e].r <== etr[e];
        eyg[e] = SLtY();
        eyg[e].a <== 119808 - 256 * eOf2[e];
        eyg[e].b <== epy[e].y - 1024;
        eyb1[e] = SLtY();
        eyb1[e].a <== 256 * eOf2[e] + etri[e].off;
        eyb1[e].b <== epy[e].y - 1024;
        eyb2[e] = SLtY();
        eyb2[e].a <== epy[e].y - 17408;
        eyb2[e].b <== 256 * eOf2[e] + etri[e].off + 8192;
        eOvB[e] <== eyb1[e].out * eyb2[e].out;
        eOvd[e] <== eOcls[e] * (eOvB[e] - eyg[e].out);
        isDT[e] * (1 - eyg[e].out - eOvd[e]) === 0;
    }

    // ================= section 5: kills =================
    component kfb[NK];
    component khb[NK];
    component kcool[NK];
    component klife1[NK];
    component klife2[NK];
    component khT[NK];
    component kspT[NK];
    component kfa[NK];
    component klj[NK];
    component kpy[NK];
    component kodot[NK];
    component ksdot[NK];
    component koub[NK];
    component ksub[NK];
    signal kOsp[NK];
    signal kOf1[NK];
    signal kOf2[NK];
    signal kOcls[NK];
    signal kSevt[NK];
    component kdsm[NK];
    component kdth[NK];
    signal kRq[NK];
    component kx1[NK];
    component kx2[NK];
    component kx3[NK];
    signal kPb[NK];
    component ktri[NK];
    signal kTopD[NK];
    signal kBotD[NK];
    component ky1[NK];
    component ky2[NK];

    for (var k = 0; k < NK; k++) {
        kact[k] * (kact[k] - 1) === 0;
        if (k > 0) {
            kact[k] * (1 - kact[k - 1]) === 0;
        }
        kfb[k] = Num2Bits(12);
        kfb[k].in <== kfire[k];
        khb[k] = Num2Bits(12);
        khb[k].in <== khit[k];
        (1 - kact[k]) * (kfire[k] - 4000) === 0;
        (1 - kact[k]) * (khit[k] - 4000) === 0;

        // fire cooldown: entries sorted by fire tick, >= 14 apart.
        if (k > 0) {
            kcool[k] = LessEqThan(13);
            kcool[k].in[0] <== kfire[k - 1] + 14;
            kcool[k].in[1] <== kfire[k];
            kact[k] * (1 - kcool[k].out) === 0;
        }

        // fireball lifetime: fire <= hit <= fire + 39 (padding satisfies).
        klife1[k] = LessEqThan(12);
        klife1[k].in[0] <== kfire[k];
        klife1[k].in[1] <== khit[k];
        klife1[k].out === 1;
        klife2[k] = LessEqThan(13);
        klife2[k].in[0] <== khit[k];
        klife2[k].in[1] <== kfire[k] + 39;
        klife2[k].out === 1;

        khT[k] = LessEqThan(12);
        khT[k].in[0] <== khit[k];
        khT[k].in[1] <== ticks;
        kact[k] * (1 - khT[k].out) === 0;

        // must be in Fire form at the fire tick.
        kfa[k] = FormAt(NE);
        kfa[k].t <== kfire[k];
        kfa[k].need <== kact[k];
        for (var i = 0; i <= NE; i++) {
            kfa[k].sel[i] <== kfsel[k][i];
            kfa[k].packV[i] <== epackV[i];
            kfa[k].tickNextV[i] <== tickNextV[i];
        }
        kact[k] * (kfa[k].form - 2) === 0;

        // fireball spawn y = player y at fire tick - 40px.
        klj[k] = LastJumpAt(NJ, 0);
        klj[k].t <== kfire[k];
        klj[k].need <== kact[k];
        for (var i = 0; i <= NJ; i++) {
            klj[k].sel[i] <== kjsel[k][i];
            klj[k].packV[i] <== jpackV[i];
            klj[k].nextV[i] <== jnextTickV[i];
        }
        kpy[k] = PlayerY1();
        kpy[k].t <== kfire[k];
        kpy[k].need <== kact[k];
        kpy[k].jt <== klj[k].jt;
        kpy[k].land <== klj[k].land;
        kpy[k].isSup <== klj[k].isSup;
        kpy[k].isCap <== klj[k].isCap;

        // target obstacle: must be marked killed at exactly this hit tick.
        var osv = 0;
        for (var i = 0; i < NO; i++) {
            kosel[k][i] * (kosel[k][i] - 1) === 0;
            osv += kosel[k][i];
        }
        kact[k] === osv;

        kodot[k] = EscalarProduct(NO);
        ksdot[k] = EscalarProduct(NO);
        for (var i = 0; i < NO; i++) {
            kodot[k].in1[i] <== kosel[k][i];
            kodot[k].in2[i] <== opackAll[i];
            ksdot[k].in1[i] <== kosel[k][i];
            ksdot[k].in2[i] <== statPackAll[i];
        }
        koub[k] = Num2Bits(30);
        koub[k].in <== kodot[k].out;
        var v = 0;
        for (var i = 0; i < 12; i++) { v += koub[k].out[i] * (1 << i); }
        kOsp[k] <== v;
        v = 0;
        for (var i = 12; i < 19; i++) { v += koub[k].out[i] * (1 << (i - 12)); }
        kOf1[k] <== v;
        v = 0;
        for (var i = 19; i < 28; i++) { v += koub[k].out[i] * (1 << (i - 19)); }
        kOf2[k] <== v;
        kOcls[k] <== koub[k].out[28];
        koub[k].out[29] === kact[k];

        ksub[k] = Num2Bits(15);
        ksub[k].in <== ksdot[k].out;
        v = 0;
        for (var i = 0; i < 12; i++) { v += ksub[k].out[i] * (1 << i); }
        kSevt[k] <== v;
        kact[k] * (1 - ksub[k].out[12]) === 0;
        kact[k] * (kSevt[k] - khit[k]) === 0;

        // target alive at hit: hit tick >= spawn.
        kspT[k] = LessEqThan(12);
        kspT[k].in[0] <== kOsp[k];
        kspT[k].in[1] <== khit[k];
        kact[k] * (1 - kspT[k].out) === 0;

        // fireball x at hit tick: FB_X0 + (t + 1 - fireTick) * 529000.
        kdsm[k] = D100();
        kdsm[k].t <== kOsp[k] - kact[k];
        kdth[k] = D100();
        kdth[k].t <== khit[k];
        kRq[k] <== kOcls[k] * (1024000 - 25600 * kOf1[k] + 102400);
        kx1[k] = SLtX();
        kx1[k].a <== 4505600 + 529000 * (khit[k] + 1 - kfire[k]);
        kx1[k].b <== 25600000 + kdsm[k].out - kdth[k].out + 25600 * kOf1[k] - 102400 + kRq[k];
        kact[k] * (1 - kx1[k].out) === 0;
        kx2[k] = SLtX();
        kx2[k].a <== 25600000 + kdsm[k].out - kdth[k].out + 102400 - 102400 * kOcls[k];
        kx2[k].b <== 4505600 + 529000 * (khit[k] + 1 - kfire[k]) + 460800;
        kact[k] * (1 - kx2[k].out) === 0;
        // fireball never travels past the obstacle spawn column.
        kx3[k] = LessEqThan(32);
        kx3[k].in[0] <== 4505600 + 529000 * (khit[k] + 1 - kfire[k]);
        kx3[k].in[1] <== 25600000;
        kact[k] * (1 - kx3[k].out) === 0;

        // fireball y overlap with target box.
        kPb[k] <== kOcls[k] * (kOf1[k] + khit[k] - kOsp[k]);
        ktri[k] = TriOff();
        ktri[k].p <== kPb[k];
        ktri[k].q <== ktq[k];
        ktri[k].r <== ktr[k];
        kTopD[k] <== kOcls[k] * (256 * kOf2[k] + ktri[k].off - 119808 + 256 * kOf2[k]);
        kBotD[k] <== kOcls[k] * (256 * kOf2[k] + ktri[k].off + 8192 - 118784);
        ky1[k] = SLtY();
        ky1[k].a <== kpy[k].y - 10240;
        ky1[k].b <== 118784 + kBotD[k];
        kact[k] * (1 - ky1[k].out) === 0;
        ky2[k] = SLtY();
        ky2[k].a <== 119808 - 256 * kOf2[k] + kTopD[k];
        ky2[k].b <== kpy[k].y - 10240 + 4608;
        kact[k] * (1 - ky2[k].out) === 0;
    }

    // ================= section 6: ground obstacles =================
    component gw1b[NG];
    component gw2b[NG];
    component gdsm[NG];
    component gltc[NG];
    signal gs4a[NG];
    component gdw1[NG];
    component gdw1m[NG];
    component gdw2[NG];
    component gdw2p[NG];
    component gcA[NG];
    component gcB[NG];
    component gcC[NG];
    component gcD[NG];
    component gcW[NG];
    signal gmT[NG];
    component gcE[NG];
    signal gm2[NG];
    signal gb1[NG];
    signal gb2[NG];
    signal gb3[NG];
    signal gB[NG];
    component gncr[NG];
    signal gnc[NG];
    component gjdot[NG];
    component gjub[NG];
    signal gCjt[NG];
    signal gCland[NG];
    component gle1[NG];
    component gle2[NG];
    signal gnA0[NG];
    component gzA[NG];
    signal gnAg[NG];
    component gnAb[NG];
    component gdispA[NG];
    signal gnB0[NG];
    component gzB[NG];
    signal gnBg[NG];
    component gnBb[NG];
    component gdispB[NG];
    component gclA[NG];
    component gclB[NG];

    for (var i = 0; i < NG; i++) {
        gw1b[i] = Num2Bits(12);
        gw1b[i].in <== gw1[i];
        gw2b[i] = Num2Bits(12);
        gw2b[i].in <== gw2[i];

        gdsm[i] = D100();
        gdsm[i].t <== gspawn[i] - gact[i];

        // status "unreached": left edge never crossed into the player zone.
        gltc[i] = SLtX();
        gltc[i].a <== 25600000 + gdsm[i].out - dT.out + 102400;
        gltc[i].b <== 4300800;
        gs4a[i] <== gs[i][4] * gact[i];
        gs4a[i] * gltc[i].out === 0;

        // overlap window [w1, w2] is exact: boundary checks on both sides.
        gdw1[i] = D100();
        gdw1[i].t <== gw1[i];
        gdw1m[i] = D100();
        gdw1m[i].t <== gw1[i] - 1 + gs[i][4];
        gdw2[i] = D100();
        gdw2[i].t <== gw2[i];
        gdw2p[i] = D100();
        gdw2p[i].t <== gw2[i] + 1 - gs[i][4];

        gcA[i] = SLtX();
        gcA[i].a <== 25600000 + gdsm[i].out - gdw1[i].out + 102400;
        gcA[i].b <== 4300800;
        (1 - gs[i][4]) * (1 - gcA[i].out) === 0;
        gcB[i] = SLtX();
        gcB[i].a <== 25600000 + gdsm[i].out - gdw1m[i].out + 102400;
        gcB[i].b <== 4300800;
        (1 - gs[i][4]) * gcB[i].out === 0;
        gcC[i] = SLtX();
        gcC[i].a <== 3532800;
        gcC[i].b <== 25600000 + gdsm[i].out - gdw2[i].out + 25600 * gw[i] - 102400;
        (1 - gs[i][4]) * (1 - gcC[i].out) === 0;
        gcD[i] = SLtX();
        gcD[i].a <== 3532800;
        gcD[i].b <== 25600000 + gdsm[i].out - gdw2p[i].out + 25600 * gw[i] - 102400;
        (1 - gs[i][4]) * gcD[i].out === 0;

        // clearance range end b by status:
        //   alive: min(w2, T); killed: min(w2, T, evt-1); damaged/touched: evt-1.
        gcW[i] = LessEqThan(12);
        gcW[i].in[0] <== gw2[i];
        gcW[i].in[1] <== ticks;
        gmT[i] <== gcW[i].out * (gw2[i] - ticks) + ticks;
        gcE[i] = LessEqThan(13);
        gcE[i].in[0] <== gevt[i];
        gcE[i].in[1] <== gmT[i];
        gm2[i] <== gcE[i].out * (gevt[i] - 1 - gmT[i]) + gmT[i];
        gb1[i] <== gs[i][0] * gmT[i];
        gb2[i] <== gs[i][1] * gm2[i];
        gb3[i] <== (gs[i][2] + gs[i][3]) * (gevt[i] - 1);
        gB[i] <== gb1[i] + gb2[i] + gb3[i];

        gncr[i] = LessThan(13);
        gncr[i].in[0] <== gw1[i];
        gncr[i].in[1] <== gB[i] + 1;
        gnc[i] <== gncr[i].out * (1 - gs[i][4]);

        // a single covering jump must clear the whole clearance range.
        var sv = 0;
        for (var k = 0; k < NJ; k++) {
            gcsel[i][k] * (gcsel[i][k] - 1) === 0;
            sv += gcsel[i][k];
        }
        gnc[i] === sv;
        gjdot[i] = EscalarProduct(NJ);
        for (var k = 0; k < NJ; k++) {
            gjdot[i].in1[k] <== gcsel[i][k];
            gjdot[i].in2[k] <== jpack[k];
        }
        gjub[i] = Num2Bits(21);
        gjub[i].in <== gjdot[i].out;
        var v = 0;
        for (var k = 0; k < 12; k++) { v += gjub[i].out[k] * (1 << k); }
        gCjt[i] <== v;
        v = 0;
        for (var k = 12; k < 18; k++) { v += gjub[i].out[k] * (1 << (k - 12)); }
        gCland[i] <== v;
        gjub[i].out[20] === gnc[i];

        gle1[i] = LessEqThan(12);
        gle1[i].in[0] <== gCjt[i];
        gle1[i].in[1] <== gw1[i];
        gnc[i] * (1 - gle1[i].out) === 0;
        gle2[i] = LessEqThan(13);
        gle2[i].in[0] <== gB[i] + 1;
        gle2[i].in[1] <== gCjt[i] + gCland[i];
        gnc[i] * (1 - gle2[i].out) === 0;

        // airborne displacement at both window endpoints must clear the top:
        // disp <= 2048 - 256*h  (parabola/glide convexity covers the interior).
        gnA0[i] <== gnc[i] * (gw1[i] - gCjt[i]);
        gzA[i] = IsZero();
        gzA[i].in <== gnA0[i];
        gnAg[i] <== gnA0[i] + gzA[i].out;
        gnAb[i] = Num2Bits(6);
        gnAb[i].in <== gnAg[i];
        gdispA[i] = JumpDisp();
        gdispA[i].n <== gnAg[i];
        gdispA[i].isSup <== gjub[i].out[18];
        gdispA[i].isCap <== gjub[i].out[19];
        gnB0[i] <== gnc[i] * (gB[i] - gCjt[i]);
        gzB[i] = IsZero();
        gzB[i].in <== gnB0[i];
        gnBg[i] <== gnB0[i] + gzB[i].out;
        gnBb[i] = Num2Bits(6);
        gnBb[i].in <== gnBg[i];
        gdispB[i] = JumpDisp();
        gdispB[i].n <== gnBg[i];
        gdispB[i].isSup <== gjub[i].out[18];
        gdispB[i].isCap <== gjub[i].out[19];

        gclA[i] = SLeqY();
        gclA[i].a <== gdispA[i].out;
        gclA[i].b <== 2048 - 256 * gh[i];
        gnc[i] * (1 - gclA[i].out) === 0;
        gclB[i] = SLeqY();
        gclB[i].a <== gdispB[i].out;
        gclB[i].b <== 2048 - 256 * gh[i];
        gnc[i] * (1 - gclB[i].out) === 0;
    }

    // ================= section 7: bats =================
    component bw1b[NB];
    component bw2b[NB];
    component bdsm[NB];
    component bltc[NB];
    signal bs4a[NB];
    component bdw1[NB];
    component bdw1m[NB];
    component bdw2[NB];
    component bdw2p[NB];
    component bcA[NB];
    component bcB[NB];
    component bcC[NB];
    component bcD[NB];
    component bcW[NB];
    signal bmT[NB];
    component bcE[NB];
    signal bm2[NB];
    signal bb1[NB];
    signal bb2[NB];
    signal bb3[NB];
    signal bB[NB];
    component bncr[NB];
    signal bnc[NB];
    component bwlen[NB];
    component blj[NB];
    component btau[NB][8];
    signal bg[NB][8];
    component bpy[NB][8];
    signal bPb[NB][8];
    component btri[NB][8];
    component bcl1[NB][8];
    component bcl2[NB][8];
    signal bclp[NB][8];

    for (var j = 0; j < NB; j++) {
        bw1b[j] = Num2Bits(12);
        bw1b[j].in <== bw1[j];
        bw2b[j] = Num2Bits(12);
        bw2b[j].in <== bw2[j];

        bdsm[j] = D100();
        bdsm[j].t <== bspawn[j] - bactv[j];

        bltc[j] = SLtX();
        bltc[j].a <== 25600000 + bdsm[j].out - dT.out;
        bltc[j].b <== 4300800;
        bs4a[j] <== bs[j][4] * bactv[j];
        bs4a[j] * bltc[j].out === 0;

        bdw1[j] = D100();
        bdw1[j].t <== bw1[j];
        bdw1m[j] = D100();
        bdw1m[j].t <== bw1[j] - 1 + bs[j][4];
        bdw2[j] = D100();
        bdw2[j].t <== bw2[j];
        bdw2p[j] = D100();
        bdw2p[j].t <== bw2[j] + 1 - bs[j][4];

        bcA[j] = SLtX();
        bcA[j].a <== 25600000 + bdsm[j].out - bdw1[j].out;
        bcA[j].b <== 4300800;
        (1 - bs[j][4]) * (1 - bcA[j].out) === 0;
        bcB[j] = SLtX();
        bcB[j].a <== 25600000 + bdsm[j].out - bdw1m[j].out;
        bcB[j].b <== 4300800;
        (1 - bs[j][4]) * bcB[j].out === 0;
        bcC[j] = SLtX();
        bcC[j].a <== 3532800;
        bcC[j].b <== 25600000 + bdsm[j].out - bdw2[j].out + 1024000;
        (1 - bs[j][4]) * (1 - bcC[j].out) === 0;
        bcD[j] = SLtX();
        bcD[j].a <== 3532800;
        bcD[j].b <== 25600000 + bdsm[j].out - bdw2p[j].out + 1024000;
        (1 - bs[j][4]) * bcD[j].out === 0;

        bcW[j] = LessEqThan(12);
        bcW[j].in[0] <== bw2[j];
        bcW[j].in[1] <== ticks;
        bmT[j] <== bcW[j].out * (bw2[j] - ticks) + ticks;
        bcE[j] = LessEqThan(13);
        bcE[j].in[0] <== bevt[j];
        bcE[j].in[1] <== bmT[j];
        bm2[j] <== bcE[j].out * (bevt[j] - 1 - bmT[j]) + bmT[j];
        bb1[j] <== bs[j][0] * bmT[j];
        bb2[j] <== bs[j][1] * bm2[j];
        bb3[j] <== (bs[j][2] + bs[j][3]) * (bevt[j] - 1);
        bB[j] <== bb1[j] + bb2[j] + bb3[j];

        bncr[j] = LessThan(13);
        bncr[j].in[0] <== bw1[j];
        bncr[j].in[1] <== bB[j] + 1;
        bnc[j] <== bncr[j].out * (1 - bs[j][4]);

        // overlap window is at most 8 ticks (locked by dash_zk tests).
        bwlen[j] = LessEqThan(13);
        bwlen[j].in[0] <== bw2[j];
        bwlen[j].in[1] <== bw1[j] + 7;
        bnc[j] * (1 - bwlen[j].out) === 0;

        // player context: last jump at w1 plus the one after it.
        blj[j] = LastJumpAt(NJ, 1);
        blj[j].t <== bw1[j];
        blj[j].need <== bnc[j];
        for (var k = 0; k <= NJ; k++) {
            blj[j].sel[k] <== bjsel[j][k];
            blj[j].packV[k] <== jpackV[k];
            blj[j].nextV[k] <== jnextPackV[k];
        }

        for (var d = 0; d < 8; d++) {
            btau[j][d] = LessEqThan(13);
            btau[j][d].in[0] <== bw1[j] + d;
            btau[j][d].in[1] <== bB[j];
            bg[j][d] <== bnc[j] * btau[j][d].out;

            bpy[j][d] = PlayerY2();
            bpy[j][d].t <== bw1[j] + d;
            bpy[j][d].need <== bg[j][d];
            bpy[j][d].jt1 <== blj[j].jt;
            bpy[j][d].land1 <== blj[j].land;
            bpy[j][d].isSup1 <== blj[j].isSup;
            bpy[j][d].isCap1 <== blj[j].isCap;
            bpy[j][d].jt2 <== blj[j].jt2;
            bpy[j][d].land2 <== blj[j].land2;
            bpy[j][d].isSup2 <== blj[j].isSup2;
            bpy[j][d].isCap2 <== blj[j].isCap2;

            bPb[j][d] <== bg[j][d] * (bphase[j] + bw1[j] + d - bspawn[j]);
            btri[j][d] = TriOff();
            btri[j][d].p <== bPb[j][d];
            btri[j][d].q <== btq[j][d];
            btri[j][d].r <== btr[j][d];

            // clear: pbot <= batTop OR ptop >= batBot.
            bcl1[j][d] = SLeqY();
            bcl1[j][d].a <== bpy[j][d].y - 1024;
            bcl1[j][d].b <== 256 * bbase[j] + btri[j][d].off;
            bcl2[j][d] = SLeqY();
            bcl2[j][d].a <== 256 * bbase[j] + btri[j][d].off + 8192;
            bcl2[j][d].b <== bpy[j][d].y - 17408;
            bclp[j][d] <== bcl1[j][d].out * bcl2[j][d].out;
            bg[j][d] * (1 - bcl1[j][d].out - bcl2[j][d].out + bclp[j][d]) === 0;
        }
    }

    // ================= section 8: counts & score =================
    var sumS1 = 0;
    var sumS2 = 0;
    var sumS3 = 0;
    for (var i = 0; i < NG; i++) {
        sumS1 += gs[i][1];
        sumS2 += gs[i][2];
        sumS3 += gs[i][3];
    }
    for (var j = 0; j < NB; j++) {
        sumS1 += bs[j][1];
        sumS2 += bs[j][2];
        sumS3 += bs[j][3];
    }
    var sumK = 0;
    for (var k = 0; k < NK; k++) { sumK += kact[k]; }
    var sumDmg = 0;
    var sumTch = 0;
    var sumPick = 0;
    for (var e = 0; e < NE; e++) {
        sumDmg += isDmg[e];
        sumTch += isTouch[e];
        sumPick += isPick[e];
    }
    signal sk <== sumK;
    sk === sumS1;
    signal sd <== sumDmg;
    sd === sumS2;
    signal st <== sumTch;
    st === sumS3;
    signal sp <== sumPick;

    // score = min(1500, floor(d100(T) / 1280000) + 50*pickups + 25*kills).
    component qb = Num2Bits(12);
    qb.in <== scoreQ;
    component rb = Num2Bits(21);
    rb.in <== scoreR;
    component rlt = LessThan(21);
    rlt.in[0] <== scoreR;
    rlt.in[1] <== 1280000;
    rlt.out === 1;
    dT.out === 1280000 * scoreQ + scoreR;
    signal rawScore <== scoreQ + 50 * sp + 25 * sk;
    component rawScoreBits = Num2Bits(13);
    rawScoreBits.in <== rawScore;
    component rawScoreLtCap = LessThan(13);
    rawScoreLtCap.in[0] <== rawScore;
    rawScoreLtCap.in[1] <== 1500;
    score === rawScoreLtCap.out * (rawScore - 1500) + 1500;

    component scoreCapped = IsEqual();
    scoreCapped.in[0] <== score;
    scoreCapped.in[1] <== 1500;

    // run end: survived => ticks = 3600; died => ticks = fatal-entry tick;
    // score-capped non-death runs may end early.
    signal naturalFinish <== (1 - died) * (1 - scoreCapped.out);
    naturalFinish * (ticks - 3600) === 0;
    died * (tfAcc[NE] - ticks) === 0;
}

component main {public [score, ticks, groundCount, batCount, itemCount, gspawn, gw, gh, bspawn, bbase, bphase, ispawn, ikind, iy, acct]} = DashZK(128, 24, 56, 160, 64, 32);

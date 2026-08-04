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

// Signed a <= b for x-domain values.
template SLeqX() {
    signal input a;
    signal input b;
    signal output out;
    component lt = SLtX();
    lt.a <== b;
    lt.b <== a;
    out <== 1 - lt.out;
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

// Prove first-collision semantics for one obstacle class.  Obstacles are
// spawn-ordered, so a fireball can only encounter a contiguous suffix window.
// Six slots cover at most five candidates plus the first out-of-window
// entry; the circuit proves both boundaries rather than trusting the bound.
template FirstCollisionClass(M, NS, CLS) {
    signal input fire;
    signal input hit;
    signal input need;
    signal input fireY;
    signal input targetClass;
    signal input targetIdx;
    signal input opack[M + 1];
    signal input statpack[M + 1];
    signal input sel[NS][M + 1];
    signal input first[NS];
    signal input tq[2 * NS];
    signal input tr[2 * NS];

    signal selSum[NS];
    signal idx[NS];
    component odot[NS];
    component sdot[NS];
    component oub[NS];
    component sub[NS];
    signal spawn[NS];
    signal f1[NS];
    signal f2[NS];
    signal cact[NS];
    signal evt[NS];
    signal terminal[NS];
    signal aliveOrUnreached[NS];
    component withinEnd[NS];
    signal cand[NS];

    for (var s = 0; s < NS; s++) {
        var ss = 0;
        var si = 0;
        for (var i = 0; i <= M; i++) {
            sel[s][i] * (sel[s][i] - 1) === 0;
            ss += sel[s][i];
            si += i * sel[s][i];
        }
        selSum[s] <== ss;
        idx[s] <== si;
        if (s == 0) {
            selSum[s] === need;
        } else {
            selSum[s] === cand[s - 1];
            cand[s - 1] * (idx[s] - idx[s - 1] - 1) === 0;
        }

        odot[s] = EscalarProduct(M + 1);
        sdot[s] = EscalarProduct(M + 1);
        for (var i = 0; i <= M; i++) {
            odot[s].in1[i] <== sel[s][i];
            odot[s].in2[i] <== opack[i];
            sdot[s].in1[i] <== sel[s][i];
            sdot[s].in2[i] <== statpack[i];
        }
        oub[s] = Num2Bits(30);
        oub[s].in <== odot[s].out;
        var v = 0;
        for (var b = 0; b < 12; b++) { v += oub[s].out[b] * (1 << b); }
        spawn[s] <== v;
        v = 0;
        for (var b = 12; b < 19; b++) { v += oub[s].out[b] * (1 << (b - 12)); }
        f1[s] <== v;
        v = 0;
        for (var b = 19; b < 28; b++) { v += oub[s].out[b] * (1 << (b - 19)); }
        f2[s] <== v;
        cact[s] <== oub[s].out[29];
        cact[s] * (oub[s].out[28] - CLS) === 0;

        sub[s] = Num2Bits(15);
        sub[s].in <== sdot[s].out;
        v = 0;
        for (var b = 0; b < 12; b++) { v += sub[s].out[b] * (1 << b); }
        evt[s] <== v;
        terminal[s] <== sub[s].out[12] + sub[s].out[13] + sub[s].out[14];
        aliveOrUnreached[s] <== cact[s] - terminal[s];

        withinEnd[s] = LessEqThan(13);
        withinEnd[s].in[0] <== spawn[s];
        withinEnd[s].in[1] <== fire + 38;
        cand[s] <== cact[s] * withinEnd[s].out;
    }
    // The last selected slot is the boundary immediately after all possible
    // candidates.  If this fails, NS must be increased rather than truncating.
    cand[NS - 1] === 0;

    // Slot zero is the first active obstacle that has not already passed the
    // fireball at the relevant start boundary.
    component startSpawnCap = LessEqThan(13);
    startSpawnCap.in[0] <== spawn[0];
    startSpawnCap.in[1] <== fire + 38;
    signal startCappedSpawn <== startSpawnCap.out * (spawn[0] - fire - 38) + fire + 38;
    component startSpawnLeFire = LessEqThan(13);
    startSpawnLeFire.in[0] <== startCappedSpawn;
    startSpawnLeFire.in[1] <== fire;
    signal startAnchor <== startSpawnLeFire.out * (fire - startCappedSpawn) + startCappedSpawn;
    component startD = D100();
    startD.t <== startAnchor;
    component startDs = D100();
    startDs.t <== spawn[0] - cact[0];
    component startA = SLtX();
    startA.a <== 4505600 + 529000 * (startAnchor + 1 - fire);
    if (CLS == 0) {
        startA.b <== 25600000 + startDs.out - startD.out + 25600 * f1[0] - 102400;
    } else {
        startA.b <== 25600000 + startDs.out - startD.out + 1024000;
    }
    cact[0] * (1 - startA.out) === 0;

    component prevDot = EscalarProduct(M + 1);
    for (var i = 0; i <= M; i++) {
        prevDot.in1[i] <== sel[0][i];
        if (i == 0) {
            prevDot.in2[i] <== 0;
        } else {
            prevDot.in2[i] <== opack[i - 1];
        }
    }
    component prevUb = Num2Bits(30);
    prevUb.in <== prevDot.out;
    var pv = 0;
    for (var b = 0; b < 12; b++) { pv += prevUb.out[b] * (1 << b); }
    signal prevSpawn <== pv;
    pv = 0;
    for (var b = 12; b < 19; b++) { pv += prevUb.out[b] * (1 << (b - 12)); }
    signal prevF1 <== pv;
    signal prevAct <== prevUb.out[29];
    component startIdxZero = IsZero();
    startIdxZero.in <== idx[0];
    signal startInactive <== selSum[0] - cact[0];
    signal sentinelValid <== startIdxZero.out + prevAct;
    startInactive * (1 - sentinelValid) === 0;
    component prevSpawnCap = LessEqThan(13);
    prevSpawnCap.in[0] <== prevSpawn;
    prevSpawnCap.in[1] <== fire + 38;
    signal prevCappedSpawn <== prevSpawnCap.out * (prevSpawn - fire - 38) + fire + 38;
    component prevSpawnLeFire = LessEqThan(13);
    prevSpawnLeFire.in[0] <== prevCappedSpawn;
    prevSpawnLeFire.in[1] <== fire;
    signal prevAnchor <== prevSpawnLeFire.out * (fire - prevCappedSpawn) + prevCappedSpawn;
    component prevD = D100();
    prevD.t <== prevAnchor;
    component prevDs = D100();
    prevDs.t <== prevSpawn - prevAct;
    component prevA = SLtX();
    prevA.a <== 4505600 + 529000 * (prevAnchor + 1 - fire);
    if (CLS == 0) {
        prevA.b <== 25600000 + prevDs.out - prevD.out + 25600 * prevF1 - 102400;
    } else {
        prevA.b <== 25600000 + prevDs.out - prevD.out + 1024000;
    }
    prevAct * prevA.out === 0;

    component fb[NS];
    component baseOrder[NS];
    signal base[NS];
    component baseLeFirst[NS];
    component firstLeEnd[NS];
    component dspawn[NS];
    component dfirst[NS];
    component isBase[NS];
    component dprev[NS];
    component enterFirst[NS];
    component enterPrev[NS];
    signal prevGate[NS];
    component aFirst[NS];
    signal pair[NS];
    component dt[NS][3];
    component exitWindow[NS];
    component ax[NS][2];
    component bx[NS][2];
    signal h1[NS][2];
    signal horiz[NS][2];
    signal phase[NS][2];
    component tri[NS][2];
    signal top[NS][2];
    signal bot[NS][2];
    component y1[NS][2];
    component y2[NS][2];
    signal yy[NS][2];
    signal yOverlap[NS][2];
    component before[NS][2];
    component same[NS][2];
    component localEarlier[NS][2];
    signal earlier[NS][2];
    signal sameEarlier[NS][2];
    signal relevant[NS][2];
    component evtGe[NS][2];
    signal liveTerm[NS][2];
    signal live[NS][2];
    signal no1[NS][2];
    signal no2[NS][2];

    for (var s = 0; s < NS; s++) {
        fb[s] = Num2Bits(12);
        fb[s].in <== first[s];
        (1 - cand[s]) * (first[s] - 4000) === 0;
        baseOrder[s] = LessEqThan(12);
        baseOrder[s].in[0] <== spawn[s];
        baseOrder[s].in[1] <== fire;
        base[s] <== baseOrder[s].out * (fire - spawn[s]) + spawn[s];
        baseLeFirst[s] = LessEqThan(13);
        baseLeFirst[s].in[0] <== base[s];
        baseLeFirst[s].in[1] <== first[s];
        cand[s] * (1 - baseLeFirst[s].out) === 0;
        firstLeEnd[s] = LessEqThan(13);
        firstLeEnd[s].in[0] <== first[s];
        firstLeEnd[s].in[1] <== fire + 38;
        cand[s] * (1 - firstLeEnd[s].out) === 0;

        dspawn[s] = D100();
        dspawn[s].t <== spawn[s] - cact[s];
        dfirst[s] = D100();
        dfirst[s].t <== first[s];
        isBase[s] = IsEqual();
        isBase[s].in[0] <== first[s];
        isBase[s].in[1] <== base[s];
        dprev[s] = D100();
        dprev[s].t <== first[s] - 1 + isBase[s].out;
        enterFirst[s] = SLtX();
        if (CLS == 0) {
            enterFirst[s].a <== 25600000 + dspawn[s].out - dfirst[s].out + 102400;
        } else {
            enterFirst[s].a <== 25600000 + dspawn[s].out - dfirst[s].out;
        }
        enterFirst[s].b <== 4505600 + 529000 * (first[s] + 1 - fire) + 460800;
        cand[s] * (1 - enterFirst[s].out) === 0;

        enterPrev[s] = SLtX();
        if (CLS == 0) {
            enterPrev[s].a <== 25600000 + dspawn[s].out - dprev[s].out + 102400;
        } else {
            enterPrev[s].a <== 25600000 + dspawn[s].out - dprev[s].out;
        }
        enterPrev[s].b <== 4505600 + 529000 * (first[s] - fire + isBase[s].out) + 460800;
        prevGate[s] <== cand[s] * (1 - isBase[s].out);
        prevGate[s] * enterPrev[s].out === 0;

        aFirst[s] = SLtX();
        aFirst[s].a <== 4505600 + 529000 * (first[s] + 1 - fire);
        if (CLS == 0) {
            aFirst[s].b <== 25600000 + dspawn[s].out - dfirst[s].out + 25600 * f1[s] - 102400;
        } else {
            aFirst[s].b <== 25600000 + dspawn[s].out - dfirst[s].out + 1024000;
        }
        pair[s] <== cand[s] * aFirst[s].out;

        for (var d = 1; d < 3; d++) {
            dt[s][d] = D100();
            dt[s][d].t <== first[s] + d;
        }
        exitWindow[s] = SLeqX();
        if (CLS == 0) {
            exitWindow[s].a <== 25600000 + dspawn[s].out - dt[s][2].out + 25600 * f1[s] - 102400;
        } else {
            exitWindow[s].a <== 25600000 + dspawn[s].out - dt[s][2].out + 1024000;
        }
        exitWindow[s].b <== 4505600 + 529000 * (first[s] + 3 - fire);
        pair[s] * (1 - exitWindow[s].out) === 0;

        for (var d = 0; d < 2; d++) {
            if (d == 0) {
                h1[s][d] <== cand[s] * aFirst[s].out;
                horiz[s][d] <== h1[s][d] * enterFirst[s].out;
            } else {
                ax[s][d] = SLtX();
                ax[s][d].a <== 4505600 + 529000 * (first[s] + d + 1 - fire);
                if (CLS == 0) {
                    ax[s][d].b <== 25600000 + dspawn[s].out - dt[s][d].out + 25600 * f1[s] - 102400;
                } else {
                    ax[s][d].b <== 25600000 + dspawn[s].out - dt[s][d].out + 1024000;
                }
                bx[s][d] = SLtX();
                if (CLS == 0) {
                    bx[s][d].a <== 25600000 + dspawn[s].out - dt[s][d].out + 102400;
                } else {
                    bx[s][d].a <== 25600000 + dspawn[s].out - dt[s][d].out;
                }
                bx[s][d].b <== 4505600 + 529000 * (first[s] + d + 1 - fire) + 460800;
                h1[s][d] <== cand[s] * ax[s][d].out;
                horiz[s][d] <== h1[s][d] * bx[s][d].out;
            }

            if (CLS == 0) {
                phase[s][d] <== 0;
                tq[2 * s + d] === 0;
                tr[2 * s + d] === 0;
            } else {
                phase[s][d] <== horiz[s][d] * (f1[s] + first[s] + d - spawn[s]);
                tri[s][d] = TriOff();
                tri[s][d].p <== phase[s][d];
                tri[s][d].q <== tq[2 * s + d];
                tri[s][d].r <== tr[2 * s + d];
            }
            if (CLS == 0) {
                top[s][d] <== 119808 - 256 * f2[s];
                bot[s][d] <== 118784;
            } else {
                top[s][d] <== 256 * f2[s] + tri[s][d].off;
                bot[s][d] <== 256 * f2[s] + tri[s][d].off + 8192;
            }
            y1[s][d] = SLtY();
            y1[s][d].a <== fireY;
            y1[s][d].b <== bot[s][d];
            y2[s][d] = SLtY();
            y2[s][d].a <== top[s][d];
            y2[s][d].b <== fireY + 4608;
            yy[s][d] <== y1[s][d].out * y2[s][d].out;
            yOverlap[s][d] <== horiz[s][d] * yy[s][d];

            before[s][d] = LessThan(12);
            before[s][d].in[0] <== first[s] + d;
            before[s][d].in[1] <== hit;
            same[s][d] = IsEqual();
            same[s][d].in[0] <== first[s] + d;
            same[s][d].in[1] <== hit;
            localEarlier[s][d] = LessThan(8);
            localEarlier[s][d].in[0] <== idx[s];
            localEarlier[s][d].in[1] <== targetIdx;
            if (CLS == 0) {
                earlier[s][d] <== targetClass + (1 - targetClass) * localEarlier[s][d].out;
            } else {
                earlier[s][d] <== targetClass * localEarlier[s][d].out;
            }
            sameEarlier[s][d] <== same[s][d].out * earlier[s][d];
            relevant[s][d] <== before[s][d].out + sameEarlier[s][d];

            evtGe[s][d] = LessEqThan(13);
            evtGe[s][d].in[0] <== first[s] + d;
            evtGe[s][d].in[1] <== evt[s];
            liveTerm[s][d] <== terminal[s] * evtGe[s][d].out;
            live[s][d] <== cand[s] * (aliveOrUnreached[s] + liveTerm[s][d]);
            no1[s][d] <== yOverlap[s][d] * live[s][d];
            no2[s][d] <== no1[s][d] * relevant[s][d];
            no2[s][d] === 0;
        }
    }
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
    signal input kgsel[NK][6 * (NG + 1)];
    signal input kbsel[NK][6 * (NB + 1)];
    signal input kgfirst[NK][6];
    signal input kbfirst[NK][6];
    signal input kgq[NK][12];
    signal input kgr[NK][12];
    signal input kbq[NK][12];
    signal input kbr[NK][12];
    signal input iw1[NI];
    signal input iw2[NI];
    signal input ijsel[NI][NJ + 1];
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
    signal input preScoreQ;
    signal input preScoreR;

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

    // ================= section 4: mandatory item pickups =================
    //
    // The Rust simulator auto-collects an item on the first tick where both
    // hitboxes overlap. Bind each item to its exact horizontal window, scan
    // that (at most eight-tick) window, and require one pickup event exactly
    // at the first vertical overlap.
    component iw1b[NI];
    component iw2b[NI];
    component idsm[NI];
    component idw1[NI];
    component idw1m[NI];
    component idw2[NI];
    component idw2p[NI];
    component icA[NI];
    component icB[NI];
    component icC[NI];
    component icD[NI];
    component iwlen[NI];
    component iwReach[NI];
    signal ireach[NI];
    component ilj[NI];
    component itauEnd[NI][8];
    component itauTicks[NI][8];
    component itauCapacity[NI][8];
    signal ig1[NI][8];
    signal ig2[NI][8];
    signal ig[NI][8];
    component ipy[NI][8];
    component iyt[NI][8];
    component iyb[NI][8];
    signal iov1[NI][8];
    signal iov[NI][8];
    signal iseen[NI][9];
    signal ifirst[NI][8];
    signal iPickCount[NI];
    signal iPickTick[NI];
    component ipickdot[NI];
    component ifdot[NI];

    for (var i = 0; i < NI; i++) {
        iw1b[i] = Num2Bits(12);
        iw1b[i].in <== iw1[i];
        iw2b[i] = Num2Bits(12);
        iw2b[i].in <== iw2[i];
        (1 - iactv[i]) * (iw1[i] - 4000) === 0;
        (1 - iactv[i]) * (iw2[i] - 4000) === 0;

        idsm[i] = D100();
        idsm[i].t <== ispawn[i] - iactv[i];
        idw1[i] = D100();
        idw1[i].t <== iw1[i];
        idw1m[i] = D100();
        idw1m[i].t <== iw1[i] - 1 + (1 - iactv[i]);
        idw2[i] = D100();
        idw2[i].t <== iw2[i];
        idw2p[i] = D100();
        idw2p[i].t <== iw2[i] + iactv[i];

        // Exact horizontal overlap window.
        icA[i] = SLtX();
        icA[i].a <== 25344000 + idsm[i].out - idw1[i].out;
        icA[i].b <== 4300800;
        iactv[i] * (1 - icA[i].out) === 0;
        icB[i] = SLtX();
        icB[i].a <== 25344000 + idsm[i].out - idw1m[i].out;
        icB[i].b <== 4300800;
        iactv[i] * icB[i].out === 0;
        icC[i] = SLtX();
        icC[i].a <== 3532800;
        icC[i].b <== 25344000 + idsm[i].out - idw2[i].out + 870400;
        iactv[i] * (1 - icC[i].out) === 0;
        icD[i] = SLtX();
        icD[i].a <== 3532800;
        icD[i].b <== 25344000 + idsm[i].out - idw2p[i].out + 870400;
        iactv[i] * icD[i].out === 0;

        iwlen[i] = LessEqThan(13);
        iwlen[i].in[0] <== iw2[i];
        iwlen[i].in[1] <== iw1[i] + 7;
        iactv[i] * (1 - iwlen[i].out) === 0;

        iwReach[i] = LessEqThan(12);
        iwReach[i].in[0] <== iw1[i];
        iwReach[i].in[1] <== ticks;
        ireach[i] <== iactv[i] * iwReach[i].out;

        ilj[i] = LastJumpAt(NJ, 1);
        ilj[i].t <== iw1[i];
        ilj[i].need <== ireach[i];
        for (var k = 0; k <= NJ; k++) {
            ilj[i].sel[k] <== ijsel[i][k];
            ilj[i].packV[k] <== jpackV[k];
            ilj[i].nextV[k] <== jnextPackV[k];
        }

        iseen[i][0] <== 0;
        var pc = 0;
        for (var e = 0; e < NE; e++) {
            pc += eisel[e][i];
        }
        iPickCount[i] <== pc;
        ipickdot[i] = EscalarProduct(NE);
        for (var e = 0; e < NE; e++) {
            ipickdot[i].in1[e] <== eisel[e][i];
            ipickdot[i].in2[e] <== etick[e];
        }
        iPickTick[i] <== ipickdot[i].out;
        ifdot[i] = EscalarProduct(8);

        for (var d = 0; d < 8; d++) {
            itauEnd[i][d] = LessEqThan(13);
            itauEnd[i][d].in[0] <== iw1[i] + d;
            itauEnd[i][d].in[1] <== iw2[i];
            itauTicks[i][d] = LessEqThan(13);
            itauTicks[i][d].in[0] <== iw1[i] + d;
            itauTicks[i][d].in[1] <== ticks;
            // An inactive last timeline slot is padded at tick 4000. If the
            // slot is active, its tick is when the shared 64-entry form log
            // becomes full. Pickups run before player collisions, so a pickup
            // remains recordable through that tick.
            itauCapacity[i][d] = LessEqThan(13);
            itauCapacity[i][d].in[0] <== iw1[i] + d;
            itauCapacity[i][d].in[1] <== etick[NE - 1];
            ig1[i][d] <== iactv[i] * itauEnd[i][d].out;
            ig2[i][d] <== ig1[i][d] * itauTicks[i][d].out;
            ig[i][d] <== ig2[i][d] * itauCapacity[i][d].out;

            ipy[i][d] = PlayerY2();
            ipy[i][d].t <== iw1[i] + d;
            ipy[i][d].need <== ig[i][d];
            ipy[i][d].jt1 <== ilj[i].jt;
            ipy[i][d].land1 <== ilj[i].land;
            ipy[i][d].isSup1 <== ilj[i].isSup;
            ipy[i][d].isCap1 <== ilj[i].isCap;
            ipy[i][d].jt2 <== ilj[i].jt2;
            ipy[i][d].land2 <== ilj[i].land2;
            ipy[i][d].isSup2 <== ilj[i].isSup2;
            ipy[i][d].isCap2 <== ilj[i].isCap2;

            iyt[i][d] = SLtY();
            iyt[i][d].a <== ipy[i][d].y - 17408;
            iyt[i][d].b <== 256 * iy[i] + 8704;
            iyb[i][d] = SLtY();
            iyb[i][d].a <== 256 * iy[i];
            iyb[i][d].b <== ipy[i][d].y - 1024;
            iov1[i][d] <== ig[i][d] * iyt[i][d].out;
            iov[i][d] <== iov1[i][d] * iyb[i][d].out;
            ifirst[i][d] <== iov[i][d] * (1 - iseen[i][d]);
            iseen[i][d + 1] <== iseen[i][d] + ifirst[i][d];
            ifdot[i].in1[d] <== ifirst[i][d];
            ifdot[i].in2[d] <== iw1[i] + d;
        }

        iPickCount[i] === iseen[i][8];
        iPickTick[i] === ifdot[i].out;
    }

    // ================= section 5: timeline entry checks =================
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

    // ================= section 6: kills =================
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
    signal kTargetGroundIdx[NK];
    signal kTargetBatIdx[NK];

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
        var targetGroundIdx = 0;
        var targetBatIdx = 0;
        for (var i = 0; i < NO; i++) {
            kosel[k][i] * (kosel[k][i] - 1) === 0;
            osv += kosel[k][i];
            if (i < NG) {
                targetGroundIdx += i * kosel[k][i];
            } else {
                targetBatIdx += (i - NG) * kosel[k][i];
            }
        }
        kact[k] === osv;
        kTargetGroundIdx[k] <== targetGroundIdx;
        kTargetBatIdx[k] <== targetBatIdx;

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

    // ================= section 7: fireball first collision =================
    component kGroundFirst[NK];
    component kBatFirst[NK];
    for (var k = 0; k < NK; k++) {
        kGroundFirst[k] = FirstCollisionClass(NG, 6, 0);
        kGroundFirst[k].fire <== kfire[k];
        kGroundFirst[k].hit <== khit[k];
        kGroundFirst[k].need <== kact[k];
        kGroundFirst[k].fireY <== kpy[k].y - 10240;
        kGroundFirst[k].targetClass <== kOcls[k];
        kGroundFirst[k].targetIdx <== kTargetGroundIdx[k];
        for (var i = 0; i <= NG; i++) {
            if (i < NG) {
                kGroundFirst[k].opack[i] <== opackAll[i];
                kGroundFirst[k].statpack[i] <== statPackAll[i];
            } else {
                kGroundFirst[k].opack[i] <== 0;
                kGroundFirst[k].statpack[i] <== 0;
            }
        }
        for (var s = 0; s < 6; s++) {
            for (var i = 0; i <= NG; i++) {
                kGroundFirst[k].sel[s][i] <== kgsel[k][s * (NG + 1) + i];
            }
            kGroundFirst[k].first[s] <== kgfirst[k][s];
            kGroundFirst[k].tq[2 * s] <== kgq[k][2 * s];
            kGroundFirst[k].tq[2 * s + 1] <== kgq[k][2 * s + 1];
            kGroundFirst[k].tr[2 * s] <== kgr[k][2 * s];
            kGroundFirst[k].tr[2 * s + 1] <== kgr[k][2 * s + 1];
        }

        kBatFirst[k] = FirstCollisionClass(NB, 6, 1);
        kBatFirst[k].fire <== kfire[k];
        kBatFirst[k].hit <== khit[k];
        kBatFirst[k].need <== kact[k];
        kBatFirst[k].fireY <== kpy[k].y - 10240;
        kBatFirst[k].targetClass <== kOcls[k];
        kBatFirst[k].targetIdx <== kTargetBatIdx[k];
        for (var j = 0; j <= NB; j++) {
            if (j < NB) {
                kBatFirst[k].opack[j] <== opackAll[NG + j];
                kBatFirst[k].statpack[j] <== statPackAll[NG + j];
            } else {
                kBatFirst[k].opack[j] <== 0;
                kBatFirst[k].statpack[j] <== 0;
            }
        }
        for (var s = 0; s < 6; s++) {
            for (var j = 0; j <= NB; j++) {
                kBatFirst[k].sel[s][j] <== kbsel[k][s * (NB + 1) + j];
            }
            kBatFirst[k].first[s] <== kbfirst[k][s];
            kBatFirst[k].tq[2 * s] <== kbq[k][2 * s];
            kBatFirst[k].tq[2 * s + 1] <== kbq[k][2 * s + 1];
            kBatFirst[k].tr[2 * s] <== kbr[k][2 * s];
            kBatFirst[k].tr[2 * s + 1] <== kbr[k][2 * s + 1];
        }
    }

    // ================= section 8: ground obstacles =================
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
    component gcCap[NG];
    signal gm2[NG];
    signal gb1[NG];
    signal gb2[NG];
    signal gb3[NG];
    signal gB[NG];
    signal gBCap[NG];
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
        // Once the shared form-event log fills, Rust ignores later player
        // collisions. The last active event is at etick[NE-1]; if that event
        // is a pickup the same-tick collision is already suppressed, while a
        // damage/touch event's own obstacle already ends at evt-1. Truncating
        // every clearance range at the preceding tick covers both cases and
        // Rust's first-collision scan order for other same-tick obstacles.
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
        gcCap[i] = LessEqThan(13);
        gcCap[i].in[0] <== etick[NE - 1] - eact[NE - 1];
        gcCap[i].in[1] <== gB[i];
        gBCap[i] <== gcCap[i].out * (etick[NE - 1] - eact[NE - 1] - gB[i]) + gB[i];

        gncr[i] = LessThan(13);
        gncr[i].in[0] <== gw1[i];
        gncr[i].in[1] <== gBCap[i] + 1;
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
        gle2[i].in[0] <== gBCap[i] + 1;
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
        gnB0[i] <== gnc[i] * (gBCap[i] - gCjt[i]);
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

    // ================= section 9: bats =================
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
    component bcCap[NB];
    signal bm2[NB];
    signal bb1[NB];
    signal bb2[NB];
    signal bb3[NB];
    signal bB[NB];
    signal bBCap[NB];
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
        bcCap[j] = LessEqThan(13);
        bcCap[j].in[0] <== etick[NE - 1] - eact[NE - 1];
        bcCap[j].in[1] <== bB[j];
        bBCap[j] <== bcCap[j].out * (etick[NE - 1] - eact[NE - 1] - bB[j]) + bB[j];

        bncr[j] = LessThan(13);
        bncr[j].in[0] <== bw1[j];
        bncr[j].in[1] <== bBCap[j] + 1;
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
            btau[j][d].in[1] <== bBCap[j];
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

    // ================= section 10: counts & score =================
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

    // A score-capped run ends on the first tick that reaches the cap.  The
    // previous tick's raw score must therefore still be below 1500.
    component ticksZero = IsZero();
    ticksZero.in <== ticks;
    signal preTick <== ticks - 1 + ticksZero.out;
    component dPre = D100();
    dPre.t <== preTick;
    component preQb = Num2Bits(12);
    preQb.in <== preScoreQ;
    component preRb = Num2Bits(21);
    preRb.in <== preScoreR;
    component preRlt = LessThan(21);
    preRlt.in[0] <== preScoreR;
    preRlt.in[1] <== 1280000;
    preRlt.out === 1;
    dPre.out === 1280000 * preScoreQ + preScoreR;

    component pickBefore[NE];
    signal prePickFlag[NE];
    var prePick = 0;
    for (var e = 0; e < NE; e++) {
        pickBefore[e] = LessThan(12);
        pickBefore[e].in[0] <== etick[e];
        pickBefore[e].in[1] <== ticks;
        prePickFlag[e] <== isPick[e] * pickBefore[e].out;
        prePick += prePickFlag[e];
    }
    component killBefore[NK];
    signal preKillFlag[NK];
    var preKill = 0;
    for (var k = 0; k < NK; k++) {
        killBefore[k] = LessThan(12);
        killBefore[k].in[0] <== khit[k];
        killBefore[k].in[1] <== ticks;
        preKillFlag[k] <== kact[k] * killBefore[k].out;
        preKill += preKillFlag[k];
    }
    signal preRawScore <== preScoreQ + 50 * prePick + 25 * preKill;
    component preRawBits = Num2Bits(13);
    preRawBits.in <== preRawScore;
    component preRawLtCap = LessThan(13);
    preRawLtCap.in[0] <== preRawScore;
    preRawLtCap.in[1] <== 1500;
    scoreCapped.out * (1 - preRawLtCap.out) === 0;

    // run end: survived => ticks = 3600; died => ticks = fatal-entry tick;
    // score-capped non-death runs may end early.
    signal naturalFinish <== (1 - died) * (1 - scoreCapped.out);
    naturalFinish * (ticks - 3600) === 0;
    died * (tfAcc[NE] - ticks) === 0;
}

component main {public [score, ticks, groundCount, batCount, itemCount, gspawn, gw, gh, bspawn, bbase, bphase, ispawn, ikind, iy, acct]} = DashZK(128, 24, 56, 160, 64, 32);

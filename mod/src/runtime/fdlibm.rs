/*
 * ====================================================
 * Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 *
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 *
 * This Rust module is a clean-room port of the original netlib/fdlibm
 * sources (e_pow.c, e_exp.c, e_log.c, e_log10.c, s_cbrt.c, s_tanh.c,
 * s_expm1.c, s_tan.c and their fdlibm argument-reduction helpers).
 */
#![allow(
    clippy::excessive_precision,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::eq_op,
    clippy::precedence,
    clippy::collapsible_if,
    clippy::unnecessary_cast
)]

#[inline]
fn high(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}
#[inline]
fn low(x: f64) -> u32 {
    x.to_bits() as u32
}
#[inline]
fn from_words(hi: i32, lo: u32) -> f64 {
    f64::from_bits(((hi as u32 as u64) << 32) | lo as u64)
}
#[inline]
fn with_high(x: f64, hi: i32) -> f64 {
    from_words(hi, low(x))
}
#[inline]
fn with_low(x: f64, lo: u32) -> f64 {
    from_words(high(x), lo)
}
#[inline]
fn abs_bits(x: f64) -> i32 {
    high(x) & 0x7fff_ffff
}

fn scalbn(mut x: f64, n: i32) -> f64 {
    const TWO54: f64 = 1.80143985094819840000e+16;
    const TWOM54: f64 = 5.55111512312578270212e-17;
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    let hx = high(x);
    let lx = low(x);
    let mut k = (hx & 0x7ff0_0000) >> 20;
    if k == 0 {
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return x;
        }
        x *= TWO54;
        k = ((high(x) & 0x7ff0_0000) >> 20) - 54;
        if n < -50_000 {
            return TINY * x;
        }
    }
    if k == 0x7ff {
        return x + x;
    }
    k += n;
    if k > 0x7fe {
        return (HUGE * HUGE).copysign(x);
    }
    if k > 0 {
        return with_high(x, (high(x) & 0x800f_ffffu32 as i32) | (k << 20));
    }
    if k <= -54 {
        if n > 50_000 {
            return (HUGE * HUGE).copysign(x);
        }
        return (TINY * TINY).copysign(x);
    }
    let y = with_high(x, (high(x) & 0x800f_ffffu32 as i32) | ((k + 54) << 20));
    y * TWOM54
}

pub fn log(mut x: f64) -> f64 {
    const LN2_HI: f64 = 6.93147180369123816490e-01;
    const LN2_LO: f64 = 1.90821492927058770002e-10;
    const TWO54: f64 = 1.80143985094819840000e+16;
    const LG1: f64 = 6.666666666666735130e-01;
    const LG2: f64 = 3.999999999940941908e-01;
    const LG3: f64 = 2.857142874366239149e-01;
    const LG4: f64 = 2.222219843214978396e-01;
    const LG5: f64 = 1.818357216161805012e-01;
    const LG6: f64 = 1.531383769920937332e-01;
    const LG7: f64 = 1.479819860511658591e-01;
    let mut hx = high(x);
    let lx = low(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return -TWO54 / 0.0;
        }
        if hx < 0 {
            return (x - x) / 0.0;
        }
        k -= 54;
        x *= TWO54;
        hx = high(x);
    }
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    // fdlibm's i = (hx + 0x95f64) & 0x100000; normalize x or x/2.
    let i = ((hx & 0x000f_ffff) + 0x95f64) & 0x0010_0000;
    let x = with_high(x, (hx & 0x000f_ffff) | (i ^ 0x3ff0_0000));
    k += i >> 20;
    let f = x - 1.0;
    if (0x000f_ffff & (2 + (hx & 0x000f_ffff))) < 3 {
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk = k as f64;
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        if k == 0 {
            return f - r;
        }
        let dk = k as f64;
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = k as f64;
    let z = s * s;
    let i = (hx & 0x000f_ffff) - 0x6147a;
    let j = 0x6b851 - (hx & 0x000f_ffff);
    let w = z * z;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    let r = t2 + t1;
    if (i | j) > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

pub fn log10(x: f64) -> f64 {
    const TWO54: f64 = 1.80143985094819840000e+16;
    let mut hx = high(x);
    let lx = low(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return -TWO54 / 0.0;
        }
        if hx < 0 {
            return (x - x) / 0.0;
        }
        k -= 54;
        let x2 = x * TWO54;
        hx = high(x2);
        return log10_core(x2, hx, k);
    }
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    log10_core(x, hx, k)
}
fn log10_core(mut x: f64, mut hx: i32, k0: i32) -> f64 {
    const IVLN10: f64 = 4.34294481903251816668e-01;
    const LOG10_2HI: f64 = 3.01029995663611771306e-01;
    const LOG10_2LO: f64 = 3.69423907715893078616e-13;
    let k = k0 + (hx >> 20) - 1023;
    let i = ((k as u32 & 0x8000_0000) >> 31) as i32;
    hx = (hx & 0x000f_ffff) | ((0x3ff - i) << 20);
    let y = (k + i) as f64;
    x = with_high(x, hx);
    let z = y * LOG10_2LO + IVLN10 * log(x);
    z + y * LOG10_2HI
}

pub fn exp(mut x: f64) -> f64 {
    const ONE: f64 = 1.0;
    const HALF: [f64; 2] = [0.5, -0.5];
    const HUGE: f64 = 1.0e300;
    const TWOM1000: f64 = 9.33263618503218878990e-302;
    const O_THRESHOLD: f64 = 7.09782712893383973096e+02;
    const U_THRESHOLD: f64 = -7.45133219101941108420e+02;
    const LN2HI: [f64; 2] = [6.93147180369123816490e-01, -6.93147180369123816490e-01];
    const LN2LO: [f64; 2] = [1.90821492927058770002e-10, -1.90821492927058770002e-10];
    const INVLN2: f64 = 1.44269504088896338700e+00;
    const P1: f64 = 1.66666666666666019037e-01;
    const P2: f64 = -2.77777777770155933842e-03;
    const P3: f64 = 6.61375632143793436117e-05;
    const P4: f64 = -1.65339022054652515390e-06;
    const P5: f64 = 4.13813679705723846039e-08;
    let mut hx = high(x);
    let xsb = ((hx as u32) >> 31) as usize;
    hx &= 0x7fff_ffff;
    if hx >= 0x4086_2e42 {
        if hx >= 0x7ff0_0000 {
            if ((hx & 0xfffff) as u32 | low(x)) != 0 {
                return x + x;
            }
            return if xsb == 0 { x } else { 0.0 };
        }
        if x > O_THRESHOLD {
            return HUGE * HUGE;
        }
        if x < U_THRESHOLD {
            return TWOM1000 * TWOM1000;
        }
    }
    let mut hi = 0.0;
    let mut lo = 0.0;
    let mut k: i32 = 0;
    if hx > 0x3fd6_2e42 {
        if hx < 0x3ff0_a2b2 {
            hi = x - LN2HI[xsb];
            lo = LN2LO[xsb];
            k = 1 - (xsb as i32) - (xsb as i32);
        } else {
            k = (INVLN2 * x + HALF[xsb]) as i32;
            let t = k as f64;
            hi = x - t * LN2HI[0];
            lo = t * LN2LO[0];
        }
        x = hi - lo;
    } else if hx < 0x3e30_0000 {
        if HUGE + x > ONE {
            return ONE + x;
        }
    }
    let t = x * x;
    let c = x - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    if k == 0 {
        return ONE - ((x * c) / (c - 2.0) - x);
    }
    let mut y = ONE - ((lo - (x * c) / (2.0 - c)) - hi);
    if k >= -1021 {
        y = with_high(y, high(y) + (k << 20));
        y
    } else {
        y = with_high(y, high(y) + ((k + 1000) << 20));
        y * TWOM1000
    }
}

pub fn cbrt(mut x: f64) -> f64 {
    const B1: i32 = 715094163;
    const B2: i32 = 696219795;
    const C: f64 = 5.42857142857142815906e-01;
    const D: f64 = -7.05306122448979611050e-01;
    const E: f64 = 1.41428571428571436819e+00;
    const F: f64 = 1.60714285714285720630e+00;
    const G: f64 = 3.57142857142857150787e-01;
    let hx0 = high(x);
    let sign = hx0 & (0x8000_0000u32 as i32);
    let hx = hx0 ^ sign;
    if hx >= 0x7ff0_0000 {
        return x;
    }
    if (hx as u32 | low(x)) == 0 {
        return x;
    }
    x = with_high(x, hx);
    let mut t = 0.0;
    if hx < 0x0010_0000 {
        t = with_high(t, 0x4350_0000);
        t *= x;
        t = with_high(t, high(t) / 3 + B2);
    } else {
        t = with_high(t, hx / 3 + B1);
    }
    let r = t * t / x;
    let s = C + r * t;
    t *= G + F / (s + E + D / s);
    t = with_low(t, 0);
    t = with_high(t, high(t) + 1);
    let s = t * t;
    let r = x / s;
    let w = t + t;
    let r = (r - t) / (w + r);
    t += t * r;
    with_high(t, high(t) | sign)
}

#[allow(unused_assignments)]
pub fn expm1(mut x: f64) -> f64 {
    const ONE: f64 = 1.0;
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    const O_THRESHOLD: f64 = 7.09782712893383973096e+02;
    const LN2_HI: f64 = 6.93147180369123816490e-01;
    const LN2_LO: f64 = 1.90821492927058770002e-10;
    const INVLN2: f64 = 1.44269504088896338700e+00;
    const Q1: f64 = -3.33333333333331316428e-02;
    const Q2: f64 = 1.58730158725481460165e-03;
    const Q3: f64 = -7.93650757867487942473e-05;
    const Q4: f64 = 4.00821782732936239552e-06;
    const Q5: f64 = -2.01099218183624371326e-07;
    let hx0 = high(x);
    let xsb = hx0 & (0x8000_0000u32 as i32);
    let hx = hx0 & 0x7fff_ffff;
    if hx >= 0x4043_687a {
        if hx >= 0x4086_2e42 {
            if hx >= 0x7ff0_0000 {
                if ((hx & 0xfffff) as u32 | low(x)) != 0 {
                    return x + x;
                }
                return if xsb == 0 { x } else { -1.0 };
            }
            if x > O_THRESHOLD {
                return HUGE * HUGE;
            }
        }
        if xsb != 0 {
            if x + TINY < 0.0 {
                return TINY - ONE;
            }
            return TINY - ONE;
        }
    }
    let mut hi = 0.0;
    let mut lo = 0.0;
    let mut c = 0.0;
    let mut k: i32 = 0;
    if hx > 0x3fd6_2e42 {
        if hx < 0x3ff0_a2b2 {
            if xsb == 0 {
                hi = x - LN2_HI;
                lo = LN2_LO;
                k = 1;
            } else {
                hi = x + LN2_HI;
                lo = -LN2_LO;
                k = -1;
            }
        } else {
            k = (INVLN2 * x + if xsb == 0 { 0.5 } else { -0.5 }) as i32;
            let t = k as f64;
            hi = x - t * LN2_HI;
            lo = t * LN2_LO;
        }
        x = hi - lo;
        c = (hi - x) - lo;
    } else if hx < 0x3c90_0000 {
        let t = HUGE + x;
        return x - (t - (HUGE + x));
    }
    let hfx = 0.5 * x;
    let hxs = x * hfx;
    let r1 = ONE + hxs * (Q1 + hxs * (Q2 + hxs * (Q3 + hxs * (Q4 + hxs * Q5))));
    let t = 3.0 - r1 * hfx;
    let mut e = hxs * ((r1 - t) / (6.0 - x * t));
    if k == 0 {
        return x - (x * e - hxs);
    }
    e = x * (e - c) - c;
    e -= hxs;
    if k == -1 {
        return 0.5 * (x - e) - 0.5;
    }
    if k == 1 {
        if x < -0.25 {
            return -2.0 * (e - (x + 0.5));
        }
        return ONE + 2.0 * (x - e);
    }
    if k <= -2 || k > 56 {
        let mut y = ONE - (e - x);
        y = with_high(y, high(y) + (k << 20));
        return y - ONE;
    }
    let mut t = ONE;
    if k < 20 {
        t = with_high(t, 0x3ff0_0000 - (0x200000 >> k));
        let mut y = t - (e - x);
        y = with_high(y, high(y) + (k << 20));
        y
    } else {
        t = with_high(t, (0x3ff - k) << 20);
        let mut y = x - (e + t);
        y += ONE;
        y = with_high(y, high(y) + (k << 20));
        y
    }
}

pub fn tanh(x: f64) -> f64 {
    const ONE: f64 = 1.0;
    const TWO: f64 = 2.0;
    const TINY: f64 = 1.0e-300;
    let jx = high(x);
    let ix = jx & 0x7fff_ffff;
    if ix >= 0x7ff0_0000 {
        if jx >= 0 {
            return 1.0 / x + ONE;
        }
        return 1.0 / x - ONE;
    }
    let z = if ix < 0x4036_0000 {
        if ix < 0x3c80_0000 {
            return x * (ONE + x);
        }
        if ix >= 0x3ff0_0000 {
            let t = expm1(TWO * x.abs());
            ONE - TWO / (t + TWO)
        } else {
            let t = expm1(-TWO * x.abs());
            -t / (t + TWO)
        }
    } else {
        ONE - TINY
    };
    if jx >= 0 { z } else { -z }
}

pub fn pow(x: f64, y: f64) -> f64 {
    const BP: [f64; 2] = [1.0, 1.5];
    const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01];
    const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08];
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO: f64 = 2.0;
    const TWO53: f64 = 9007199254740992.0;
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    const L1: f64 = 5.99999999999994648725e-01;
    const L2: f64 = 4.28571428578550184252e-01;
    const L3: f64 = 3.33333329818377432918e-01;
    const L4: f64 = 2.72728123808534006489e-01;
    const L5: f64 = 2.30660745775561754067e-01;
    const L6: f64 = 2.06975017800338417784e-01;
    const P1: f64 = 1.66666666666666019037e-01;
    const P2: f64 = -2.77777777770155933842e-03;
    const P3: f64 = 6.61375632143793436117e-05;
    const P4: f64 = -1.65339022054652515390e-06;
    const P5: f64 = 4.13813679705723846039e-08;
    const LG2: f64 = 6.93147180559945286227e-01;
    const LG2_H: f64 = 6.93147182464599609375e-01;
    const LG2_L: f64 = -1.90465429995776804525e-09;
    const OVT: f64 = 8.0085662595372944372e-0017;
    const CP: f64 = 9.61796693925975554329e-01;
    const CP_H: f64 = 9.61796700954437255859e-01;
    const CP_L: f64 = -7.02846165095275826516e-09;
    const IVLN2: f64 = 1.44269504088896338700e+00;
    const IVLN2_H: f64 = 1.44269502162933349609e+00;
    const IVLN2_L: f64 = 1.92596299112661746887e-08;
    let hx = high(x);
    let lx = low(x);
    let hy = high(y);
    let ly = low(y);
    let ix = hx & 0x7fff_ffff;
    let iy = hy & 0x7fff_ffff;
    if (iy as u32 | ly) == 0 {
        return ONE;
    }
    if ix > 0x7ff0_0000
        || (ix == 0x7ff0_0000 && lx != 0)
        || iy > 0x7ff0_0000
        || (iy == 0x7ff0_0000 && ly != 0)
    {
        return x + y;
    }
    let mut yisint = 0;
    if hx < 0 {
        if iy >= 0x4340_0000 {
            yisint = 2;
        } else if iy >= 0x3ff0_0000 {
            let k = (iy >> 20) - 0x3ff;
            if k > 20 {
                let j = ly >> (52 - k);
                if (j << (52 - k)) == ly {
                    yisint = 2 - (j & 1) as i32;
                }
            } else if ly == 0 {
                let j = (iy >> (20 - k)) as u32;
                if (j << (20 - k)) == (iy as u32) {
                    yisint = 2 - (j & 1) as i32;
                }
            }
        }
    }
    if ly == 0 {
        if iy == 0x7ff0_0000 {
            if ((ix - 0x3ff0_0000) | lx as i32) == 0 {
                return y - y;
            }
            if ix >= 0x3ff0_0000 {
                return if hy >= 0 { y } else { ZERO };
            }
            return if hy < 0 { -y } else { ZERO };
        }
        if iy == 0x3ff0_0000 {
            return if hy < 0 { ONE / x } else { x };
        }
        if iy == 0x4000_0000 {
            return x * x;
        }
        if iy == 0x3fe0_0000 && hx >= 0 {
            return x.sqrt();
        }
    }
    let ax = x.abs();
    if lx == 0 && (ix == 0x7ff0_0000 || ix == 0 || ix == 0x3ff0_0000) {
        let mut z = ax;
        if hy < 0 {
            z = ONE / z;
        }
        if hx < 0 {
            if ((ix - 0x3ff0_0000) | yisint) == 0 {
                z = (z - z) / (z - z);
            } else if yisint == 1 {
                z = -z;
            }
        }
        return z;
    }
    let nsign = (hx >> 31) + 1;
    if (nsign | yisint) == 0 {
        return (x - x) / (x - x);
    }
    let mut s = ONE;
    if (nsign | (yisint - 1)) == 0 {
        s = -ONE;
    }
    if iy > 0x41e0_0000 {
        if iy > 0x43f0_0000 {
            if ix <= 0x3fefffff {
                return if hy < 0 {
                    HUGE * HUGE * s
                } else {
                    TINY * TINY * s
                };
            }
            if ix >= 0x3ff0_0000 {
                return if hy > 0 {
                    HUGE * HUGE * s
                } else {
                    TINY * TINY * s
                };
            }
        }
        if ix < 0x3fefffff {
            return if hy < 0 {
                HUGE * HUGE * s
            } else {
                TINY * TINY * s
            };
        }
        if ix > 0x3ff0_0000 {
            return if hy > 0 {
                HUGE * HUGE * s
            } else {
                TINY * TINY * s
            };
        }
        let t = ax - ONE;
        let w = (t * t) * (0.5 - t * (0.33333333333333333333 - t * 0.25));
        let u = IVLN2_H * t;
        let v = t * IVLN2_L - w * IVLN2;
        let t1 = u + v;
        let t1 = with_low(t1, 0);
        let t2 = v - (t1 - u);
        return pow_tail(s, y, t1, t2, LG2_H, LG2_L, OVT, P1, P2, P3, P4, P5);
    }
    let mut n = 0i32;
    let (t1, t2) = {
        let mut ax = ax;
        if ix < 0x0010_0000 {
            ax *= TWO53;
            n -= 53;
        }
        let ix2 = high(ax);
        n += (ix2 >> 20) - 0x3ff;
        let j = ix2 & 0x000f_ffff;
        let (k, ixn) = if j <= 0x3988e {
            (0, j | 0x3ff0_0000)
        } else if j < 0xbb67a {
            (1, j | 0x3ff0_0000)
        } else {
            (0, j | 0x3ff0_0000 - 0x0010_0000)
        };
        if j >= 0xbb67a {
            n += 1;
        }
        ax = with_high(ax, ixn);
        let u = ax - BP[k];
        let v = ONE / (ax + BP[k]);
        let ss = u * v;
        let sh = with_low(ss, 0);
        let th = with_high(
            0.0,
            ((ixn >> 1) | 0x2000_0000) + 0x0008_0000 + ((k as i32) << 18),
        );
        let tl = ax - (th - BP[k]);
        let sl = v * ((u - sh * th) - sh * tl);
        let s2 = ss * ss;
        let r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        let r = r + sl * (sh + ss);
        let s2 = sh * sh;
        let th = with_low(3.0 + s2 + r, 0);
        let tl = r - ((th - 3.0) - s2);
        let u = sh * th;
        let v = sl * th + tl * ss;
        let ph = with_low(u + v, 0);
        let pl = v - (ph - u);
        let zh = CP_H * ph;
        let zl = CP_L * ph + pl * CP + DP_L[k];
        let t = n as f64;
        let t1 = with_low(((zh + zl) + DP_H[k]) + t, 0);
        let t2 = zl - (((t1 - t) - DP_H[k]) - zh);
        (t1, t2)
    };
    let y1 = with_low(y, 0);
    let pl = (y - y1) * t1 + y * t2;
    let mut ph = y1 * t1;
    let z = pl + ph;
    let j = high(z);
    let i = low(z);
    if j >= 0x4090_0000 {
        if ((j - 0x4090_0000) | i as i32) != 0 {
            return s * HUGE * HUGE;
        }
        if pl + OVT > z - ph {
            return s * HUGE * HUGE;
        }
    } else if (j & 0x7fff_ffff) >= 0x4090_cc00 {
        if ((j - (0xc090_cc00u32 as i32)) | i as i32) != 0 {
            return s * TINY * TINY;
        }
        if pl <= z - ph {
            return s * TINY * TINY;
        }
    }
    let iz = j & 0x7fff_ffff;
    let mut n2 = 0i32;
    let t;
    if iz > 0x3fe0_0000 {
        let nword = j + (0x0010_0000 >> ((iz >> 20) - 0x3ff + 1));
        let k = ((nword & 0x7fff_ffff) >> 20) - 0x3ff;
        t = with_high(0.0, nword & !(0x000f_ffff >> k));
        n2 = ((nword & 0x000f_ffff) | 0x0010_0000) >> (20 - k);
        if j < 0 {
            n2 = -n2;
        }
        ph -= t;
    }
    let t = with_low(ph + pl, 0);
    let u = t * LG2_H;
    let v = (pl - (t - ph)) * LG2 + t * LG2_L;
    let z = u + v;
    let w = v - (z - u);
    let zz = z * z;
    let t1 = z - zz * (P1 + zz * (P2 + zz * (P3 + zz * (P4 + zz * P5))));
    let r = (z * t1) / (t1 - TWO) - (w + z * w);
    let z = ONE - (r - z);
    let jz = high(z) + (n2 << 20);
    if (jz >> 20) <= 0 {
        s * scalbn(z, n2)
    } else {
        s * with_high(z, jz)
    }
}

fn pow_tail(
    s: f64,
    y: f64,
    t1: f64,
    t2: f64,
    lg2_h: f64,
    lg2_l: f64,
    ovt: f64,
    p1: f64,
    p2: f64,
    p3: f64,
    p4: f64,
    p5: f64,
) -> f64 {
    let y1 = with_low(y, 0);
    let pl = (y - y1) * t1 + y * t2;
    let ph = y1 * t1;
    let z = pl + ph;
    let j = high(z);
    let i = low(z);
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    if j >= 0x4090_0000 {
        if ((j - 0x4090_0000) | i as i32) != 0 || pl + ovt > z - ph {
            return s * HUGE * HUGE;
        }
    } else if (j & 0x7fff_ffff) >= 0x4090_cc00 {
        if ((j - (0xc090_cc00u32 as i32)) | i as i32) != 0 || pl <= z - ph {
            return s * TINY * TINY;
        }
    }
    let iz = j & 0x7fff_ffff;
    let mut n = 0i32;
    let mut ph = ph;
    if iz > 0x3fe0_0000 {
        let k = (iz >> 20) - 0x3ff;
        let nword = j + (0x0010_0000 >> (k + 1));
        let k2 = ((nword & 0x7fff_ffff) >> 20) - 0x3ff;
        let t = with_high(0.0, nword & !(0x000f_ffff >> k2));
        n = ((nword & 0x000f_ffff) | 0x0010_0000) >> (20 - k2);
        if j < 0 {
            n = -n;
        }
        ph -= t;
    }
    let t = with_low(ph + pl, 0);
    let u = t * lg2_h;
    let v = (pl - (t - ph)) * 6.93147180559945286227e-01 + t * lg2_l;
    let z = u + v;
    let w = v - (z - u);
    let zz = z * z;
    let t1 = z - zz * (p1 + zz * (p2 + zz * (p3 + zz * (p4 + zz * p5))));
    let r = (z * t1) / (t1 - 2.0) - (w + z * w);
    let z = 1.0 - (r - z);
    let jz = high(z) + (n << 20);
    if jz >> 20 <= 0 {
        s * scalbn(z, n)
    } else {
        s * with_high(z, jz)
    }
}

const TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];
const NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];
const PIO2_1: f64 = 1.57079632673412561417e+00;
const PIO2_1T: f64 = 6.07710050650619224932e-11;
const PIO2_2: f64 = 6.07710050630396597660e-11;
const PIO2_2T: f64 = 2.02226624879595063154e-21;
const PIO2_3: f64 = 2.02226624871116645580e-21;
const PIO2_3T: f64 = 8.47842766036889956997e-32;

fn kernel_rem_pio2(x: &[f64], e0: i32, prec: usize) -> (i32, [f64; 3]) {
    const INIT_JK: [usize; 4] = [2, 3, 4, 6];
    const PIO2: [f64; 8] = [
        1.57079625129699707031e+00,
        7.54978941586159635335e-08,
        5.39030252995776476554e-15,
        3.28200341580791294123e-22,
        1.27065575308067607349e-29,
        1.22933308981111328932e-36,
        2.73370053816464559624e-44,
        2.16741683877804819444e-51,
    ];
    const TWO24: f64 = 1.67772160000000000000e+07;
    const TWON24: f64 = 5.96046447753906250000e-08;
    let jk = INIT_JK[prec];
    let jp = jk;
    let jx = x.len() - 1;
    let jv = ((e0 - 3) / 24).max(0);
    let mut q0 = e0 - 24 * (jv + 1);
    let j = jv as i32 - jx as i32;
    let m = jx + jk;
    let mut f = [0.0f64; 20];
    for i in 0..=m {
        let idx = j + i as i32;
        f[i] = if idx < 0 {
            0.0
        } else {
            TWO_OVER_PI[idx as usize] as f64
        };
    }
    let mut q = [0.0f64; 20];
    for i in 0..=jk {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw += x[j] * f[jx + i - j];
        }
        q[i] = fw;
    }
    let mut jz = jk;
    let (n, y) = loop {
        let mut iq = [0i32; 20];
        let mut z = q[jz];
        for (i, j) in (1..=jz).rev().enumerate() {
            let fw = (TWON24 * z) as i32 as f64;
            iq[i] = (z - TWO24 * fw) as i32;
            z = q[j - 1] + fw;
        }
        z = scalbn(z, q0);
        z -= 8.0 * (z * 0.125).floor();
        let mut n = z as i32;
        z -= n as f64;
        let mut ih = 0i32;
        if q0 > 0 {
            let i = (iq[jz - 1] >> (24 - q0)) as i32;
            n += i;
            iq[jz - 1] -= i << (24 - q0);
            ih = iq[jz - 1] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[jz - 1] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }
        if ih > 0 {
            n += 1;
            let mut carry = 0;
            for item in iq.iter_mut().take(jz) {
                let j = *item;
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        *item = 0x1000000 - j;
                    }
                } else {
                    *item = 0xffffff - j;
                }
            }
            if q0 == 1 {
                iq[jz - 1] &= 0x7fffff;
            } else if q0 == 2 {
                iq[jz - 1] &= 0x3fffff;
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry != 0 {
                    z -= scalbn(1.0, q0);
                }
            }
        }
        if z == 0.0 {
            let mut j = 0;
            for i in (jk..jz).rev() {
                j |= iq[i];
            }
            if j == 0 {
                let mut k = 1;
                while iq[jk - k] == 0 {
                    k += 1;
                }
                for i in (jz + 1)..=(jz + k) {
                    f[jx + i] = TWO_OVER_PI[jv as usize + i] as f64;
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw += x[j] * f[jx + i - j];
                    }
                    q[i] = fw;
                }
                jz += k;
                continue;
            }
        }
        if z == 0.0 {
            jz -= 1;
            q0 -= 24;
            while iq[jz] == 0 {
                jz -= 1;
                q0 -= 24;
            }
        } else {
            z = scalbn(z, -q0);
            if z >= TWO24 {
                let fw = (TWON24 * z) as i32 as f64;
                iq[jz] = (z - TWO24 * fw) as i32;
                jz += 1;
                q0 += 24;
                iq[jz] = fw as i32;
            } else {
                iq[jz] = z as i32;
            }
        }
        let mut fw = scalbn(1.0, q0);
        for i in (0..=jz).rev() {
            q[i] = fw * iq[i] as f64;
            fw *= TWON24;
        }
        let mut fq = [0.0f64; 20];
        for i in (0..=jz).rev() {
            let mut sum = 0.0;
            for k in 0..=jp.min(jz - i) {
                sum += PIO2[k] * q[i + k];
            }
            fq[jz - i] = sum;
        }
        let mut out = [0.0f64; 3];
        match prec {
            0 => {
                let mut sum = 0.0;
                for i in (0..=jz).rev() {
                    sum += fq[i];
                }
                out[0] = if ih == 0 { sum } else { -sum };
            }
            1 | 2 => {
                let mut sum = 0.0;
                for i in (0..=jz).rev() {
                    sum += fq[i];
                }
                out[0] = if ih == 0 { sum } else { -sum };
                let mut fw = fq[0] - sum;
                for i in 1..=jz {
                    fw += fq[i];
                }
                out[1] = if ih == 0 { fw } else { -fw };
            }
            _ => {
                for i in (1..=jz).rev() {
                    let fw = fq[i - 1] + fq[i];
                    fq[i] += fq[i - 1] - fw;
                    fq[i - 1] = fw;
                }
                for i in (2..=jz).rev() {
                    let fw = fq[i - 1] + fq[i];
                    fq[i] += fq[i - 1] - fw;
                    fq[i - 1] = fw;
                }
                let mut fw = 0.0;
                for i in (2..=jz).rev() {
                    fw += fq[i];
                }
                if ih == 0 {
                    out[0] = fq[0];
                    out[1] = fq[1];
                    out[2] = fw;
                } else {
                    out[0] = -fq[0];
                    out[1] = -fq[1];
                    out[2] = -fw;
                }
            }
        }
        break (n & 7, out);
    };
    (n, y)
}

fn rem_pio2(x: f64) -> (i32, [f64; 2]) {
    let hx = high(x);
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return (0, [x, 0.0]);
    }
    if ix < 0x4002_d97c {
        if hx > 0 {
            let mut z = x - PIO2_1;
            if ix != 0x3ff9_21fb {
                let y0 = z - PIO2_1T;
                return (1, [y0, (z - y0) - PIO2_1T]);
            }
            z -= PIO2_2;
            let y0 = z - PIO2_2T;
            return (1, [y0, (z - y0) - PIO2_2T]);
        }
        let mut z = x + PIO2_1;
        if ix != 0x3ff9_21fb {
            let y0 = z + PIO2_1T;
            return (-1, [y0, (z - y0) + PIO2_1T]);
        }
        z += PIO2_2;
        let y0 = z + PIO2_2T;
        return (-1, [y0, (z - y0) + PIO2_2T]);
    }
    if ix <= 0x4139_21fb {
        let t = x.abs();
        let n = (t * 6.36619772367581382433e-01 + 0.5) as i32;
        let fnv = n as f64;
        let mut r = t - fnv * PIO2_1;
        let mut w = fnv * PIO2_1T;
        let mut y0 = r - w;
        if n >= 32 || ix == NPIO2_HW[(n - 1) as usize] {
            let j = ix >> 20;
            let i = j - ((high(y0) as u32 >> 20) as i32 & 0x7ff);
            if i > 16 {
                let t0 = r;
                w = fnv * PIO2_2;
                r = t0 - w;
                w = fnv * PIO2_2T - ((t0 - r) - w);
                y0 = r - w;
                let i = j - ((high(y0) as u32 >> 20) as i32 & 0x7ff);
                if i > 49 {
                    let t0 = r;
                    w = fnv * PIO2_3;
                    r = t0 - w;
                    w = fnv * PIO2_3T - ((t0 - r) - w);
                    y0 = r - w;
                }
            }
        }
        let y1 = (r - y0) - w;
        return if hx < 0 {
            (-n, [-y0, -y1])
        } else {
            (n, [y0, y1])
        };
    }
    if ix >= 0x7ff0_0000 {
        return (0, [x - x, x - x]);
    }
    let mut z = with_low(0.0, low(x));
    let e0 = (ix >> 20) - 1046;
    z = with_high(z, ix - (e0 << 20));
    let mut tx = [0.0; 3];
    for item in &mut tx {
        *item = (z as i32) as f64;
        z = (z - *item) * 1.6777216e7;
    }
    let mut nx = 3;
    while nx > 0 && tx[nx - 1] == 0.0 {
        nx -= 1;
    }
    let (n, y3) = kernel_rem_pio2(&tx[..nx], e0, 2);
    if hx < 0 {
        (-n, [-y3[0], -y3[1]])
    } else {
        (n, [y3[0], y3[1]])
    }
}

pub fn tan(x: f64) -> f64 {
    let ix = abs_bits(x);
    if ix <= 0x3fe9_21fb {
        return kernel_tan(x, 0.0, 1);
    }
    if ix >= 0x7ff0_0000 {
        return x - x;
    }
    let (n, y) = rem_pio2(x);
    kernel_tan(y[0], y[1], 1 - ((n & 1) << 1))
}

fn kernel_tan(mut x: f64, mut y: f64, iy: i32) -> f64 {
    const T: [f64; 16] = [
        3.33333333333334091986e-01,
        1.33333333333201242699e-01,
        5.39682539762260521377e-02,
        2.18694882948595424599e-02,
        8.86323982359930005737e-03,
        3.59207910759131235356e-03,
        1.45620945432529025516e-03,
        5.88041240820264096874e-04,
        2.46463134818469906812e-04,
        7.81794442939557092300e-05,
        7.14072491382608190305e-05,
        -1.85586374855275456654e-05,
        2.59073051863633712884e-05,
        1.0,
        7.85398163397448278999e-01,
        3.06161699786838301793e-17,
    ];
    let hx = high(x);
    let ix = hx & 0x7fff_ffff;
    if ix < 0x3e30_0000 && (x as i32) == 0 {
        if ((ix as u32 | low(x)) | (iy + 1) as u32) == 0 {
            return 1.0 / x.abs();
        }
        if iy == 1 {
            return x;
        }
        let mut z = x + y;
        let w = z;
        z = with_low(z, 0);
        let v = y - (z - x);
        let mut t = -1.0 / w;
        let a = t;
        t = with_low(t, 0);
        let s = 1.0 + t * z;
        return t + a * (s + t * v);
    }
    if ix >= 0x3fe5_9428 {
        if hx < 0 {
            x = -x;
            y = -y;
        }
        let z = T[14] - x;
        let w = T[15] - y;
        x = z + w;
        y = 0.0;
    }
    let z = x * x;
    let w = z * z;
    let r = T[1] + w * (T[3] + w * (T[5] + w * (T[7] + w * (T[9] + w * T[11]))));
    let v = z * (T[2] + w * (T[4] + w * (T[6] + w * (T[8] + w * (T[10] + w * T[12])))));
    let s = z * x;
    let mut r = y + z * (s * (r + v) + y);
    r += T[0] * s;
    let w = x + r;
    if ix >= 0x3fe5_9428 {
        let v = iy as f64;
        return (1 - ((hx >> 30) & 2)) as f64 * (v - 2.0 * (x - (w * w / (w + v) - r)));
    }
    if iy == 1 {
        return w;
    }
    let z = w;
    let z = with_low(z, 0);
    let v = r - (z - x);
    let mut t = -1.0 / w;
    let a = t;
    t = with_low(t, 0);
    let s = 1.0 + t * z;
    t + a * (s + t * v)
}

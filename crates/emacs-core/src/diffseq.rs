pub trait Context {
    type Stop;
    fn equal(&mut self, x: usize, y: usize) -> Result<bool, Self::Stop>;
    fn note_delete(&mut self, x: usize);
    fn note_insert(&mut self, y: usize);
    fn early_abort(&mut self) -> bool;
}

const SNAKE_LIMIT: isize = 20;
const OFFSET_MAX: isize = isize::MAX;

struct Partition {
    xmid: isize,
    ymid: isize,
    lo_minimal: bool,
    hi_minimal: bool,
}

struct State<'a, C: Context> {
    ctxt: &'a mut C,
    fdiag: Vec<isize>,
    bdiag: Vec<isize>,
    origin: isize,
    heuristic: bool,
    too_expensive: isize,
}

pub fn compareseq<C: Context>(
    ctxt: &mut C,
    size_a: usize,
    size_b: usize,
    too_expensive: isize,
    heuristic: bool,
) -> Result<bool, C::Stop> {
    let diags = size_a + size_b + 3;
    let mut state = State {
        ctxt,
        fdiag: vec![0; diags],
        bdiag: vec![0; diags],
        origin: size_b as isize + 1,
        heuristic,
        too_expensive,
    };
    state.compareseq(0, size_a as isize, 0, size_b as isize, false)
}

impl<C: Context> State<'_, C> {
    fn equal(&mut self, x: isize, y: isize) -> Result<bool, C::Stop> {
        self.ctxt.equal(x as usize, y as usize)
    }

    fn fd(&self, d: isize) -> isize {
        self.fdiag[(d + self.origin) as usize]
    }

    fn set_fd(&mut self, d: isize, v: isize) {
        self.fdiag[(d + self.origin) as usize] = v;
    }

    fn bd(&self, d: isize) -> isize {
        self.bdiag[(d + self.origin) as usize]
    }

    fn set_bd(&mut self, d: isize, v: isize) {
        self.bdiag[(d + self.origin) as usize] = v;
    }

    fn diag(
        &mut self,
        xoff: isize,
        xlim: isize,
        yoff: isize,
        ylim: isize,
        find_minimal: bool,
    ) -> Result<Partition, C::Stop> {
        let dmin = xoff - ylim;
        let dmax = xlim - yoff;
        let fmid = xoff - yoff;
        let bmid = xlim - ylim;
        let (mut fmin, mut fmax) = (fmid, fmid);
        let (mut bmin, mut bmax) = (bmid, bmid);
        let odd = (fmid - bmid) & 1 != 0;
        self.set_fd(fmid, xoff);
        self.set_bd(bmid, xlim);
        let mut c: isize = 1;
        loop {
            let mut big_snake = false;
            if fmin > dmin {
                fmin -= 1;
                self.set_fd(fmin - 1, -1);
            } else {
                fmin += 1;
            }
            if fmax < dmax {
                fmax += 1;
                self.set_fd(fmax + 1, -1);
            } else {
                fmax -= 1;
            }
            let mut d = fmax;
            while d >= fmin {
                let tlo = self.fd(d - 1);
                let thi = self.fd(d + 1);
                let x0 = if tlo < thi { thi } else { tlo + 1 };
                let (mut x, mut y) = (x0, x0 - d);
                while x < xlim && y < ylim && self.equal(x, y)? {
                    x += 1;
                    y += 1;
                }
                if x - x0 > SNAKE_LIMIT {
                    big_snake = true;
                }
                self.set_fd(d, x);
                if odd && bmin <= d && d <= bmax && self.bd(d) <= x {
                    return Ok(Partition {
                        xmid: x,
                        ymid: y,
                        lo_minimal: true,
                        hi_minimal: true,
                    });
                }
                d -= 2;
            }
            if bmin > dmin {
                bmin -= 1;
                self.set_bd(bmin - 1, OFFSET_MAX);
            } else {
                bmin += 1;
            }
            if bmax < dmax {
                bmax += 1;
                self.set_bd(bmax + 1, OFFSET_MAX);
            } else {
                bmax -= 1;
            }
            let mut d = bmax;
            while d >= bmin {
                let tlo = self.bd(d - 1);
                let thi = self.bd(d + 1);
                let x0 = if tlo < thi { tlo } else { thi - 1 };
                let (mut x, mut y) = (x0, x0 - d);
                while xoff < x && yoff < y && self.equal(x - 1, y - 1)? {
                    x -= 1;
                    y -= 1;
                }
                if x0 - x > SNAKE_LIMIT {
                    big_snake = true;
                }
                self.set_bd(d, x);
                if !odd && fmin <= d && d <= fmax && x <= self.fd(d) {
                    return Ok(Partition {
                        xmid: x,
                        ymid: y,
                        lo_minimal: true,
                        hi_minimal: true,
                    });
                }
                d -= 2;
            }
            if !find_minimal {
                if 200 < c && big_snake && self.heuristic {
                    let mut best = 0;
                    let mut part = Partition {
                        xmid: 0,
                        ymid: 0,
                        lo_minimal: true,
                        hi_minimal: false,
                    };
                    let mut d = fmax;
                    while d >= fmin {
                        let dd = d - fmid;
                        let x = self.fd(d);
                        let y = x - d;
                        let v = (x - xoff) * 2 - dd;
                        if v > 12 * (c + dd.abs())
                            && v > best
                            && xoff + SNAKE_LIMIT <= x
                            && x < xlim
                            && yoff + SNAKE_LIMIT <= y
                            && y < ylim
                        {
                            let mut k = 1;
                            while self.equal(x - k, y - k)? {
                                if k == SNAKE_LIMIT {
                                    best = v;
                                    part.xmid = x;
                                    part.ymid = y;
                                    break;
                                }
                                k += 1;
                            }
                        }
                        d -= 2;
                    }
                    if best > 0 {
                        return Ok(part);
                    }
                    let mut best = 0;
                    let mut part = Partition {
                        xmid: 0,
                        ymid: 0,
                        lo_minimal: false,
                        hi_minimal: true,
                    };
                    let mut d = bmax;
                    while d >= bmin {
                        let dd = d - bmid;
                        let x = self.bd(d);
                        let y = x - d;
                        let v = (xlim - x) * 2 + dd;
                        if v > 12 * (c + dd.abs())
                            && v > best
                            && xoff < x
                            && x <= xlim - SNAKE_LIMIT
                            && yoff < y
                            && y <= ylim - SNAKE_LIMIT
                        {
                            let mut k = 0;
                            while self.equal(x + k, y + k)? {
                                if k == SNAKE_LIMIT - 1 {
                                    best = v;
                                    part.xmid = x;
                                    part.ymid = y;
                                    break;
                                }
                                k += 1;
                            }
                        }
                        d -= 2;
                    }
                    if best > 0 {
                        return Ok(part);
                    }
                }
                if c >= self.too_expensive {
                    let (mut fxybest, mut fxbest) = (-1, 0);
                    let mut d = fmax;
                    while d >= fmin {
                        let mut x = self.fd(d).min(xlim);
                        let mut y = x - d;
                        if ylim < y {
                            x = ylim + d;
                            y = ylim;
                        }
                        if fxybest < x + y {
                            fxybest = x + y;
                            fxbest = x;
                        }
                        d -= 2;
                    }
                    let (mut bxybest, mut bxbest) = (OFFSET_MAX, 0);
                    let mut d = bmax;
                    while d >= bmin {
                        let mut x = xoff.max(self.bd(d));
                        let mut y = x - d;
                        if y < yoff {
                            x = yoff + d;
                            y = yoff;
                        }
                        if x + y < bxybest {
                            bxybest = x + y;
                            bxbest = x;
                        }
                        d -= 2;
                    }
                    return Ok(if (xlim + ylim) - bxybest < fxybest - (xoff + yoff) {
                        Partition {
                            xmid: fxbest,
                            ymid: fxybest - fxbest,
                            lo_minimal: true,
                            hi_minimal: false,
                        }
                    } else {
                        Partition {
                            xmid: bxbest,
                            ymid: bxybest - bxbest,
                            lo_minimal: false,
                            hi_minimal: true,
                        }
                    });
                }
            }
            c += 1;
        }
    }

    fn compareseq(
        &mut self,
        mut xoff: isize,
        mut xlim: isize,
        mut yoff: isize,
        mut ylim: isize,
        mut find_minimal: bool,
    ) -> Result<bool, C::Stop> {
        loop {
            while xoff < xlim && yoff < ylim && self.equal(xoff, yoff)? {
                xoff += 1;
                yoff += 1;
            }
            while xoff < xlim && yoff < ylim && self.equal(xlim - 1, ylim - 1)? {
                xlim -= 1;
                ylim -= 1;
            }
            if xoff == xlim {
                while yoff < ylim {
                    self.ctxt.note_insert(yoff as usize);
                    if self.ctxt.early_abort() {
                        return Ok(true);
                    }
                    yoff += 1;
                }
                break;
            }
            if yoff == ylim {
                while xoff < xlim {
                    self.ctxt.note_delete(xoff as usize);
                    if self.ctxt.early_abort() {
                        return Ok(true);
                    }
                    xoff += 1;
                }
                break;
            }
            let part = self.diag(xoff, xlim, yoff, ylim, find_minimal)?;
            let (first, second) = if (xlim + ylim) - (part.xmid + part.ymid)
                < (part.xmid + part.ymid) - (xoff + yoff)
            {
                (
                    (part.xmid, xlim, part.ymid, ylim, part.hi_minimal),
                    (xoff, part.xmid, yoff, part.ymid, part.lo_minimal),
                )
            } else {
                (
                    (xoff, part.xmid, yoff, part.ymid, part.lo_minimal),
                    (part.xmid, xlim, part.ymid, ylim, part.hi_minimal),
                )
            };
            if self.compareseq(first.0, first.1, first.2, first.3, first.4)? {
                return Ok(true);
            }
            (xoff, xlim, yoff, ylim, find_minimal) = second;
        }
        Ok(false)
    }
}

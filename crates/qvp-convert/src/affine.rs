//! Minimal 2-D affine transform: [a c e; b d f; 0 0 1].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(tx: f64, ty: f64) -> Affine {
        Affine { e: tx, f: ty, ..Affine::IDENTITY }
    }

    /// `self` then `other` in SVG nesting order: result = self ∘ other
    /// (apply `other` first in local space, then `self`).
    pub fn then(&self, o: &Affine) -> Affine {
        Affine {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    pub fn parse(s: &str) -> Result<Affine, String> {
        let t = s.parse::<svgtypes::Transform>().map_err(|e| format!("bad transform {s:?}: {e}"))?;
        Ok(Affine { a: t.a, b: t.b, c: t.c, d: t.d, e: t.e, f: t.f })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nesting_order() {
        // outer: scale 2, inner: translate (1,0)  → point (0,0) → inner → (1,0) → outer → (2,0)
        let outer = Affine { a: 2.0, d: 2.0, ..Affine::IDENTITY };
        let inner = Affine::translate(1.0, 0.0);
        assert_eq!(outer.then(&inner).apply(0.0, 0.0), (2.0, 0.0));
        let m = Affine::parse("matrix(1.3333 0 0 -1.3333 -55 640)").unwrap();
        let (x, y) = m.apply(0.0, 0.0);
        assert_eq!((x, y), (-55.0, 640.0));
        let t = Affine::parse("translate(10 20) scale(0.5 -0.5)").unwrap();
        assert_eq!(t.apply(2.0, 2.0), (11.0, 19.0));
    }
}

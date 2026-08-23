/**
 * Vec2 - High-performance 2D Vector mathematics engine.
 * Designed with Zero-GC in-place mutations and Zero-NaN arithmetic guards.
 */
export class Vec2 {
  public x: number;
  public y: number;

  constructor(x: number = 0, y: number = 0) {
    this.x = Number.isFinite(x) ? x : 0;
    this.y = Number.isFinite(y) ? y : 0;
  }

  // ==========================================
  // In-place Mutation Operations (Zero-GC)
  // ==========================================

  public set(x: number, y: number): this {
    this.x = Number.isFinite(x) ? x : 0;
    this.y = Number.isFinite(y) ? y : 0;
    return this;
  }

  public copy(v: Vec2): this {
    this.x = v.x;
    this.y = v.y;
    return this;
  }

  public addInPlace(v: Vec2): this {
    this.x += v.x;
    this.y += v.y;
    return this;
  }

  public subInPlace(v: Vec2): this {
    this.x -= v.x;
    this.y -= v.y;
    return this;
  }

  public scaleInPlace(s: number): this {
    if (Number.isFinite(s)) {
      this.x *= s;
      this.y *= s;
    }
    return this;
  }

  public addScaledInPlace(v: Vec2, s: number): this {
    if (Number.isFinite(s)) {
      this.x += v.x * s;
      this.y += v.y * s;
    }
    return this;
  }

  public perpInPlace(): this {
    const tempX = this.x;
    this.x = -this.y;
    this.y = tempX;
    return this;
  }

  public negateInPlace(): this {
    this.x = -this.x;
    this.y = -this.y;
    return this;
  }

  public rotateInPlace(radians: number): this {
    const cos = Math.cos(radians);
    const sin = Math.sin(radians);
    const rx = this.x * cos - this.y * sin;
    const ry = this.x * sin + this.y * cos;
    this.x = rx;
    this.y = ry;
    return this;
  }

  /**
   * Zero-NaN Protected Safe Normalization.
   * Checks magSq() > 1e-6 before computing inverse square root.
   */
  public normalizeSafe(fallback?: Vec2): this {
    const mSq = this.x * this.x + this.y * this.y;
    if (mSq > 1e-6) {
      const invMag = 1.0 / Math.sqrt(mSq);
      this.x *= invMag;
      this.y *= invMag;
    } else if (fallback) {
      this.x = fallback.x;
      this.y = fallback.y;
    } else {
      this.x = 0;
      this.y = 0;
    }
    return this;
  }

  public setLengthInPlace(len: number): this {
    this.normalizeSafe();
    this.scaleInPlace(len);
    return this;
  }

  public clampLengthInPlace(min: number, max: number): this {
    const mSq = this.magSq();
    if (mSq > 1e-6) {
      const m = Math.sqrt(mSq);
      if (m != 0 && m < min && min > 0) {
        this.scaleInPlace(min / m);
      } else if (m != 0 && m > max && max > 0) {
        this.scaleInPlace(max / m);
      }
    }
    return this;
  }

  // ==========================================
  // Geometric & Scalar Projections
  // ==========================================

  public dot(v: Vec2): number {
    return this.x * v.x + this.y * v.y;
  }

  public cross(v: Vec2): number {
    return this.x * v.y - this.y * v.x;
  }

  public magSq(): number {
    return this.x * this.x + this.y * this.y;
  }

  public mag(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }

  public distSq(v: Vec2): number {
    const dx = this.x - v.x;
    const dy = this.y - v.y;
    return dx * dx + dy * dy;
  }

  public dist(v: Vec2): number {
    return Math.sqrt(this.distSq(v));
  }

  public angle(): number {
    return Math.atan2(this.y, this.x);
  }

  // ==========================================
  // Pure Calculation Helpers (Non-hot path)
  // ==========================================

  public clone(): Vec2 {
    return new Vec2(this.x, this.y);
  }

  public add(v: Vec2): Vec2 {
    return new Vec2(this.x + v.x, this.y + v.y);
  }

  public sub(v: Vec2): Vec2 {
    return new Vec2(this.x - v.x, this.y - v.y);
  }

  public scale(s: number): Vec2 {
    return new Vec2(this.x * s, this.y * s);
  }

  // ==========================================
  // Static Operations & Cross Products
  // ==========================================

  public static create(x: number = 0, y: number = 0): Vec2 {
    return new Vec2(x, y);
  }

  public static dot(a: Vec2, b: Vec2): number {
    return a.x * b.x + a.y * b.y;
  }

  public static cross(a: Vec2, b: Vec2): number {
    return a.x * b.y - a.y * b.x;
  }

  /**
   * 2D Cross product: Vector x Scalar = (s * v.y, -s * v.x)
   */
  public static crossScalar(v: Vec2, s: number, out: Vec2): Vec2 {
    out.x = s * v.y;
    out.y = -s * v.x;
    return out;
  }

  /**
   * 2D Cross product: Scalar x Vector = (-s * v.y, s * v.x)
   */
  public static crossVector(s: number, v: Vec2, out: Vec2): Vec2 {
    out.x = -s * v.y;
    out.y = s * v.x;
    return out;
  }

  public static distSq(a: Vec2, b: Vec2): number {
    const dx = a.x - b.x;
    const dy = a.y - b.y;
    return dx * dx + dy * dy;
  }

  public static dist(a: Vec2, b: Vec2): number {
    return Math.sqrt(Vec2.distSq(a, b));
  }

  public static lerp(a: Vec2, b: Vec2, t: number, out: Vec2): Vec2 {
    out.x = a.x + (b.x - a.x) * t;
    out.y = a.y + (b.y - a.y) * t;
    return out;
  }
}

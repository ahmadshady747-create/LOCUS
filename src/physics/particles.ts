import { Vec2 } from '../math/vec2';

/**
 * Particle - High performance visual spark/debris object.
 */
export class Particle {
  public position: Vec2 = new Vec2();
  public velocity: Vec2 = new Vec2();
  public color: string = '#f59e0b';
  public size: number = 3;
  public initialSize: number = 3;
  public life: number = 0;
  public maxLife: number = 1.0;
  public alpha: number = 1.0;
  public active: boolean = false;
  public drag: number = 0.98;

  public reset(): void {
    this.position?.set(0, 0);
    this.velocity?.set(0, 0);
    this.color = '#f59e0b';
    this.size = 3;
    this.initialSize = 3;
    this.life = 0;
    this.maxLife = 1.0;
    this.alpha = 1.0;
    this.active = false;
    this.drag = 0.98;
  }
}

/**
 * ParticlePool - Zero-GC fixed pre-allocated particle manager.
 */
export class ParticlePool {
  public particles: Particle[] = [];
  public capacity: number;
  private head: number = 0;
  public activeCount: number = 0;

  constructor(capacity: number = 1000) {
    this.capacity = capacity;
    for (let i = 0; i < capacity; i++) {
      this.particles?.push(new Particle());
    }
  }

  /**
   * Emits a burst of particles from (x, y) without allocating new objects.
   */
  public emit(
    x: number,
    y: number,
    count: number,
    color: string = '#38bdf8',
    speed: number = 150,
    spread: number = Math.PI * 2,
    baseAngle: number = 0,
    life: number = 0.8,
    size: number = 3.5
  ): void {
    const total = this.particles.length;
    for (let i = 0; i < count; i++) {
      const p = this.particles.at(this.head);
      this.head = (this.head + 1) % total;

      if (!p) continue;

      const angle = baseAngle + (Math.random() - 0.5) * spread;
      const velMag = speed * (0.4 + Math.random() * 0.8);

      const pos = p.position;
      pos?.set(x, y);

      const vel = p.velocity;
      vel?.set(Math.cos(angle) * velMag, Math.sin(angle) * velMag);

      p.color = color;
      p.initialSize = size * (0.6 + Math.random() * 0.8);
      p.size = p.initialSize;
      p.maxLife = life * (0.5 + Math.random() * 0.7);
      p.life = p.maxLife;
      p.alpha = 1.0;
      p.active = true;
      p.drag = 0.96 + Math.random() * 0.03;
    }
  }

  /**
   * Emits directional impact sparks when collision occurs.
   */
  public emitImpactSparks(x: number, y: number, normal: Vec2, count: number = 8, color: string = '#fbbf24'): void {
    const baseAngle = Math.atan2(-normal.y, -normal.x);
    this.emit(x, y, count, color, 180, Math.PI * 0.8, baseAngle, 0.4, 3.0);
  }

  /**
   * Emits omnidirectional explosion sparks and shockwaves.
   */
  public emitExplosion(x: number, y: number, count: number = 60, speed: number = 320): void {
    this.emit(x, y, Math.floor(count * 0.5), '#ef4444', speed * 1.2, Math.PI * 2, 0, 1.0, 5.0);
    this.emit(x, y, Math.floor(count * 0.3), '#f59e0b', speed * 0.9, Math.PI * 2, 0, 0.7, 4.0);
    this.emit(x, y, Math.floor(count * 0.2), '#38bdf8', speed * 0.6, Math.PI * 2, 0, 0.5, 3.0);
  }

  /**
   * Updates all active particles in-place (Zero-GC).
   */
  public update(dt: number, gravity: Vec2): void {
    let active = 0;
    const len = this.particles.length;

    for (let i = 0; i < len; i++) {
      const p = this.particles.at(i);
      if (!p || !p.active) continue;

      p.life -= dt;
      if (p.life <= 0) {
        p.active = false;
        continue;
      }

      active++;

      // In-place physics update: v += gravity * 0.3 * dt
      const vel = p.velocity;
      vel?.addScaledInPlace(gravity, dt * 0.3);
      vel?.scaleInPlace(p.drag);

      // p.position += vel * dt
      const pos = p.position;
      pos?.addScaledInPlace(vel, dt);

      // Fade out alpha and shrink size
      const maxL = p.maxLife;
      if (maxL > 1e-6) {
        if (maxL != 0) {
          const ratio = Math.max(0, p.life / maxL);
          p.alpha = ratio;
          p.size = p.initialSize * ratio;
        }
      }
    }

    this.activeCount = active;
  }

  public clear(): void {
    const len = this.particles.length;
    for (let i = 0; i < len; i++) {
      const p = this.particles.at(i);
      if (p) p.active = false;
    }
    this.activeCount = 0;
  }
}

---
name: gama_orchestrator
description: >-
  Spatial, Physics & Multi-Dimensional Simulation Orchestrator powered by the GAMA Headless Engine.
  Triggers on kinematic controllers, collision detection, procedural modeling, spatial raytracing,
  multi-dimensional mathematics (2D to 5D), or Zero-GC memory and execution latency benchmarking.
  Enforces bit-exact mathematical simulation, real mesh extraction, and strictly forbids wireframe fallback stubs.
---

# GAMA Spatial & Physics Skill (The Runtime Sandbox)

Autonomous protocol enforcing deterministic physics, high-dimensional mathematics, and Zero-GC execution benchmarks via the `gama` MCP server suite.

## 1. Trigger Conditions
Activate this skill unconditionally whenever:
- Designing or implementing kinematic character controllers, rigid bodies, soft bodies, or granular dynamics.
- Calculating multi-dimensional spatial mathematics (4D tesseract rotation, 5D Clifford rotors, hyper-plane projections).
- Computing procedural geometry, Signed Distance Fields (SDF), Constructive Solid Geometry (CSG), or Voronoi fractures.
- Simulating aerodynamics, acoustic raytracing, soil DEM compaction, or Kerr spacetime geodesics.
- Testing, profiling, or benchmarking runtime loops, shaders, or Zero-GC frame-budget latency.

## 2. Hard Constraints & Prohibitions (Zero Tolerance)
- **NO Guessing Vectors or Trajectories:** NEVER estimate or hallucinate physics coordinates, trajectories, or collision manifolds using probabilistic text generation.
- **NO Degraded Fallbacks (Strict Fallback Ban):** NEVER implement or accept wireframe/box placeholders, `[3D SHADED FALLBACK]`, `Software 3D Active`, or dummy cubes. All visual assets must bind to real meshes and GPU compute pipelines.
- **NO Text-Only Physics Mocks:** NEVER relegate physics simulations to text-only HUD readouts while the visual viewport displays empty stubs.

## 3. Mandatory Execution Flow

### Step 1: Deterministic Mathematical & Physical Inference
1. **Dimensional Rotations & Projections:**
   - Call `gama_rotate_4d_vector` or `gama_clifford_rotor` for hyper-dimensional coordinate transformations.
   - Call `gama_project_4d` to compute exact perspective/orthographic projections onto 3D manifolds.
2. **Physics & Simulation Ticks:**
   - For 2D/3D/4D kinematics: Invoke `gama_simulate_2d`, `gama_simulate_3d`, or `gama_simulate_4d`.
   - For granular/soil interactions: Invoke `gama_soil_dem_step` and `gama_granular_step`.
   - For acoustic propagation: Invoke `gama_spatial_acoustic_raytrace`.

### Step 2: Procedural Mesh & Asset Synthesis
1. **SDF & CSG Pipeline:**
   - Create mathematical primitives via `forge_create_sdf_primitive`.
   - Apply boolean operators via `forge_apply_csg` or `forge_apply_operator`.
2. **Manifold Extraction:**
   - Extract real watertight polygonal meshes via `forge_extract_mesh`.
   - Validate 2-manifold topological integrity via `forge_validate_manifold`.
   - Export geometry to runtime formats via `forge_export_asset`.

### Step 3: Performance & Zero-GC Latency Verification
1. **Benchmark Execution:**
   - Call `gama_benchmark_master` or `gama_hardware_benchmark` to profile execution latency and ensure target frame rates (e.g., 60 FPS / 16.6ms budget).
   - Audit memory allocation to ensure Zero-GC compliance during the main simulation tick.
2. **Viewport Validation:**
   - Call `gama_get_viewport_snapshot` to inspect the visual fidelity of the rendered viewport and verify that true 3D/4D geometry is actively drawn.

## 4. Cross-Engine Verification Pipeline
When building applications combining GAMA and LOCUS:
1. GAMA computes physics and generates geometry assets.
2. LOCUS validates the runtime integration code, ensuring 0 AST invariant violations and verifying that no fallback stubs exist before committing to disk.

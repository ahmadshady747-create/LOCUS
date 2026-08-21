# LOCUS Workspace Guidelines

- **AST Safety First:** Validate code transformations using `locus check <file>` or the native `locus` MCP server (`check_safety`) to prevent concurrency deadlocks (`std::sync::Mutex` across `.await`), panic unwraps, and delimiter imbalances.
- **Context Economy:** Utilize `locus-engine` AST skeletonization to minimize token overhead during multi-file inspection.
- **Zero Unsafe:** Maintain 100% safe Rust standards across all core subsystems.

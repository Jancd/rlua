## ADDED Requirements

### Requirement: Workspace and Runtime Module Topology
The project MUST provide a Rust workspace topology that separates runtime concerns into dedicated crates for core types, parser, compiler, VM, IR, JIT, stdlib, and CLI.

#### Scenario: Workspace crates exist
- **WHEN** a developer runs a workspace metadata inspection
- **THEN** the output includes `rlua-core`, `rlua-parser`, `rlua-compiler`, `rlua-vm`, `rlua-ir`, `rlua-jit`, `rlua-stdlib`, and `rlua-cli`

### Requirement: Lua Value and Bytecode Baseline
The runtime MUST define a baseline value model and bytecode instruction model suitable for interpreter-first execution.

#### Scenario: Value model supports core Lua scalar types
- **WHEN** runtime value constructors are inspected and unit tested
- **THEN** values for nil, boolean, number, and string are representable and comparable according to baseline semantics

#### Scenario: Opcode table is macro-driven
- **WHEN** opcode declarations are added or modified in the single source macro table
- **THEN** enum declarations and helper metadata update without duplicate manual edits

### Requirement: Interpreter Correctness Baseline
The VM MUST execute bytecode with a correctness-first interpreter path that remains available regardless of JIT state.

#### Scenario: JIT is disabled
- **WHEN** CLI is run in interpreter-only mode
- **THEN** bytecode execution still proceeds through VM interpreter path without JIT dependency

### Requirement: Garbage Collection Foundation
The runtime MUST provide a non-moving mark-sweep garbage collection foundation and explicit root ownership boundaries.

#### Scenario: Root sets are explicit
- **WHEN** GC root scanning is invoked
- **THEN** stack/registers, globals, open upvalues, and JIT metadata roots are scanned through explicit APIs

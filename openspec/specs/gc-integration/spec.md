## Capability: gc-integration

Mark-sweep GC wired into the VM allocation path with root traversal, sweep reclamation, and configurable collection thresholds.

### Requirement: Allocation Tracking Through VM

Every heap allocation (table creation, string interning, closure creation) notifies the GC via `notify_alloc()`. When the allocation count exceeds the configurable threshold, a collection cycle is triggered.

#### Scenario: Table allocation triggers GC notification

WHEN a new table is created via `NEWTABLE` opcode
THEN `MarkSweepGc::notify_alloc()` is called
AND if the threshold is exceeded, a collection cycle runs before continuing execution

#### Scenario: Closure allocation triggers GC notification

WHEN a new closure is created via `CLOSURE` opcode
THEN `MarkSweepGc::notify_alloc()` is called

#### Scenario: String allocation triggers GC notification

WHEN a new string is created (concatenation, substring, etc.)
THEN `MarkSweepGc::notify_alloc()` is called

### Requirement: Mark-Phase Root Traversal

During the mark phase, the collector enumerates all root values from registered `GcRootProvider` implementations and transitively marks all reachable objects.

#### Scenario: Stack roots scanned

WHEN a GC collection cycle begins
THEN all values currently on the VM stack are enumerated as roots via `GcRootProvider`

#### Scenario: Global table roots scanned

WHEN a GC collection cycle begins
THEN the global environment table and all its transitive contents are enumerated as roots

#### Scenario: Open upvalue roots scanned

WHEN a GC collection cycle begins
THEN all open upvalues (not yet closed into closures) are enumerated as roots

#### Scenario: Transitive marking

WHEN a root value is a table containing other tables, closures, or strings
THEN all transitively reachable objects are marked as live

### Requirement: Sweep Phase Reclamation

After marking, the sweep phase identifies unmarked objects. In M2 (with `Rc<RefCell<>>` still in use), sweep phase tracks statistics but does not perform actual deallocation — `Rc` reference counting handles that.

#### Scenario: Sweep phase completes

WHEN the mark phase finishes
THEN the sweep phase runs and transitions back to `GcPhase::Idle`
AND `GcStats` reports the number of roots scanned and cycle count

#### Scenario: Allocation counter reset after collection

WHEN a collection cycle completes
THEN the allocation counter is reset to zero
AND the next cycle triggers after `alloc_threshold` new allocations

### Requirement: GC Safepoints

The VM inserts GC safepoints at function calls and loop back-edges to ensure timely collection.

#### Scenario: Safepoint at function call

WHEN the VM executes a CALL or TAILCALL opcode
THEN it checks whether a GC cycle is pending and runs it if so

#### Scenario: Safepoint at loop back-edge

WHEN the VM executes a backward jump (loop iteration)
THEN it checks whether a GC cycle is pending and runs it if so

### Requirement: Configurable Collection Thresholds

The GC threshold can be configured to control collection frequency.

#### Scenario: Default threshold

WHEN the VM starts with default settings
THEN the GC allocation threshold is 256

#### Scenario: Threshold modification

WHEN `MarkSweepGc::set_threshold(n)` is called
THEN subsequent collection cycles trigger after `n` allocations

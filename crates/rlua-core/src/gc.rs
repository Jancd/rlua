use crate::value::LuaValue;

/// Root source categories for GC scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSource {
    /// VM stack/register values.
    Stack,
    /// Global table entries.
    Globals,
    /// Open upvalues (not yet closed into closures).
    OpenUpvalues,
    /// JIT metadata roots (compiled traces, guard maps).
    JitMetadata,
}

/// A single GC root entry: a live reference the collector must not reclaim.
#[derive(Debug, Clone)]
pub struct GcRoot {
    pub source: RootSource,
    pub value: LuaValue,
}

/// Trait for types that can enumerate their GC roots.
///
/// Implementors provide an explicit list of live values that the garbage
/// collector must treat as reachable. This covers the spec requirement for
/// "explicit root ownership boundaries" — each subsystem (VM stack, globals,
/// upvalues, JIT) implements this trait to declare what it keeps alive.
pub trait GcRootProvider {
    /// Append all live root values to `roots`.
    fn gc_roots(&self, roots: &mut Vec<GcRoot>);
}

/// GC phase for the non-moving mark-sweep collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcPhase {
    /// Idle — no collection in progress.
    Idle,
    /// Mark phase — tracing reachable objects from roots.
    Mark,
    /// Sweep phase — reclaiming unmarked objects.
    Sweep,
}

/// Non-moving mark-sweep garbage collector foundation.
///
/// M1 provides the interface and phase skeleton. Actual object tracking and
/// reclamation will be implemented in M2 when `Rc<RefCell<>>` is replaced
/// with GC-managed heap objects.
#[derive(Debug)]
pub struct MarkSweepGc {
    phase: GcPhase,
    /// Number of allocations since last collection.
    alloc_count: usize,
    /// Allocation threshold that triggers a collection cycle.
    alloc_threshold: usize,
    /// Total number of completed collection cycles.
    cycle_count: u64,
}

impl MarkSweepGc {
    pub fn new() -> Self {
        Self {
            phase: GcPhase::Idle,
            alloc_count: 0,
            alloc_threshold: 256,
            cycle_count: 0,
        }
    }

    /// Notify the GC that an allocation occurred.
    /// Returns `true` if a collection cycle should be triggered.
    pub fn notify_alloc(&mut self) -> bool {
        self.alloc_count += 1;
        self.alloc_count >= self.alloc_threshold
    }

    /// Reset allocation counter (called after a collection cycle).
    pub fn reset_alloc_count(&mut self) {
        self.alloc_count = 0;
    }

    /// Set the allocation threshold for triggering collections.
    pub fn set_threshold(&mut self, threshold: usize) {
        self.alloc_threshold = threshold;
    }

    pub fn threshold(&self) -> usize {
        self.alloc_threshold
    }

    pub fn alloc_count(&self) -> usize {
        self.alloc_count
    }

    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    pub fn phase(&self) -> GcPhase {
        self.phase
    }

    /// Run a full mark-sweep collection cycle.
    ///
    /// `root_providers` supplies the root sets to scan. In M1 this performs
    /// the phase transitions and root enumeration but does not reclaim memory
    /// (objects are still managed by `Rc`). M2 will add actual heap tracking.
    pub fn collect(&mut self, root_providers: &[&dyn GcRootProvider]) -> GcStats {
        // Mark phase: enumerate all roots
        self.phase = GcPhase::Mark;
        let mut roots = Vec::new();
        for provider in root_providers {
            provider.gc_roots(&mut roots);
        }

        let root_count = roots.len();

        // Sweep phase: in M1 this is a no-op since Rc manages lifetimes.
        // M2 will iterate the heap and free unmarked objects.
        self.phase = GcPhase::Sweep;

        // Return to idle
        self.phase = GcPhase::Idle;
        self.cycle_count += 1;
        self.reset_alloc_count();

        GcStats {
            roots_scanned: root_count,
            cycle: self.cycle_count,
        }
    }
}

impl Default for MarkSweepGc {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics returned from a GC collection cycle.
#[derive(Debug, Clone, Copy)]
pub struct GcStats {
    /// Number of root values scanned.
    pub roots_scanned: usize,
    /// Which cycle this was (monotonically increasing).
    pub cycle: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyRoots(Vec<LuaValue>);

    impl GcRootProvider for DummyRoots {
        fn gc_roots(&self, roots: &mut Vec<GcRoot>) {
            for val in &self.0 {
                roots.push(GcRoot {
                    source: RootSource::Stack,
                    value: val.clone(),
                });
            }
        }
    }

    #[test]
    fn gc_phase_lifecycle() {
        let mut gc = MarkSweepGc::new();
        assert_eq!(gc.phase(), GcPhase::Idle);
        assert_eq!(gc.cycle_count(), 0);

        let roots = DummyRoots(vec![LuaValue::Number(1.0), LuaValue::from("hello")]);
        let stats = gc.collect(&[&roots]);

        assert_eq!(gc.phase(), GcPhase::Idle);
        assert_eq!(stats.roots_scanned, 2);
        assert_eq!(stats.cycle, 1);
        assert_eq!(gc.cycle_count(), 1);
    }

    #[test]
    fn alloc_threshold() {
        let mut gc = MarkSweepGc::new();
        gc.set_threshold(3);

        assert!(!gc.notify_alloc()); // 1
        assert!(!gc.notify_alloc()); // 2
        assert!(gc.notify_alloc()); // 3 — triggers

        gc.reset_alloc_count();
        assert_eq!(gc.alloc_count(), 0);
    }

    #[test]
    fn collect_with_no_roots() {
        let mut gc = MarkSweepGc::new();
        let stats = gc.collect(&[]);
        assert_eq!(stats.roots_scanned, 0);
        assert_eq!(stats.cycle, 1);
    }

    #[test]
    fn collect_multiple_providers() {
        let mut gc = MarkSweepGc::new();
        let stack_roots = DummyRoots(vec![LuaValue::Number(1.0)]);
        let global_roots = DummyRoots(vec![LuaValue::from("g1"), LuaValue::from("g2")]);

        let stats = gc.collect(&[&stack_roots, &global_roots]);
        assert_eq!(stats.roots_scanned, 3);
        assert_eq!(stats.cycle, 1);

        let stats2 = gc.collect(&[&stack_roots]);
        assert_eq!(stats2.roots_scanned, 1);
        assert_eq!(stats2.cycle, 2);
    }

    #[test]
    fn root_sources() {
        let mut roots = Vec::new();
        let provider = DummyRoots(vec![LuaValue::Nil]);
        provider.gc_roots(&mut roots);
        assert_eq!(roots[0].source, RootSource::Stack);
    }
}

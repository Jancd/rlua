use std::collections::HashSet;
use std::rc::Rc;

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
    /// `root_providers` supplies the root sets to scan. The collector
    /// transitively walks table fields and closure upvalues to count all
    /// reachable objects. Since objects are still managed by `Rc<RefCell<>>`,
    /// the sweep phase doesn't actually free memory — `Rc` handles that.
    /// This is a foundation for a future real GC heap.
    pub fn collect(&mut self, root_providers: &[&dyn GcRootProvider]) -> GcStats {
        // Mark phase: enumerate all roots
        self.phase = GcPhase::Mark;
        let mut roots = Vec::new();
        for provider in root_providers {
            provider.gc_roots(&mut roots);
        }

        let root_count = roots.len();

        // Transitive marking: walk table fields, closure upvalues, and function protos.
        // Use pointer-based identity (Rc raw pointer) to avoid revisiting.
        let mut visited_tables: HashSet<usize> = HashSet::new();
        let mut visited_closures: HashSet<usize> = HashSet::new();
        let mut work: Vec<LuaValue> = roots.iter().map(|r| r.value.clone()).collect();
        let mut reachable_count: usize = 0;

        while let Some(val) = work.pop() {
            match &val {
                LuaValue::Table(t) => {
                    let ptr = Rc::as_ptr(t) as usize;
                    if visited_tables.insert(ptr) {
                        reachable_count += 1;
                        let borrowed = t.borrow();
                        // Walk all key-value pairs
                        for (k, v) in borrowed.iter_pairs() {
                            work.push(k);
                            work.push(v);
                        }
                        // Walk metatable if present
                        if let Some(mt) = borrowed.metatable() {
                            work.push(LuaValue::Table(mt.clone()));
                        }
                    }
                }
                LuaValue::Function(f) => {
                    let ptr = Rc::as_ptr(f) as usize;
                    if visited_closures.insert(ptr) {
                        reachable_count += 1;
                        if let crate::function::LuaFunction::Lua(closure) = f.as_ref() {
                            for uv in &closure.upvalues {
                                work.push(uv.borrow().clone());
                            }
                        }
                    }
                }
                LuaValue::String(_) => {
                    reachable_count += 1;
                }
                LuaValue::Thread(_) => {
                    reachable_count += 1;
                }
                _ => {}
            }
        }

        // Sweep phase: in M2 this is still a no-op since Rc manages lifetimes.
        // A future milestone will replace Rc with GC-managed heap objects.
        self.phase = GcPhase::Sweep;

        // Return to idle
        self.phase = GcPhase::Idle;
        self.cycle_count += 1;
        self.reset_alloc_count();

        GcStats {
            roots_scanned: root_count,
            reachable_objects: reachable_count,
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
    /// Number of unique reachable objects found during transitive marking.
    pub reachable_objects: usize,
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

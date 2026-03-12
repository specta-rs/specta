use std::{
    collections::{HashMap, HashSet, hash_map},
    fmt,
};

use crate::{
    CircularReference, Type,
    datatype::{DataType, Fields, NamedDataType, NamedId, Reference},
};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone)]
pub struct Types(
    // `None` indicates that the entry is a placeholder.
    // It is a reference and we are currently resolving it's definition.
    pub(crate) HashMap<NamedId, Option<NamedDataType>>,
    // The count of non-`None` items in the collection.
    // We store this to avoid expensive iteration.
    pub(crate) usize,
);

/// A wrapper around [`Types`] indicating the type graph has already been
/// transformed for a specific export format.
///
/// This is generally constructed by a format crate (for example
/// [`specta-serde`](https://docs.rs/specta-serde)) after applying
/// format-specific rewrites.
///
/// Constructing this wrapper from plain [`Types`] is explicit because the
/// conversion may change type shapes. Prefer using your format crate's
/// conversion entry points when possible.
#[derive(Debug, Clone)]
pub struct ResolvedTypes(Types);

impl fmt::Debug for Types {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Types").field(&self.0).finish()
    }
}

impl Types {
    /// Register a [`Type`] with the collection.
    pub fn register<T: Type>(mut self) -> Self {
        T::definition(&mut self);
        self
    }

    /// Register a [`Type`](crate::Type) with the collection.
    pub fn register_mut<T: Type>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Get the length of the collection.
    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.1,
            self.0.iter().filter_map(|(_, ndt)| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.1
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Merge types from another collection into this one.
    pub fn merge(&mut self, other: &Self) {
        for (id, other) in &other.0 {
            match self.0.get(id) {
                // Key doesn't exist - insert from other
                None => {
                    if other.is_some() {
                        self.1 += 1;
                    }
                    self.0.insert(id.clone(), other.clone());
                }
                // Key exists with Some - keep self (prefer self over other)
                Some(Some(_)) => {}
                // Key exists with None, but other has Some - use other (prefer Some over None)
                Some(None) if other.is_some() => {
                    self.1 += 1;
                    self.0.insert(id.clone(), other.clone());
                }
                // Key exists with None, other also None - do nothing
                Some(None) => {}
            }
        }
    }

    /// Sort the collection into a consistent order and return an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        let mut v = self
            .0
            .iter()
            .filter_map(|(_, ndt)| ndt.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(v.len(), self.1, "Types count logic mismatch");
        v.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        v.into_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl ExactSizeIterator<Item = &NamedDataType> {
        UnsortedIter {
            iter: self.0.iter(),
            count: self.1,
        }
    }

    /// Topologically sort the collection (dependencies before dependents).
    ///
    /// Uses a recursive DFS alg (vs BFS / Kahn's) as it has a lower memory footprint for wide (vs deep) trees.
    /// Types with no dependency relationships are emitted in an unspecified but deterministic order.
    ///
    /// Returns the same iterator type as [Self::into_sorted_iter] or [Self::into_unsorted_iter],
    /// but wrapped in a Result to handle [CircularReference] cases (e.g. A -> B -> C -> A).
    ///
    /// NOTE: Internal `deps` and `visit` functions handle the traversal for us based on the DataType.
    /// This could be extract to a more general solution in the future.
    pub fn into_topological_iter(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = &'_ NamedDataType>, CircularReference> {
        // Walk a DataType and collect every directly-referenced named type into `out`.
        fn deps<'a>(dt: &'a DataType, types: &'a Types, res: &mut Vec<&'a NamedDataType>) {
            let field_ty = |fields: &'a Fields| fields.values().flat_map(|f| f.ty());

            match dt {
                DataType::Primitive(_) => {}
                DataType::Nullable(val) => deps(val, types, res),
                DataType::List(l) => deps(l.ty(), types, res),
                DataType::Tuple(t) => t.elements().iter().for_each(|e| deps(e, types, res)),
                DataType::Struct(s) => field_ty(s.fields()).for_each(|ty| deps(ty, types, res)),
                DataType::Map(m) => {
                    deps(m.key_ty(), types, res);
                    deps(m.value_ty(), types, res);
                }
                DataType::Enum(e) => e
                    .variants()
                    .iter()
                    .flat_map(|(_, v)| field_ty(v.fields()))
                    .for_each(|ty| deps(ty, types, res)),
                DataType::Reference(Reference::Named(n)) => {
                    res.extend(n.get(types));
                    n.generics().iter().for_each(|(_, g)| deps(g, types, res));
                }
                DataType::Reference(_) => {}
            }
        }

        fn visit<'a>(
            curr: &'a NamedDataType,
            types: &'a Types,
            visited: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>,
            res: &mut Vec<&'a NamedDataType>,
        ) -> Result<(), CircularReference> {
            let name: &'a str = curr.name().as_ref();

            // Early exit if we've already processed this node
            if visited.contains(name) {
                return Ok(());
            }

            // Detect and extract cycle paths for clear error message
            if path.contains(&name) {
                let i = path.iter().position(|&n| n == name).unwrap_or(0);
                let last = std::iter::once(name.to_string());
                let cycle = path[i..].iter().map(|s| s.to_string()).chain(last);
                return Err(CircularReference::new(cycle.collect()));
            }

            // Add curr to path and process dependencies
            path.push(name);
            let mut dependencies = Vec::new();
            deps(curr.ty(), types, &mut dependencies);
            dependencies
                .into_iter()
                .try_for_each(|dep| visit(dep, types, visited, path, res))?;

            // Remove curr from path, mark it as visited and add it to the result
            path.pop();
            visited.insert(name);
            res.push(curr);

            Ok(())
        }

        let mut visited: HashSet<&str> = HashSet::new();
        let mut path: Vec<&str> = Vec::new();
        let mut res = Vec::with_capacity(self.len());

        // Need to pre-sort collection for deterministic output.
        self.into_sorted_iter()
            .try_for_each(|n| visit(n, self, &mut visited, &mut path, &mut res))?;

        Ok(res.into_iter())
    }

    /// Return an mutable iterator over the type collection.
    /// Note: The order returned is unsorted.
    pub fn iter_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        for (_, ndt) in self.0.iter_mut() {
            if let Some(ndt) = ndt {
                f(ndt);
            }
        }
    }

    /// Map over the collection, transforming each `NamedDataType` with the given closure.
    /// This preserves the `ArcId` keys, ensuring that `Reference`s remain valid.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for (_, slot) in self.0.iter_mut() {
            if let Some(ndt) = slot.take() {
                *slot = Some(f(ndt));
            }
        }
        self
    }
}

impl ResolvedTypes {
    /// Wrap already-resolved [`Types`] as [`ResolvedTypes`].
    ///
    /// This should generally be called by format crates after they finish their
    /// own transformation pass (for example `specta_serde::apply` or
    /// `specta_serde::apply_phases`).
    ///
    /// If you call this in end-user code your types may not look how you expect!
    pub fn from_resolved_types(types: Types) -> Self {
        Self(types)
    }

    /// Borrow the underlying [`Types`] collection.
    ///
    /// # Notes
    ///
    /// This does not undo format-specific resolution. If a format crate already
    /// rewrote type shapes, this still returns those rewritten shapes. It is your
    /// responsibility to ensure consumers treat these as already-resolved types.
    pub fn as_types(&self) -> &Types {
        &self.0
    }

    /// Consume [`ResolvedTypes`] and return the underlying [`Types`].
    ///
    /// # Notes
    ///
    /// This does not undo format-specific resolution. The returned [`Types`]
    /// remain whatever shape they were resolved into.
    pub fn into_types(self) -> Types {
        self.0
    }
}

struct UnsortedIter<'a> {
    iter: hash_map::Iter<'a, NamedId, Option<NamedDataType>>,
    count: usize,
}

impl<'a> Iterator for UnsortedIter<'a> {
    type Item = &'a NamedDataType;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, ndt)| ndt.as_ref())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl ExactSizeIterator for UnsortedIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatype::{
        DataType, Field, NamedDataType, NamedFields, Primitive, StructBuilder,
    };

    fn prim() -> DataType {
        DataType::Primitive(Primitive::str)
    }

    fn named(name: &'static str, ty: DataType, types: &mut Types) -> NamedDataType {
        let ndt = NamedDataType::new(name, vec![], ty);
        ndt.register(types);
        ndt
    }

    fn names<'a>(result: impl Iterator<Item = &'a NamedDataType>) -> Vec<&'a str> {
        result.map(|ndt| ndt.name().as_ref()).collect()
    }

    fn pos(ns: &[&str], name: &str) -> usize {
        ns.iter().position(|&x| x == name).unwrap()
    }

    #[test]
    fn empty_collection() {
        let types = Types::default();
        assert_eq!(types.into_topological_iter().unwrap().len(), 0);
    }

    #[test]
    fn single_type_no_deps() {
        let mut types = Types::default();
        named("Standalone", prim(), &mut types);
        assert_eq!(names(types.into_topological_iter().unwrap()), [
            "Standalone"
        ]);
    }

    #[test]
    fn linear_chain() {
        // Leaf <- Mid <- Root; expected: Leaf before Mid before Root.
        let mut types = Types::default();
        let leaf = named("Leaf", prim(), &mut types);
        let mid = named("Mid", leaf.reference(vec![]).into(), &mut types);
        named("Root", mid.reference(vec![]).into(), &mut types);
        let ns = names(types.into_topological_iter().unwrap());
        assert!(pos(&ns, "Leaf") < pos(&ns, "Mid"));
        assert!(pos(&ns, "Mid") < pos(&ns, "Root"));
    }

    #[test]
    fn diamond_dependency() {
        // Bottom <- Left, Right <- Top; Bottom must be first, Top last.
        let mut types = Types::default();
        let bottom = named("Bottom", prim(), &mut types);
        let left = named("Left", bottom.reference(vec![]).into(), &mut types);
        let right = named("Right", bottom.reference(vec![]).into(), &mut types);
        let top_ty = StructBuilder {
            fields: NamedFields {
                fields: vec![],
            },
        }
        .field("a", Field::new(left.reference(vec![]).into()))
        .field("b", Field::new(right.reference(vec![]).into()))
        .build();
        named("Top", top_ty, &mut types);
        let ns = names(types.into_topological_iter().unwrap());
        assert!(pos(&ns, "Bottom") < pos(&ns, "Left"));
        assert!(pos(&ns, "Bottom") < pos(&ns, "Right"));
        assert!(pos(&ns, "Left") < pos(&ns, "Top"));
        assert!(pos(&ns, "Right") < pos(&ns, "Top"));
    }

    #[test]
    fn multiple_valid_orderings() {
        // Left and Right both depend on Base but not on each other.
        // [Base, Left, Right] and [Base, Right, Left] are both valid; we only
        // assert the ordering constraint, not the exact sequence.
        let mut types = Types::default();
        let base = named("Base", prim(), &mut types);
        named("Left", base.reference(vec![]).into(), &mut types);
        named("Right", base.reference(vec![]).into(), &mut types);
        let ns = names(types.into_topological_iter().unwrap());
        assert_eq!(ns.len(), 3);
        assert!(pos(&ns, "Base") < pos(&ns, "Left"));
        assert!(pos(&ns, "Base") < pos(&ns, "Right"));
    }

    #[test]
    fn disconnected_types_both_present() {
        // Two unrelated types: both must appear regardless of iteration order.
        let mut types = Types::default();
        named("A", prim(), &mut types);
        named("B", prim(), &mut types);
        let ns = names(types.into_topological_iter().unwrap());
        assert_eq!(ns.len(), 2);
        assert!(ns.contains(&"A"));
        assert!(ns.contains(&"B"));
    }

    #[test]
    fn zero_in_degree_sources_included() {
        // Sources (in-degree 0, i.e. nothing depends on them) must still appear.
        // Root and Orphan are both sources; Leaf is a sink.
        let mut types = Types::default();
        let leaf = named("Leaf", prim(), &mut types);
        named("Root", leaf.reference(vec![]).into(), &mut types);
        named("Orphan", prim(), &mut types); // source with no relations at all
        let ns = names(types.into_topological_iter().unwrap());
        assert_eq!(ns.len(), 3);
        assert!(ns.contains(&"Root"));
        assert!(ns.contains(&"Orphan"));
        assert!(pos(&ns, "Leaf") < pos(&ns, "Root"));
    }

    #[test]
    fn zero_out_degree_sinks_come_first() {
        // Sinks (out-degree 0, i.e. no dependencies) must precede their dependents.
        let mut types = Types::default();
        let sink_a = named("SinkA", prim(), &mut types);
        let sink_b = named("SinkB", prim(), &mut types);
        let top_ty = StructBuilder {
            fields: NamedFields {
                fields: vec![],
            },
        }
        .field("a", Field::new(sink_a.reference(vec![]).into()))
        .field("b", Field::new(sink_b.reference(vec![]).into()))
        .build();
        named("Top", top_ty, &mut types);
        let ns = names(types.into_topological_iter().unwrap());
        assert!(pos(&ns, "SinkA") < pos(&ns, "Top"));
        assert!(pos(&ns, "SinkB") < pos(&ns, "Top"));
    }

    #[test]
    fn self_cycle_returns_err() {
        let mut types = Types::default();
        let srt = named("SelfRef", prim(), &mut types);
        types.0.get_mut(&srt.id).unwrap().as_mut().unwrap().inner = srt.reference(vec![]).into();
        let Err(err) = types.into_topological_iter() else {
            panic!("expected cycle error")
        };
        assert_eq!(
            err.cycle().first(),
            err.cycle().last(),
            "cycle should be closed"
        );
        assert!(err.cycle().iter().any(|s| s == "SelfRef"), "got: {err:?}");
    }

    #[test]
    fn multi_step_cycle_returns_err() {
        // A -> B -> C -> A
        let mut types = Types::default();
        let a = named("A", prim(), &mut types);
        let b = named("B", prim(), &mut types);
        let c = named("C", prim(), &mut types);
        types.0.get_mut(&a.id).unwrap().as_mut().unwrap().inner = b.reference(vec![]).into();
        types.0.get_mut(&b.id).unwrap().as_mut().unwrap().inner = c.reference(vec![]).into();
        types.0.get_mut(&c.id).unwrap().as_mut().unwrap().inner = a.reference(vec![]).into();
        let Err(err) = types.into_topological_iter() else {
            panic!("expected cycle error")
        };
        assert_eq!(
            err.cycle().first(),
            err.cycle().last(),
            "cycle should be closed"
        );
        assert!(
            err.cycle().len() >= 3,
            "cycle should span all members, got: {err:?}"
        );
    }
}

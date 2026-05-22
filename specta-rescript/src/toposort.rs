use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Fields, NamedDataType, NamedReferenceType, Reference},
};

use crate::error::{Error, Result};

/// Collect all directly-referenced named types from a `DataType` into `out`.
fn deps<'a>(dt: &'a DataType, types: &'a Types, out: &mut Vec<&'a NamedDataType>) {
    match dt {
        DataType::Primitive(_) | DataType::Generic(_) => {}
        DataType::Nullable(inner) => deps(inner, types, out),
        DataType::List(l) => deps(&l.ty, types, out),
        DataType::Tuple(t) => t.elements.iter().for_each(|e| deps(e, types, out)),
        DataType::Struct(s) => match &s.fields {
            Fields::Unit => {}
            Fields::Unnamed(uf) => uf.fields.iter().filter_map(|f| f.ty.as_ref()).for_each(|ty| deps(ty, types, out)),
            Fields::Named(nf) => nf.fields.iter().filter_map(|(_, f)| f.ty.as_ref()).for_each(|ty| deps(ty, types, out)),
        },
        DataType::Map(m) => {
            deps(m.key_ty(), types, out);
            deps(m.value_ty(), types, out);
        }
        DataType::Enum(e) => {
            for (_, v) in &e.variants {
                match &v.fields {
                    Fields::Unit => {}
                    Fields::Unnamed(uf) => uf.fields.iter().filter_map(|f| f.ty.as_ref()).for_each(|ty| deps(ty, types, out)),
                    Fields::Named(nf) => nf.fields.iter().filter_map(|(_, f)| f.ty.as_ref()).for_each(|ty| deps(ty, types, out)),
                }
            }
        }
        DataType::Reference(Reference::Named(n)) => match &n.inner {
            NamedReferenceType::Reference { generics, .. } => {
                out.extend(types.get(n));
                generics.iter().for_each(|(_, g)| deps(g, types, out));
            }
            NamedReferenceType::Inline { dt, .. } => deps(dt, types, out),
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Reference(_) | DataType::Intersection(_) => {}
    }
}

/// DFS visitor for topological sort.
fn visit<'a>(
    curr: &'a NamedDataType,
    types: &'a Types,
    visited: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
    res: &mut Vec<&'a NamedDataType>,
) -> Result<()> {
    let name: &'a str = curr.name.as_ref();

    if visited.contains(name) {
        return Ok(());
    }

    if path.contains(&name) {
        let i = path.iter().position(|&n| n == name).unwrap_or(0);
        let cycle = path[i..].iter().map(|s| s.to_string()).chain(std::iter::once(name.to_string())).collect::<Vec<_>>();
        return Err(Error::TopoSort(format!("circular reference: {}", cycle.join(" -> "))));
    }

    path.push(name);
    let mut dependencies = Vec::new();
    if let Some(ty) = &curr.ty {
        deps(ty, types, &mut dependencies);
    }
    dependencies.into_iter().try_for_each(|dep| visit(dep, types, visited, path, res))?;
    path.pop();
    visited.insert(name);
    res.push(curr);

    Ok(())
}

/// Sort named types in topological dependency order (dependencies before dependents).
/// Falls back to alphabetical within the same depth for deterministic output.
pub(crate) fn topological_sort(types: &Types) -> Result<Vec<&NamedDataType>> {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();
    let mut res = Vec::with_capacity(types.len());

    types.into_sorted_iter().try_for_each(|n| visit(n, types, &mut visited, &mut path, &mut res))?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::topological_sort;
    use specta::{Type, Types};

    fn sorted_names(types: &Types) -> Vec<&str> {
        topological_sort(types)
            .unwrap()
            .into_iter()
            .map(|n| n.name.as_ref())
            .collect()
    }

    fn pos(names: &[&str], target: &str) -> usize {
        names.iter().position(|&n| n == target).unwrap()
    }

    #[derive(Type)] struct Leaf { _value: i32 }
    #[derive(Type)] struct Mid { _leaf: Leaf }
    #[derive(Type)] struct Root { _mid: Mid }

    #[test]
    fn linear_chain_ordered() {
        let types = Types::default().register::<Root>();
        let names = sorted_names(&types);
        assert!(pos(&names, "Leaf") < pos(&names, "Mid"));
        assert!(pos(&names, "Mid") < pos(&names, "Root"));
    }

    #[derive(Type)] struct Bottom { _value: i32 }
    #[derive(Type)] struct Left { _bottom: Bottom }
    #[derive(Type)] struct Right { _bottom: Bottom }
    #[derive(Type)] struct Top { _left: Left, _right: Right }

    #[test]
    fn diamond_dependency_ordered() {
        let types = Types::default().register::<Top>();
        let names = sorted_names(&types);
        assert!(pos(&names, "Bottom") < pos(&names, "Left"));
        assert!(pos(&names, "Bottom") < pos(&names, "Right"));
        assert!(pos(&names, "Left") < pos(&names, "Top"));
        assert!(pos(&names, "Right") < pos(&names, "Top"));
    }

    #[derive(Type)] struct Orphan { _value: String }

    #[test]
    fn disconnected_types_all_present() {
        let types = Types::default().register::<Leaf>().register::<Orphan>();
        let names = sorted_names(&types);
        assert!(names.contains(&"Leaf"));
        assert!(names.contains(&"Orphan"));
    }
}

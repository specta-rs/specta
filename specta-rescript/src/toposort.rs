use std::collections::HashMap;

use specta::{
    Types,
    datatype::{DataType, Fields, NamedDataType, NamedReferenceType, Reference},
};

fn deps<'a>(dt: &'a DataType, types: &'a Types, out: &mut Vec<&'a NamedDataType>) {
    match dt {
        DataType::Primitive(_) | DataType::Generic(_) => {}
        DataType::Nullable(inner) => deps(inner, types, out),
        DataType::List(list) => deps(&list.ty, types, out),
        DataType::Tuple(tuple) => tuple.elements.iter().for_each(|ty| deps(ty, types, out)),
        DataType::Struct(structure) => deps_fields(&structure.fields, types, out),
        DataType::Map(map) => {
            deps(map.key_ty(), types, out);
            deps(map.value_ty(), types, out);
        }
        DataType::Enum(enumeration) => enumeration
            .variants
            .iter()
            .for_each(|(_, variant)| deps_fields(&variant.fields, types, out)),
        DataType::Reference(Reference::Named(named)) => match &named.inner {
            NamedReferenceType::Reference { generics, .. } => {
                out.extend(types.get(named));
                generics.iter().for_each(|(_, ty)| deps(ty, types, out));
            }
            NamedReferenceType::Inline { dt, .. } => deps(dt, types, out),
            NamedReferenceType::Recursive(recursive) => {
                out.extend(types.get(named));
                recursive
                    .generics()
                    .iter()
                    .for_each(|(_, ty)| deps(ty, types, out));
            }
        },
        DataType::Reference(_) | DataType::Intersection(_) => {}
    }
}

fn deps_fields<'a>(fields: &'a Fields, types: &'a Types, out: &mut Vec<&'a NamedDataType>) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .for_each(|ty| deps(ty, types, out)),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(_, field)| field.ty.as_ref())
            .for_each(|ty| deps(ty, types, out)),
    }
}

pub(crate) fn is_self_recursive(dt: &DataType, types: &Types, ty: &NamedDataType) -> bool {
    let mut dependencies = Vec::new();
    deps(dt, types, &mut dependencies);
    dependencies
        .iter()
        .any(|dependency| dependency.name == ty.name)
}

struct Tarjan<'a> {
    types: &'a Types,
    next_index: usize,
    indices: HashMap<&'a str, usize>,
    lowlinks: HashMap<&'a str, usize>,
    stack: Vec<&'a NamedDataType>,
    on_stack: HashMap<&'a str, bool>,
    groups: Vec<Vec<&'a NamedDataType>>,
}

impl<'a> Tarjan<'a> {
    fn visit(&mut self, ty: &'a NamedDataType) {
        let name = ty.name.as_ref();
        let index = self.next_index;
        self.next_index += 1;
        self.indices.insert(name, index);
        self.lowlinks.insert(name, index);
        self.stack.push(ty);
        self.on_stack.insert(name, true);

        let mut dependencies = Vec::new();
        if let Some(dt) = &ty.ty {
            deps(dt, self.types, &mut dependencies);
        }
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        dependencies.dedup_by(|a, b| a.name == b.name);

        for dependency in dependencies {
            let dependency_name = dependency.name.as_ref();
            if !self.indices.contains_key(dependency_name) {
                self.visit(dependency);
                let dependency_lowlink = self.lowlinks[dependency_name];
                self.lowlinks
                    .entry(name)
                    .and_modify(|lowlink| *lowlink = (*lowlink).min(dependency_lowlink));
            } else if self.on_stack.get(dependency_name) == Some(&true) {
                let dependency_index = self.indices[dependency_name];
                self.lowlinks
                    .entry(name)
                    .and_modify(|lowlink| *lowlink = (*lowlink).min(dependency_index));
            }
        }

        if self.lowlinks[name] == self.indices[name] {
            let mut group = Vec::new();
            loop {
                let member = self
                    .stack
                    .pop()
                    .expect("current type is on the Tarjan stack");
                self.on_stack.insert(member.name.as_ref(), false);
                group.push(member);
                if member.name == ty.name {
                    break;
                }
            }
            group.sort_by(|a, b| a.name.cmp(&b.name));
            self.groups.push(group);
        }
    }
}

/// Sort types into dependency-ordered strongly connected groups.
pub(crate) fn topological_sort(types: &Types) -> Vec<Vec<&NamedDataType>> {
    let mut tarjan = Tarjan {
        types,
        next_index: 0,
        indices: HashMap::new(),
        lowlinks: HashMap::new(),
        stack: Vec::new(),
        on_stack: HashMap::new(),
        groups: Vec::new(),
    };

    for ty in types.into_sorted_iter() {
        if !tarjan.indices.contains_key(ty.name.as_ref()) {
            tarjan.visit(ty);
        }
    }
    tarjan.groups
}

#[cfg(test)]
mod tests {
    use super::topological_sort;
    use specta::{Type, Types};

    #[derive(Type)]
    struct Leaf {
        _value: i32,
    }
    #[derive(Type)]
    struct Root {
        _leaf: Leaf,
    }
    #[derive(Type)]
    struct Node {
        _next: Option<Box<Node>>,
    }

    #[test]
    fn dependencies_precede_dependents() {
        let types = Types::default().register::<Root>();
        let names = topological_sort(&types)
            .into_iter()
            .flatten()
            .map(|ty| ty.name.as_ref())
            .collect::<Vec<_>>();
        assert!(
            names.iter().position(|name| *name == "Leaf")
                < names.iter().position(|name| *name == "Root")
        );
    }

    #[test]
    fn recursive_type_forms_a_group() {
        let types = Types::default().register::<Node>();
        assert!(
            topological_sort(&types)
                .iter()
                .any(|group| group.iter().any(|ty| ty.name == "Node"))
        );
    }
}

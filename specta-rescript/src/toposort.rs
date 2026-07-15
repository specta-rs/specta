use std::collections::HashMap;
use std::ptr;

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
        .any(|dependency| ptr::eq(*dependency, ty))
}

type TypeId = *const NamedDataType;

fn type_id(ty: &NamedDataType) -> TypeId {
    ptr::from_ref(ty)
}

struct Tarjan<'a> {
    types: &'a Types,
    next_index: usize,
    indices: HashMap<TypeId, usize>,
    lowlinks: HashMap<TypeId, usize>,
    stack: Vec<&'a NamedDataType>,
    on_stack: HashMap<TypeId, bool>,
    groups: Vec<Vec<&'a NamedDataType>>,
}

impl<'a> Tarjan<'a> {
    fn visit(&mut self, ty: &'a NamedDataType) {
        let id = type_id(ty);
        let index = self.next_index;
        self.next_index += 1;
        self.indices.insert(id, index);
        self.lowlinks.insert(id, index);
        self.stack.push(ty);
        self.on_stack.insert(id, true);

        let mut dependencies = Vec::new();
        if let Some(dt) = &ty.ty {
            deps(dt, self.types, &mut dependencies);
        }
        dependencies.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        dependencies.dedup_by(|a, b| ptr::eq(*a, *b));

        for dependency in dependencies {
            let dependency_id = type_id(dependency);
            if !self.indices.contains_key(&dependency_id) {
                self.visit(dependency);
                let dependency_lowlink = self.lowlinks[&dependency_id];
                self.lowlinks
                    .entry(id)
                    .and_modify(|lowlink| *lowlink = (*lowlink).min(dependency_lowlink));
            } else if self.on_stack.get(&dependency_id) == Some(&true) {
                let dependency_index = self.indices[&dependency_id];
                self.lowlinks
                    .entry(id)
                    .and_modify(|lowlink| *lowlink = (*lowlink).min(dependency_index));
            }
        }

        if self.lowlinks[&id] == self.indices[&id] {
            let mut group = Vec::new();
            loop {
                let member = self
                    .stack
                    .pop()
                    .expect("current type is on the Tarjan stack");
                self.on_stack.insert(type_id(member), false);
                group.push(member);
                if ptr::eq(member, ty) {
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
        if !tarjan.indices.contains_key(&type_id(ty)) {
            tarjan.visit(ty);
        }
    }
    tarjan.groups
}

#[cfg(test)]
mod tests {
    use super::topological_sort;
    use specta::{
        Type, Types,
        datatype::{NamedDataType, Primitive},
    };

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

    #[test]
    fn duplicate_display_names_keep_distinct_identities() {
        let mut types = Types::default();
        NamedDataType::new("Duplicate", &mut types, |_, ty| {
            ty.ty = Some(Primitive::str.into());
        });
        NamedDataType::new("Duplicate", &mut types, |_, ty| {
            ty.ty = Some(Primitive::i32.into());
        });

        assert_eq!(topological_sort(&types).into_iter().flatten().count(), 2);
    }
}

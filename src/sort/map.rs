use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapContainer {
    do_rebuild_keys: bool,
    do_rebuild_vals: bool,
    pub data: BTreeMap<Value, Value>,
}

impl ContainerValue for MapContainer {
    fn rebuild_contents(&mut self, rebuilder: &dyn Rebuilder) -> bool {
        let mut changed = false;
        if self.do_rebuild_keys {
            self.data = self
                .data
                .iter()
                .map(|(old, v)| {
                    let new = rebuilder.rebuild_val(*old);
                    changed |= *old != new;
                    (new, *v)
                })
                .collect();
        }
        if self.do_rebuild_vals {
            for old in self.data.values_mut() {
                let new = rebuilder.rebuild_val(*old);
                changed |= *old != new;
                *old = new;
            }
        }
        changed
    }
    fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        self.data.iter().flat_map(|(k, v)| [k, v]).copied()
    }
}

/// A map from a key type to a value type supporting these primitives:
/// - `map-empty`
/// - `map-insert`
/// - `map-get`
/// - `map-contains`
/// - `map-not-contains`
/// - `map-remove`
/// - `map-length`
/// - `map-pick-key`
/// - `map-keys`
/// - `map-fold-kv`
/// - `map-filter-kv`
/// - `map-filter-defined-kv`
/// - `map-map-values`
/// - `map-merge-with`
/// - `map-merge-with-swapped`
/// - `map-intersect-with`
/// - `map-drop-zero-values`
/// - `map-bigrat-subtract`
/// - `map-bigrat-intersect-min`
/// - `map-contains-key-swapped`
/// - `map-not-contains-key-swapped`
/// - `map-restrict-keys`
/// - `map-remove-keys`
/// - `map-subtract-bigrat-from-keys`
/// - `map-nonconst-nonunit-f64-values`
/// - `map-divide-all-values-by-f64`
/// - `map-shared-factor-atoms`
#[derive(Clone, Debug)]
pub struct MapSort {
    name: String,
    key: ArcSort,
    value: ArcSort,
}

impl MapSort {
    pub fn key(&self) -> ArcSort {
        self.key.clone()
    }

    pub fn value(&self) -> ArcSort {
        self.value.clone()
    }
}

impl Presort for MapSort {
    fn presort_name() -> &'static str {
        "Map"
    }

    fn reserved_primitives() -> Vec<&'static str> {
        vec![
            "map-empty",
            "map-insert",
            "map-get",
            "map-not-contains",
            "map-contains",
            "map-remove",
            "map-length",
            "map-pick-key",
            "map-keys",
            "map-fold-kv",
            "map-filter-kv",
            "map-filter-defined-kv",
            "map-map-values",
            "map-merge-with",
            "map-merge-with-swapped",
            "map-intersect-with",
            "map-drop-zero-values",
            "map-bigrat-subtract",
            "map-bigrat-intersect-min",
            "map-contains-key-swapped",
            "map-not-contains-key-swapped",
            "map-restrict-keys",
            "map-remove-keys",
            "map-subtract-bigrat-from-keys",
            "map-nonconst-nonunit-f64-values",
            "map-divide-all-values-by-f64",
            "map-shared-factor-atoms",
        ]
    }

    fn make_sort(
        typeinfo: &mut TypeInfo,
        name: String,
        args: &[Expr],
    ) -> Result<ArcSort, TypeError> {
        if let [Expr::Var(k_span, k), Expr::Var(v_span, v)] = args {
            let k = typeinfo
                .get_sort_by_name(k)
                .ok_or(TypeError::UndefinedSort(k.clone(), k_span.clone()))?;
            let v = typeinfo
                .get_sort_by_name(v)
                .ok_or(TypeError::UndefinedSort(v.clone(), v_span.clone()))?;

            let out = Self {
                name,
                key: k.clone(),
                value: v.clone(),
            };
            Ok(out.to_arcsort())
        } else {
            panic!()
        }
    }
}

impl ContainerSort for MapSort {
    type Container = MapContainer;

    fn name(&self) -> &str {
        &self.name
    }

    fn inner_sorts(&self) -> Vec<ArcSort> {
        vec![self.key.clone(), self.value.clone()]
    }

    fn is_eq_container_sort(&self) -> bool {
        self.key.is_eq_sort()
            || self.value.is_eq_sort()
            || self.key.is_eq_container_sort()
            || self.value.is_eq_container_sort()
    }

    fn inner_values(
        &self,
        container_values: &ContainerValues,
        value: Value,
    ) -> Vec<(ArcSort, Value)> {
        let val = container_values
            .get_val::<MapContainer>(value)
            .unwrap()
            .clone();
        val.data
            .iter()
            .flat_map(|(k, v)| [(self.key.clone(), *k), (self.value.clone(), *v)])
            .collect()
    }

    fn register_primitives(&self, eg: &mut EGraph) {
        let arc = self.clone().to_arcsort();

        add_primitive!(eg, "map-empty" = {self.clone(): MapSort} || -> @MapContainer (arc) { MapContainer {
            do_rebuild_keys: self.ctx.key.is_eq_sort() || self.ctx.key.is_eq_container_sort(),
            do_rebuild_vals: self.ctx.value.is_eq_sort() || self.ctx.value.is_eq_container_sort(),
            data: BTreeMap::new()
        } });

        add_primitive!(eg, "map-get"    = |    xs: @MapContainer (arc), x: # (self.key())                     | -?> # (self.value()) { xs.data.get(&x).copied() });
        add_primitive!(eg, "map-insert" = |mut xs: @MapContainer (arc), x: # (self.key()), y: # (self.value())| -> @MapContainer (arc) {{ xs.data.insert(x, y); xs }});
        add_primitive!(eg, "map-remove" = |mut xs: @MapContainer (arc), x: # (self.key())                     | -> @MapContainer (arc) {{ xs.data.remove(&x);   xs }});

        add_primitive!(eg, "map-length"       = |xs: @MapContainer (arc)| -> i64 { xs.data.len() as i64 });
        add_primitive!(eg, "map-pick-key"     = |xs: @MapContainer (arc)| -?> # (self.key()) { xs.data.keys().next().copied() });
        add_primitive!(eg, "map-contains"     = |xs: @MapContainer (arc), x: # (self.key())| -?> () { ( xs.data.contains_key(&x)).then_some(()) });
        add_primitive!(eg, "map-not-contains" = |xs: @MapContainer (arc), x: # (self.key())| -?> () { (!xs.data.contains_key(&x)).then_some(()) });
        add_primitive!(eg, "map-contains-key-swapped" = |x: # (self.key()), xs: @MapContainer (arc)| -?> () { ( xs.data.contains_key(&x)).then_some(()) });
        add_primitive!(eg, "map-not-contains-key-swapped" = |x: # (self.key()), xs: @MapContainer (arc)| -?> () { (!xs.data.contains_key(&x)).then_some(()) });

        if let Some(drop_zero_kind) = DropZeroKind::from_value_sort(self.value()) {
            eg.add_primitive(DropZeroValues {
                name: "map-drop-zero-values".into(),
                map: self.clone().to_arcsort(),
                value_kind: drop_zero_kind,
            });
        }

        if self.value().name() == "f64" {
            eg.add_primitive(DivideAllValuesByF64 {
                name: "map-divide-all-values-by-f64".into(),
                float: self.value(),
                map: self.clone().to_arcsort(),
            });
        }

        if self.value().name() == "BigRat" {
            eg.add_primitive(BigRatSubtract {
                name: "map-bigrat-subtract".into(),
                map: self.clone().to_arcsort(),
            });
            eg.add_primitive(BigRatIntersectMin {
                name: "map-bigrat-intersect-min".into(),
                map: self.clone().to_arcsort(),
            });
        }

        if self.key().value_type() == Some(TypeId::of::<MapContainer>()) {
            let inner_sorts = self.key().inner_sorts();
            if inner_sorts.len() == 2 && inner_sorts[1].name() == "BigRat" {
                eg.add_primitive(SubtractBigRatFromKeys {
                    name: "map-subtract-bigrat-from-keys".into(),
                    outer_map: self.clone().to_arcsort(),
                    inner_map: self.key(),
                });
            }
        }

        for multiset in eg
            .type_info
            .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<MultiSetContainer>()))
        {
            try_registering_map_primitives_for_multiset(
                eg,
                self.clone().to_arcsort(),
                multiset.clone(),
            );
        }
        for set in eg
            .type_info
            .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<SetContainer>()))
        {
            try_registering_map_primitives_for_set(eg, self.clone().to_arcsort(), set.clone());
        }
        for fn_sort in eg.type_info.get_sorts::<FunctionSort>() {
            try_registering_map_primitives_for_function(
                eg,
                fn_sort.clone(),
                self.clone().to_arcsort(),
            );
        }
    }

    fn reconstruct_termdag(
        &self,
        _container_values: &ContainerValues,
        _value: Value,
        termdag: &mut TermDag,
        element_terms: Vec<TermId>,
    ) -> TermId {
        let mut term = termdag.app("map-empty".into(), vec![]);

        for x in element_terms.chunks(2) {
            term = termdag.app("map-insert".into(), vec![term, x[0], x[1]])
        }

        term
    }

    fn serialized_name(&self, _container_values: &ContainerValues, _: Value) -> String {
        self.name().to_owned()
    }
}

pub(crate) fn register_map_primitives_for_function(eg: &mut EGraph, fn_: Arc<FunctionSort>) {
    let all_map_sorts = eg
        .type_info
        .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<MapContainer>()));

    for map in &all_map_sorts {
        try_registering_map_primitives_for_function(eg, fn_.clone(), map.clone());
    }
}

pub(crate) fn register_map_primitives_for_multiset(eg: &mut EGraph, multiset: ArcSort) {
    let all_map_sorts = eg
        .type_info
        .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<MapContainer>()));

    for map in &all_map_sorts {
        try_registering_map_primitives_for_multiset(eg, map.clone(), multiset.clone());
    }
}

pub(crate) fn register_map_primitives_for_set(eg: &mut EGraph, set: ArcSort) {
    let all_map_sorts = eg
        .type_info
        .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<MapContainer>()));

    for map in &all_map_sorts {
        try_registering_map_primitives_for_set(eg, map.clone(), set.clone());
    }
}

fn try_registering_map_primitives_for_function(
    eg: &mut EGraph,
    fn_: Arc<FunctionSort>,
    map: ArcSort,
) {
    let key = map.inner_sorts()[0].clone();
    let value = map.inner_sorts()[1].clone();
    let key_name = key.name();
    let value_name = value.name();

    if fn_.inputs().len() == 2
        && fn_.inputs()[0].name() == key_name
        && fn_.inputs()[1].name() == value_name
    {
        eg.add_primitive(FilterKv {
            name: "map-filter-kv".into(),
            map: map.clone(),
            fn_: fn_.clone(),
        });
        eg.add_primitive(FilterDefinedKv {
            name: "map-filter-defined-kv".into(),
            map: map.clone(),
            fn_: fn_.clone(),
        });
    }

    if fn_.inputs().len() == 2
        && fn_.inputs()[0].name() == key_name
        && fn_.inputs()[1].name() == value_name
    {
        let all_map_sorts = eg
            .type_info
            .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<MapContainer>()));
        for output_map in &all_map_sorts {
            if output_map.inner_sorts()[0].name() == key_name
                && output_map.inner_sorts()[1].name() == fn_.output().name()
            {
                eg.add_primitive(MapValues {
                    name: "map-map-values".into(),
                    input_map: map.clone(),
                    output_map: output_map.clone(),
                    fn_: fn_.clone(),
                });
            }
        }
    }

    if fn_.inputs().len() == 3
        && fn_.inputs()[1].name() == key_name
        && fn_.inputs()[2].name() == value_name
    {
        eg.add_primitive(FoldKv {
            name: "map-fold-kv".into(),
            map: map.clone(),
            accumulator: fn_.output(),
            fn_: fn_.clone(),
        });
    }

    if fn_.inputs().len() == 2
        && fn_.inputs()[0].name() == value_name
        && fn_.inputs()[1].name() == value_name
        && fn_.output().name() == value_name
    {
        eg.add_primitive(MergeWith {
            name: "map-merge-with".into(),
            map: map.clone(),
            fn_: fn_.clone(),
        });
        eg.add_primitive(MergeWithSwapped {
            name: "map-merge-with-swapped".into(),
            map: map.clone(),
            fn_: fn_.clone(),
        });
        eg.add_primitive(IntersectWith {
            name: "map-intersect-with".into(),
            map,
            fn_: fn_.clone(),
        });
    }
}

fn try_registering_map_primitives_for_multiset(eg: &mut EGraph, map: ArcSort, multiset: ArcSort) {
    if multiset.value_type() != Some(TypeId::of::<MultiSetContainer>()) {
        return;
    }
    if map.inner_sorts()[0].name() == multiset.inner_sorts()[0].name() {
        eg.add_primitive(MapKeys {
            name: "map-keys".into(),
            map: map.clone(),
            multiset: multiset.clone(),
        });
        eg.add_primitive(RestrictKeys {
            name: "map-restrict-keys".into(),
            map: map.clone(),
            multiset: multiset.clone(),
        });
        eg.add_primitive(RemoveKeys {
            name: "map-remove-keys".into(),
            map: map.clone(),
            multiset: multiset.clone(),
        });
    }
    if map.inner_sorts()[1].name() == "f64"
        && multiset.inner_sorts()[0].name() == "f64"
        && map.inner_sorts()[0].value_type() == Some(TypeId::of::<MapContainer>())
    {
        eg.add_primitive(NonConstNonUnitF64Values {
            name: "map-nonconst-nonunit-f64-values".into(),
            map,
            multiset,
        });
    }
}

fn try_registering_map_primitives_for_set(eg: &mut EGraph, map: ArcSort, set: ArcSort) {
    if set.value_type() != Some(TypeId::of::<SetContainer>()) {
        return;
    }
    if map.inner_sorts()[0].value_type() == Some(TypeId::of::<MapContainer>()) {
        let inner_sorts = map.inner_sorts()[0].inner_sorts();
        if inner_sorts.len() == 2
            && inner_sorts[1].name() == "BigRat"
            && set.inner_sorts()[0].name() == inner_sorts[0].name()
        {
            eg.add_primitive(SharedFactorAtoms {
                name: "map-shared-factor-atoms".into(),
                map,
                set,
            });
        }
    }
}

#[derive(Clone)]
struct FilterKv {
    name: String,
    map: ArcSort,
    fn_: Arc<FunctionSort>,
}

#[derive(Clone)]
struct FilterDefinedKv {
    name: String,
    map: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for FilterKv {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.fn_.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v) in &map.data {
            if fc.apply(exec_state, &[*k, *v]).is_some() {
                new_data.insert(*k, *v);
            }
        }
        let new_map = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(new_map, exec_state),
        )
    }
}

impl Primitive for FilterDefinedKv {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.fn_.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v) in &map.data {
            if fc.apply(exec_state, &[*k, *v]).is_some() {
                new_data.insert(*k, *v);
            }
        }
        let new_map = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(new_map, exec_state),
        )
    }
}

#[derive(Clone)]
struct MapValues {
    name: String,
    input_map: ArcSort,
    output_map: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for MapValues {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.input_map.clone(),
                self.output_map.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let input_map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let output_do_rebuild_vals = self.output_map.inner_sorts()[1].is_eq_sort()
            || self.output_map.inner_sorts()[1].is_eq_container_sort();
        let mut new_data = BTreeMap::new();
        for (k, v) in &input_map.data {
            if let Some(mapped_v) = fc.apply(exec_state, &[*k, *v]) {
                new_data.insert(*k, mapped_v);
            }
        }
        let output_map = MapContainer {
            do_rebuild_keys: input_map.do_rebuild_keys,
            do_rebuild_vals: output_do_rebuild_vals,
            data: new_data,
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(output_map, exec_state),
        )
    }
}

#[derive(Clone)]
struct FoldKv {
    name: String,
    map: ArcSort,
    accumulator: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for FoldKv {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.accumulator.clone(),
                self.map.clone(),
                self.accumulator.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[2])?
            .clone();
        let mut acc = args[1];
        for (k, v) in &map.data {
            acc = fc.apply(exec_state, &[acc, *k, *v])?;
        }
        Some(acc)
    }
}

#[derive(Clone)]
struct MergeWith {
    name: String,
    map: ArcSort,
    fn_: Arc<FunctionSort>,
}

#[derive(Clone)]
struct MergeWithSwapped {
    name: String,
    map: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for MergeWithSwapped {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.map.clone(),
                self.map.clone(),
                self.map.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let right = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let left = exec_state
            .container_values()
            .get_val::<MapContainer>(args[2])?
            .clone();
        let mut new_data = left.data.clone();
        for (k, v_right) in &right.data {
            if let Some(v_left) = new_data.get(k).copied() {
                let merged = fc.apply(exec_state, &[v_left, *v_right])?;
                new_data.insert(*k, merged);
            } else {
                new_data.insert(*k, *v_right);
            }
        }
        let merged_map = MapContainer {
            data: new_data,
            ..left
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(merged_map, exec_state),
        )
    }
}

impl Primitive for MergeWith {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.map.clone(),
                self.map.clone(),
                self.map.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let left = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let right = exec_state
            .container_values()
            .get_val::<MapContainer>(args[2])?
            .clone();
        let mut new_data = left.data.clone();
        for (k, v_right) in &right.data {
            if let Some(v_left) = new_data.get(k).copied() {
                let merged = fc.apply(exec_state, &[v_left, *v_right])?;
                new_data.insert(*k, merged);
            } else {
                new_data.insert(*k, *v_right);
            }
        }
        let merged_map = MapContainer {
            data: new_data,
            ..left
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(merged_map, exec_state),
        )
    }
}

#[derive(Clone)]
struct IntersectWith {
    name: String,
    map: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for IntersectWith {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.map.clone(),
                self.map.clone(),
                self.map.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[0])?
            .clone();
        let left = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let right = exec_state
            .container_values()
            .get_val::<MapContainer>(args[2])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v_left) in &left.data {
            if let Some(v_right) = right.data.get(k) {
                let merged = fc.apply(exec_state, &[*v_left, *v_right])?;
                new_data.insert(*k, merged);
            }
        }
        let intersected_map = MapContainer {
            data: new_data,
            ..left
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(intersected_map, exec_state),
        )
    }
}

#[derive(Clone, Copy)]
enum DropZeroKind {
    F64,
    BigRat,
}

impl DropZeroKind {
    fn from_value_sort(value: ArcSort) -> Option<Self> {
        match value.name() {
            "f64" => Some(Self::F64),
            "BigRat" => Some(Self::BigRat),
            _ => None,
        }
    }

    fn is_zero(self, exec_state: &ExecutionState, value: Value) -> bool {
        match self {
            Self::F64 => exec_state.base_values().unwrap::<F>(value).0 == 0.0,
            Self::BigRat => exec_state.base_values().unwrap::<Q>(value).is_zero(),
        }
    }
}

#[derive(Clone)]
struct DropZeroValues {
    name: String,
    map: ArcSort,
    value_kind: DropZeroKind,
}

#[derive(Clone)]
struct BigRatSubtract {
    name: String,
    map: ArcSort,
}

#[derive(Clone)]
struct BigRatIntersectMin {
    name: String,
    map: ArcSort,
}

#[derive(Clone)]
struct SubtractBigRatFromKeys {
    name: String,
    outer_map: ArcSort,
    inner_map: ArcSort,
}

#[derive(Clone)]
struct MapKeys {
    name: String,
    map: ArcSort,
    multiset: ArcSort,
}

#[derive(Clone)]
struct NonConstNonUnitF64Values {
    name: String,
    map: ArcSort,
    multiset: ArcSort,
}

#[derive(Clone)]
struct DivideAllValuesByF64 {
    name: String,
    float: ArcSort,
    map: ArcSort,
}

#[derive(Clone)]
struct SharedFactorAtoms {
    name: String,
    map: ArcSort,
    set: ArcSort,
}

impl Primitive for MapKeys {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.multiset.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let multiset = MultiSetContainer {
            do_rebuild: self.multiset.is_eq_container_sort(),
            data: map.data.keys().copied().collect(),
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(multiset, exec_state),
        )
    }
}

#[derive(Clone)]
struct RestrictKeys {
    name: String,
    map: ArcSort,
    multiset: ArcSort,
}

impl Primitive for RestrictKeys {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.multiset.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let keys = exec_state
            .container_values()
            .get_val::<MultiSetContainer>(args[0])?
            .clone();
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let new_data = map
            .data
            .iter()
            .filter(|(k, _)| keys.data.contains(k))
            .map(|(k, v)| (*k, *v))
            .collect();
        let restricted = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(restricted, exec_state),
        )
    }
}

#[derive(Clone)]
struct RemoveKeys {
    name: String,
    map: ArcSort,
    multiset: ArcSort,
}

impl Primitive for RemoveKeys {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.multiset.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let keys = exec_state
            .container_values()
            .get_val::<MultiSetContainer>(args[0])?
            .clone();
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let new_data = map
            .data
            .iter()
            .filter(|(k, _)| !keys.data.contains(k))
            .map(|(k, v)| (*k, *v))
            .collect();
        let restricted = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(restricted, exec_state),
        )
    }
}

impl Primitive for NonConstNonUnitF64Values {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.multiset.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let mut distinct = BTreeSet::new();
        for (k, v) in &map.data {
            let monomial = exec_state
                .container_values()
                .get_val::<MapContainer>(*k)?
                .clone();
            if monomial.data.is_empty() {
                continue;
            }
            let coeff = exec_state.base_values().unwrap::<F>(*v).0.0;
            if !coeff.is_finite() || coeff == 0.0 || coeff == 1.0 || coeff == -1.0 {
                continue;
            }
            distinct.insert(*v);
        }
        let multiset = MultiSetContainer {
            do_rebuild: self.multiset.is_eq_container_sort(),
            data: distinct.into_iter().collect(),
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(multiset, exec_state),
        )
    }
}

impl Primitive for DivideAllValuesByF64 {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.float.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let factor = exec_state.base_values().unwrap::<F>(args[0]).0.0;
        if factor == 0.0 || !factor.is_finite() {
            return None;
        }
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v) in &map.data {
            let coeff = exec_state.base_values().unwrap::<F>(*v).0.0;
            let mut quotient = coeff / factor;
            if !quotient.is_finite() {
                return None;
            }
            if quotient == 0.0 {
                quotient = 0.0;
            }
            let quotient_value = exec_state
                .base_values()
                .get::<F>(F::from(OrderedFloat(quotient)));
            new_data.insert(*k, quotient_value);
        }
        let divided = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(divided, exec_state),
        )
    }
}

impl Primitive for SharedFactorAtoms {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.set.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let outer_map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let mut counts = BTreeMap::<Value, usize>::new();
        for monomial_value in outer_map.data.keys() {
            let monomial = exec_state
                .container_values()
                .get_val::<MapContainer>(*monomial_value)?
                .clone();
            if monomial.data.is_empty() {
                continue;
            }
            for atom in monomial.data.keys() {
                *counts.entry(*atom).or_insert(0) += 1;
            }
        }
        let shared = SetContainer {
            do_rebuild: self.set.is_eq_container_sort(),
            data: counts
                .into_iter()
                .filter_map(|(atom, count)| (count >= 2).then_some(atom))
                .collect(),
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(shared, exec_state),
        )
    }
}

impl Primitive for DropZeroValues {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let map = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v) in &map.data {
            if !self.value_kind.is_zero(exec_state, *v) {
                new_data.insert(*k, *v);
            }
        }
        let filtered_map = MapContainer {
            data: new_data,
            ..map
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(filtered_map, exec_state),
        )
    }
}

fn subtract_bigrat_maps(
    exec_state: &ExecutionState,
    left: &MapContainer,
    right: &MapContainer,
) -> MapContainer {
    let mut new_data = left.data.clone();
    for (k, v_right) in &right.data {
        let right_q = exec_state.base_values().unwrap::<Q>(*v_right);
        let merged = if let Some(v_left) = new_data.get(k).copied() {
            let left_q = exec_state.base_values().unwrap::<Q>(v_left);
            Q::new(left_q.checked_sub(&right_q).unwrap())
        } else {
            Q::new(-right_q.0.clone())
        };
        let merged_value = exec_state.base_values().get::<Q>(merged);
        new_data.insert(*k, merged_value);
    }
    MapContainer {
        data: new_data,
        ..left.clone()
    }
}

fn drop_zero_bigrat_values(exec_state: &ExecutionState, map: &MapContainer) -> MapContainer {
    let mut new_data = BTreeMap::new();
    for (k, v) in &map.data {
        if !exec_state.base_values().unwrap::<Q>(*v).is_zero() {
            new_data.insert(*k, *v);
        }
    }
    MapContainer {
        data: new_data,
        ..map.clone()
    }
}

impl Primitive for BigRatSubtract {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let right = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let left = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let subtracted = subtract_bigrat_maps(exec_state, &left, &right);
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(subtracted, exec_state),
        )
    }
}

impl Primitive for BigRatIntersectMin {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.map.clone(), self.map.clone(), self.map.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let left = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let right = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v_left) in &left.data {
            if let Some(v_right) = right.data.get(k) {
                let left_q = exec_state.base_values().unwrap::<Q>(*v_left);
                let right_q = exec_state.base_values().unwrap::<Q>(*v_right);
                let merged_q = if left_q <= right_q {
                    left_q.clone()
                } else {
                    right_q.clone()
                };
                let merged = exec_state.base_values().get::<Q>(merged_q);
                new_data.insert(*k, merged);
            }
        }
        let intersected = MapContainer {
            data: new_data,
            ..left
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(intersected, exec_state),
        )
    }
}

impl Primitive for SubtractBigRatFromKeys {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.inner_map.clone(),
                self.outer_map.clone(),
                self.outer_map.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let factor = exec_state
            .container_values()
            .get_val::<MapContainer>(args[0])?
            .clone();
        let outer = exec_state
            .container_values()
            .get_val::<MapContainer>(args[1])?
            .clone();
        let mut new_data = BTreeMap::new();
        for (k, v) in &outer.data {
            let key_map = exec_state
                .container_values()
                .get_val::<MapContainer>(*k)?
                .clone();
            let subtracted = subtract_bigrat_maps(exec_state, &key_map, &factor);
            let normalized = drop_zero_bigrat_values(exec_state, &subtracted);
            let new_key = exec_state
                .clone()
                .container_values()
                .register_val(normalized, exec_state);
            new_data.insert(new_key, *v);
        }
        let transformed = MapContainer {
            data: new_data,
            ..outer
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(transformed, exec_state),
        )
    }
}

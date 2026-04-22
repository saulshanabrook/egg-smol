use super::*;
use egglog_bridge::UnionAction;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SetContainer {
    pub do_rebuild: bool,
    pub data: BTreeSet<Value>,
}

impl ContainerValue for SetContainer {
    fn rebuild_contents(&mut self, rebuilder: &dyn Rebuilder) -> bool {
        if self.do_rebuild {
            let mut xs: Vec<_> = self.data.iter().copied().collect();
            let changed = rebuilder.rebuild_slice(&mut xs);
            self.data = xs.into_iter().collect();
            changed
        } else {
            false
        }
    }
    fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        self.data.iter().copied()
    }
}

#[derive(Clone, Debug)]
pub struct SetSort {
    name: String,
    element: ArcSort,
}

impl SetSort {
    pub fn element(&self) -> ArcSort {
        self.element.clone()
    }
}

impl Presort for SetSort {
    fn presort_name() -> &'static str {
        "Set"
    }

    fn reserved_primitives() -> Vec<&'static str> {
        vec![
            "set-of",
            "set-empty",
            "set-insert",
            "set-not-contains",
            "set-contains",
            "set-remove",
            "set-union",
            "set-diff",
            "set-intersect",
            "set-get",
            "set-length",
            "unstable-set-map",
            "set-union-values",
        ]
    }

    fn make_sort(
        typeinfo: &mut TypeInfo,
        name: String,
        args: &[Expr],
    ) -> Result<ArcSort, TypeError> {
        if let [Expr::Var(span, e)] = args {
            let e = typeinfo
                .get_sort_by_name(e)
                .ok_or(TypeError::UndefinedSort(e.clone(), span.clone()))?;

            let out = Self {
                name,
                element: e.clone(),
            };
            Ok(out.to_arcsort())
        } else {
            panic!()
        }
    }
}

impl ContainerSort for SetSort {
    type Container = SetContainer;

    fn name(&self) -> &str {
        &self.name
    }

    fn inner_sorts(&self) -> Vec<ArcSort> {
        vec![self.element.clone()]
    }

    fn is_eq_container_sort(&self) -> bool {
        self.element.is_eq_sort() || self.element.is_eq_container_sort()
    }

    fn inner_values(
        &self,
        container_values: &ContainerValues,
        value: Value,
    ) -> Vec<(ArcSort, Value)> {
        let val = container_values
            .get_val::<SetContainer>(value)
            .unwrap()
            .clone();
        val.data
            .iter()
            .map(|e| (self.element.clone(), *e))
            .collect()
    }

    fn register_primitives(&self, eg: &mut EGraph) {
        let arc = self.clone().to_arcsort();

        add_primitive!(eg, "set-empty" = {self.clone(): SetSort} |                      | -> @SetContainer (arc) { SetContainer {
            do_rebuild: self.ctx.is_eq_container_sort(),
            data: BTreeSet::new()
        } });
        add_primitive!(eg, "set-of"    = {self.clone(): SetSort} [xs: # (self.element())] -> @SetContainer (arc) { SetContainer {
            do_rebuild: self.ctx.is_eq_container_sort(),
            data: xs.collect()
        } });

        add_primitive!(eg, "set-get" = |xs: @SetContainer (arc), i: i64| -?> # (self.element()) { xs.data.iter().nth(i as usize).copied() });
        add_primitive!(eg, "set-insert" = |mut xs: @SetContainer (arc), x: # (self.element())| -> @SetContainer (arc) {{ xs.data.insert( x); xs }});
        add_primitive!(eg, "set-remove" = |mut xs: @SetContainer (arc), x: # (self.element())| -> @SetContainer (arc) {{ xs.data.remove(&x); xs }});

        add_primitive!(eg, "set-length"       = |xs: @SetContainer (arc)| -> i64 { xs.data.len() as i64 });
        add_primitive!(eg, "set-contains"     = |xs: @SetContainer (arc), x: # (self.element())| -?> () { ( xs.data.contains(&x)).then_some(()) });
        add_primitive!(eg, "set-not-contains" = |xs: @SetContainer (arc), x: # (self.element())| -?> () { (!xs.data.contains(&x)).then_some(()) });

        add_primitive!(eg, "set-union"      = |mut xs: @SetContainer (arc), ys: @SetContainer (arc)| -> @SetContainer (arc) {{ xs.data.extend(ys.data);                  xs }});
        add_primitive!(eg, "set-diff"       = |mut xs: @SetContainer (arc), ys: @SetContainer (arc)| -> @SetContainer (arc) {{ xs.data.retain(|k| !ys.data.contains(k)); xs }});
        add_primitive!(eg, "set-intersect"  = |mut xs: @SetContainer (arc), ys: @SetContainer (arc)| -> @SetContainer (arc) {{ xs.data.retain(|k|  ys.data.contains(k)); xs }});

        register_map_primitives_for_set(eg, arc.clone());
        for fn_sort in eg.type_info.get_sorts::<FunctionSort>() {
            try_registering_set_map(eg, fn_sort.clone(), arc.clone());
        }
        if self.element.is_eq_sort() {
            eg.add_primitive(SetUnionValues {
                name: "set-union-values".into(),
                set: arc.clone(),
                action: eg.new_union_action(),
                element: self.element.clone(),
            });
        }
    }

    fn reconstruct_termdag(
        &self,
        _container_values: &ContainerValues,
        _value: Value,
        termdag: &mut TermDag,
        element_terms: Vec<TermId>,
    ) -> TermId {
        termdag.app("set-of".into(), element_terms)
    }

    fn serialized_name(&self, _container_values: &ContainerValues, _: Value) -> String {
        "set-of".to_owned()
    }
}

pub(crate) fn register_set_primitives_for_function(eg: &mut EGraph, fn_: Arc<FunctionSort>) {
    let all_set_sorts = eg
        .type_info
        .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<SetContainer>()));
    for input_set in &all_set_sorts {
        try_registering_set_map(eg, fn_.clone(), input_set.clone());
    }
}

fn try_registering_set_map(eg: &mut EGraph, fn_: Arc<FunctionSort>, input_set: ArcSort) {
    if input_set.value_type() != Some(TypeId::of::<SetContainer>()) || fn_.inputs().len() != 1 {
        return;
    }
    let input_element = input_set.inner_sorts()[0].clone();
    if fn_.inputs()[0].name() != input_element.name() {
        return;
    }
    let all_set_sorts = eg
        .type_info
        .get_arcsorts_by(|f| f.value_type() == Some(TypeId::of::<SetContainer>()));
    for output_set in &all_set_sorts {
        if output_set.inner_sorts()[0].name() == fn_.output().name() {
            eg.add_primitive(SetMap {
                name: "unstable-set-map".into(),
                input_set: input_set.clone(),
                output_set: output_set.clone(),
                fn_: fn_.clone(),
            });
        }
    }
}

#[derive(Clone)]
struct SetMap {
    name: String,
    input_set: ArcSort,
    output_set: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for SetMap {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.fn_.clone(),
                self.input_set.clone(),
                self.output_set.clone(),
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
        let input_set = exec_state
            .container_values()
            .get_val::<SetContainer>(args[1])?
            .clone();
        let mut new_data = BTreeSet::new();
        for v in &input_set.data {
            if let Some(mapped_v) = fc.apply(exec_state, &[*v]) {
                new_data.insert(mapped_v);
            }
        }
        let output_set = SetContainer {
            do_rebuild: self.output_set.is_eq_container_sort(),
            data: new_data,
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(output_set, exec_state),
        )
    }
}

#[derive(Clone)]
struct SetUnionValues {
    name: String,
    set: ArcSort,
    element: ArcSort,
    action: UnionAction,
}

impl Primitive for SetUnionValues {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.set.clone(), self.element.clone()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let values = exec_state
            .container_values()
            .get_val::<SetContainer>(args[0])?
            .clone()
            .data;
        let values: Vec<_> = values.iter().copied().collect();
        if values.is_empty() {
            return None;
        }
        let first = values[0];
        for v in values.into_iter().skip(1) {
            self.action.union(exec_state, first, v);
        }
        Some(first)
    }
}

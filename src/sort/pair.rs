use super::*;
use std::any::TypeId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PairContainer {
    pub do_rebuild_left: bool,
    pub do_rebuild_right: bool,
    pub left: Value,
    pub right: Value,
}

impl ContainerValue for PairContainer {
    fn rebuild_contents(&mut self, rebuilder: &dyn Rebuilder) -> bool {
        let mut changed = false;
        if self.do_rebuild_left {
            let new = rebuilder.rebuild_val(self.left);
            changed |= self.left != new;
            self.left = new;
        }
        if self.do_rebuild_right {
            let new = rebuilder.rebuild_val(self.right);
            changed |= self.right != new;
            self.right = new;
        }
        changed
    }

    fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        [self.left, self.right].into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct PairSort {
    name: String,
    left: ArcSort,
    right: ArcSort,
}

impl PairSort {
    pub fn left(&self) -> ArcSort {
        self.left.clone()
    }

    pub fn right(&self) -> ArcSort {
        self.right.clone()
    }
}

impl Presort for PairSort {
    fn presort_name() -> &'static str {
        "Pair"
    }

    fn reserved_primitives() -> Vec<&'static str> {
        vec![
            "pair",
            "pair-left",
            "pair-right",
            "unstable-pair-match",
            "unstable-pair-map-left",
            "unstable-pair-map-right",
        ]
    }

    fn make_sort(
        typeinfo: &mut TypeInfo,
        name: String,
        args: &[Expr],
    ) -> Result<ArcSort, TypeError> {
        if let [Expr::Var(left_span, left), Expr::Var(right_span, right)] = args {
            let left = typeinfo
                .get_sort_by_name(left)
                .ok_or(TypeError::UndefinedSort(left.clone(), left_span.clone()))?;
            let right = typeinfo
                .get_sort_by_name(right)
                .ok_or(TypeError::UndefinedSort(right.clone(), right_span.clone()))?;

            Ok(Self {
                name,
                left: left.clone(),
                right: right.clone(),
            }
            .to_arcsort())
        } else {
            panic!("Pair sort must have left and right sort arguments. Got {args:?}")
        }
    }
}

impl ContainerSort for PairSort {
    type Container = PairContainer;

    fn name(&self) -> &str {
        &self.name
    }

    fn inner_sorts(&self) -> Vec<ArcSort> {
        vec![self.left.clone(), self.right.clone()]
    }

    fn is_eq_container_sort(&self) -> bool {
        self.left.is_eq_sort()
            || self.right.is_eq_sort()
            || self.left.is_eq_container_sort()
            || self.right.is_eq_container_sort()
    }

    fn inner_values(
        &self,
        _container_values: &ContainerValues,
        value: Value,
    ) -> Vec<(ArcSort, Value)> {
        let val = _container_values
            .get_val::<PairContainer>(value)
            .unwrap()
            .clone();
        vec![
            (self.left.clone(), val.left),
            (self.right.clone(), val.right),
        ]
    }

    fn register_primitives(&self, eg: &mut EGraph) {
        let arc = self.clone().to_arcsort();

        add_primitive!(eg, "pair" = {self.clone(): PairSort} |left: # (self.left()), right: # (self.right())| -> @PairContainer (arc) {
            PairContainer {
                do_rebuild_left: self.ctx.left.is_eq_sort() || self.ctx.left.is_eq_container_sort(),
                do_rebuild_right: self.ctx.right.is_eq_sort() || self.ctx.right.is_eq_container_sort(),
                left,
                right,
            }
        });
        add_primitive!(eg, "pair-left" = |pair: @PairContainer (arc)| -> # (self.left()) {
            pair.left
        });
        add_primitive!(eg, "pair-right" = |pair: @PairContainer (arc)| -> # (self.right()) {
            pair.right
        });

        register_map_primitives_for_pair(eg, arc.clone());

        let pair = eg.type_info.get_sort_by_name(self.name()).unwrap().clone();
        for fn_sort in eg.type_info.get_sorts::<FunctionSort>() {
            try_registering_pair_match(eg, pair.clone(), fn_sort.clone());
            for output_pair in eg
                .type_info
                .get_arcsorts_by(|sort| sort.value_type() == Some(TypeId::of::<PairContainer>()))
            {
                try_registering_pair_map_left(
                    eg,
                    pair.clone(),
                    output_pair.clone(),
                    fn_sort.clone(),
                );
                try_registering_pair_map_right(eg, pair.clone(), output_pair, fn_sort.clone());
            }
        }
    }

    fn reconstruct_termdag(
        &self,
        _container_values: &ContainerValues,
        _value: Value,
        termdag: &mut TermDag,
        element_terms: Vec<TermId>,
    ) -> TermId {
        termdag.app("pair".into(), element_terms)
    }

    fn serialized_name(&self, _container_values: &ContainerValues, _value: Value) -> String {
        "pair".to_owned()
    }
}

pub(crate) fn register_pair_primitives_for_function(eg: &mut EGraph, fn_: Arc<FunctionSort>) {
    let all_pairs = eg
        .type_info
        .get_arcsorts_by(|sort| sort.value_type() == Some(TypeId::of::<PairContainer>()));
    for input_pair in &all_pairs {
        try_registering_pair_match(eg, input_pair.clone(), fn_.clone());
        for output_pair in &all_pairs {
            try_registering_pair_map_left(eg, input_pair.clone(), output_pair.clone(), fn_.clone());
            try_registering_pair_map_right(
                eg,
                input_pair.clone(),
                output_pair.clone(),
                fn_.clone(),
            );
        }
    }
}

fn try_registering_pair_match(eg: &mut EGraph, pair: ArcSort, fn_: Arc<FunctionSort>) {
    if fn_.inputs().len() != 2
        || pair.value_type() != Some(TypeId::of::<PairContainer>())
        || fn_.inputs()[0].name() != pair.inner_sorts()[0].name()
        || fn_.inputs()[1].name() != pair.inner_sorts()[1].name()
    {
        return;
    }
    eg.add_primitive(PairMatch {
        name: "unstable-pair-match".into(),
        pair,
        fn_,
    });
}

fn try_registering_pair_map_left(
    eg: &mut EGraph,
    input_pair: ArcSort,
    output_pair: ArcSort,
    fn_: Arc<FunctionSort>,
) {
    if fn_.inputs().len() != 1
        || input_pair.value_type() != Some(TypeId::of::<PairContainer>())
        || output_pair.value_type() != Some(TypeId::of::<PairContainer>())
        || fn_.inputs()[0].name() != input_pair.inner_sorts()[0].name()
        || fn_.output().name() != output_pair.inner_sorts()[0].name()
        || input_pair.inner_sorts()[1].name() != output_pair.inner_sorts()[1].name()
    {
        return;
    }
    eg.add_primitive(PairMapLeft {
        name: "unstable-pair-map-left".into(),
        input_pair,
        output_pair,
        fn_,
    });
}

fn try_registering_pair_map_right(
    eg: &mut EGraph,
    input_pair: ArcSort,
    output_pair: ArcSort,
    fn_: Arc<FunctionSort>,
) {
    if fn_.inputs().len() != 1
        || input_pair.value_type() != Some(TypeId::of::<PairContainer>())
        || output_pair.value_type() != Some(TypeId::of::<PairContainer>())
        || fn_.inputs()[0].name() != input_pair.inner_sorts()[1].name()
        || fn_.output().name() != output_pair.inner_sorts()[1].name()
        || input_pair.inner_sorts()[0].name() != output_pair.inner_sorts()[0].name()
    {
        return;
    }
    eg.add_primitive(PairMapRight {
        name: "unstable-pair-map-right".into(),
        input_pair,
        output_pair,
        fn_,
    });
}

#[derive(Clone)]
struct PairMatch {
    name: String,
    pair: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for PairMatch {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![self.pair.clone(), self.fn_.clone(), self.fn_.output()],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let pair = exec_state
            .container_values()
            .get_val::<PairContainer>(args[0])?
            .clone();
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[1])?
            .clone();
        fc.apply(exec_state, &[pair.left, pair.right])
    }
}

#[derive(Clone)]
struct PairMapLeft {
    name: String,
    input_pair: ArcSort,
    output_pair: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for PairMapLeft {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.input_pair.clone(),
                self.fn_.clone(),
                self.output_pair.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let pair = exec_state
            .container_values()
            .get_val::<PairContainer>(args[0])?
            .clone();
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[1])?
            .clone();
        let left = fc.apply(exec_state, &[pair.left])?;
        let mapped = PairContainer {
            do_rebuild_left: self.output_pair.inner_sorts()[0].is_eq_sort()
                || self.output_pair.inner_sorts()[0].is_eq_container_sort(),
            do_rebuild_right: self.output_pair.inner_sorts()[1].is_eq_sort()
                || self.output_pair.inner_sorts()[1].is_eq_container_sort(),
            left,
            right: pair.right,
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(mapped, exec_state),
        )
    }
}

#[derive(Clone)]
struct PairMapRight {
    name: String,
    input_pair: ArcSort,
    output_pair: ArcSort,
    fn_: Arc<FunctionSort>,
}

impl Primitive for PairMapRight {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint> {
        SimpleTypeConstraint::new(
            self.name(),
            vec![
                self.input_pair.clone(),
                self.fn_.clone(),
                self.output_pair.clone(),
            ],
            span.clone(),
        )
        .into_box()
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        let pair = exec_state
            .container_values()
            .get_val::<PairContainer>(args[0])?
            .clone();
        let fc = exec_state
            .container_values()
            .get_val::<FunctionContainer>(args[1])?
            .clone();
        let right = fc.apply(exec_state, &[pair.right])?;
        let mapped = PairContainer {
            do_rebuild_left: self.output_pair.inner_sorts()[0].is_eq_sort()
                || self.output_pair.inner_sorts()[0].is_eq_container_sort(),
            do_rebuild_right: self.output_pair.inner_sorts()[1].is_eq_sort()
                || self.output_pair.inner_sorts()[1].is_eq_container_sort(),
            left: pair.left,
            right,
        };
        Some(
            exec_state
                .clone()
                .container_values()
                .register_val(mapped, exec_state),
        )
    }
}

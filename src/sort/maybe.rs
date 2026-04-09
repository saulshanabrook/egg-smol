use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaybeContainer {
    pub do_rebuild: bool,
    pub data: Option<Value>,
}

impl ContainerValue for MaybeContainer {
    fn rebuild_contents(&mut self, rebuilder: &dyn Rebuilder) -> bool {
        if self.do_rebuild {
            if let Some(old) = self.data {
                let new = rebuilder.rebuild_val(old);
                self.data = Some(new);
                old != new
            } else {
                false
            }
        } else {
            false
        }
    }

    fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        self.data.iter().copied()
    }
}

#[derive(Clone, Debug)]
pub struct MaybeSort {
    name: String,
    element: ArcSort,
}

impl MaybeSort {
    pub fn element(&self) -> ArcSort {
        self.element.clone()
    }
}

impl Presort for MaybeSort {
    fn presort_name() -> &'static str {
        "Maybe"
    }

    fn reserved_primitives() -> Vec<&'static str> {
        vec![
            "maybe-none",
            "maybe-some",
            "maybe-unwrap",
            "maybe-unwrap-or",
            "maybe-f64-merge-with-tol",
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

            Ok(Self {
                name,
                element: e.clone(),
            }
            .to_arcsort())
        } else {
            panic!("Maybe sort must have sort as argument. Got {args:?}")
        }
    }
}

impl ContainerSort for MaybeSort {
    type Container = MaybeContainer;

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
            .get_val::<MaybeContainer>(value)
            .unwrap()
            .clone();
        val.data
            .iter()
            .map(|v| (self.element.clone(), *v))
            .collect()
    }

    fn register_primitives(&self, eg: &mut EGraph) {
        let arc = self.clone().to_arcsort();

        add_primitive!(eg, "maybe-none" = {self.clone(): MaybeSort} || -> @MaybeContainer (arc) { MaybeContainer {
            do_rebuild: self.ctx.is_eq_container_sort(),
            data: None,
        } });

        add_primitive!(eg, "maybe-some" = {self.clone(): MaybeSort} |x: # (self.element())| -> @MaybeContainer (arc) { MaybeContainer {
            do_rebuild: self.ctx.is_eq_container_sort(),
            data: Some(x),
        } });

        add_primitive!(eg, "maybe-unwrap" = |xs: @MaybeContainer (arc)| -?> # (self.element()) { xs.data });
        add_primitive!(eg, "maybe-unwrap-or" = |xs: @MaybeContainer (arc), default: # (self.element())| -> # (self.element()) {
            xs.data.unwrap_or(default)
        });

        if self.element().name() == "f64" {
            add_primitive!(eg, "maybe-f64-merge-with-tol" = |old: @MaybeContainer (arc), new: @MaybeContainer (arc), tol: F| -?> @MaybeContainer (arc) {{
                match (old.data, new.data) {
                    (None, _) | (_, None) => Some(MaybeContainer { data: None, ..old }),
                    (Some(old_value), Some(new_value)) => {
                        let old_f = exec_state.base_values().unwrap::<F>(old_value).0.0;
                        let new_f = exec_state.base_values().unwrap::<F>(new_value).0.0;
                        let tolerance = tol.0.0.abs();
                        let merged =
                            old_f == new_f ||
                            (old_f == 0.0 && new_f == -0.0) ||
                            (old_f == -0.0 && new_f == 0.0) ||
                            (old_f - new_f).abs() <= tolerance;
                        merged.then_some(old)
                    }
                }
            }});
        }
    }

    fn reconstruct_termdag(
        &self,
        _container_values: &ContainerValues,
        _value: Value,
        termdag: &mut TermDag,
        element_terms: Vec<TermId>,
    ) -> TermId {
        match element_terms.as_slice() {
            [] => termdag.app("maybe-none".into(), vec![]),
            [value] => termdag.app("maybe-some".into(), vec![*value]),
            _ => panic!(
                "Maybe sort expected at most one element, got {}",
                element_terms.len()
            ),
        }
    }

    fn serialized_name(&self, container_values: &ContainerValues, value: Value) -> String {
        let maybe = container_values.get_val::<MaybeContainer>(value).unwrap();
        if maybe.data.is_some() {
            "maybe-some".to_owned()
        } else {
            "maybe-none".to_owned()
        }
    }
}

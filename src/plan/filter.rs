//! Predicate expression plan.

use std::collections::HashMap;

use timely::communication::Allocate;
use timely::dataflow::scopes::child::{Child, Iterative};
use timely::worker::Worker;

use plan::Implementable;
use Relation;
use {QueryMap, RelationMap, SimpleRelation, Value, Var};

/// Permitted comparison predicates.
#[derive(Deserialize, Clone, Debug)]
pub enum Predicate {
    /// Less than
    LT,
    /// Greater than
    GT,
    /// Less than or equal to
    LTE,
    /// Greater than or equal to
    GTE,
    /// Equal
    EQ,
    /// Not equal
    NEQ,
}

fn lt(a: &Value, b: &Value) -> bool {
    a < b
}
fn lte(a: &Value, b: &Value) -> bool {
    a <= b
}
fn gt(a: &Value, b: &Value) -> bool {
    a > b
}
fn gte(a: &Value, b: &Value) -> bool {
    a >= b
}
fn eq(a: &Value, b: &Value) -> bool {
    a == b
}
fn neq(a: &Value, b: &Value) -> bool {
    a != b
}

/// A plan stage filtering source tuples by the specified
/// predicate. Frontends are responsible for ensuring that the source
/// binds the argument symbols.
#[derive(Deserialize, Clone, Debug)]
pub struct Filter<P: Implementable> {
    /// TODO
    pub variables: Vec<Var>,
    /// Logical predicate to apply.
    pub predicate: Predicate,
    /// Plan for the data source.
    pub plan: Box<P>,
    /// Constant intputs
    pub constants: HashMap<u32, Value>,
}

impl<P: Implementable> Implementable for Filter<P> {
    fn implement<'a, 'b, A: Allocate>(
        &self,
        nested: &mut Iterative<'b, Child<'a, Worker<A>, u64>, u64>,
        local_arrangements: &RelationMap<Iterative<'b, Child<'a, Worker<A>, u64>, u64>>,
        global_arrangements: &mut QueryMap<isize>,
    ) -> SimpleRelation<'b, Child<'a, Worker<A>, u64>> {
        let rel = self.plan
            .implement(nested, local_arrangements, global_arrangements);

        let key_offsets: Vec<usize> = self.variables
            .iter()
            .map(|sym| {
                rel.symbols()
                    .iter()
                    .position(|&v| *sym == v)
                    .expect("Symbol not found.")
            })
            .collect();

        let binary_predicate = match self.predicate {
            Predicate::LT => lt,
            Predicate::LTE => lte,
            Predicate::GT => gt,
            Predicate::GTE => gte,
            Predicate::EQ => eq,
            Predicate::NEQ => neq,
        };

        if self.constants.contains_key(&0) {
            let constant = self.constants.get(&0).unwrap().clone();
            SimpleRelation {
                symbols: rel.symbols().to_vec(),
                tuples: rel.tuples()
                    .filter(move |tuple| binary_predicate(&constant, &tuple[key_offsets[0]])),
            }
        } else if self.constants.contains_key(&1) {
            let constant = self.constants.get(&1).unwrap().clone();
            SimpleRelation {
                symbols: rel.symbols().to_vec(),
                tuples: rel.tuples()
                    .filter(move |tuple| binary_predicate(&tuple[key_offsets[0]], &constant)),
            }
        } else {
            SimpleRelation {
                symbols: rel.symbols().to_vec(),
                tuples: rel.tuples().filter(move |tuple| {
                    binary_predicate(&tuple[key_offsets[0]], &tuple[key_offsets[1]])
                }),
            }
        }
    }
}

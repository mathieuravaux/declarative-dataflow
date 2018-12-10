extern crate declarative_dataflow;
extern crate num_rational;
extern crate timely;

use std::sync::mpsc::channel;
use std::thread;

use timely::Configuration;

use declarative_dataflow::plan::{Join, Hector, Project, Binding};
use declarative_dataflow::server::{Register, Server, Transact, TxData};
use declarative_dataflow::{Plan, Rule, Value};

#[test]
fn match_ea() {
    timely::execute(Configuration::Thread, move |worker| {
        let mut server = Server::new(Default::default());
        let (send_results, results) = channel();

        // [:find ?v :where [1 :name ?n]]
        let plan = Plan::MatchEA(1, ":name".to_string(), 1);

        worker.dataflow::<u64, _, _>(|scope| {
            server.create_input(":name".to_string(), scope);

            let query_name = "match_ea";
            server.register(
                Register {
                    rules: vec![Rule {
                        name: query_name.to_string(),
                        plan: plan,
                    }],
                    publish: vec![query_name.to_string()],
                },
                scope,
            );

            server
                .interest(query_name.to_string(), scope)
                .inspect(move |x| {
                    send_results.send((x.0.clone(), x.2)).unwrap();
                });
        });

        server.transact(
            Transact {
                tx: Some(0),
                tx_data: vec![
                    TxData(
                        1,
                        1,
                        ":name".to_string(),
                        Value::String("Dipper".to_string()),
                    ),
                    TxData(
                        1,
                        1,
                        ":name".to_string(),
                        Value::String("Alias".to_string()),
                    ),
                    TxData(
                        1,
                        2,
                        ":name".to_string(),
                        Value::String("Mabel".to_string()),
                    ),
                ],
            },
            0,
            0,
        );

        worker.step_while(|| server.is_any_outdated());

        thread::spawn(move || {
            assert_eq!(
                results.recv().unwrap(),
                (vec![Value::String("Alias".to_string())], 1)
            );
            assert_eq!(
                results.recv().unwrap(),
                (vec![Value::String("Dipper".to_string())], 1)
            );
        }).join()
            .unwrap();
    }).unwrap();
}

#[test]
fn join() {
    timely::execute(Configuration::Thread, move |worker| {
        let mut server = Server::new(Default::default());
        let (send_results, results) = channel();

        // [:find ?e ?n ?a :where [?e :age ?a] [?e :name ?n]]
        let (e, a, n) = (1, 2, 3);
        let plan = Plan::Project(Project {
            variables: vec![e, n, a],
            plan: Box::new(Plan::Join(Join {
                variables: vec![e],
                left_plan: Box::new(Plan::MatchA(e, ":name".to_string(), n)),
                right_plan: Box::new(Plan::MatchA(e, ":age".to_string(), a)),
            })),
        });

        worker.dataflow::<u64, _, _>(|mut scope| {
            server.create_input(":name".to_string(), scope);
            server.create_input(":age".to_string(), scope);

            let query_name = "join";
            server.register(
                Register {
                    rules: vec![Rule {
                        name: query_name.to_string(),
                        plan: plan,
                    }],
                    publish: vec![query_name.to_string()],
                },
                &mut scope,
            );

            server
                .interest(query_name.to_string(), &mut scope)
                .inspect(move |x| {
                    send_results.send((x.0.clone(), x.2)).unwrap();
                });
        });

        server.transact(
            Transact {
                tx: Some(0),
                tx_data: vec![
                    TxData(
                        1,
                        1,
                        ":name".to_string(),
                        Value::String("Dipper".to_string()),
                    ),
                    TxData(1, 1, ":age".to_string(), Value::Number(12)),
                ],
            },
            0,
            0,
        );

        worker.step_while(|| server.is_any_outdated());

        thread::spawn(move || {
            assert_eq!(
                results.recv().unwrap(),
                (
                    vec![
                        Value::Eid(1),
                        Value::String("Dipper".to_string()),
                        Value::Number(12),
                    ],
                    1
                )
            );
        }).join()
            .unwrap();
    }).unwrap();
}

#[test]
fn hector() {
    timely::execute(Configuration::Thread, move |worker| {
        let mut server = Server::new(Default::default());
        // let (send_results, results) = channel();

        // [?a :edge ?b] [?b :edge ?c] [?a :edge ?c]
        let (a,b,c) = (1,2,3);
        let plan = Plan::Hector(Hector {
            bindings: vec![
                Binding { symbols: (a,b), source_name: "edge".to_string() },
                Binding { symbols: (b,c), source_name: "edge".to_string() },
                Binding { symbols: (c,a), source_name: "edge".to_string() },
            ]
        });

        worker.dataflow::<u64, _, _>(|mut scope| {
            server.create_input("edge".to_string(), scope);

            let query_name = "hector";
            server.register(
                Register {
                    rules: vec![Rule {
                        name: query_name.to_string(),
                        plan: plan,
                    }],
                    publish: vec![query_name.to_string()],
                },
                &mut scope,
            );

            server
                .interest(query_name.to_string(), &mut scope)
                .inspect(move |x| {
                    println!("{:?}", x);
                    // send_results.send((x.0.clone(), x.2)).unwrap();
                });
        });

        server.transact(
            Transact {
                tx: Some(0),
                tx_data: vec![
                    TxData(1, 100, "edge".to_string(), Value::Eid(200)),
                    TxData(1, 200, "edge".to_string(), Value::Eid(300)),
                    TxData(1, 100, "edge".to_string(), Value::Eid(300)),
                    TxData(1, 100, "edge".to_string(), Value::Eid(400)),
                    TxData(1, 400, "edge".to_string(), Value::Eid(500)),
                    TxData(1, 500, "edge".to_string(), Value::Eid(100)),
                ],
            },
            0,
            0,
        );

        worker.step_while(|| server.is_any_outdated());

        // thread::spawn(move || {
        //     // assert_eq!(results.recv().unwrap(), (vec![], 1));
        // }).join()
        //     .unwrap();
    }).unwrap();
}

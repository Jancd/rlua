mod property_support;

use proptest::prop_assert_eq;
use proptest::test_runner::TestCaseError;

use property_support::{
    deterministic_runner, execute_program, expr_pair_strategy, expr_strategy, locals_program,
    return_program, snapshot_proto, value_as_i64,
};

#[test]
fn parser_generated_return_programs_are_stable() {
    let mut runner = deterministic_runner();
    runner
        .run(&expr_strategy(), |expr| {
            let source = return_program(&expr);
            let first = rlua_parser::parse(&source).map_err(|error| {
                TestCaseError::fail(format!(
                    "parser rejected generated source {source:?}: {error:?}"
                ))
            })?;
            let second = rlua_parser::parse(&source).map_err(|error| {
                TestCaseError::fail(format!(
                    "parser rejected generated source {source:?}: {error:?}"
                ))
            })?;
            prop_assert_eq!(first, second);
            Ok(())
        })
        .unwrap();
}

#[test]
fn compiler_generated_programs_are_deterministic() {
    let mut runner = deterministic_runner();
    runner
        .run(&expr_pair_strategy(), |(left, right)| {
            let (source, _) = locals_program(&left, &right);
            let first = rlua_compiler::compile_source(&source).map_err(|error| {
                TestCaseError::fail(format!(
                    "compiler rejected generated source {source:?}: {error}"
                ))
            })?;
            let second = rlua_compiler::compile_source(&source).map_err(|error| {
                TestCaseError::fail(format!(
                    "compiler rejected generated source {source:?}: {error}"
                ))
            })?;
            prop_assert_eq!(snapshot_proto(&first), snapshot_proto(&second));
            Ok(())
        })
        .unwrap();
}

#[test]
fn runtime_generated_local_programs_match_rust_model() {
    let mut runner = deterministic_runner();
    runner
        .run(&expr_pair_strategy(), |(left, right)| {
            let (source, expected) = locals_program(&left, &right);
            let results = execute_program(&source).map_err(TestCaseError::fail)?;
            prop_assert_eq!(results.len(), 3);
            prop_assert_eq!(value_as_i64(&results[0]), Some(expected.sum));
            prop_assert_eq!(value_as_i64(&results[1]), Some(expected.diff));
            prop_assert_eq!(value_as_i64(&results[2]), Some(expected.product));
            Ok(())
        })
        .unwrap();
}
